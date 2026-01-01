//! Configuration and input validation
//!
//! Provides comprehensive validation for:
//! - Configuration files
//! - User inputs
//! - File paths
//! - Patterns and formats
//!
//! # Example
//!
//! ```rust,ignore
//! use foodshare_core::validation::{Validator, ValidationResult};
//!
//! let result = Validator::new()
//!     .required("name", &config.name)
//!     .min_length("name", &config.name, 3)
//!     .max_length("name", &config.name, 50)
//!     .validate();
//!
//! if !result.is_valid() {
//!     for error in result.errors() {
//!         eprintln!("Validation error: {}", error);
//!     }
//! }
//! ```

use crate::error::{Error, ErrorCode, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Field that failed validation
    pub field: String,
    /// Error message
    pub message: String,
    /// Error code
    pub code: String,
    /// Expected value (if applicable)
    pub expected: Option<String>,
    /// Actual value (if applicable)
    pub actual: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Validation result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationResult {
    errors: Vec<ValidationError>,
    warnings: Vec<ValidationError>,
}

impl ValidationResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if validation passed
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get all errors
    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    /// Get all warnings
    pub fn warnings(&self) -> &[ValidationError] {
        &self.warnings
    }

    /// Add an error
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: ValidationError) {
        self.warnings.push(warning);
    }

    /// Merge another result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }

    /// Convert to Result type
    pub fn to_result(self) -> Result<()> {
        if self.is_valid() {
            Ok(())
        } else {
            let messages: Vec<String> = self.errors.iter().map(|e| e.to_string()).collect();
            Err(Error::new(
                ErrorCode::ValidationError,
                format!("Validation failed: {}", messages.join("; ")),
            ))
        }
    }
}

