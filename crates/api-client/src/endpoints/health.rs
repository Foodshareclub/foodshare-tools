//! Health check endpoints

use crate::client::FoodshareClient;
use crate::error::ApiResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Health check API interface
#[derive(Clone)]
pub struct HealthApi {
    client: FoodshareClient,
}

impl HealthApi {
    /// Create a new health API interface
    pub(crate) fn new(client: FoodshareClient) -> Self {
        Self { client }
    }

    /// Check health of the get-translations endpoint
    pub async fn check(&self) -> ApiResult<HealthResponse> {
        self.client.get("get-translations/health").await
    }

    /// Check health with timing information
    pub async fn check_timed(&self) -> ApiResult<(HealthResponse, Duration)> {
        self.client.timed_get("get-translations/health").await
    }

    /// Check if a specific endpoint is reachable
    pub async fn check_endpoint(&self, url: &str) -> ApiResult<EndpointStatus> {
        let start = std::time::Instant::now();

        let request = self
            .client
            .request_builder(reqwest::Method::GET, url);

        let response = self.client.execute_raw(request).await?;
        let elapsed = start.elapsed();

        Ok(EndpointStatus {
            url: url.to_string(),
            status_code: response.status().as_u16(),
            response_time: elapsed,
            is_healthy: response.status().is_success(),
        })
    }
}

/// Health check response from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Health status (e.g., "healthy", "ok")
    pub status: String,
    /// API version
    pub version: String,
    /// Timestamp of the health check
    pub timestamp: String,
    /// Optional feature flags
    pub features: Option<HealthFeatures>,
}

/// Feature flags in health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthFeatures {
    /// Delta sync support
    #[serde(rename = "deltaSync")]
    pub delta_sync: Option<bool>,
    /// Prefetch support
    pub prefetch: Option<bool>,
}

/// Endpoint status information
#[derive(Debug, Clone, Serialize)]
pub struct EndpointStatus {
    /// URL that was checked
    pub url: String,
    /// HTTP status code
    pub status_code: u16,
    /// Response time
    pub response_time: Duration,
    /// Whether the endpoint is healthy
    pub is_healthy: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_deserialize() {
        let json = r#"{
            "status": "healthy",
            "version": "1.0.0",
            "timestamp": "2024-01-01T00:00:00Z",
            "features": {
                "deltaSync": true,
                "prefetch": false
            }
        }"#;

        let response: HealthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "healthy");
        assert_eq!(response.version, "1.0.0");
        assert!(response.features.is_some());
        assert_eq!(response.features.unwrap().delta_sync, Some(true));
    }
}
