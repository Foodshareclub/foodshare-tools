//! Error types for the image crate.

use thiserror::Error;

/// Result type alias for image operations.
pub type Result<T> = std::result::Result<T, ImageError>;

/// Errors that can occur during image operations.
#[derive(Debug, Error)]
pub enum ImageError {
    /// Unknown image format
    #[error("Unknown image format")]
    UnknownFormat,

    /// Invalid image data
    #[error("Invalid image data: {0}")]
    InvalidData(String),

    /// Resize error
    #[error("Resize error: {0}")]
    ResizeError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Image processing error
    #[cfg(feature = "processing")]
    #[error("Image processing error: {0}")]
    ProcessingError(#[from] image::ImageError),
}
