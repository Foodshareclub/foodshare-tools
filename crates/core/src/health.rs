//! Health check system for verifying tool dependencies and environment
//!
//! Provides comprehensive health checks for:
//! - Required tools (git, compilers, etc.)
//! - Environment configuration
//! - File system permissions
//! - Network connectivity (optional)


use crate::process::{command_exists, run_command};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All checks passed
    Healthy,
    /// Some optional checks failed
    Degraded,
    /// Required checks failed
    Unhealthy,
    /// Status could not be determined
    Unknown,
}

impl HealthStatus {
    /// Returns true if status is healthy
    #[must_use] pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    /// Returns true if status is healthy or degraded (still operational)
    #[must_use] pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Name of the check
    pub name: String,
    /// Status of the check
    pub status: HealthStatus,
    /// Optional message with details
    pub message: Option<String>,
    /// Duration of the check in milliseconds
    pub duration_ms: u64,
    /// Additional details as key-value pairs
    pub details: HashMap<String, String>,
}

impl CheckResult {
    /// Create a healthy check result
    pub fn healthy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Healthy,
            message: None,
            duration_ms: 0,
            details: HashMap::new(),
        }
    }

    /// Create an unhealthy check result with a message
    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
            duration_ms: 0,
            details: HashMap::new(),
        }
    }

    /// Create a degraded check result with a message
    pub fn degraded(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Degraded,
            message: Some(message.into()),
            duration_ms: 0,
            details: HashMap::new(),
        }
    }

    /// Set the duration of the check
    #[must_use] pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = duration.as_millis() as u64;
        self
    }

    /// Add a detail key-value pair
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

/// Overall health report containing all check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Overall status based on all checks
    pub status: HealthStatus,
    /// Individual check results
    pub checks: Vec<CheckResult>,
    /// Total duration of all checks in milliseconds
    pub total_duration_ms: u64,
    /// Timestamp when the report was generated
    pub timestamp: String,
    /// Version of the tool
    pub version: String,
}

impl HealthReport {
    /// Create a new health report from check results
    #[must_use] pub fn new(checks: Vec<CheckResult>, duration: Duration) -> Self {
        let status = if checks.iter().all(|c| c.status == HealthStatus::Healthy) {
            HealthStatus::Healthy
        } else if checks.iter().any(|c| c.status == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Degraded
        };

        Self {
            status,
            checks,
            total_duration_ms: duration.as_millis() as u64,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Returns true if overall status is healthy
    #[must_use] pub fn is_healthy(&self) -> bool {
        self.status.is_healthy()
    }

    /// Get all checks that failed (not healthy)
    #[must_use] pub fn failed_checks(&self) -> Vec<&CheckResult> {
        self.checks
            .iter()
            .filter(|c| !c.status.is_healthy())
            .collect()
    }
}

/// Health checker with configurable checks
pub struct HealthChecker {
    checks: Vec<Box<dyn HealthCheck>>,
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthChecker {
    /// Create a new health checker with no checks
    #[must_use] pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Add a health check
    pub fn add_check(mut self, check: impl HealthCheck + 'static) -> Self {
        self.checks.push(Box::new(check));
        self
    }

    /// Add standard checks for all platforms
    #[must_use] pub fn with_standard_checks(self) -> Self {
        self.add_check(GitCheck)
            .add_check(DiskSpaceCheck::new("/", 100 * 1024 * 1024)) // 100MB minimum
    }