/// Fluent validator builder
pub struct Validator {
    result: ValidationResult,
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            result: ValidationResult::new(),
        }
    }

    /// Validate that a field is not empty
    pub fn required(mut self, field: &str, value: &str) -> Self {
        if value.trim().is_empty() {
            self.result.add_error(ValidationError {
                field: field.to_string(),
                message: "Field is required".to_string(),
                code: "REQUIRED".to_string(),
                expected: Some("non-empty value".to_string()),
                actual: Some("empty".to_string()),
            });
        }
        self
    }

    /// Validate minimum length
    pub fn min_length(mut self, field: &str, value: &str, min: usize) -> Self {
        if value.len() < min {
            self.result.add_error(ValidationError {
                field: field.to_string(),
                message: format!("Must be at least {} characters", min),
                code: "MIN_LENGTH".to_string(),
                expected: Some(format!(">= {} chars", min)),
                actual: Some(format!("{} chars", value.len())),
            });
        }
        self
    }

    /// Validate maximum length
    pub fn max_length(mut self, field: &str, value: &str, max: usize) -> Self {
        if value.len() > max {
            self.result.add_error(ValidationError {
                field: field.to_string(),
                message: format!("Must be at most {} characters", max),
                code: "MAX_LENGTH".to_string(),
                expected: Some(format!("<= {} chars", max)),
                actual: Some(format!("{} chars", value.len())),
            });
        }
        self
    }

    /// Validate against a regex pattern
    pub fn pattern(mut self, field: &str, value: &str, pattern: &str, description: &str) -> Self {
        match Regex::new(pattern) {
            Ok(re) => {
                if !re.is_match(value) {
                    self.result.add_error(ValidationError {
                        field: field.to_string(),
                        message: format!("Must match {}", description),
                        code: "PATTERN".to_string(),
                        expected: Some(description.to_string()),
                        actual: Some(value.to_string()),
                    });
                }
            }
            Err(_) => {
                self.result.add_error(ValidationError {
                    field: field.to_string(),
                    message: "Invalid validation pattern".to_string(),
                    code: "INTERNAL".to_string(),
                    expected: None,
                    actual: None,
                });
            }
        }
        self
    }

    /// Validate that a value is in a list of allowed values
    pub fn one_of(mut self, field: &str, value: &str, allowed: &[&str]) -> Self {
        if !allowed.contains(&value) {
            self.result.add_error(ValidationError {
                field: field.to_string(),
                message: format!("Must be one of: {}", allowed.join(", ")),
                code: "ONE_OF".to_string(),
                expected: Some(allowed.join(", ")),
                actual: Some(value.to_string()),
            });
        }
        self
    }

    /// Validate a numeric range
    pub fn range<T: PartialOrd + std::fmt::Display>(
        mut self,
        field: &str,
        value: T,
        min: T,
        max: T,
    ) -> Self {
        if value < min || value > max {
            self.result.add_error(ValidationError {
                field: field.to_string(),
                message: format!("Must be between {} and {}", min, max),
                code: "RANGE".to_string(),
                expected: Some(format!("{} - {}", min, max)),
                actual: Some(value.to_string()),
            });
        }
        self
    }

    /// Validate that a path exists
    pub fn path_exists(mut self, field: &str, path: &Path) -> Self {
        if !path.exists() {
            self.result.add_error(ValidationError {
                field: field.to_string(),
                message: format!("Path does not exist: {}", path.display()),
                code: "PATH_NOT_FOUND".to_string(),
                expected: Some("existing path".to_string()),
                actual: Some(path.display().to_string()),
            });
        }
        self
    }

    /// Validate that a path is a file
    pub fn is_file(mut self, field: &str, path: &Path) -> Self {
        if !path.is_file() {
            self.result.add_error(ValidationError {
                field: field.to_string(),
                message: format!("Not a file: {}", path.display()),
                code: "NOT_A_FILE".to_string(),
                expected: Some("file".to_string()),
                actual: Some(if path.is_dir() {
                    "directory".to_string()
                } else {
                    "not found".to_string()
                }),
            });
        }
        self
    }

    /// Validate that a path is a directory
    pub fn is_directory(mut self, field: &str, path: &Path) -> Self {
        if !path.is_dir() {
            self.result.add_error(ValidationError {
                field: field.to_string(),
                message: format!("Not a directory: {}", path.display()),
                code: "NOT_A_DIRECTORY".to_string(),
                expected: Some("directory".to_string()),
                actual: Some(if path.is_file() {
                    "file".to_string()
                } else {
                    "not found".to_string()
                }),
            });
        }
        self
    }

    /// Add a custom validation
    pub fn custom<F>(mut self, field: &str, f: F) -> Self
    where
        F: FnOnce() -> Option<String>,
    {
        if let Some(message) = f() {
            self.result.add_error(ValidationError {
                field: field.to_string(),
                message,
                code: "CUSTOM".to_string(),
                expected: None,
                actual: None,
            });
        }
        self
    }

    /// Add a warning (non-blocking)
    pub fn warn_if<F>(mut self, field: &str, condition: bool, message: &str) -> Self {
        if condition {
            self.result.add_warning(ValidationError {
                field: field.to_string(),
                message: message.to_string(),
                code: "WARNING".to_string(),
                expected: None,
                actual: None,
            });
        }
        self
    }

    /// Complete validation and return result
    pub fn validate(self) -> ValidationResult {
        self.result
    }
}

/// Validate a commit message format
pub fn validate_commit_message(message: &str, types: &[&str]) -> ValidationResult {
    let mut validator = Validator::new()
        .required("message", message)
        .max_length("subject", message.lines().next().unwrap_or(""), 72);

    // Check conventional commit format
    let pattern = format!(r"^({})(\(.+\))?: .+", types.join("|"));
    validator = validator.pattern(
        "format",
        message.lines().next().unwrap_or(""),
        &pattern,
        "conventional commit format (type(scope): description)",
    );

    validator.validate()
}

