//! Enterprise-grade error handling with context and recovery suggestions
//!
//! This module provides structured error types with:
//! - Detailed error context
//! - Recovery suggestions
//! - Error codes for programmatic handling
//! - Serializable error reports

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Error codes for programmatic error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // General errors (1xxx)
    /// Unknown or unclassified error
    Unknown = 1000,
    /// Internal error in the tool
    Internal = 1001,
    /// Feature not yet implemented
    NotImplemented = 1002,
    /// Operation timed out
    Timeout = 1003,

    // IO errors (2xxx)
    /// General I/O error
    IoError = 2000,
    /// File not found
    FileNotFound = 2001,
    /// Permission denied
    PermissionDenied = 2002,
    /// Invalid file path
    InvalidPath = 2003,
    /// Directory not found
    DirectoryNotFound = 2004,

    // Configuration errors (3xxx)
    /// General configuration error
    ConfigError = 3000,
    /// Configuration file not found
    ConfigNotFound = 3001,
    /// Failed to parse configuration
    ConfigParseError = 3002,
    /// Configuration validation failed
    ConfigValidationError = 3003,
    /// Invalid configuration value
    InvalidConfigValue = 3004,

    // Git errors (4xxx)
    /// General git error
    GitError = 4000,
    /// Not a git repository
    NotAGitRepo = 4001,
    /// Git command failed
    GitCommandFailed = 4002,
    /// No staged files found
    NoStagedFiles = 4003,
    /// Branch not found
    BranchNotFound = 4004,
    /// Merge conflict detected
    MergeConflict = 4005,

    // Process errors (5xxx)
    /// General process error
    ProcessError = 5000,
    /// Command not found
    CommandNotFound = 5001,
    /// Command execution failed
    CommandFailed = 5002,
    /// Process timed out
    ProcessTimeout = 5003,

    // Validation errors (6xxx)
    /// General validation error
    ValidationError = 6000,
    /// Invalid input provided
    InvalidInput = 6001,
    /// Invalid format
    InvalidFormat = 6002,
    /// Constraint violation
    ConstraintViolation = 6003,

    // Security errors (7xxx)
    /// General security error
    SecurityError = 7000,
    /// Secret/credential detected in code
    SecretDetected = 7001,
    /// Unauthorized access attempt
    UnauthorizedAccess = 7002,

    // Platform-specific errors (8xxx)
    /// General platform error
    PlatformError = 8000,
    /// Xcode-related error
    XcodeError = 8001,
    /// Gradle-related error
    GradleError = 8002,
    /// Swift-related error
    SwiftError = 8003,
    /// Kotlin-related error
    KotlinError = 8004,
}

impl ErrorCode {
    /// Get the numeric code
    #[must_use] pub fn code(&self) -> u32 {
        *self as u32
    }

    /// Get a human-readable category
    #[must_use] pub fn category(&self) -> &'static str {
        match self.code() / 1000 {
            1 => "General",
            2 => "IO",
            3 => "Configuration",
            4 => "Git",
            5 => "Process",
            6 => "Validation",
            7 => "Security",
            8 => "Platform",
            _ => "Unknown",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{:04}", self.code())
    }
}

/// Main error type with rich context
#[derive(Error, Debug)]
pub struct Error {
    /// Error code for programmatic handling
    pub code: ErrorCode,
    /// Human-readable message
    pub message: String,
    /// Additional context
    pub context: Option<String>,
    /// Recovery suggestion
    pub suggestion: Option<String>,
    /// Source error
    #[source]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(ctx) = &self.context {
            write!(f, "\n  Context: {ctx}")?;
        }
        if let Some(suggestion) = &self.suggestion {
            write!(f, "\n  Suggestion: {suggestion}")?;
        }
        Ok(())
    }
}

