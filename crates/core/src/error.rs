//! Error types for the core crate

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Process error: {0}")]
    Process(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Exit codes for CLI commands
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const FAILURE: i32 = 1;
    pub const VALIDATION_ERROR: i32 = 2;
    pub const CONFIG_ERROR: i32 = 3;
}
