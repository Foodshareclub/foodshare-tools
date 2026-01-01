//! Configuration schema definitions
//!
//! Shared configuration types for all platforms.

use serde::{Deserialize, Serialize};

/// Root configuration schema
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigSchema {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub commit_msg: CommitMsgConfig,

    #[serde(default)]
    pub analyze: AnalyzeConfig,

    #[serde(default)]
    pub test: TestConfig,

    #[serde(default)]
    pub secrets: SecretsConfig,
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
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            project_name: default_project_name(),
            source_dir: default_source_dir(),
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
    #[serde(default = "default_threshold_force")]
    pub threshold_force: usize,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            threshold_todos: default_threshold_todos(),
            threshold_fixmes: default_threshold_fixmes(),
            threshold_force: default_threshold_force(),
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
    #[serde(default)]
    pub additional_patterns: Vec<String>,

    /// Patterns to exclude from scanning
    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    /// Files to exclude from scanning
    #[serde(default)]
    pub exclude_files: Vec<String>,
}
