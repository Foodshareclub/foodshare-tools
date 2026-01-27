//! Error types for the API client

use std::fmt;
use thiserror::Error;

/// Result type alias for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// API client errors
#[derive(Error, Debug)]
pub enum ApiError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// JSON serialization/deserialization failed
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Missing environment variable
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    /// API returned an error response
    #[error("API error ({status}): {message}")]
    ApiResponse {
        /// HTTP status code
        status: u16,
        /// Error message from API
        message: String,
    },

    /// Circuit breaker is open
    #[error("Circuit breaker is open - service temporarily unavailable")]
    CircuitOpen,

    /// Rate limited
    #[error("Rate limited - too many requests")]
    RateLimited,

    /// Request timeout
    #[error("Request timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// All retry attempts exhausted
    #[error("All {attempts} retry attempts failed: {last_error}")]
    RetriesExhausted {
        /// Number of attempts made
        attempts: u32,
        /// Last error message
        last_error: String,
    },

    /// Invalid URL
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

impl ApiError {
    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a missing env var error
    pub fn missing_env(var: impl Into<String>) -> Self {
        Self::MissingEnvVar(var.into())
    }

    /// Create an API response error
    pub fn api_response(status: u16, message: impl Into<String>) -> Self {
        Self::ApiResponse {
            status,
            message: message.into(),
        }
    }

    /// Check if this error is retryable
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Request(e) => {
                // Retry on connection errors, timeouts
                e.is_connect() || e.is_timeout()
            }
            Self::ApiResponse { status, .. } => {
                // Retry on 5xx errors and 429 (rate limited)
                *status >= 500 || *status == 429
            }
            Self::Timeout(_) => true,
            Self::CircuitOpen | Self::RateLimited => false,
            Self::Config(_)
            | Self::MissingEnvVar(_)
            | Self::Json(_)
            | Self::InvalidUrl(_)
            | Self::RetriesExhausted { .. } => false,
        }
    }

    /// Check if this is a client error (4xx)
    #[must_use]
    pub fn is_client_error(&self) -> bool {
        matches!(self, Self::ApiResponse { status, .. } if (400..500).contains(status))
    }

    /// Check if this is a server error (5xx)
    #[must_use]
    pub fn is_server_error(&self) -> bool {
        matches!(self, Self::ApiResponse { status, .. } if *status >= 500)
    }
}

/// Error context for better debugging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Request ID for correlation
    pub request_id: Option<String>,
    /// Endpoint that was called
    pub endpoint: String,
    /// HTTP method used
    pub method: String,
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.method, self.endpoint)?;
        if let Some(ref id) = self.request_id {
            write!(f, " (request_id: {id})")?;
        }
        Ok(())
    }
}
