//! Main API client implementation

use crate::config::ClientConfig;
use crate::endpoints::{BffApi, HealthApi, LocalizationApi, ProductsApi, TranslationsApi};
use crate::error::{ApiError, ApiResult};
use foodshare_core::rate_limit::RateLimiter;
use foodshare_core::retry::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, instrument, warn};
use uuid::Uuid;

/// Request correlation ID header
const X_REQUEST_ID: &str = "X-Request-ID";

/// API key header for Supabase
const APIKEY_HEADER: &str = "apikey";

/// Foodshare API client with built-in resilience patterns
///
/// This client wraps `reqwest` and adds:
/// - Automatic retry with exponential backoff
/// - Circuit breaker to prevent cascading failures
/// - Rate limiting to avoid throttling
/// - Request correlation IDs for tracing
#[derive(Clone)]
pub struct FoodshareClient {
    inner: Client,
    config: Arc<ClientConfig>,
    circuit_breaker: Arc<CircuitBreaker>,
    rate_limiter: Arc<RateLimiter>,
}

impl FoodshareClient {
    /// Create a new client with default configuration from environment
    pub fn new() -> ApiResult<Self> {
        let config = ClientConfig::from_env()?;
        Self::with_config(config)
    }

    /// Create a new client with specific configuration
    pub fn with_config(config: ClientConfig) -> ApiResult<Self> {
        config.validate()?;

        let mut default_headers = HeaderMap::new();
        default_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        default_headers.insert(
            USER_AGENT,
            HeaderValue::from_static("foodshare-api-client/1.0"),
        );

        // Add API key header if available
        if let Some(ref key) = config.anon_key {
            if let Ok(value) = HeaderValue::from_str(key) {
                default_headers.insert(APIKEY_HEADER, value);
            }
        }

        let inner = Client::builder()
            .timeout(config.timeout)
            .default_headers(default_headers)
            .build()
            .map_err(ApiError::Request)?;

        let circuit_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));

        Ok(Self {
            inner,
            config: Arc::new(config),
            circuit_breaker,
            rate_limiter,
        })
    }

    /// Get the current configuration
    #[must_use]
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Get the base URL
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Get the BFF URL
    #[must_use]
    pub fn bff_url(&self) -> &str {
        &self.config.bff_url
    }

    /// Get circuit breaker state
    #[must_use]
    pub fn circuit_state(&self) -> CircuitState {
        self.circuit_breaker.state()
    }

    /// Reset the circuit breaker
    pub fn reset_circuit(&self) {
        self.circuit_breaker.reset();
    }

    /// Reset rate limits for a specific endpoint
    pub fn reset_rate_limit(&self, endpoint: &str) {
        self.rate_limiter.reset(endpoint);
    }

    // -------------------------------------------------------------------------
    // Endpoint API accessors
    // -------------------------------------------------------------------------

    /// Access translation endpoints
    #[must_use]
    pub fn translations(&self) -> TranslationsApi {
        TranslationsApi::new(self.clone())
    }

    /// Access health check endpoints
    #[must_use]
    pub fn health(&self) -> HealthApi {
        HealthApi::new(self.clone())
    }

    /// Access BFF endpoints
    #[must_use]
    pub fn bff(&self) -> BffApi {
        BffApi::new(self.clone())
    }

    /// Access localization endpoints (consolidated service)
    #[must_use]
    pub fn localization(&self) -> LocalizationApi {
        LocalizationApi::new(self.clone())
    }

    /// Access products/listings endpoints
    #[must_use]
    pub fn products(&self) -> ProductsApi {
        ProductsApi::new(self.clone())
    }

    // -------------------------------------------------------------------------
    // Low-level HTTP methods with resilience
    // -------------------------------------------------------------------------

    /// Perform a GET request with resilience patterns
    #[instrument(skip(self), fields(request_id))]
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> ApiResult<T> {
        self.request(Method::GET, path, Option::<&()>::None).await
    }

    /// Perform a GET request to an absolute URL
    #[instrument(skip(self), fields(request_id))]
    pub async fn get_url<T: DeserializeOwned>(&self, url: &str) -> ApiResult<T> {
        self.request_url(Method::GET, url, Option::<&()>::None)
            .await
    }

    /// Perform a POST request with resilience patterns
    #[instrument(skip(self, body), fields(request_id))]
    pub async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> ApiResult<T> {
        self.request(Method::POST, path, Some(body)).await
    }

    /// Perform a POST request to an absolute URL
    #[instrument(skip(self, body), fields(request_id))]
    pub async fn post_url<T: DeserializeOwned, B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> ApiResult<T> {
        self.request_url(Method::POST, url, Some(body)).await
    }

    /// Build a request builder for custom requests
    pub fn request_builder(&self, method: Method, path: &str) -> RequestBuilder {
        let url = format!("{}/{}", self.config.base_url.trim_end_matches('/'), path);
        let request_id = Uuid::new_v4().to_string();

        self.inner
            .request(method, &url)
            .header(X_REQUEST_ID, &request_id)
    }

    /// Execute a request with full resilience patterns
    async fn request<T: DeserializeOwned, B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> ApiResult<T> {
        let url = format!("{}/{}", self.config.base_url.trim_end_matches('/'), path);
        self.request_url(method, &url, body).await
    }

    /// Execute a request to an absolute URL with full resilience patterns
    async fn request_url<T: DeserializeOwned, B: Serialize>(
        &self,
        method: Method,
        url: &str,
        body: Option<&B>,
    ) -> ApiResult<T> {
        let request_id = Uuid::new_v4().to_string();
        let rate_limit_key = extract_rate_limit_key(url);

        // Check circuit breaker
        if !self.circuit_breaker.can_execute() {
            warn!(
                request_id = %request_id,
                url = %url,
                "Circuit breaker is open, rejecting request"
            );
            return Err(ApiError::CircuitOpen);
        }

        // Check rate limiter
        if !self.rate_limiter.try_acquire(&rate_limit_key) {
            warn!(
                request_id = %request_id,
                url = %url,
                "Rate limited"
            );
            return Err(ApiError::RateLimited);
        }

        // Execute with retry
        self.execute_with_retry(&request_id, method, url, body)
            .await
    }

    /// Execute request with retry logic
    async fn execute_with_retry<T: DeserializeOwned, B: Serialize>(
        &self,
        request_id: &str,
        method: Method,
        url: &str,
        body: Option<&B>,
    ) -> ApiResult<T> {
        let retry_config = &self.config.retry;
        let mut last_error: Option<ApiError> = None;

        for attempt in 0..retry_config.max_attempts {
            // Wait before retry (except first attempt)
            if attempt > 0 {
                let delay = retry_config.delay_for_attempt(attempt);
                debug!(
                    request_id = %request_id,
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying after delay"
                );
                tokio::time::sleep(delay).await;
            }

            let start = Instant::now();
            let result = self
                .execute_single_request(request_id, method.clone(), url, body)
                .await;
            let elapsed = start.elapsed();

            match result {
                Ok(value) => {
                    self.circuit_breaker.record_success();
                    debug!(
                        request_id = %request_id,
                        attempt = attempt + 1,
                        elapsed_ms = elapsed.as_millis(),
                        "Request succeeded"
                    );
                    return Ok(value);
                }
                Err(e) => {
                    self.circuit_breaker.record_failure();

                    if e.is_retryable() && attempt + 1 < retry_config.max_attempts {
                        debug!(
                            request_id = %request_id,
                            attempt = attempt + 1,
                            error = %e,
                            "Request failed, will retry"
                        );
                        last_error = Some(e);
                    } else {
                        debug!(
                            request_id = %request_id,
                            attempt = attempt + 1,
                            error = %e,
                            "Request failed, not retrying"
                        );
                        return Err(e);
                    }
                }
            }
        }

        Err(ApiError::RetriesExhausted {
            attempts: retry_config.max_attempts,
            last_error: last_error.map_or_else(|| "Unknown error".to_string(), |e| e.to_string()),
        })
    }

    /// Execute a single request without retry
    async fn execute_single_request<T: DeserializeOwned, B: Serialize>(
        &self,
        request_id: &str,
        method: Method,
        url: &str,
        body: Option<&B>,
    ) -> ApiResult<T> {
        let mut request = self
            .inner
            .request(method, url)
            .header(X_REQUEST_ID, request_id);

        // Add auth header if service role key is set
        if let Some(ref key) = self.config.service_role_key {
            request = request.header(AUTHORIZATION, format!("Bearer {key}"));
        }

        if let Some(b) = body {
            request = request.json(b);
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    /// Handle HTTP response and deserialize
    async fn handle_response<T: DeserializeOwned>(&self, response: Response) -> ApiResult<T> {
        let status = response.status();

        if status.is_success() {
            response.json().await.map_err(ApiError::Request)
        } else {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(ApiError::api_response(status.as_u16(), message))
        }
    }

    /// Execute a raw request and return the response (for ETag/conditional requests)
    pub async fn execute_raw(&self, request: RequestBuilder) -> ApiResult<Response> {
        let request_id = Uuid::new_v4().to_string();

        // Check circuit breaker
        if !self.circuit_breaker.can_execute() {
            return Err(ApiError::CircuitOpen);
        }

        let response = request.header(X_REQUEST_ID, &request_id).send().await?;

        if response.status().is_success() || response.status().as_u16() == 304 {
            self.circuit_breaker.record_success();
        } else {
            self.circuit_breaker.record_failure();
        }

        Ok(response)
    }

    /// Get duration timing for a request
    pub async fn timed_get<T: DeserializeOwned>(&self, path: &str) -> ApiResult<(T, Duration)> {
        let start = Instant::now();
        let result = self.get(path).await?;
        Ok((result, start.elapsed()))
    }

    /// Get duration timing for a URL request
    pub async fn timed_get_url<T: DeserializeOwned>(
        &self,
        url: &str,
    ) -> ApiResult<(T, Duration)> {
        let start = Instant::now();
        let result = self.get_url(url).await?;
        Ok((result, start.elapsed()))
    }
}

/// Extract a rate limit key from a URL (uses the path)
fn extract_rate_limit_key(url: &str) -> String {
    url.split('?')
        .next()
        .and_then(|s| s.split("://").nth(1))
        .and_then(|s| s.split('/').skip(1).next())
        .unwrap_or("default")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rate_limit_key() {
        assert_eq!(
            extract_rate_limit_key("https://example.com/api/v1/endpoint?foo=bar"),
            "api"
        );
        assert_eq!(
            extract_rate_limit_key("http://localhost:8080/health"),
            "health"
        );
    }

    #[test]
    fn test_client_creation() {
        let config = ClientConfig::development();
        let client = FoodshareClient::with_config(config);
        assert!(client.is_ok());
    }
}
