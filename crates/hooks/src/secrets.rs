//! Enterprise-grade secret scanning for Foodshare development tools.
//!
//! This module provides comprehensive secret detection with:
//! - **Stable API** - Versioned patterns and backwards-compatible interfaces
//! - **Configuration-driven** - External pattern definitions via TOML/JSON
//! - **Typed errors** - No panics, proper `Result` types throughout
//! - **Extensibility** - Custom patterns, allowlists, and hooks
//! - **Observability** - Statistics, callbacks, and audit trails
//!
//! # Quick Start
//!
//! ```
//! use foodshare_hooks::{SecretScanner, Severity};
//!
//! let result = SecretScanner::new()
//!     .min_severity(Severity::High)
//!     .scan_str("AWS_KEY=AKIAIOSFODNN7EXAMPLE", "config.env");
//!
//! if result.has_secrets() {
//!     for finding in result.findings() {
//!         println!("{}: {}", finding.pattern_id, finding.masked_value);
//!     }
//! }
//! ```
//!
//! # Enterprise Usage
//!
//! ```ignore
//! use foodshare_hooks::{SecretScanner, ScannerConfig};
//!
//! // Load configuration from file
//! let config = ScannerConfig::from_file("secrets.toml")?;
//!
//! let scanner = SecretScanner::from_config(config)
//!     .on_finding(|f| log::warn!("Secret found: {}", f.pattern_id));
//!
//! let result = scanner.scan_paths(&["src/", "config/"])?;
//! ```
//!
//! # Pattern Versioning
//!
//! All built-in patterns are versioned. The current pattern set version is
//! [`PATTERN_VERSION`]. When patterns change, the version increments to
//! allow tracking which pattern set was used for a scan.

use foodshare_core::config::SecretsConfig;
use foodshare_core::error::exit_codes;
use once_cell::sync::Lazy;
use owo_colors::OwoColorize;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

// =============================================================================
// Constants & Versioning
// =============================================================================

/// Current version of the built-in pattern set.
/// Increment when patterns are added, removed, or modified.
pub const PATTERN_VERSION: &str = "2.0.0";

/// API version for serialized configurations.
pub const CONFIG_API_VERSION: u32 = 1;

/// Default entropy threshold (bits per character).
const DEFAULT_ENTROPY_THRESHOLD: f64 = 4.5;

/// Default minimum length for entropy detection.
const DEFAULT_ENTROPY_MIN_LENGTH: usize = 20;

/// Default line truncation length.
const DEFAULT_MAX_LINE_LENGTH: usize = 120;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during secret scanning.
#[derive(Debug, Clone)]
pub enum ScanError {
    /// Failed to read a file.
    FileRead { path: PathBuf, message: String },
    /// Invalid regex pattern.
    InvalidPattern { pattern: String, message: String },
    /// Configuration error.
    Config { message: String },
    /// I/O error.
    Io { message: String },
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileRead { path, message } => {
                write!(f, "Failed to read {}: {}", path.display(), message)
            }
            Self::InvalidPattern { pattern, message } => {
                write!(f, "Invalid pattern '{}': {}", pattern, message)
            }
            Self::Config { message } => write!(f, "Configuration error: {}", message),
            Self::Io { message } => write!(f, "I/O error: {}", message),
        }
    }
}

impl std::error::Error for ScanError {}

impl From<std::io::Error> for ScanError {
    fn from(e: std::io::Error) -> Self {
        Self::Io { message: e.to_string() }
    }
}

/// Result type for scan operations.
pub type ScanResult<T> = Result<T, ScanError>;

// =============================================================================
// Severity & Categories
// =============================================================================

/// Severity level for detected secrets.
///
/// Severity is ordered from most to least severe: `Critical > High > Medium > Low`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Critical: Immediate action required (production credentials, signing keys).
    Critical,
    /// High: Serious security risk (API keys, database credentials).
    High,
    /// Medium: Moderate risk (webhooks, internal tokens).
    Medium,
    /// Low: Informational (debug statements, test credentials).
    Low,
}

impl Default for Severity {
    fn default() -> Self {
        Self::Medium
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "CRITICAL"),
            Self::High => write!(f, "HIGH"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::Low => write!(f, "LOW"),
        }
    }
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Self::Critical),
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            _ => Err(format!("Unknown severity: {}", s)),
        }
    }
}

/// Category of secret pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternCategory {
    /// Cloud provider credentials (AWS, GCP, Azure).
    CloudProvider,
    /// Source control tokens (GitHub, GitLab, Bitbucket).
    SourceControl,
    /// Database connection strings.
    Database,
    /// Payment processors (Stripe, PayPal).
    Payment,
    /// Communication services (Slack, Discord, Twilio).
    Communication,
    /// Email services (SendGrid, Mailchimp).
    Email,
    /// Authentication (JWT, OAuth, passwords).
    Authentication,
    /// Cryptographic keys.
    Cryptography,
    /// Package registries (npm, PyPI).
    PackageRegistry,
    /// Debugging and logging.
    Debug,
    /// Custom user-defined patterns.
    Custom,
}

impl Default for PatternCategory {
    fn default() -> Self {
        Self::Custom
    }
}

// =============================================================================
// Pattern Definition
// =============================================================================

/// A pattern definition for secret detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDef {
    /// Unique identifier (e.g., "aws-access-key", "github-token").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Regex pattern string.
    pub pattern: String,
    /// Severity level.
    #[serde(default)]
    pub severity: Severity,
    /// Pattern category.
    #[serde(default)]
    pub category: PatternCategory,
    /// Description of what this pattern detects.
    #[serde(default)]
    pub description: String,
    /// Whether this pattern is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Internal compiled pattern.
struct CompiledPattern {
    def: PatternDef,
    regex: Regex,
}

// =============================================================================
// Findings
// =============================================================================

/// A detected secret finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique identifier for this finding (hash-based).
    pub id: String,
    /// Pattern ID that matched.
    pub pattern_id: String,
    /// Pattern name for display.
    pub pattern_name: String,
    /// File path where secret was found.
    pub file: String,
    /// Line number (1-indexed).
    pub line: usize,
    /// Column number (1-indexed).
    pub column: usize,
    /// Masked version of the matched text.
    pub masked_value: String,
    /// Severity level.
    pub severity: Severity,
    /// Pattern category.
    pub category: PatternCategory,
    /// Line content (truncated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_content: Option<String>,
    /// Fingerprint for deduplication.
    #[serde(skip)]
    pub fingerprint: String,
}