/// Validate a file path for safety
pub fn validate_path_safety(path: &Path) -> ValidationResult {
    let mut result = ValidationResult::new();
    let path_str = path.to_string_lossy();

    // Check for path traversal
    if path_str.contains("..") {
        result.add_error(ValidationError {
            field: "path".to_string(),
            message: "Path traversal not allowed".to_string(),
            code: "PATH_TRAVERSAL".to_string(),
            expected: None,
            actual: Some(path_str.to_string()),
        });
    }

    // Check for absolute paths (might be dangerous in some contexts)
    if path.is_absolute() {
        result.add_warning(ValidationError {
            field: "path".to_string(),
            message: "Absolute path detected".to_string(),
            code: "ABSOLUTE_PATH".to_string(),
            expected: None,
            actual: Some(path_str.to_string()),
        });
    }

    // Check for hidden files
    if path_str.starts_with('.') || path_str.contains("/.") {
        result.add_warning(ValidationError {
            field: "path".to_string(),
            message: "Hidden file/directory detected".to_string(),
            code: "HIDDEN_PATH".to_string(),
            expected: None,
            actual: Some(path_str.to_string()),
        });
    }

    result
}

/// Validate configuration schema
pub fn validate_config(config: &HashMap<String, serde_json::Value>) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Check for unknown keys
    let known_keys = [
        "general",
        "commit_msg",
        "analyze",
        "test",
        "secrets",
        "format",
        "lint",
    ];

    for key in config.keys() {
        if !known_keys.contains(&key.as_str()) {
            result.add_warning(ValidationError {
                field: key.clone(),
                message: format!("Unknown configuration key: {}", key),
                code: "UNKNOWN_KEY".to_string(),
                expected: Some(known_keys.join(", ")),
                actual: Some(key.clone()),
            });
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_validation() {
        let result = Validator::new().required("name", "").validate();
        assert!(!result.is_valid());
        assert_eq!(result.errors()[0].code, "REQUIRED");
    }

    #[test]
    fn test_min_length_validation() {
        let result = Validator::new().min_length("name", "ab", 3).validate();
        assert!(!result.is_valid());
        assert_eq!(result.errors()[0].code, "MIN_LENGTH");
    }

    #[test]
    fn test_max_length_validation() {
        let result = Validator::new()
            .max_length("name", "abcdefghijk", 5)
            .validate();
        assert!(!result.is_valid());
        assert_eq!(result.errors()[0].code, "MAX_LENGTH");
    }

    #[test]
    fn test_pattern_validation() {
        let result = Validator::new()
            .pattern("email", "invalid", r"^[\w.-]+@[\w.-]+\.\w+$", "email format")
            .validate();
        assert!(!result.is_valid());
        assert_eq!(result.errors()[0].code, "PATTERN");
    }

    #[test]
    fn test_one_of_validation() {
        let result = Validator::new()
            .one_of("type", "invalid", &["feat", "fix", "docs"])
            .validate();
        assert!(!result.is_valid());
        assert_eq!(result.errors()[0].code, "ONE_OF");
    }

    #[test]
    fn test_range_validation() {
        let result = Validator::new().range("count", 150, 1, 100).validate();
        assert!(!result.is_valid());
        assert_eq!(result.errors()[0].code, "RANGE");
    }

    #[test]
    fn test_valid_commit_message() {
        let result = validate_commit_message(
            "feat(auth): add login functionality",
            &["feat", "fix", "docs"],
        );
        assert!(result.is_valid());
    }

    #[test]
    fn test_invalid_commit_message() {
        let result = validate_commit_message("invalid message", &["feat", "fix", "docs"]);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_path_safety_traversal() {
        let result = validate_path_safety(Path::new("../../../etc/passwd"));
        assert!(!result.is_valid());
    }

    #[test]
    fn test_chained_validation() {
        let result = Validator::new()
            .required("name", "test")
            .min_length("name", "test", 2)
            .max_length("name", "test", 10)
            .validate();
        assert!(result.is_valid());
    }
}
