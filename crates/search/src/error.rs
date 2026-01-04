//! Error types for the search crate.

use thiserror::Error;

/// Result type alias for search operations.
pub type Result<T> = std::result::Result<T, SearchError>;

/// Errors that can occur during search operations.
#[derive(Debug, Error)]
pub enum SearchError {
    /// Invalid query
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Index error
    #[error("Index error: {0}")]
    IndexError(String),
}
