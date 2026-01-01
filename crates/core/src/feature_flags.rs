//! Feature flags for runtime configuration
//!
//! Provides a simple feature flag system for:
//! - Gradual rollouts
//! - A/B testing
//! - Kill switches
//! - Environment-based features
//!
//! # Example
//!
//! ```rust,ignore
//! use foodshare_core::feature_flags::{FeatureFlags, Flag};
//!
//! let flags = FeatureFlags::new()
//!     .with_flag("new_ui", true)
//!     .with_flag("experimental", false);
//!
//! if flags.is_enabled("new_ui") {
//!     // Use new UI
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Feature flag value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlagValue {
    /// Boolean flag
    Bool(bool),
    /// String value
    String(String),
    /// Numeric value
    Number(f64),
    /// Percentage rollout (0-100)
    Percentage(u8),
}

impl FlagValue {
    /// Check if flag is enabled (truthy)
    #[must_use] pub fn is_enabled(&self) -> bool {
        match self {
            FlagValue::Bool(b) => *b,
            FlagValue::String(s) => !s.is_empty() && s != "false" && s != "0",
            FlagValue::Number(n) => *n != 0.0,
            FlagValue::Percentage(p) => *p > 0,
        }
    }

    /// Get as boolean
    #[must_use] pub fn as_bool(&self) -> bool {
        self.is_enabled()
    }

    /// Get as string
    #[must_use] pub fn as_string(&self) -> String {
        match self {
            FlagValue::Bool(b) => b.to_string(),
            FlagValue::String(s) => s.clone(),
            FlagValue::Number(n) => n.to_string(),
            FlagValue::Percentage(p) => format!("{p}%"),
        }
    }

    /// Get as number
    #[must_use] pub fn as_number(&self) -> Option<f64> {
        match self {
            FlagValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            FlagValue::String(s) => s.parse().ok(),
            FlagValue::Number(n) => Some(*n),
            FlagValue::Percentage(p) => Some(f64::from(*p)),
        }
    }
}

/// Feature flag definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flag {
    /// Flag name
    pub name: String,
    /// Flag value
    pub value: FlagValue,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Environment override variable
    #[serde(default)]
    pub env_var: Option<String>,
    /// Tags for grouping
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Flag {
    /// Create a new boolean flag
    pub fn bool(name: impl Into<String>, value: bool) -> Self {
        Self {
            name: name.into(),
            value: FlagValue::Bool(value),
            description: String::new(),
            env_var: None,
            tags: Vec::new(),
        }
    }

    /// Create a new string flag
    pub fn string(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: FlagValue::String(value.into()),
            description: String::new(),
            env_var: None,
            tags: Vec::new(),
        }
    }

    /// Create a percentage rollout flag
    pub fn percentage(name: impl Into<String>, percent: u8) -> Self {
        Self {
            name: name.into(),
            value: FlagValue::Percentage(percent.min(100)),
            description: String::new(),
            env_var: None,
            tags: Vec::new(),
        }
    }

    /// Add description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add environment variable override
    pub fn with_env_var(mut self, var: impl Into<String>) -> Self {
        self.env_var = Some(var.into());
        self
    }

    /// Add tags
    #[must_use] pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Check if enabled (considering env override)
    #[must_use] pub fn is_enabled(&self) -> bool {
        // Check environment override first
        if let Some(ref env_var) = self.env_var {
            if let Ok(val) = env::var(env_var) {
                return val != "0" && val.to_lowercase() != "false";
            }
        }
        self.value.is_enabled()
    }

    /// Check if enabled for a specific user/key (for percentage rollouts)
    #[must_use] pub fn is_enabled_for(&self, key: &str) -> bool {
        match &self.value {
            FlagValue::Percentage(p) => {
                // Simple hash-based rollout
                let hash = key.bytes().fold(0u64, |acc, b| acc.wrapping_add(u64::from(b)));
                (hash % 100) < u64::from(*p)
            }
            _ => self.is_enabled(),
        }
    }
}