impl Finding {
    /// Generate a stable fingerprint for this finding.
    fn generate_fingerprint(pattern_id: &str, file: &str, line: usize, matched: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        pattern_id.hash(&mut hasher);
        file.hash(&mut hasher);
        line.hash(&mut hasher);
        matched.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Generate a unique ID for this finding.
    fn generate_id(fingerprint: &str) -> String {
        format!("SEC-{}", &fingerprint[..8].to_uppercase())
    }
}

// =============================================================================
// Scan Statistics
// =============================================================================

/// Statistics from a scan operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanStats {
    /// Number of files scanned.
    pub files_scanned: usize,
    /// Number of files skipped (excluded or unreadable).
    pub files_skipped: usize,
    /// Number of lines scanned.
    pub lines_scanned: usize,
    /// Number of secrets found.
    pub findings_count: usize,
    /// Number of findings by severity.
    pub by_severity: HashMap<String, usize>,
    /// Number of findings by category.
    pub by_category: HashMap<String, usize>,
    /// Scan duration in milliseconds.
    pub duration_ms: u64,
    /// Pattern version used.
    pub pattern_version: String,
}

// =============================================================================
// Scan Result
// =============================================================================

/// Result of a scan operation.
#[derive(Debug, Clone, Default)]
pub struct ScanOutput {
    findings: Vec<Finding>,
    stats: ScanStats,
    errors: Vec<ScanError>,
}

impl ScanOutput {
    /// Create a new empty scan output.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any secrets were found.
    #[must_use]
    pub fn has_secrets(&self) -> bool {
        !self.findings.is_empty()
    }

    /// Get all findings.
    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// Get findings filtered by severity.
    #[must_use]
    pub fn findings_by_severity(&self, severity: Severity) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.severity == severity).collect()
    }

    /// Get findings filtered by category.
    #[must_use]
    pub fn findings_by_category(&self, category: PatternCategory) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.category == category).collect()
    }

    /// Get scan statistics.
    #[must_use]
    pub fn stats(&self) -> &ScanStats {
        &self.stats
    }

    /// Get any errors that occurred during scanning.
    #[must_use]
    pub fn errors(&self) -> &[ScanError] {
        &self.errors
    }

    /// Check if scan completed without errors.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty() && self.errors.is_empty()
    }

    /// Merge another scan output into this one.
    pub fn merge(&mut self, other: ScanOutput) {
        self.findings.extend(other.findings);
        self.errors.extend(other.errors);
        self.stats.files_scanned += other.stats.files_scanned;
        self.stats.files_skipped += other.stats.files_skipped;
        self.stats.lines_scanned += other.stats.lines_scanned;
        self.stats.findings_count = self.findings.len();
    }
}

// =============================================================================
// Scanner Configuration
// =============================================================================

/// Configuration for the secret scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    /// API version for this config.
    #[serde(default = "default_api_version")]
    pub api_version: u32,

    /// Minimum severity to report.
    #[serde(default)]
    pub min_severity: Option<Severity>,

    /// Patterns to exclude lines (e.g., "noqa", "pragma: allowlist").
    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    /// File patterns to exclude (glob-style).
    #[serde(default)]
    pub exclude_files: Vec<String>,

    /// Allowlisted values (exact match, will not be reported).
    #[serde(default)]
    pub allowlist: Vec<String>,

    /// Allowlisted fingerprints (for suppressing known findings).
    #[serde(default)]
    pub allowlist_fingerprints: HashSet<String>,

    /// Custom pattern definitions.
    #[serde(default)]
    pub custom_patterns: Vec<PatternDef>,

    /// Disabled built-in pattern IDs.
    #[serde(default)]
    pub disabled_patterns: HashSet<String>,

    /// Enable entropy-based detection.
    #[serde(default)]
    pub enable_entropy: bool,

    /// Entropy threshold (default 4.5).
    #[serde(default = "default_entropy_threshold")]
    pub entropy_threshold: f64,

    /// Minimum length for entropy detection (default 20).
    #[serde(default = "default_entropy_min_length")]
    pub entropy_min_length: usize,

    /// Maximum line length for context (default 120).
    #[serde(default = "default_max_line_length")]
    pub max_line_length: usize,

    /// Include line content in findings.
    #[serde(default = "default_true")]
    pub include_line_content: bool,
}

fn default_api_version() -> u32 { CONFIG_API_VERSION }
fn default_entropy_threshold() -> f64 { DEFAULT_ENTROPY_THRESHOLD }
fn default_entropy_min_length() -> usize { DEFAULT_ENTROPY_MIN_LENGTH }
fn default_max_line_length() -> usize { DEFAULT_MAX_LINE_LENGTH }

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            api_version: CONFIG_API_VERSION,
            min_severity: None,
            exclude_patterns: Vec::new(),
            exclude_files: Vec::new(),
            allowlist: Vec::new(),
            allowlist_fingerprints: HashSet::new(),
            custom_patterns: Vec::new(),
            disabled_patterns: HashSet::new(),
            enable_entropy: false,
            entropy_threshold: DEFAULT_ENTROPY_THRESHOLD,
            entropy_min_length: DEFAULT_ENTROPY_MIN_LENGTH,
            max_line_length: DEFAULT_MAX_LINE_LENGTH,
            include_line_content: true,
        }
    }
}

impl ScannerConfig {
    /// Create a new default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a TOML file.
    pub fn from_toml_file(path: impl AsRef<Path>) -> ScanResult<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ScanError::FileRead {
                path: path.as_ref().to_path_buf(),
                message: e.to_string(),
            })?;
        Self::from_toml(&content)
    }

    /// Parse configuration from TOML string.
    pub fn from_toml(content: &str) -> ScanResult<Self> {
        toml::from_str(content).map_err(|e| ScanError::Config {
            message: e.to_string(),
        })
    }

    /// Load configuration from a JSON file.
    pub fn from_json_file(path: impl AsRef<Path>) -> ScanResult<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ScanError::FileRead {
                path: path.as_ref().to_path_buf(),
                message: e.to_string(),
            })?;
        Self::from_json(&content)
    }

    /// Parse configuration from JSON string.
    pub fn from_json(content: &str) -> ScanResult<Self> {
        serde_json::from_str(content).map_err(|e| ScanError::Config {
            message: e.to_string(),
        })
    }

    /// Serialize configuration to TOML.
    #[must_use]
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }

    /// Serialize configuration to JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

// =============================================================================
// Built-in Patterns
// =============================================================================

