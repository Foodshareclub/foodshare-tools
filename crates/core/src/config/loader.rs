//! Configuration file loading

use super::schema::ConfigSchema;
use super::unknown;
use crate::error::{Error, Result};
use std::path::Path;

/// Configuration wrapper
#[derive(Debug, Clone)]
pub struct Config {
    /// The parsed configuration schema
    pub schema: ConfigSchema,
    /// Path to the configuration file, if loaded from file
    pub path: Option<String>,
    /// The keys in the file that this build does not implement
    ///
    /// They are reported, not rejected. A repository that writes a section
    /// the tool does not read should still be able to run the tool; it
    /// should just learn that the section does nothing.
    pub unknown_keys: Vec<String>,
}

impl Config {
    /// Load configuration from a file path or use defaults
    pub fn load(path: Option<&str>) -> Result<Self> {
        let config_path = path.map(String::from).or_else(find_config_file);

        let (schema, unknown_keys) = if let Some(ref p) = config_path {
            load_config_file(p)?
        } else {
            (ConfigSchema::default(), Vec::new())
        };

        Ok(Self {
            schema,
            path: config_path,
            unknown_keys,
        })
    }

    /// Load with defaults only (no file)
    #[must_use]
    pub fn default() -> Self {
        Self {
            schema: ConfigSchema::default(),
            path: None,
            unknown_keys: Vec::new(),
        }
    }

    /// The report of the keys this build did not implement.
    ///
    /// Empty when the configuration is complete, or when there was no
    /// configuration file to read.
    #[must_use]
    pub fn unknown_keys_report(&self) -> String {
        if self.unknown_keys.is_empty() {
            return String::new();
        }

        let where_ = self.path.as_deref().unwrap_or("the configuration");
        let mut report = format!(
            "{where_}: {} configuration key(s) are not implemented by this tool and were ignored:",
            self.unknown_keys.len()
        );
        for key in &self.unknown_keys {
            report.push_str(&format!("\n  {key}"));
        }
        report
    }
}

/// Find configuration file in standard locations
fn find_config_file() -> Option<String> {
    let candidates = [
        ".foodshare-hooks.toml",
        "foodshare-hooks.toml",
        ".config/foodshare-hooks.toml",
    ];

    for candidate in candidates {
        if Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }

    None
}

/// Load and parse a TOML configuration file
fn load_config_file(path: &str) -> Result<(ConfigSchema, Vec<String>)> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::config(format!("Failed to read config file {path}: {e}")))?;

    let schema: ConfigSchema = toml::from_str(&content)
        .map_err(|e| Error::config(format!("Failed to parse config file {path}: {e}")))?;

    // The keys the file writes that this build does not implement. The
    // report is best effort: a file that parses is a file the tool can
    // work with, and a report that cannot be produced is not a reason to
    // refuse to run.
    let unknown_keys = unknown::unknown_keys_in(&content, &schema).unwrap_or_default();

    Ok((schema, unknown_keys))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.path.is_none());
        assert_eq!(config.schema.commit_msg.max_length, 72);
    }

    #[test]
    fn test_config_load_missing_file() {
        let config = Config::load(None);
        assert!(config.is_ok());
    }

    #[test]
    fn test_secrets_patterns_key_is_honoured() {
        // Every repository in the estate writes its extra patterns as
        // `patterns = [...]`. The key used to be dropped on the floor
        // because the struct declared a different name for it, and a
        // repository that believes it has extra patterns and has none is
        // worse than one that knows it has none.
        let config: ConfigSchema = toml::from_str(
            r#"
[secrets]
patterns = [
    "(?i)(secret|credential)\\s*[=:]\\s*[\"'][^\"']{8,}[\"']",
]
exclude_files = ["compose.yaml"]
"#,
        )
        .expect("the configuration should parse");
        assert_eq!(
            config.secrets.additional_patterns.len(),
            1,
            "the declared pattern should have been read"
        );
        assert!(config.secrets.additional_patterns[0].contains("credential"));
        assert_eq!(
            config.secrets.exclude_files,
            vec!["compose.yaml".to_string()]
        );
    }

    #[test]
    fn test_secrets_additional_patterns_key_still_works() {
        let config: ConfigSchema = toml::from_str(
            r#"
[secrets]
additional_patterns = ["a-secret-shape"]
"#,
        )
        .expect("the configuration should parse");
        assert_eq!(config.secrets.additional_patterns, vec!["a-secret-shape"]);
    }
}