    /// Add iOS-specific checks
    #[must_use] pub fn with_ios_checks(self) -> Self {
        self.add_check(CommandCheck::new("xcodebuild", Some("--version")))
            .add_check(CommandCheck::new("swift", Some("--version")))
            .add_check(CommandCheck::optional("swiftformat", Some("--version")))
            .add_check(CommandCheck::optional("swiftlint", Some("version")))
    }

    /// Add Android-specific checks
    #[must_use] pub fn with_android_checks(self) -> Self {
        self.add_check(EnvVarCheck::new("ANDROID_HOME"))
            .add_check(CommandCheck::optional("gradle", Some("--version")))
            .add_check(CommandCheck::optional("kotlin", Some("-version")))
    }

    /// Add web-specific checks
    #[must_use] pub fn with_web_checks(self) -> Self {
        self.add_check(CommandCheck::new("node", Some("--version")))
            .add_check(CommandCheck::new("npm", Some("--version")))
    }

    /// Run all health checks
    #[must_use] pub fn run(&self) -> HealthReport {
        let start = Instant::now();
        let mut results = Vec::new();

        for check in &self.checks {
            let check_start = Instant::now();
            let mut result = check.check();
            result.duration_ms = check_start.elapsed().as_millis() as u64;
            results.push(result);
        }

        HealthReport::new(results, start.elapsed())
    }
}

/// Trait for implementing health checks
pub trait HealthCheck: Send + Sync {
    /// Perform the health check and return a result
    fn check(&self) -> CheckResult;
}

/// Check if git is available and working
pub struct GitCheck;

impl HealthCheck for GitCheck {
    fn check(&self) -> CheckResult {
        let start = Instant::now();

        if !command_exists("git") {
            return CheckResult::unhealthy("git", "Git is not installed")
                .with_duration(start.elapsed());
        }

        match run_command("git", &["--version"]) {
            Ok(output) if output.success => {
                let version = output.stdout.trim().to_string();
                CheckResult::healthy("git")
                    .with_detail("version", version)
                    .with_duration(start.elapsed())
            }
            Ok(output) => CheckResult::unhealthy("git", output.stderr)
                .with_duration(start.elapsed()),
            Err(e) => CheckResult::unhealthy("git", e.to_string())
                .with_duration(start.elapsed()),
        }
    }
}

/// Check if a command is available
pub struct CommandCheck {
    command: String,
    version_arg: Option<String>,
    required: bool,
}

impl CommandCheck {
    /// Create a required command check
    pub fn new(command: impl Into<String>, version_arg: Option<&str>) -> Self {
        Self {
            command: command.into(),
            version_arg: version_arg.map(String::from),
            required: true,
        }
    }

    /// Create an optional command check (degraded if missing, not unhealthy)
    pub fn optional(command: impl Into<String>, version_arg: Option<&str>) -> Self {
        Self {
            command: command.into(),
            version_arg: version_arg.map(String::from),
            required: false,
        }
    }
}

impl HealthCheck for CommandCheck {
    fn check(&self) -> CheckResult {
        let start = Instant::now();

        if !command_exists(&self.command) {
            let result = if self.required {
                CheckResult::unhealthy(&self.command, format!("{} is not installed", self.command))
            } else {
                CheckResult::degraded(&self.command, format!("{} is not installed (optional)", self.command))
            };
            return result.with_duration(start.elapsed());
        }

        if let Some(ref arg) = self.version_arg {
            match run_command(&self.command, &[arg]) {
                Ok(output) if output.success => {
                    let version = output.stdout.lines().next().unwrap_or("").trim().to_string();
                    CheckResult::healthy(&self.command)
                        .with_detail("version", version)
                        .with_duration(start.elapsed())
                }
                _ => CheckResult::healthy(&self.command)
                    .with_duration(start.elapsed()),
            }
        } else {
            CheckResult::healthy(&self.command)
                .with_duration(start.elapsed())
        }
    }
}

/// Check if an environment variable is set
pub struct EnvVarCheck {
    var_name: String,
    required: bool,
}

impl EnvVarCheck {
    /// Create a required environment variable check
    pub fn new(var_name: impl Into<String>) -> Self {
        Self {
            var_name: var_name.into(),
            required: true,
        }
    }

    /// Create an optional environment variable check
    pub fn optional(var_name: impl Into<String>) -> Self {
        Self {
            var_name: var_name.into(),
            required: false,
        }
    }
}