/// Built-in pattern definitions.
static BUILTIN_PATTERNS: Lazy<Vec<PatternDef>> = Lazy::new(|| {
    vec![
        // Cloud Providers
        PatternDef {
            id: "aws-access-key".into(),
            name: "AWS Access Key".into(),
            pattern: r"AKIA[0-9A-Z]{16}".into(),
            severity: Severity::Critical,
            category: PatternCategory::CloudProvider,
            description: "AWS Access Key ID".into(),
            enabled: true,
        },
        PatternDef {
            id: "aws-secret-key".into(),
            name: "AWS Secret Key".into(),
            pattern: r#"(?i)aws[_\-]?secret[_\-]?access[_\-]?key\s*[=:]\s*["']?[A-Za-z0-9/+=]{40}"#.into(),
            severity: Severity::Critical,
            category: PatternCategory::CloudProvider,
            description: "AWS Secret Access Key".into(),
            enabled: true,
        },
        PatternDef {
            id: "google-api-key".into(),
            name: "Google API Key".into(),
            pattern: r"AIza[0-9A-Za-z_-]{35}".into(),
            severity: Severity::High,
            category: PatternCategory::CloudProvider,
            description: "Google Cloud API Key".into(),
            enabled: true,
        },
        PatternDef {
            id: "firebase-url".into(),
            name: "Firebase URL".into(),
            pattern: r"https://[a-z0-9-]+\.firebaseio\.com".into(),
            severity: Severity::Medium,
            category: PatternCategory::CloudProvider,
            description: "Firebase Realtime Database URL".into(),
            enabled: true,
        },
        PatternDef {
            id: "heroku-api-key".into(),
            name: "Heroku API Key".into(),
            pattern: r"(?i)heroku[_-]?api[_-]?key\s*[=:]\s*[A-Fa-f0-9-]{36}".into(),
            severity: Severity::High,
            category: PatternCategory::CloudProvider,
            description: "Heroku API Key (UUID format)".into(),
            enabled: true,
        },

        // Source Control
        PatternDef {
            id: "github-token".into(),
            name: "GitHub Token".into(),
            pattern: r"gh[pousr]_[A-Za-z0-9_]{36,}".into(),
            severity: Severity::Critical,
            category: PatternCategory::SourceControl,
            description: "GitHub Personal Access Token or OAuth Token".into(),
            enabled: true,
        },
        PatternDef {
            id: "npm-token".into(),
            name: "NPM Token".into(),
            pattern: r"npm_[A-Za-z0-9]{36}".into(),
            severity: Severity::High,
            category: PatternCategory::PackageRegistry,
            description: "NPM Access Token".into(),
            enabled: true,
        },
        PatternDef {
            id: "pypi-token".into(),
            name: "PyPI Token".into(),
            pattern: r"pypi-[A-Za-z0-9_-]{50,}".into(),
            severity: Severity::High,
            category: PatternCategory::PackageRegistry,
            description: "PyPI API Token".into(),
            enabled: true,
        },

        // Database
        PatternDef {
            id: "database-url".into(),
            name: "Database URL".into(),
            pattern: r"(?i)(postgres|mysql|mongodb)://[^:]+:[^@]+@".into(),
            severity: Severity::Critical,
            category: PatternCategory::Database,
            description: "Database connection string with credentials".into(),
            enabled: true,
        },
        PatternDef {
            id: "supabase-key".into(),
            name: "Supabase Service Key".into(),
            pattern: r"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+".into(),
            severity: Severity::Critical,
            category: PatternCategory::Database,
            description: "Supabase service role JWT (anon keys are also matched)".into(),
            enabled: true,
        },

        // Payment
        PatternDef {
            id: "stripe-secret-key".into(),
            name: "Stripe Secret Key".into(),
            pattern: r"sk_(live|test)_[A-Za-z0-9]{24,}".into(),
            severity: Severity::Critical,
            category: PatternCategory::Payment,
            description: "Stripe Secret API Key".into(),
            enabled: true,
        },

        // Communication
        PatternDef {
            id: "slack-webhook".into(),
            name: "Slack Webhook".into(),
            pattern: r"https://hooks\.slack\.com/services/T[A-Z0-9]+/B[A-Z0-9]+/[A-Za-z0-9]+".into(),
            severity: Severity::Medium,
            category: PatternCategory::Communication,
            description: "Slack Incoming Webhook URL".into(),
            enabled: true,
        },
        PatternDef {
            id: "discord-webhook".into(),
            name: "Discord Webhook".into(),
            pattern: r"https://discord(app)?\.com/api/webhooks/\d+/[A-Za-z0-9_-]+".into(),
            severity: Severity::Medium,
            category: PatternCategory::Communication,
            description: "Discord Webhook URL".into(),
            enabled: true,
        },
        PatternDef {
            id: "twilio-auth-token".into(),
            name: "Twilio Auth Token".into(),
            pattern: r#"(?i)twilio[_\-]?(auth[_\-]?)?token\s*[=:]\s*["']?[a-f0-9]{32}"#.into(),
            severity: Severity::High,
            category: PatternCategory::Communication,
            description: "Twilio Auth Token".into(),
            enabled: true,
        },

        // Email
        PatternDef {
            id: "sendgrid-api-key".into(),
            name: "SendGrid API Key".into(),
            pattern: r"SG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}".into(),
            severity: Severity::High,
            category: PatternCategory::Email,
            description: "SendGrid API Key".into(),
            enabled: true,
        },

        // Authentication
        PatternDef {
            id: "private-key".into(),
            name: "Private Key".into(),
            pattern: r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----".into(),
            severity: Severity::Critical,
            category: PatternCategory::Cryptography,
            description: "PEM-encoded private key".into(),
            enabled: true,
        },
        PatternDef {
            id: "password-assignment".into(),
            name: "Password Assignment".into(),
            pattern: r#"(?i)(password|passwd|pwd)\s*[=:]\s*["'][^"']{8,}["']"#.into(),
            severity: Severity::High,
            category: PatternCategory::Authentication,
            description: "Hardcoded password in assignment".into(),
            enabled: true,
        },
        PatternDef {
            id: "generic-api-key".into(),
            name: "Generic API Key".into(),
            pattern: r#"(?i)(api[_\-]?key|apikey)\s*[=:]\s*["']?[A-Za-z0-9_\-]{20,}"#.into(),
            severity: Severity::Medium,
            category: PatternCategory::Authentication,
            description: "Generic API key pattern".into(),
            enabled: true,
        },

        // Debug (lower severity)
        PatternDef {
            id: "debug-print".into(),
            name: "Debug Print".into(),
            pattern: r#"(?i)(console\.log|print|NSLog|debugPrint)\s*\(\s*["'].*password.*["']"#.into(),
            severity: Severity::Low,
            category: PatternCategory::Debug,
            description: "Debug statement containing password".into(),
            enabled: true,
        },
    ]
});

