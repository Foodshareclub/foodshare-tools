//! Configuration for the Foodshare API client
//!
//! Supports environment-based configuration with sensible defaults.

use crate::error::{ApiError, ApiResult};
use foodshare_core::rate_limit::RateLimitConfig;
use foodshare_core::retry::RetryConfig;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

/// Default production Supabase URL
const DEFAULT_SUPABASE_URL: &str = "https://iazmjdjwnkilycbjwpzp.supabase.co/functions/v1";

/// Environment types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Local development (typically localhost Supabase)
    Development,
    /// Staging environment
    Staging,
    /// Production environment
    Production,
}

impl Default for Environment {
    fn default() -> Self {
        Self::Production
    }
}

impl Environment {
    /// Parse from environment variable
    pub fn from_env() -> Self {
        match env::var("FOODSHARE_ENV")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "development" | "dev" | "local" => Self::Development,
            "staging" | "stage" => Self::Staging,
            _ => Self::Production,
        }
    }
}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Base URL for Supabase Edge Functions
    pub base_url: String,
    /// BFF endpoint URL (derived from base_url if not set)
    pub bff_url: String,
    /// Supabase anonymous key (for public endpoints)
    pub anon_key: Option<String>,
    /// Supabase service role key (for admin endpoints)
    pub service_role_key: Option<String>,
    /// Request timeout
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    /// Retry configuration
    pub retry: RetryConfig,
    /// Rate limit configuration
    pub rate_limit: RateLimitConfig,
    /// Current environment
    pub environment: Environment,
}

mod humantime_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_SUPABASE_URL.to_string(),
            bff_url: format!("{DEFAULT_SUPABASE_URL}/bff"),
            anon_key: None,
            service_role_key: None,
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            rate_limit: RateLimitConfig::per_minute(100),
            environment: Environment::default(),
        }
    }
}

impl ClientConfig {
    /// Create configuration from environment variables
    ///
    /// Reads the following environment variables:
    /// - `FOODSHARE_API_URL` or `SUPABASE_URL`: Base URL for Edge Functions
    /// - `FOODSHARE_BFF_URL`: BFF endpoint URL (optional, derived from base URL)
    /// - `SUPABASE_ANON_KEY`: Anonymous key for public endpoints
    /// - `SUPABASE_SERVICE_ROLE_KEY`: Service role key for admin endpoints
    /// - `FOODSHARE_ENV`: Environment (development/staging/production)
    /// - `FOODSHARE_TIMEOUT_SECS`: Request timeout in seconds
    pub fn from_env() -> ApiResult<Self> {
        let environment = Environment::from_env();

        // Try FOODSHARE_API_URL first, then SUPABASE_URL, then default
        let base_url = env::var("FOODSHARE_API_URL")
            .or_else(|_| env::var("SUPABASE_URL").map(|url| format!("{url}/functions/v1")))
            .unwrap_or_else(|_| DEFAULT_SUPABASE_URL.to_string());

        // BFF URL defaults to base_url + /bff
        let bff_url =
            env::var("FOODSHARE_BFF_URL").unwrap_or_else(|_| format!("{base_url}/bff"));

        let anon_key = env::var("SUPABASE_ANON_KEY").ok();
        let service_role_key = env::var("SUPABASE_SERVICE_ROLE_KEY").ok();

        let timeout = env::var("FOODSHARE_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(30));

        // Adjust retry config based on environment
        let retry = match environment {
            Environment::Development => RetryConfig::quick(),
            Environment::Staging => RetryConfig::default(),
            Environment::Production => RetryConfig::patient(),
        };

        // Adjust rate limits based on environment
        let rate_limit = match environment {
            Environment::Development => RateLimitConfig::per_minute(1000), // More lenient locally
            Environment::Staging => RateLimitConfig::per_minute(200),
            Environment::Production => RateLimitConfig::per_minute(100),
        };

        Ok(Self {
            base_url,
            bff_url,
            anon_key,
            service_role_key,
            timeout,
            retry,
            rate_limit,
            environment,
        })
    }

    /// Create development configuration (local Supabase)
    #[must_use]
    pub fn development() -> Self {
        Self {
            base_url: "http://localhost:54321/functions/v1".to_string(),
            bff_url: "http://localhost:54321/functions/v1/bff".to_string(),
            anon_key: env::var("SUPABASE_ANON_KEY").ok(),
            service_role_key: env::var("SUPABASE_SERVICE_ROLE_KEY").ok(),
            timeout: Duration::from_secs(10),
            retry: RetryConfig::quick(),
            rate_limit: RateLimitConfig::per_minute(1000),
            environment: Environment::Development,
        }
    }

    /// Create staging configuration
    #[must_use]
    pub fn staging() -> Self {
        Self {
            base_url: env::var("STAGING_SUPABASE_URL")
                .map(|url| format!("{url}/functions/v1"))
                .unwrap_or_else(|_| DEFAULT_SUPABASE_URL.to_string()),
            bff_url: env::var("STAGING_BFF_URL")
                .unwrap_or_else(|_| format!("{DEFAULT_SUPABASE_URL}/bff")),
            anon_key: env::var("STAGING_SUPABASE_ANON_KEY")
                .or_else(|_| env::var("SUPABASE_ANON_KEY"))
                .ok(),
            service_role_key: env::var("STAGING_SUPABASE_SERVICE_ROLE_KEY")
                .or_else(|_| env::var("SUPABASE_SERVICE_ROLE_KEY"))
                .ok(),
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            rate_limit: RateLimitConfig::per_minute(200),
            environment: Environment::Staging,
        }
    }

    /// Create production configuration
    #[must_use]
    pub fn production() -> Self {
        Self {
            base_url: DEFAULT_SUPABASE_URL.to_string(),
            bff_url: format!("{DEFAULT_SUPABASE_URL}/bff"),
            anon_key: env::var("SUPABASE_ANON_KEY").ok(),
            service_role_key: env::var("SUPABASE_SERVICE_ROLE_KEY").ok(),
            timeout: Duration::from_secs(30),
            retry: RetryConfig::patient(),
            rate_limit: RateLimitConfig::per_minute(100),
            environment: Environment::Production,
        }
    }

    /// Builder-style method to set base URL
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        let url = url.into();
        self.bff_url = format!("{url}/bff");
        self.base_url = url;
        self
    }