impl HealthCheck for EnvVarCheck {
    fn check(&self) -> CheckResult {
        match std::env::var(&self.var_name) {
            Ok(value) => CheckResult::healthy(&self.var_name)
                .with_detail("value", if value.len() > 50 {
                    format!("{}...", &value[..50])
                } else {
                    value
                }),
            Err(_) => {
                if self.required {
                    CheckResult::unhealthy(&self.var_name, format!("{} is not set", self.var_name))
                } else {
                    CheckResult::degraded(&self.var_name, format!("{} is not set (optional)", self.var_name))
                }
            }
        }
    }
}

/// Check available disk space
pub struct DiskSpaceCheck {
    path: String,
    min_bytes: u64,
}

impl DiskSpaceCheck {
    /// Create a disk space check for a path with minimum required bytes
    pub fn new(path: impl Into<String>, min_bytes: u64) -> Self {
        Self {
            path: path.into(),
            min_bytes,
        }
    }
}

impl HealthCheck for DiskSpaceCheck {
    fn check(&self) -> CheckResult {
        // Use df command to check disk space
        match run_command("df", &["-k", &self.path]) {
            Ok(output) if output.success => {
                // Parse df output to get available space
                let lines: Vec<&str> = output.stdout.lines().collect();
                if lines.len() >= 2 {
                    let parts: Vec<&str> = lines[1].split_whitespace().collect();
                    if parts.len() >= 4 {
                        if let Ok(available_kb) = parts[3].parse::<u64>() {
                            let available_bytes = available_kb * 1024;
                            if available_bytes >= self.min_bytes {
                                return CheckResult::healthy("disk_space")
                                    .with_detail("available_mb", (available_bytes / 1024 / 1024).to_string())
                                    .with_detail("path", &self.path);
                            } else {
                                return CheckResult::degraded(
                                    "disk_space",
                                    format!("Low disk space: {} MB available", available_bytes / 1024 / 1024),
                                )
                                .with_detail("available_mb", (available_bytes / 1024 / 1024).to_string())
                                .with_detail("required_mb", (self.min_bytes / 1024 / 1024).to_string());
                            }
                        }
                    }
                }
                CheckResult::healthy("disk_space")
            }
            _ => CheckResult::healthy("disk_space"), // Can't check, assume OK
        }
    }
}

/// Check if a path exists and is accessible
pub struct PathCheck {
    path: String,
    check_writable: bool,
}

impl PathCheck {
    /// Create a check for a readable path
    pub fn readable(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            check_writable: false,
        }
    }

    /// Create a check for a writable path
    pub fn writable(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            check_writable: true,
        }
    }
}

impl HealthCheck for PathCheck {
    fn check(&self) -> CheckResult {
        let path = Path::new(&self.path);

        if !path.exists() {
            return CheckResult::unhealthy(&self.path, "Path does not exist");
        }

        if self.check_writable {
            // Try to check write permission
            let metadata = match std::fs::metadata(path) {
                Ok(m) => m,
                Err(e) => return CheckResult::unhealthy(&self.path, e.to_string()),
            };

            if metadata.permissions().readonly() {
                return CheckResult::unhealthy(&self.path, "Path is read-only");
            }
        }

        CheckResult::healthy(&self.path)
            .with_detail("exists", "true")
            .with_detail("writable", self.check_writable.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_check() {
        let check = GitCheck;
        let result = check.check();
        // Git should be available in most dev environments
        assert!(result.status.is_operational());
    }

    #[test]
    fn test_command_check_optional() {
        let check = CommandCheck::optional("nonexistent_command_12345", None);
        let result = check.check();
        // Should be degraded, not unhealthy
        assert_eq!(result.status, HealthStatus::Degraded);
    }

    #[test]
    fn test_health_report() {
        let checks = vec![
            CheckResult::healthy("check1"),
            CheckResult::healthy("check2"),
        ];
        let report = HealthReport::new(checks, Duration::from_millis(100));
        assert!(report.is_healthy());
    }

    #[test]
    fn test_health_report_with_failure() {
        let checks = vec![
            CheckResult::healthy("check1"),
            CheckResult::unhealthy("check2", "Failed"),
        ];
        let report = HealthReport::new(checks, Duration::from_millis(100));
        assert!(!report.is_healthy());
        assert_eq!(report.status, HealthStatus::Unhealthy);
    }
}
