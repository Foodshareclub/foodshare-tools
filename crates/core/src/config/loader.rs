//! Configuration file loading

use super::schema::ConfigSchema;
use crate::error::{Error, Result};
use std::path::Path;

/// Configuration wrapper
#[derive(Debug, Clone)]
pub struct Config {
    pub schema: ConfigSchema,
    pub path: Option<String>,
}

impl Config {
    /// Load configuration from a file path or use defaults
    pub fn load(path: Option<&str>) -> Result<Self> {
        let config_path = path
            .map(String::from)
            .or_else(|| find_config_file());

        let schema = if let Some(ref p) = config_path {
            load_config_file(p)?
        } else {
            ConfigSchema::default()
        };

        Ok(Self {
            schema,
            path: config_path,
        })
    }

    /// Load with defaults only (no file)
    pub fn default() -> Self {
        Self {
            schema: ConfigSchema::default(),
            path: None,
        }
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
fn load_config_file(path: &str) -> Result<ConfigSchema> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::Config(format!("Failed to read config file {}: {}", path, e)))?;

    toml::from_str(&content)
        .map_err(|e| Error::Config(format!("Failed to parse config file {}: {}", path, e)))
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
}