impl Error {
    /// Create a new error with the given code and message
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            context: None,
            suggestion: None,
            source: None,
        }
    }

    /// Add context to the error
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Add a recovery suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Add a source error
    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Convert to a serializable report
    #[must_use] pub fn to_report(&self) -> ErrorReport {
        ErrorReport {
            code: self.code,
            code_str: self.code.to_string(),
            category: self.code.category().to_string(),
            message: self.message.clone(),
            context: self.context.clone(),
            suggestion: self.suggestion.clone(),
            source: self.source.as_ref().map(std::string::ToString::to_string),
        }
    }

    // Convenience constructors

    /// Create an I/O error
    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::IoError, message)
    }

    /// Create a file not found error
    pub fn file_not_found(path: impl AsRef<std::path::Path>) -> Self {
        Self::new(
            ErrorCode::FileNotFound,
            format!("File not found: {}", path.as_ref().display()),
        )
        .with_suggestion("Check that the file exists and you have read permissions")
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ConfigError, message)
    }

    /// Create a config not found error
    pub fn config_not_found(path: impl AsRef<std::path::Path>) -> Self {
        Self::new(
            ErrorCode::ConfigNotFound,
            format!("Configuration file not found: {}", path.as_ref().display()),
        )
        .with_suggestion("Create a .foodshare-hooks.toml file or use --config to specify a path")
    }

    /// Create a git error
    pub fn git(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::GitError, message)
    }

    /// Create a not-a-git-repo error
    #[must_use] pub fn not_a_git_repo() -> Self {
        Self::new(ErrorCode::NotAGitRepo, "Not a git repository")
            .with_suggestion("Run this command from within a git repository")
    }

    /// Create a process error
    pub fn process(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ProcessError, message)
    }

    /// Create a command not found error
    #[must_use] pub fn command_not_found(cmd: &str) -> Self {
        Self::new(
            ErrorCode::CommandNotFound,
            format!("Command not found: {cmd}"),
        )
        .with_suggestion(format!("Install {cmd} and ensure it's in your PATH"))
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ValidationError, message)
    }

    /// Create a security error
    pub fn security(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::SecurityError, message)
    }

    /// Create a secret detected error
    #[must_use] pub fn secret_detected(file: &str, line: usize) -> Self {
        Self::new(
            ErrorCode::SecretDetected,
            format!("Potential secret detected in {file} at line {line}"),
        )
        .with_suggestion("Remove the secret and use environment variables or a secrets manager")
    }
}

/// Serializable error report for logging and API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorReport {
    /// Error code
    pub code: ErrorCode,
    /// Error code as string
    pub code_str: String,
    /// Error category
    pub category: String,
    /// Error message
    pub message: String,
    /// Additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Recovery suggestion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Source error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

/// Exit codes for CLI commands
pub mod exit_codes {
    /// Successful execution
    pub const SUCCESS: i32 = 0;
    /// General failure
    pub const FAILURE: i32 = 1;
    /// Validation error
    pub const VALIDATION_ERROR: i32 = 2;
    /// Configuration error
    pub const CONFIG_ERROR: i32 = 3;
    /// Git error
    pub const GIT_ERROR: i32 = 4;
    /// Security error
    pub const SECURITY_ERROR: i32 = 5;
    /// Command timed out
    pub const TIMEOUT: i32 = 124;
    /// Command not found
    pub const COMMAND_NOT_FOUND: i32 = 127;
}

// Implement From for common error types

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        let code = match err.kind() {
            std::io::ErrorKind::NotFound => ErrorCode::FileNotFound,
            std::io::ErrorKind::PermissionDenied => ErrorCode::PermissionDenied,
            _ => ErrorCode::IoError,
        };
        Error::new(code, err.to_string()).with_source(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::new(ErrorCode::ConfigParseError, format!("JSON parse error: {err}"))
            .with_source(err)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::new(ErrorCode::ConfigParseError, format!("TOML parse error: {err}"))
            .with_source(err)
    }
}

impl From<regex::Error> for Error {
    fn from(err: regex::Error) -> Self {
        Error::new(ErrorCode::InvalidFormat, format!("Regex error: {err}"))
            .with_source(err)
    }
}

/// Extension trait for adding context to Results
pub trait ResultExt<T> {
    /// Add context to an error result
    fn context(self, context: impl Into<String>) -> Result<T>;
    /// Add a recovery suggestion to an error result
    fn with_suggestion(self, suggestion: impl Into<String>) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|e| e.with_context(context))
    }

    fn with_suggestion(self, suggestion: impl Into<String>) -> Result<T> {
        self.map_err(|e| e.with_suggestion(suggestion))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::FileNotFound.to_string(), "E2001");
        assert_eq!(ErrorCode::GitError.to_string(), "E4000");
    }

    #[test]
    fn test_error_code_category() {
        assert_eq!(ErrorCode::IoError.category(), "IO");
        assert_eq!(ErrorCode::GitError.category(), "Git");
        assert_eq!(ErrorCode::SecurityError.category(), "Security");
    }

    #[test]
    fn test_error_with_context() {
        let err = Error::file_not_found("/path/to/file")
            .with_context("While loading configuration");

        assert_eq!(err.code, ErrorCode::FileNotFound);
        assert!(err.context.is_some());
        assert!(err.suggestion.is_some());
    }

    #[test]
    fn test_error_report_serialization() {
        let err = Error::git("Failed to get staged files")
            .with_context("During pre-commit hook");

        let report = err.to_report();
        let json = serde_json::to_string(&report).unwrap();

        assert!(json.contains("E4000"));
        assert!(json.contains("Git"));
    }
}
