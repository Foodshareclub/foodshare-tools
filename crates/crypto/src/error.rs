//! Error types for the crypto crate.

use thiserror::Error;

/// Result type alias for crypto operations.
pub type Result<T> = std::result::Result<T, CryptoError>;

/// Errors that can occur during crypto operations.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Invalid signature format
    #[error("Invalid signature format: {0}")]
    InvalidSignature(String),

    /// Signature verification failed
    #[error("Signature mismatch")]
    SignatureMismatch,

    /// Invalid key
    #[error("Invalid key: {0}")]
    InvalidKey(String),

    /// Encoding error
    #[error("Encoding error: {0}")]
    EncodingError(String),
}
