//! Audit logging for security and compliance
//!
//! Provides structured audit logging for:
//! - Command executions
//! - Configuration changes
//! - Security events
//! - Error occurrences
//!
//! # Example
//!
//! ```rust,ignore
//! use foodshare_core::audit::{AuditLog, AuditEvent, AuditAction};
//!
//! let audit = AuditLog::new()?;
//!
//! audit.log(AuditEvent::new(
//!     AuditAction::CommandExecuted,
//!     "secrets",
//! ).with_detail("files_scanned", "42"));
//! ```

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Mutex;

/// Audit action types for categorizing events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditAction {
    // Command actions
    /// A command was successfully executed
    CommandExecuted,
    /// A command failed to execute
    CommandFailed,
    /// A command was skipped
    CommandSkipped,

    // Configuration actions
    /// Configuration was loaded successfully
    ConfigLoaded,
    /// Configuration was changed
    ConfigChanged,
    /// Configuration validation failed
    ConfigValidationFailed,

    // Security actions
    /// A potential secret was detected in code
    SecretDetected,
    /// Security check passed
    SecurityCheckPassed,
    /// Security check failed
    SecurityCheckFailed,
    /// Unauthorized access attempt detected
    UnauthorizedAccess,

    // Git actions
    /// Commit was validated successfully
    CommitValidated,
    /// Commit was rejected
    CommitRejected,
    /// Push was allowed
    PushAllowed,
    /// Push was blocked
    PushBlocked,

    // System actions
    /// Health check passed
    HealthCheckPassed,
    /// Health check failed
    HealthCheckFailed,
    /// Cache hit occurred
    CacheHit,
    /// Cache miss occurred
    CacheMiss,

    // Error actions
    /// An error occurred
    ErrorOccurred,
    /// An error was recovered from
    ErrorRecovered,
}

impl AuditAction {
    /// Get the severity level of this action
    #[must_use] pub fn severity(&self) -> AuditSeverity {
        match self {
            AuditAction::SecretDetected
            | AuditAction::SecurityCheckFailed
            | AuditAction::UnauthorizedAccess
            | AuditAction::PushBlocked => AuditSeverity::High,

            AuditAction::CommandFailed
            | AuditAction::ConfigValidationFailed
            | AuditAction::CommitRejected
            | AuditAction::HealthCheckFailed
            | AuditAction::ErrorOccurred => AuditSeverity::Medium,

            _ => AuditSeverity::Low,
        }
    }
}

/// Audit severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    /// Low severity - informational events
    Low,
    /// Medium severity - warnings or minor issues
    Medium,
    /// High severity - security or critical issues
    High,
    /// Critical severity - requires immediate attention
    Critical,
}

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: String,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Action type
    pub action: AuditAction,
    /// Severity level
    pub severity: AuditSeverity,
    /// Target of the action (command name, file, etc.)
    pub target: String,
    /// Whether the action succeeded
    pub success: bool,
    /// Duration in milliseconds (if applicable)
    pub duration_ms: Option<u64>,
    /// Additional details
    pub details: HashMap<String, String>,
    /// User or process that triggered the event
    pub actor: String,
    /// Session ID for correlation
    pub session_id: String,
    /// Machine identifier
    pub machine_id: String,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(action: AuditAction, target: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            action,
            severity: action.severity(),
            target: target.into(),
            success: true,
            duration_ms: None,
            details: HashMap::new(),
            actor: whoami(),
            session_id: session_id(),
            machine_id: machine_id(),
        }
    }

    /// Mark as failed
    #[must_use] pub fn failed(mut self) -> Self {
        self.success = false;
        self
    }

    /// Set duration
    #[must_use] pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Add a detail
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    /// Set severity override
    #[must_use] pub fn with_severity(mut self, severity: AuditSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Set actor
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = actor.into();
        self
    }
}