/// Get all built-in pattern definitions.
#[must_use]
pub fn builtin_patterns() -> &'static [PatternDef] {
    &BUILTIN_PATTERNS
}

// =============================================================================
// Compiled Pattern Cache
// =============================================================================

/// Compiled patterns cache for performance.
static COMPILED_PATTERNS: Lazy<Vec<CompiledPattern>> = Lazy::new(|| {
    BUILTIN_PATTERNS
        .iter()
        .filter_map(|def| {
            Regex::new(&def.pattern).ok().map(|regex| CompiledPattern {
                def: def.clone(),
                regex,
            })
        })
        .collect()
});

// =============================================================================
// Entropy Detection
// =============================================================================

/// Calculate Shannon entropy of a string.
#[inline]
fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }

    let mut freq = [0u32; 256];
    let len = s.len() as f64;

    for byte in s.bytes() {
        freq[byte as usize] += 1;
    }

    freq.iter()
        .filter(|&&count| count > 0)
        .map(|&count| {
            let p = count as f64 / len;
            -p * p.log2()
        })
        .sum()
}

/// Check if a string looks like a high-entropy secret.
fn is_high_entropy_secret(s: &str, threshold: f64, min_length: usize) -> bool {
    if s.len() < min_length {
        return false;
    }

    // Skip common false positives
    if s.chars().all(|c| c.is_ascii_lowercase())
        || s.chars().all(|c| c.is_ascii_uppercase())
        || s.chars().all(|c| c.is_ascii_digit())
    {
        return false;
    }

    // Must have mixed character types
    let has_upper = s.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = s.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = s.chars().any(|c| c.is_ascii_digit());

    if !((has_upper && has_lower) || (has_upper && has_digit) || (has_lower && has_digit)) {
        return false;
    }

    shannon_entropy(s) >= threshold
}

/// Regex for extracting potential secret values.
static ASSIGNMENT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"[=:]\s*["']?([A-Za-z0-9_+/=-]{20,})["']?"#).unwrap()
});

// =============================================================================
// Utility Functions
// =============================================================================

/// Mask a secret for safe display.
///
/// Shows first/last 4 characters for secrets longer than 8 characters.
/// UTF-8 safe implementation using character-based indexing.
#[inline]
fn mask_secret(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    let len = chars.len();

    if len <= 8 {
        "*".repeat(len)
    } else {
        let prefix: String = chars[..4].iter().collect();
        let suffix: String = chars[len - 4..].iter().collect();
        format!("{}...{}", prefix, suffix)
    }
}

/// Truncate a line for display.
#[inline]
fn truncate_line(line: &str, max_len: usize) -> String {
    let trimmed = line.trim();
    if trimmed.len() <= max_len {
        trimmed.to_string()
    } else {
        let chars: Vec<char> = trimmed.chars().collect();
        if chars.len() <= max_len {
            trimmed.to_string()
        } else {
            let truncated: String = chars[..max_len].iter().collect();
            format!("{}...", truncated)
        }
    }
}

// =============================================================================
// Secret Scanner
// =============================================================================

/// Enterprise-grade secret scanner with fluent API.
///
/// # Example
///
/// ```
/// use foodshare_hooks::{SecretScanner, Severity};
///
/// let result = SecretScanner::new()
///     .min_severity(Severity::High)
///     .exclude_pattern("noqa")
///     .scan_str("AWS_KEY=AKIAIOSFODNN7EXAMPLE", "config.env");
///
/// assert!(result.has_secrets());
/// ```
#[derive(Clone)]
pub struct SecretScanner {
    config: ScannerConfig,
    custom_compiled: Vec<Arc<CompiledPattern>>,
    on_finding: Option<Arc<dyn Fn(&Finding) + Send + Sync>>,
}