/// Feature flags manager
pub struct FeatureFlags {
    flags: Arc<RwLock<HashMap<String, Flag>>>,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureFlags {
    /// Create a new feature flags manager
    #[must_use] pub fn new() -> Self {
        Self {
            flags: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load flags from a JSON file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let content = fs::read_to_string(path)?;
        let flags: HashMap<String, Flag> = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(Self {
            flags: Arc::new(RwLock::new(flags)),
        })
    }

    /// Load flags from environment variables with a prefix
    #[must_use] pub fn from_env(prefix: &str) -> Self {
        let mut flags = HashMap::new();

        for (key, value) in env::vars() {
            if key.starts_with(prefix) {
                let flag_name = key[prefix.len()..].to_lowercase().replace('_', "-");
                let flag_value = if value == "true" || value == "1" {
                    FlagValue::Bool(true)
                } else if value == "false" || value == "0" {
                    FlagValue::Bool(false)
                } else if let Ok(n) = value.parse::<f64>() {
                    FlagValue::Number(n)
                } else {
                    FlagValue::String(value)
                };

                flags.insert(
                    flag_name.clone(),
                    Flag {
                        name: flag_name,
                        value: flag_value,
                        description: String::new(),
                        env_var: Some(key),
                        tags: Vec::new(),
                    },
                );
            }
        }

        Self {
            flags: Arc::new(RwLock::new(flags)),
        }
    }

    /// Add a flag
    pub fn with_flag(self, name: impl Into<String>, value: bool) -> Self {
        let name = name.into();
        let mut flags = self.flags.write().unwrap();
        flags.insert(name.clone(), Flag::bool(name, value));
        drop(flags);
        self
    }

    /// Add a flag definition
    #[must_use] pub fn with_flag_def(self, flag: Flag) -> Self {
        let mut flags = self.flags.write().unwrap();
        flags.insert(flag.name.clone(), flag);
        drop(flags);
        self
    }

    /// Check if a flag is enabled
    #[must_use] pub fn is_enabled(&self, name: &str) -> bool {
        let flags = self.flags.read().unwrap();
        flags.get(name).is_some_and(Flag::is_enabled)
    }

    /// Check if a flag is enabled for a specific key
    #[must_use] pub fn is_enabled_for(&self, name: &str, key: &str) -> bool {
        let flags = self.flags.read().unwrap();
        flags
            .get(name)
            .is_some_and(|f| f.is_enabled_for(key))
    }

    /// Get a flag value
    #[must_use] pub fn get(&self, name: &str) -> Option<FlagValue> {
        let flags = self.flags.read().unwrap();
        flags.get(name).map(|f| f.value.clone())
    }

    /// Get a flag as string
    #[must_use] pub fn get_string(&self, name: &str) -> Option<String> {
        let flags = self.flags.read().unwrap();
        flags.get(name).map(|f| f.value.as_string())
    }

    /// Get a flag as number
    #[must_use] pub fn get_number(&self, name: &str) -> Option<f64> {
        let flags = self.flags.read().unwrap();
        flags.get(name).and_then(|f| f.value.as_number())
    }

    /// Set a flag value at runtime
    pub fn set(&self, name: &str, value: bool) {
        let mut flags = self.flags.write().unwrap();
        if let Some(flag) = flags.get_mut(name) {
            flag.value = FlagValue::Bool(value);
        } else {
            flags.insert(name.to_string(), Flag::bool(name, value));
        }
    }

    /// List all flags
    #[must_use] pub fn list(&self) -> Vec<Flag> {
        let flags = self.flags.read().unwrap();
        flags.values().cloned().collect()
    }

    /// List flags by tag
    #[must_use] pub fn list_by_tag(&self, tag: &str) -> Vec<Flag> {
        let flags = self.flags.read().unwrap();
        flags
            .values()
            .filter(|f| f.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// Export flags to JSON
    #[must_use] pub fn to_json(&self) -> String {
        let flags = self.flags.read().unwrap();
        serde_json::to_string_pretty(&*flags).unwrap_or_default()
    }

    /// Reload flags from file
    pub fn reload(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let content = fs::read_to_string(path)?;
        let new_flags: HashMap<String, Flag> = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut flags = self.flags.write().unwrap();
        *flags = new_flags;
        Ok(())
    }
}

