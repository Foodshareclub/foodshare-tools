use thiserror::Error;

pub type Result<T> = std::result::Result<T, SwiftError>;

#[derive(Error, Debug)]
pub enum SwiftError {
    #[error("Swift not found in PATH")]
    SwiftNotFound,

    #[error("Swift version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },

    #[error("Toolchain not found at path: {0}")]
    ToolchainNotFound(String),

    #[error("Invalid Package.swift file: {0}")]
    InvalidPackageFile(String),

    #[error("Failed to parse Swift version: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Regex error: {0}")]
    Regex(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("Configuration error: {0}")]
    Config(String),
}