impl Default for SecretScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretScanner {
    /// Create a new scanner with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ScannerConfig::default(),
            custom_compiled: Vec::new(),
            on_finding: None,
        }
    }

    /// Create a scanner from configuration.
    #[must_use]
    pub fn from_config(config: ScannerConfig) -> Self {
        let custom_compiled: Vec<_> = config
            .custom_patterns
            .iter()
            .filter_map(|def| {
                Regex::new(&def.pattern).ok().map(|regex| {
                    Arc::new(CompiledPattern {
                        def: def.clone(),
                        regex,
                    })
                })
            })
            .collect();

        Self {
            config,
            custom_compiled,
            on_finding: None,
        }
    }

    /// Set minimum severity to report.
    #[must_use]
    pub fn min_severity(mut self, severity: Severity) -> Self {
        self.config.min_severity = Some(severity);
        self
    }

    /// Add a pattern to exclude lines containing this text.
    #[must_use]
    pub fn exclude_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.config.exclude_patterns.push(pattern.into());
        self
    }

    /// Add a file pattern to exclude.
    #[must_use]
    pub fn exclude_file(mut self, pattern: impl Into<String>) -> Self {
        self.config.exclude_files.push(pattern.into());
        self
    }

    /// Add a value to the allowlist (will not be reported).
    #[must_use]
    pub fn allowlist_value(mut self, value: impl Into<String>) -> Self {
        self.config.allowlist.push(value.into());
        self
    }

    /// Add a fingerprint to suppress.
    #[must_use]
    pub fn allowlist_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.config.allowlist_fingerprints.insert(fingerprint.into());
        self
    }

    /// Disable a built-in pattern by ID.
    #[must_use]
    pub fn disable_pattern(mut self, pattern_id: impl Into<String>) -> Self {
        self.config.disabled_patterns.insert(pattern_id.into());
        self
    }

    /// Add a custom pattern.
    #[must_use]
    pub fn add_pattern(mut self, def: PatternDef) -> Self {
        if let Ok(regex) = Regex::new(&def.pattern) {
            self.custom_compiled.push(Arc::new(CompiledPattern {
                def: def.clone(),
                regex,
            }));
            self.config.custom_patterns.push(def);
        }
        self
    }

    /// Add a custom pattern from regex string.
    #[must_use]
    pub fn add_pattern_regex(self, id: impl Into<String>, pattern: impl Into<String>) -> Self {
        let id = id.into();
        let pattern_str = pattern.into();
        self.add_pattern(PatternDef {
            id: id.clone(),
            name: id,
            pattern: pattern_str,
            severity: Severity::Medium,
            category: PatternCategory::Custom,
            description: String::new(),
            enabled: true,
        })
    }

    /// Enable entropy-based detection.
    #[must_use]
    pub fn with_entropy_detection(mut self) -> Self {
        self.config.enable_entropy = true;
        self
    }

    /// Set entropy threshold.
    #[must_use]
    pub fn entropy_threshold(mut self, threshold: f64) -> Self {
        self.config.entropy_threshold = threshold;
        self
    }

    /// Set callback for each finding (for logging/metrics).
    #[must_use]
    pub fn on_finding<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Finding) + Send + Sync + 'static,
    {
        self.on_finding = Some(Arc::new(callback));
        self
    }

    /// Get the current configuration.
    #[must_use]
    pub fn config(&self) -> &ScannerConfig {
        &self.config
    }

    /// Scan content string and return findings.
    #[must_use]
    pub fn scan_str(&self, content: &str, file_name: &str) -> ScanOutput {
        let start = Instant::now();
        let mut output = ScanOutput::new();
        let mut seen_fingerprints = HashSet::new();

        let lines: Vec<&str> = content.lines().collect();
        output.stats.lines_scanned = lines.len();
        output.stats.files_scanned = 1;
        output.stats.pattern_version = PATTERN_VERSION.to_string();

        for (line_num, line) in lines.iter().enumerate() {
            // Skip excluded lines
            if self.config.exclude_patterns.iter().any(|p| line.contains(p)) {
                continue;
            }

            // Check built-in patterns
            for cp in COMPILED_PATTERNS.iter() {
                if self.config.disabled_patterns.contains(&cp.def.id) || !cp.def.enabled {
                    continue;
                }

                if let Some(min_sev) = self.config.min_severity {
                    if cp.def.severity > min_sev {
                        continue;
                    }
                }

                if let Some(m) = cp.regex.find(line) {
                    let matched = m.as_str();

                    // Check allowlist
                    if self.config.allowlist.iter().any(|a| matched.contains(a)) {
                        continue;
                    }

                    let fingerprint = Finding::generate_fingerprint(
                        &cp.def.id, file_name, line_num + 1, matched
                    );

                    // Check fingerprint allowlist
                    if self.config.allowlist_fingerprints.contains(&fingerprint) {
                        continue;
                    }

                    // Deduplicate
                    if seen_fingerprints.contains(&fingerprint) {
                        continue;
                    }
                    seen_fingerprints.insert(fingerprint.clone());

                    let finding = Finding {
                        id: Finding::generate_id(&fingerprint),
                        pattern_id: cp.def.id.clone(),
                        pattern_name: cp.def.name.clone(),
                        file: file_name.to_string(),
                        line: line_num + 1,
                        column: m.start() + 1,
                        masked_value: mask_secret(matched),
                        severity: cp.def.severity,
                        category: cp.def.category,
                        line_content: if self.config.include_line_content {
                            Some(truncate_line(line, self.config.max_line_length))
                        } else {
                            None
                        },
                        fingerprint,
                    };

                    if let Some(ref callback) = self.on_finding {
                        callback(&finding);
                    }

                    output.findings.push(finding);
                }
            }

            // Check custom patterns
            for cp in &self.custom_compiled {
                if !cp.def.enabled {
                    continue;
                }

                if let Some(min_sev) = self.config.min_severity {
                    if cp.def.severity > min_sev {
                        continue;
                    }
                }

                if let Some(m) = cp.regex.find(line) {
                    let matched = m.as_str();

                    if self.config.allowlist.iter().any(|a| matched.contains(a)) {
                        continue;
                    }

                    let fingerprint = Finding::generate_fingerprint(
                        &cp.def.id, file_name, line_num + 1, matched
                    );

                    if self.config.allowlist_fingerprints.contains(&fingerprint) {
                        continue;
                    }

                    if seen_fingerprints.contains(&fingerprint) {
                        continue;
                    }
                    seen_fingerprints.insert(fingerprint.clone());

                    let finding = Finding {
                        id: Finding::generate_id(&fingerprint),
                        pattern_id: cp.def.id.clone(),
                        pattern_name: cp.def.name.clone(),
                        file: file_name.to_string(),
                        line: line_num + 1,
                        column: m.start() + 1,
                        masked_value: mask_secret(matched),
                        severity: cp.def.severity,
                        category: cp.def.category,
                        line_content: if self.config.include_line_content {
                            Some(truncate_line(line, self.config.max_line_length))
                        } else {
                            None
                        },
                        fingerprint,
                    };

                    if let Some(ref callback) = self.on_finding {
                        callback(&finding);
                    }

                    output.findings.push(finding);
                }
            }

            // Entropy detection
            if self.config.enable_entropy {
                if let Some(cap) = ASSIGNMENT_PATTERN.captures(line) {
                    if let Some(value) = cap.get(1) {
                        let val_str = value.as_str();

                        // Skip if already matched
                        let already_matched = output.findings.iter().any(|f| {
                            f.line == line_num + 1 && f.file == file_name
                        });

                        if !already_matched && is_high_entropy_secret(
                            val_str,
                            self.config.entropy_threshold,
                            self.config.entropy_min_length,
                        ) {
                            let fingerprint = Finding::generate_fingerprint(
                                "entropy-detection", file_name, line_num + 1, val_str
                            );

                            if !self.config.allowlist_fingerprints.contains(&fingerprint)
                                && !seen_fingerprints.contains(&fingerprint)
                            {
                                seen_fingerprints.insert(fingerprint.clone());

                                let finding = Finding {
                                    id: Finding::generate_id(&fingerprint),
                                    pattern_id: "entropy-detection".into(),
                                    pattern_name: "High-Entropy String".into(),
                                    file: file_name.to_string(),
                                    line: line_num + 1,
                                    column: value.start() + 1,
                                    masked_value: mask_secret(val_str),
                                    severity: Severity::Low,
                                    category: PatternCategory::Custom,
                                    line_content: if self.config.include_line_content {
                                        Some(truncate_line(line, self.config.max_line_length))
                                    } else {
                                        None
                                    },
                                    fingerprint,
                                };

                                if let Some(ref callback) = self.on_finding {
                                    callback(&finding);
                                }

                                output.findings.push(finding);
                            }
                        }
                    }
                }
            }
        }

        // Update stats
        output.stats.findings_count = output.findings.len();
        output.stats.duration_ms = start.elapsed().as_millis() as u64;

        for finding in &output.findings {
            *output.stats.by_severity
                .entry(finding.severity.to_string())
                .or_insert(0) += 1;
            *output.stats.by_category
                .entry(format!("{:?}", finding.category))
                .or_insert(0) += 1;
        }

        output
    }

    /// Scan a single file.
    pub fn scan_file(&self, path: impl AsRef<Path>) -> ScanOutput {
        let path = path.as_ref();
        let file_str = path.to_string_lossy();

        // Check file exclusions
        if self.config.exclude_files.iter().any(|e| file_str.contains(e)) {
            let mut output = ScanOutput::new();
            output.stats.files_skipped = 1;
            return output;
        }

        match std::fs::read_to_string(path) {
            Ok(content) => self.scan_str(&content, &file_str),
            Err(e) => {
                let mut output = ScanOutput::new();
                output.stats.files_skipped = 1;
                output.errors.push(ScanError::FileRead {
                    path: path.to_path_buf(),
                    message: e.to_string(),
                });
                output
            }
        }
    }

    /// Scan multiple files in parallel.
    pub fn scan_files(&self, paths: &[PathBuf]) -> ScanOutput {
        let start = Instant::now();

        let results: Vec<ScanOutput> = paths
            .par_iter()
            .filter(|p| p.is_file())
            .map(|path| self.scan_file(path))
            .collect();

        let mut output = ScanOutput::new();
        for result in results {
            output.merge(result);
        }

        output.stats.duration_ms = start.elapsed().as_millis() as u64;
        output.stats.pattern_version = PATTERN_VERSION.to_string();

        output
    }

    /// Scan paths (files or directories).
    pub fn scan_paths(&self, paths: &[impl AsRef<Path>]) -> ScanOutput {
        let mut all_files = Vec::new();

        for path in paths {
            let path = path.as_ref();
            if path.is_file() {
                all_files.push(path.to_path_buf());
            } else if path.is_dir() {
                for entry in walkdir::WalkDir::new(path)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    all_files.push(entry.path().to_path_buf());
                }
            }
        }

        self.scan_files(&all_files)
    }
}