/// Audit log configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Log file path
    pub log_path: PathBuf,
    /// Minimum severity to log
    pub min_severity: AuditSeverity,
    /// Maximum log file size in bytes before rotation
    pub max_file_size: u64,
    /// Number of rotated files to keep
    pub max_files: usize,
    /// Log to stdout as well
    pub stdout: bool,
    /// JSON format (vs human-readable)
    pub json_format: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        let log_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("foodshare-tools")
            .join("audit.log");

        Self {
            log_path,
            min_severity: AuditSeverity::Low,
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_files: 5,
            stdout: false,
            json_format: true,
        }
    }
}

/// Audit log writer
pub struct AuditLog {
    config: AuditConfig,
    writer: Mutex<Option<BufWriter<File>>>,
}

impl AuditLog {
    /// Create a new audit log
    pub fn new() -> Result<Self> {
        Self::with_config(AuditConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: AuditConfig) -> Result<Self> {
        // Ensure directory exists
        if let Some(parent) = config.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_path)?;

        let writer = BufWriter::new(file);

        Ok(Self {
            config,
            writer: Mutex::new(Some(writer)),
        })
    }

    /// Create a no-op audit log (for testing)
    #[must_use] pub fn noop() -> Self {
        Self {
            config: AuditConfig::default(),
            writer: Mutex::new(None),
        }
    }

    /// Log an audit event
    pub fn log(&self, event: AuditEvent) {
        // Check severity threshold
        if event.severity < self.config.min_severity {
            return;
        }

        let line = if self.config.json_format {
            serde_json::to_string(&event).unwrap_or_default()
        } else {
            format!(
                "[{}] {} {} {} target={} success={} actor={}",
                event.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                format!("{:?}", event.severity).to_uppercase(),
                format!("{:?}", event.action),
                event.id,
                event.target,
                event.success,
                event.actor,
            )
        };

        // Write to file
        if let Ok(mut guard) = self.writer.lock() {
            if let Some(ref mut writer) = *guard {
                let _ = writeln!(writer, "{line}");
                let _ = writer.flush();
            }
        }

        // Write to stdout if configured
        if self.config.stdout {
            println!("{line}");
        }
    }

    /// Log a command execution
    pub fn log_command(&self, command: &str, success: bool, duration_ms: u64) {
        let action = if success {
            AuditAction::CommandExecuted
        } else {
            AuditAction::CommandFailed
        };

        let mut event = AuditEvent::new(action, command).with_duration(duration_ms);

        if !success {
            event = event.failed();
        }

        self.log(event);
    }

    /// Log a security event
    pub fn log_security(&self, action: AuditAction, target: &str, details: &[(&str, &str)]) {
        let mut event = AuditEvent::new(action, target);

        for (key, value) in details {
            event = event.with_detail(*key, *value);
        }

        self.log(event);
    }

    /// Rotate log files if needed
    pub fn rotate_if_needed(&self) -> Result<bool> {
        let metadata = std::fs::metadata(&self.config.log_path)?;

        if metadata.len() < self.config.max_file_size {
            return Ok(false);
        }

        // Close current writer
        if let Ok(mut guard) = self.writer.lock() {
            *guard = None;
        }

        // Rotate files
        for i in (1..self.config.max_files).rev() {
            let from = self.config.log_path.with_extension(format!("log.{i}"));
            let to = self.config.log_path.with_extension(format!("log.{}", i + 1));
            if from.exists() {
                let _ = std::fs::rename(&from, &to);
            }
        }

        // Rename current to .1
        let rotated = self.config.log_path.with_extension("log.1");
        std::fs::rename(&self.config.log_path, &rotated)?;

        // Open new file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_path)?;

        if let Ok(mut guard) = self.writer.lock() {
            *guard = Some(BufWriter::new(file));
        }

        Ok(true)
    }

    /// Get recent events (reads from file)
    pub fn recent_events(&self, count: usize) -> Result<Vec<AuditEvent>> {
        let content = std::fs::read_to_string(&self.config.log_path)?;
        let events: Vec<AuditEvent> = content
            .lines()
            .rev()
            .take(count)
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        Ok(events)
    }

    /// Get events by severity
    pub fn events_by_severity(&self, min_severity: AuditSeverity) -> Result<Vec<AuditEvent>> {
        let content = std::fs::read_to_string(&self.config.log_path)?;
        let events: Vec<AuditEvent> = content
            .lines()
            .filter_map(|line| serde_json::from_str::<AuditEvent>(line).ok())
            .filter(|e| e.severity >= min_severity)
            .collect();

        Ok(events)
    }
}

