use crate::error::{Result, SwiftError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwiftConfig {
    pub required_version: String,
    pub toolchain_path: Option<PathBuf>,
    pub auto_configure: bool,
}

impl SwiftConfig {
    /// Load configuration from .swift-version file
    pub fn from_swift_version_file(project_root: &Path) -> Result<Self> {
        let swift_version_file = project_root.join(".swift-version");
        
        if swift_version_file.exists() {
            let content = std::fs::read_to_string(&swift_version_file)?;
            let version = content.trim();
            
            // Extract version number from snapshot name
            let required_version = if version.contains("6.3") {
                "6.3".to_string()
            } else if version.contains("6.2") {
                "6.2".to_string()
            } else {
                version.to_string()
            };

            Ok(Self {
                required_version,
                toolchain_path: None,
                auto_configure: true,
            })
        } else {
            Ok(Self::default())
        }
    }

    /// Generate shell export commands for environment configuration
    pub fn generate_env_exports(&self, toolchain_path: &Path) -> Vec<String> {
        vec![
            "export TOOLCHAINS=swift".to_string(),
            format!(
                "export PATH=\"{}/usr/bin:$PATH\"",
                toolchain_path.display()
            ),
        ]
    }
}

impl Default for SwiftConfig {
    fn default() -> Self {
        Self {
            required_version: crate::REQUIRED_SWIFT_VERSION.to_string(),
            toolchain_path: None,
            auto_configure: true,
        }
    }
}
