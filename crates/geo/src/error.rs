//! Error types for the geo crate.

use thiserror::Error;

/// Result type alias for geo operations.
pub type Result<T> = std::result::Result<T, GeoError>;

/// Errors that can occur during geo operations.
#[derive(Debug, Error)]
pub enum GeoError {
    /// Invalid WKT format
    #[error("Invalid WKT format: {0}")]
    InvalidWkt(String),

    /// Invalid coordinate values
    #[error("Invalid coordinate: {0}")]
    InvalidCoordinate(String),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Error code for integration with foodshare-core error handling.
/// Range: 10xxx for geo errors.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoErrorCode {
    /// Invalid WKT format
    InvalidWkt = 10001,
    /// Invalid coordinate values
    InvalidCoordinate = 10002,
    /// JSON parsing error
    JsonParsing = 10003,
}

impl GeoError {
    /// Returns the error code for this error.
    pub fn code(&self) -> GeoErrorCode {
        match self {
            GeoError::InvalidWkt(_) => GeoErrorCode::InvalidWkt,
            GeoError::InvalidCoordinate(_) => GeoErrorCode::InvalidCoordinate,
            GeoError::JsonError(_) => GeoErrorCode::JsonParsing,
        }
    }
}
