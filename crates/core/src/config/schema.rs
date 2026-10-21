//! Configuration schema definitions
//!
//! Shared configuration types for all platforms.
//!
//! Every field here is read by something. A section that no tool consumes
//! is not declared here on purpose: it is reported as unknown by the
//! loader, so a repository that writes it learns that it has no effect
//! instead of believing that it does.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Root configuration schema
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigSchema {
    /// General project configuration
    #[serde(default)]
    pub general: GeneralConfig,

    /// Commit message validation configuration
    #[serde(default)]
    pub commit_msg: CommitMsgConfig,

    /// Code analysis configuration
    #[serde(default)]
    pub analyze: AnalyzeConfig,

    /// Test configuration
    #[serde(default)]
    pub test: TestConfig,

    /// Secrets scanning configuration
    #[serde(default)]
    pub secrets: SecretsConfig,

    /// Formatting configuration
    #[serde(default)]
    pub format: FormatConfig,

    /// Linting configuration
    #[serde(default)]
    pub lint: LintConfig,

    /// Migration configuration
    #[serde(default)]
    pub migrations: MigrationsConfig,

    /// Pre-push check configuration
    #[serde(default)]
    pub pre_push: PrePushConfig,
}

/// General project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Project name
    #[serde(default = "default_project_name")]
    pub project_name: String,

    /// Source directory
    #[serde(default = "default_source_dir")]
    pub source_dir: String,

    /// Swift version the project builds against
    #[serde(default)]
    pub swift_version: Option<String>,

    /// Xcode scheme to build and test
    #[serde(default)]
    pub scheme: Option<String>,

    /// Xcode destination to build for
    #[serde(default)]
    pub destination: Option<String>,

    /// Simulator device name
    #[serde(default)]
    pub device: Option<String>,

    /// App bundle identifier
    #[serde(default)]
    pub bundle_id: Option<String>,

    /// Android package name
    #[serde(default)]
    pub package_name: Option<String>,

    /// Paths excluded from every operation
    #[serde(default)]
    pub exclude_paths: Vec<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            project_name: default_project_name(),
            source_dir: default_source_dir(),
            swift_version: None,
            scheme: None,
            destination: None,
            device: None,
            bundle_id: None,
            package_name: None,
            exclude_paths: Vec::new(),
        }
    }
}

fn default_project_name() -> String {
    "FoodShare".to_string()
}

fn default_source_dir() -> String {
    ".".to_string()
}

/// Commit message validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitMsgConfig {
    /// Allowed commit types
    #[serde(default = "default_commit_types")]
    pub types: Vec<String>,

    /// Maximum subject line length
    #[serde(default = "default_max_length")]
    pub max_length: usize,

    /// Minimum subject line length
    #[serde(default = "default_min_length")]
    pub min_length: usize,

    /// Skip validation for merge commits
    #[serde(default = "default_true")]
    pub skip_merge: bool,

    /// Skip validation for revert commits
    #[serde(default = "default_true")]
    pub skip_revert: bool,
}

impl Default for CommitMsgConfig {
    fn default() -> Self {
        Self {
            types: default_commit_types(),
            max_length: default_max_length(),
            min_length: default_min_length(),
            skip_merge: true,
            skip_revert: true,
        }
    }
}

fn default_commit_types() -> Vec<String> {
    vec![
        "feat", "fix", "docs", "style", "refactor", "test", "chore", "perf", "ci", "build",
        "revert",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_max_length() -> usize {
    72
}

fn default_min_length() -> usize {
    10
}

fn default_true() -> bool {
    true
}

/// Code analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeConfig {
    /// Maximum TODOs before warning
    #[serde(default = "default_threshold_todos")]
    pub threshold_todos: usize,

    /// Maximum FIXMEs before warning
    #[serde(default = "default_threshold_fixmes")]
    pub threshold_fixmes: usize,

    /// Maximum force unwraps (Swift) or nullable bangs (Kotlin) before warning
    ///
    /// The alias is the name the estate writes. The schema called it
    /// `threshold_force` and the repositories call it
    /// `threshold_force_unwraps`, so the key they wrote was dropped and
    /// the threshold they set was the default.
    #[serde(default = "default_threshold_force", alias = "threshold_force_unwraps")]
    pub threshold_force: usize,

    /// Paths to analyze
    #[serde(default)]
    pub paths: Vec<String>,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            threshold_todos: default_threshold_todos(),
            threshold_fixmes: default_threshold_fixmes(),
            threshold_force: default_threshold_force(),
            paths: Vec::new(),
        }
    }
}

fn default_threshold_todos() -> usize {
    10
}

fn default_threshold_fixmes() -> usize {
    5
}

fn default_threshold_force() -> usize {
    20
}

/// Test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Minimum coverage threshold (percentage)
    #[serde(default = "default_coverage_threshold")]
    pub coverage_threshold: u8,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            coverage_threshold: default_coverage_threshold(),
        }
    }
}

fn default_coverage_threshold() -> u8 {
    70
}

/// Secrets scanning configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecretsConfig {
    /// Additional patterns to check
    ///
    /// The alias is what the repositories write. Every `.foodshare-hooks.toml`
    /// in the estate declares its extra patterns as `patterns = [...]`, and
    /// that key was silently dropped: serde ignores a field the struct does
    /// not declare, and nothing in the loader complained. A repository
    /// believed it had thirteen extra patterns and had none. The alias makes
    /// the declaration mean what it says.
    ///
    /// The patterns are raw regular expressions matched against a line, so
    /// they have to carry their own context. A bare keyword matches every
    /// line that mentions the word.
    #[serde(default, alias = "patterns")]
    pub additional_patterns: Vec<String>,

    /// Patterns to exclude from scanning
    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    /// Files to exclude from scanning
    #[serde(default)]
    pub exclude_files: Vec<String>,
}

/// Formatting configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FormatConfig {
    /// Stage the files that were formatted
    #[serde(default)]
    pub stage_fixed: bool,

    /// Paths to format
    #[serde(default)]
    pub paths: Vec<String>,
}

/// Linting configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LintConfig {
    /// Treat warnings as failures
    #[serde(default)]
    pub strict: bool,

    /// Reporter format
    #[serde(default)]
    pub reporter: Option<String>,

    /// Paths to lint
    #[serde(default)]
    pub paths: Vec<String>,
}

/// Migration configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MigrationsConfig {
    /// Directory the migration files live in
    #[serde(default)]
    pub migrations_dir: Option<String>,
}

/// Pre-push check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrePushConfig {
    /// Stop on the first failure
    #[serde(default = "default_true")]
    pub fail_fast: bool,

    /// Build configuration the checks use
    #[serde(default)]
    pub configuration: Option<String>,

    /// Timeout for a check, in seconds
    #[serde(default)]
    pub timeout: Option<u64>,

    /// Timeout for a named check, in seconds
    ///
    /// The names are the checks the runner knows: `lint`, `build` and
    /// `test`. A name that is not one of those is reported as unknown
    /// rather than quietly ignored.
    #[serde(default)]
    pub timeouts: BTreeMap<String, u64>,
}

impl Default for PrePushConfig {
    fn default() -> Self {
        Self {
            fail_fast: true,
            configuration: None,
            timeout: None,
            timeouts: BTreeMap::new(),
        }
    }
}