// =============================================================================
// Legacy API Compatibility
// =============================================================================

/// Legacy: A detected secret match (for backwards compatibility).
#[derive(Debug, Clone)]
pub struct SecretMatch {
    /// File path where secret was found.
    pub file: String,
    /// Line number (1-indexed).
    pub line: usize,
    /// Name of the pattern that matched.
    pub pattern_name: String,
    /// Masked version of the matched text.
    pub matched_text: String,
    /// Severity level.
    pub severity: Severity,
    /// Line content (for context).
    pub line_content: Option<String>,
}

impl From<Finding> for SecretMatch {
    fn from(f: Finding) -> Self {
        Self {
            file: f.file,
            line: f.line,
            pattern_name: f.pattern_name,
            matched_text: f.masked_value,
            severity: f.severity,
            line_content: f.line_content,
        }
    }
}

/// Legacy: Statistics from a scan operation (backwards compatibility alias).
pub type LegacyScanStats = ScanStats;

/// Legacy: Scan content string for secrets.
pub fn scan_content(content: &str, file_name: &str, config: &SecretsConfig) -> Vec<SecretMatch> {
    let scanner = SecretScanner::new();
    let mut scanner = scanner;

    for pattern in &config.exclude_patterns {
        scanner = scanner.exclude_pattern(pattern);
    }
    for file in &config.exclude_files {
        scanner = scanner.exclude_file(file);
    }
    for pattern in &config.additional_patterns {
        scanner = scanner.add_pattern_regex(format!("custom-{}", pattern.len()), pattern);
    }

    scanner.scan_str(content, file_name)
        .findings
        .into_iter()
        .map(SecretMatch::from)
        .collect()
}

/// Legacy: Scan content with entropy detection.
pub fn scan_content_with_entropy(
    content: &str,
    file_name: &str,
    config: &SecretsConfig,
) -> Vec<SecretMatch> {
    let scanner = SecretScanner::new().with_entropy_detection();
    let mut scanner = scanner;

    for pattern in &config.exclude_patterns {
        scanner = scanner.exclude_pattern(pattern);
    }
    for file in &config.exclude_files {
        scanner = scanner.exclude_file(file);
    }

    scanner.scan_str(content, file_name)
        .findings
        .into_iter()
        .map(SecretMatch::from)
        .collect()
}

/// Legacy: Scan a file for secrets.
pub fn scan_file(path: &Path, config: &SecretsConfig) -> anyhow::Result<Vec<SecretMatch>> {
    let file_str = path.to_string_lossy();

    if config.exclude_files.iter().any(|e| file_str.contains(e)) {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)?;
    Ok(scan_content(&content, &file_str, config))
}

/// Legacy: Scan multiple files in parallel.
pub fn scan_files(paths: &[PathBuf], config: &SecretsConfig) -> Vec<SecretMatch> {
    scan_files_with_stats(paths, config).0
}

/// Legacy: Scan files and return statistics.
pub fn scan_files_with_stats(
    paths: &[PathBuf],
    config: &SecretsConfig,
) -> (Vec<SecretMatch>, ScanStats) {
    let scanner = SecretScanner::new();
    let mut scanner = scanner;

    for pattern in &config.exclude_patterns {
        scanner = scanner.exclude_pattern(pattern);
    }
    for file in &config.exclude_files {
        scanner = scanner.exclude_file(file);
    }

    let output = scanner.scan_files(paths);

    let matches: Vec<SecretMatch> = output.findings
        .into_iter()
        .map(SecretMatch::from)
        .collect();

    let mut stats = output.stats;
    stats.findings_count = matches.len();

    (matches, stats)
}

/// Legacy: Print scan results.
pub fn print_results(matches: &[SecretMatch]) -> i32 {
    print_results_with_stats(matches, None)
}

