//! Error types for the compression crate.

use thiserror::Error;

/// Result type alias for compression operations.
pub type Result<T> = std::result::Result<T, CompressionError>;

/// Errors that can occur during compression operations.
#[derive(Debug, Error)]
pub enum CompressionError {
    /// Compression failed
    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    /// Decompression failed
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