    /// Builder-style method to set BFF URL
    #[must_use]
    pub fn with_bff_url(mut self, url: impl Into<String>) -> Self {
        self.bff_url = url.into();
        self
    }

    /// Builder-style method to set anon key
    #[must_use]
    pub fn with_anon_key(mut self, key: impl Into<String>) -> Self {
        self.anon_key = Some(key.into());
        self
    }

    /// Builder-style method to set service role key
    #[must_use]
    pub fn with_service_role_key(mut self, key: impl Into<String>) -> Self {
        self.service_role_key = Some(key.into());
        self
    }

    /// Builder-style method to set timeout
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Builder-style method to set retry config
    #[must_use]
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Builder-style method to set rate limit config
    #[must_use]
    pub fn with_rate_limit(mut self, rate_limit: RateLimitConfig) -> Self {
        self.rate_limit = rate_limit;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> ApiResult<()> {
        if self.base_url.is_empty() {
            return Err(ApiError::config("base_url cannot be empty"));
        }

        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(ApiError::config("base_url must start with http:// or https://"));
        }

        if self.timeout.is_zero() {
            return Err(ApiError::config("timeout cannot be zero"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClientConfig::default();
        assert!(config.base_url.contains("supabase.co"));
        assert!(config.bff_url.ends_with("/bff"));
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_development_config() {
        let config = ClientConfig::development();
        assert!(config.base_url.contains("localhost"));
        assert_eq!(config.environment, Environment::Development);
    }

    #[test]
    fn test_builder_pattern() {
        let config = ClientConfig::default()
            .with_base_url("https://test.supabase.co/functions/v1")
            .with_timeout(Duration::from_secs(60));

        assert_eq!(
            config.base_url,
            "https://test.supabase.co/functions/v1"
        );
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_validation() {
        let valid = ClientConfig::default();
        assert!(valid.validate().is_ok());

        let invalid = ClientConfig::default().with_base_url("");
        assert!(invalid.validate().is_err());
    }
}