/// Legacy: Print scan results with statistics.
pub fn print_results_with_stats(matches: &[SecretMatch], stats: Option<&ScanStats>) -> i32 {
    if let Some(s) = stats {
        eprintln!(
            "{} Scanned {} files ({} lines) in {}ms",
            "INFO".cyan(),
            s.files_scanned,
            s.lines_scanned,
            s.duration_ms
        );
    }

    if matches.is_empty() {
        println!("{} No secrets detected", "OK".green());
        return exit_codes::SUCCESS;
    }

    eprintln!(
        "{} Found {} potential secret(s):",
        "ERROR".red(),
        matches.len()
    );
    eprintln!();

    for m in matches {
        let severity_str = match m.severity {
            Severity::Critical => "CRITICAL".red().bold().to_string(),
            Severity::High => "HIGH".red().bold().to_string(),
            Severity::Medium => "MEDIUM".yellow().to_string(),
            Severity::Low => "LOW".dimmed().to_string(),
        };

        eprintln!(
            "  [{}] {} (line {})",
            severity_str,
            m.file,
            m.line
        );
        eprintln!("    Pattern: {}", m.pattern_name.cyan());
        eprintln!("    Match: {}", m.matched_text.dimmed());

        if let Some(ref content) = m.line_content {
            eprintln!("    Line: {}", content.dimmed());
        }
        eprintln!();
    }

    exit_codes::FAILURE
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // =========================================================================
    // Pattern Tests
    // =========================================================================

    #[test]
    fn test_pattern_version() {
        assert!(!PATTERN_VERSION.is_empty());
    }

    #[test]
    fn test_builtin_patterns_count() {
        assert_eq!(BUILTIN_PATTERNS.len(), 19);
    }

    #[test]
    fn test_all_patterns_compile() {
        for def in BUILTIN_PATTERNS.iter() {
            assert!(
                Regex::new(&def.pattern).is_ok(),
                "Pattern '{}' failed to compile",
                def.id
            );
        }
    }

    #[test]
    fn test_pattern_ids_unique() {
        let mut ids = HashSet::new();
        for def in BUILTIN_PATTERNS.iter() {
            assert!(
                ids.insert(&def.id),
                "Duplicate pattern ID: {}",
                def.id
            );
        }
    }

    // =========================================================================
    // AWS Tests
    // =========================================================================

    #[test]
    fn test_aws_access_key() {
        let result = SecretScanner::new().scan_str("AKIAIOSFODNN7EXAMPLE", "test.env");
        assert!(result.has_secrets());
        assert_eq!(result.findings()[0].pattern_id, "aws-access-key");
        assert_eq!(result.findings()[0].severity, Severity::Critical);
    }

    #[test]
    fn test_aws_access_key_in_context() {
        let result = SecretScanner::new()
            .scan_str("AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE", "test.env");
        assert!(result.has_secrets());
    }

    #[test]
    fn test_aws_secret_key() {
        let content = "aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let result = SecretScanner::new().scan_str(content, "test.env");
        assert!(result.has_secrets());
        assert_eq!(result.findings()[0].pattern_id, "aws-secret-key");
    }

    // =========================================================================
    // GitHub Tests
    // =========================================================================

    #[test]
    fn test_github_token() {
        let result = SecretScanner::new()
            .scan_str("ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", "test.env");
        assert!(result.has_secrets());
        assert_eq!(result.findings()[0].pattern_id, "github-token");
    }

    #[test]
    fn test_github_token_variations() {
        for prefix in ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"] {
            let token = format!("{}xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", prefix);
            let result = SecretScanner::new().scan_str(&token, "test.env");
            assert!(result.has_secrets(), "Failed for prefix: {}", prefix);
        }
    }

    // =========================================================================
    // Database Tests
    // =========================================================================

    #[test]
    fn test_database_url() {
        let result = SecretScanner::new()
            .scan_str("postgres://user:password@localhost:5432/db", "test.env");
        assert!(result.has_secrets());
        assert_eq!(result.findings()[0].pattern_id, "database-url");
    }

    #[test]
    fn test_database_url_no_password() {
        let result = SecretScanner::new()
            .scan_str("postgres://localhost:5432/db", "test.env");
        assert!(!result.has_secrets());
    }

    // =========================================================================
    // Payment Tests
    // =========================================================================

    #[test]
    fn test_stripe_secret_key() {
        let result = SecretScanner::new()
            .scan_str("sk_test_EXAMPLEKEYDONOTUSE12345678", "test.env");
        assert!(result.has_secrets());
        assert_eq!(result.findings()[0].pattern_id, "stripe-secret-key");
        assert_eq!(result.findings()[0].severity, Severity::Critical);
    }

    #[test]
    fn test_stripe_public_key_not_matched() {
        let result = SecretScanner::new()
            .scan_str("pk_live_abc123", "test.env");
        // Public keys don't match the secret key pattern
        let stripe_findings: Vec<_> = result.findings()
            .iter()
            .filter(|f| f.pattern_id == "stripe-secret-key")
            .collect();
        assert!(stripe_findings.is_empty());
    }

    // =========================================================================
    // Severity Filter Tests
    // =========================================================================

    #[test]
    fn test_min_severity_filter() {
        let content = r#"
            AKIAIOSFODNN7EXAMPLE
            console.log("password debug")
        "#;

        let all = SecretScanner::new().scan_str(content, "test.js");
        assert_eq!(all.findings().len(), 2);

        let high_only = SecretScanner::new()
            .min_severity(Severity::High)
            .scan_str(content, "test.js");
        assert_eq!(high_only.findings().len(), 1);
        assert_eq!(high_only.findings()[0].pattern_id, "aws-access-key");
    }

    // =========================================================================
    // Exclusion Tests
    // =========================================================================

    #[test]
    fn test_exclude_pattern() {
        let content = "AKIAIOSFODNN7EXAMPLE # noqa: secrets";

        let without = SecretScanner::new().scan_str(content, "test.env");
        assert!(without.has_secrets());

        let with = SecretScanner::new()
            .exclude_pattern("noqa")
            .scan_str(content, "test.env");
        assert!(!with.has_secrets());
    }

    #[test]
    fn test_allowlist_value() {
        let result = SecretScanner::new()
            .allowlist_value("EXAMPLE")
            .scan_str("AKIAIOSFODNN7EXAMPLE", "test.env");
        assert!(!result.has_secrets());
    }

    #[test]
    fn test_allowlist_fingerprint() {
        let first = SecretScanner::new().scan_str("AKIAIOSFODNN7EXAMPLE", "test.env");
        assert!(first.has_secrets());

        let fingerprint = &first.findings()[0].fingerprint;

        let second = SecretScanner::new()
            .allowlist_fingerprint(fingerprint)
            .scan_str("AKIAIOSFODNN7EXAMPLE", "test.env");
        assert!(!second.has_secrets());
    }

    #[test]
    fn test_disable_pattern() {
        let result = SecretScanner::new()
            .disable_pattern("aws-access-key")
            .scan_str("AKIAIOSFODNN7EXAMPLE", "test.env");
        assert!(!result.has_secrets());
    }

    // =========================================================================
    // Custom Pattern Tests
    // =========================================================================

    #[test]
    fn test_custom_pattern() {
        let result = SecretScanner::new()
            .add_pattern_regex("my-secret", r"MY_SECRET_[A-Z]{10}")
            .scan_str("MY_SECRET_ABCDEFGHIJ", "test.env");
        assert!(result.has_secrets());
        assert_eq!(result.findings()[0].pattern_id, "my-secret");
    }

    #[test]
    fn test_custom_pattern_def() {
        let def = PatternDef {
            id: "custom-token".into(),
            name: "Custom Token".into(),
            pattern: r"CUSTOM_[0-9]{10}".into(),
            severity: Severity::High,
            category: PatternCategory::Custom,
            description: "Custom token format".into(),
            enabled: true,
        };

        let result = SecretScanner::new()
            .add_pattern(def)
            .scan_str("CUSTOM_1234567890", "test.env");
        assert!(result.has_secrets());
        assert_eq!(result.findings()[0].severity, Severity::High);
    }

    // =========================================================================
    // Entropy Detection Tests
    // =========================================================================

    #[test]
    fn test_entropy_detection() {
        let content = "SECRET_KEY=aB3xY9mK2pQwE8rT5nZvL4cG7hJk0MnPq";

        let without = SecretScanner::new().scan_str(content, "test.env");
        assert!(!without.has_secrets());

        let with = SecretScanner::new()
            .with_entropy_detection()
            .scan_str(content, "test.env");
        assert!(with.has_secrets());
        assert_eq!(with.findings()[0].pattern_id, "entropy-detection");
    }

    #[test]
    fn test_entropy_threshold() {
        let content = "KEY=aB3xY9mK2pQwE8rT5nZvL4cG7hJk0MnPq";

        // High threshold should not detect
        let high = SecretScanner::new()
            .with_entropy_detection()
            .entropy_threshold(6.0)
            .scan_str(content, "test.env");
        assert!(!high.has_secrets());

        // Low threshold should detect
        let low = SecretScanner::new()
            .with_entropy_detection()
            .entropy_threshold(3.0)
            .scan_str(content, "test.env");
        assert!(low.has_secrets());
    }

    // =========================================================================
    // File Scanning Tests
    // =========================================================================

    #[test]
    fn test_scan_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# Config").unwrap();
        writeln!(file, "AWS_KEY=AKIAIOSFODNN7EXAMPLE").unwrap();

        let result = SecretScanner::new().scan_file(file.path());
        assert!(result.has_secrets());
        assert_eq!(result.stats().files_scanned, 1);
    }

    #[test]
    fn test_scan_files_parallel() {
        let mut file1 = NamedTempFile::new().unwrap();
        let mut file2 = NamedTempFile::new().unwrap();

        writeln!(file1, "AKIAIOSFODNN7EXAMPLE").unwrap();
        writeln!(file2, "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").unwrap();

        let result = SecretScanner::new()
            .scan_files(&[file1.path().to_path_buf(), file2.path().to_path_buf()]);

        assert_eq!(result.findings().len(), 2);
        assert_eq!(result.stats().files_scanned, 2);
    }

    // =========================================================================
    // Finding Tests
    // =========================================================================

    #[test]
    fn test_finding_has_id() {
        let result = SecretScanner::new().scan_str("AKIAIOSFODNN7EXAMPLE", "test.env");
        assert!(result.findings()[0].id.starts_with("SEC-"));
    }

    #[test]
    fn test_finding_has_column() {
        let result = SecretScanner::new()
            .scan_str("KEY=AKIAIOSFODNN7EXAMPLE", "test.env");
        assert!(result.findings()[0].column > 1);
    }

    #[test]
    fn test_findings_by_severity() {
        let content = r#"
            AKIAIOSFODNN7EXAMPLE
            console.log("password debug")
        "#;

        let result = SecretScanner::new().scan_str(content, "test.js");
        assert_eq!(result.findings_by_severity(Severity::Critical).len(), 1);
        assert_eq!(result.findings_by_severity(Severity::Low).len(), 1);
    }

    // =========================================================================
    // Configuration Tests
    // =========================================================================

    #[test]
    fn test_config_serialization() {
        let config = ScannerConfig {
            min_severity: Some(Severity::High),
            exclude_patterns: vec!["noqa".into()],
            ..Default::default()
        };

        let toml = config.to_toml();
        assert!(toml.contains("min_severity"));

        let json = config.to_json();
        assert!(json.contains("min_severity"));
    }

    #[test]
    fn test_config_from_toml() {
        let toml = r#"
            min_severity = "high"
            exclude_patterns = ["noqa"]
        "#;

        let config = ScannerConfig::from_toml(toml).unwrap();
        assert_eq!(config.min_severity, Some(Severity::High));
        assert_eq!(config.exclude_patterns, vec!["noqa"]);
    }

    // =========================================================================
    // Callback Tests
    // =========================================================================

    #[test]
    fn test_on_finding_callback() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        let _ = SecretScanner::new()
            .on_finding(move |_| {
                count_clone.fetch_add(1, Ordering::SeqCst);
            })
            .scan_str("AKIAIOSFODNN7EXAMPLE", "test.env");

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    // =========================================================================
    // Masking Tests
    // =========================================================================

    #[test]
    fn test_mask_secret_short() {
        assert_eq!(mask_secret("abc"), "***");
    }

    #[test]
    fn test_mask_secret_long() {
        let masked = mask_secret("abcdefghijklmnop");
        assert!(masked.starts_with("abcd"));
        assert!(masked.ends_with("mnop"));
        assert!(masked.contains("..."));
    }

    #[test]
    fn test_mask_secret_boundary() {
        assert_eq!(mask_secret("12345678"), "********");
        assert_eq!(mask_secret("123456789"), "1234...6789");
    }

    // =========================================================================
    // Legacy API Tests
    // =========================================================================

    #[test]
    fn test_legacy_scan_content() {
        let config = SecretsConfig::default();
        let matches = scan_content("AKIAIOSFODNN7EXAMPLE", "test.env", &config);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern_name, "AWS Access Key");
    }

    #[test]
    fn test_legacy_scan_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "AKIAIOSFODNN7EXAMPLE").unwrap();

        let config = SecretsConfig::default();
        let matches = scan_file(file.path(), &config).unwrap();
        assert_eq!(matches.len(), 1);
    }

    // =========================================================================
    // Property Tests
    // =========================================================================

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn fuzz_mask_secret_never_panics(s in ".*") {
                let _ = mask_secret(&s);
            }

            #[test]
            fn fuzz_scanner_never_panics(s in ".*") {
                let _ = SecretScanner::new().scan_str(&s, "test.txt");
            }

            #[test]
            fn fuzz_entropy_never_panics(s in ".*") {
                let _ = shannon_entropy(&s);
            }
        }
    }
}