// Helper functions

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn session_id() -> String {
    // Use a static session ID for the process lifetime
    use once_cell::sync::Lazy;
    static SESSION: Lazy<String> = Lazy::new(|| uuid::Uuid::new_v4().to_string());
    SESSION.clone()
}

fn machine_id() -> String {
    // Try to get a stable machine identifier
    if let Ok(hostname) = std::env::var("HOSTNAME") {
        return hostname;
    }

    // Fallback to hostname command
    if let Ok(output) = std::process::Command::new("hostname").output() {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }

    "unknown".to_string()
}

/// Global audit log instance
#[must_use] pub fn global_audit() -> &'static AuditLog {
    use once_cell::sync::Lazy;
    static AUDIT: Lazy<AuditLog> = Lazy::new(|| AuditLog::new().unwrap_or_else(|_| AuditLog::noop()));
    &AUDIT
}

/// Convenience macro for audit logging
#[macro_export]
macro_rules! audit {
    ($action:expr, $target:expr) => {
        $crate::audit::global_audit().log($crate::audit::AuditEvent::new($action, $target))
    };
    ($action:expr, $target:expr, $($key:expr => $value:expr),+) => {
        {
            let mut event = $crate::audit::AuditEvent::new($action, $target);
            $(
                event = event.with_detail($key, $value);
            )+
            $crate::audit::global_audit().log(event)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_audit() -> (AuditLog, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = AuditConfig {
            log_path: temp_dir.path().join("audit.log"),
            stdout: false,
            ..Default::default()
        };
        let audit = AuditLog::with_config(config).unwrap();
        (audit, temp_dir)
    }

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(AuditAction::CommandExecuted, "test_command");

        assert_eq!(event.action, AuditAction::CommandExecuted);
        assert_eq!(event.target, "test_command");
        assert!(event.success);
    }

    #[test]
    fn test_audit_event_with_details() {
        let event = AuditEvent::new(AuditAction::SecretDetected, "file.txt")
            .with_detail("line", "42")
            .with_detail("pattern", "AWS_KEY")
            .failed();

        assert!(!event.success);
        assert_eq!(event.details.get("line"), Some(&"42".to_string()));
    }

    #[test]
    fn test_audit_log_write() {
        let (audit, _temp) = test_audit();

        audit.log(AuditEvent::new(AuditAction::CommandExecuted, "test"));

        let events = audit.recent_events(10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].target, "test");
    }

    #[test]
    fn test_severity_filtering() {
        let (audit, _temp) = test_audit();

        audit.log(AuditEvent::new(AuditAction::CacheHit, "low"));
        audit.log(AuditEvent::new(AuditAction::CommandFailed, "medium"));
        audit.log(AuditEvent::new(AuditAction::SecretDetected, "high"));

        let high_events = audit.events_by_severity(AuditSeverity::High).unwrap();
        assert_eq!(high_events.len(), 1);
        assert_eq!(high_events[0].target, "high");
    }

    #[test]
    fn test_action_severity() {
        assert_eq!(AuditAction::SecretDetected.severity(), AuditSeverity::High);
        assert_eq!(AuditAction::CommandFailed.severity(), AuditSeverity::Medium);
        assert_eq!(AuditAction::CacheHit.severity(), AuditSeverity::Low);
    }
}