/// Default feature flags for the tooling
#[must_use] pub fn default_flags() -> FeatureFlags {
    FeatureFlags::new()
        .with_flag_def(
            Flag::bool("telemetry", true)
                .with_description("Enable telemetry collection")
                .with_env_var("FOODSHARE_TELEMETRY"),
        )
        .with_flag_def(
            Flag::bool("color-output", true)
                .with_description("Enable colored terminal output")
                .with_env_var("FOODSHARE_COLOR"),
        )
        .with_flag_def(
            Flag::bool("parallel-execution", true)
                .with_description("Enable parallel task execution")
                .with_env_var("FOODSHARE_PARALLEL"),
        )
        .with_flag_def(
            Flag::bool("cache", true)
                .with_description("Enable caching")
                .with_env_var("FOODSHARE_CACHE"),
        )
        .with_flag_def(
            Flag::bool("strict-mode", false)
                .with_description("Enable strict validation mode")
                .with_env_var("FOODSHARE_STRICT"),
        )
        .with_flag_def(
            Flag::bool("debug", false)
                .with_description("Enable debug output")
                .with_env_var("FOODSHARE_DEBUG"),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_bool() {
        let flag = Flag::bool("test", true);
        assert!(flag.is_enabled());

        let flag = Flag::bool("test", false);
        assert!(!flag.is_enabled());
    }

    #[test]
    fn test_flag_string() {
        let flag = Flag::string("test", "value");
        assert!(flag.is_enabled());

        let flag = Flag::string("test", "");
        assert!(!flag.is_enabled());

        let flag = Flag::string("test", "false");
        assert!(!flag.is_enabled());
    }

    #[test]
    fn test_flag_percentage() {
        let flag = Flag::percentage("test", 100);
        assert!(flag.is_enabled_for("any_user"));

        let flag = Flag::percentage("test", 0);
        assert!(!flag.is_enabled_for("any_user"));
    }

    #[test]
    fn test_feature_flags_basic() {
        let flags = FeatureFlags::new()
            .with_flag("enabled", true)
            .with_flag("disabled", false);

        assert!(flags.is_enabled("enabled"));
        assert!(!flags.is_enabled("disabled"));
        assert!(!flags.is_enabled("nonexistent"));
    }

    #[test]
    fn test_feature_flags_set() {
        let flags = FeatureFlags::new().with_flag("test", false);

        assert!(!flags.is_enabled("test"));
        flags.set("test", true);
        assert!(flags.is_enabled("test"));
    }

    #[test]
    fn test_feature_flags_list() {
        let flags = FeatureFlags::new()
            .with_flag("a", true)
            .with_flag("b", false);

        let list = flags.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_default_flags() {
        let flags = default_flags();
        assert!(flags.is_enabled("telemetry"));
        assert!(flags.is_enabled("color-output"));
        assert!(!flags.is_enabled("debug"));
    }

    #[test]
    fn test_flag_with_tags() {
        let flags = FeatureFlags::new()
            .with_flag_def(Flag::bool("feature1", true).with_tags(vec!["ui".to_string()]))
            .with_flag_def(Flag::bool("feature2", true).with_tags(vec!["api".to_string()]))
            .with_flag_def(Flag::bool("feature3", true).with_tags(vec!["ui".to_string()]));

        let ui_flags = flags.list_by_tag("ui");
        assert_eq!(ui_flags.len(), 2);
    }
}
