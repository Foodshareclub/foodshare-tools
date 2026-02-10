//! Error types for the migration tool

use std::path::PathBuf;

/// Errors that can occur during migration operations
#[derive(Debug, thiserror::Error)]
pub enum MigrateError {
    /// Failed to read or parse an env file
    #[error("env file error: {message}")]
    EnvFile {
        /// Description of what went wrong
        message: String,
        /// Path to the file, if applicable
        path: Option<PathBuf>,
    },

    /// Failed to interact with the Supabase vault
    #[error("vault error: {0}")]
    Vault(String),

    /// Docker is not available or container not running
    #[error("docker error: {0}")]
    Docker(String),

    /// Verification of a PG function failed
    #[error("verification failed: {0}")]
    Verification(String),
}

impl MigrateError {
    /// Create an env file error
    pub fn env_file(message: impl Into<String>) -> Self {
        Self::EnvFile {
            message: message.into(),
            path: None,
        }
    }

    /// Create an env file error with a path
    pub fn env_file_at(message: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::EnvFile {
            message: message.into(),
            path: Some(path.into()),
        }
    }

    /// Create a vault error
    pub fn vault(message: impl Into<String>) -> Self {
        Self::Vault(message.into())
    }

    /// Create a docker error
    pub fn docker(message: impl Into<String>) -> Self {
        Self::Docker(message.into())
    }

    /// Create a verification error
    pub fn verification(message: impl Into<String>) -> Self {
        Self::Verification(message.into())
    }
}

impl From<std::io::Error> for MigrateError {
    fn from(err: std::io::Error) -> Self {
        Self::EnvFile {
            message: err.to_string(),
            path: None,
        }
    }
}

/// Result type alias for migration operations
pub type Result<T> = std::result::Result<T, MigrateError>;
