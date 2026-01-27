use crate::error::{Result, SwiftError};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Represents a Swift version
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwiftVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: Option<u32>,
    pub is_dev: bool,
    pub raw: String,
}

impl SwiftVersion {
    /// Parse Swift version from string
    pub fn parse(version_str: &str) -> Result<Self> {
        let version_str = version_str.trim();
        
        // Handle "Apple Swift version 6.3-dev" format
        let version_part = version_str
            .split_whitespace()
            .find(|s| s.chars().next().map_or(false, |c| c.is_numeric()))
            .ok_or_else(|| SwiftError::ParseError(version_str.to_string()))?;

        let is_dev = version_part.contains("-dev") || version_part.contains("DEVELOPMENT");
        let clean_version = version_part.split('-').next().unwrap_or(version_part);

        let parts: Vec<&str> = clean_version.split('.').collect();
        
        if parts.len() < 2 {
            return Err(SwiftError::ParseError(version_str.to_string()));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| SwiftError::ParseError(version_str.to_string()))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| SwiftError::ParseError(version_str.to_string()))?;
        let patch = parts.get(2).and_then(|p| p.parse().ok());

        Ok(Self {
            major,
            minor,
            patch,
            is_dev,
            raw: version_str.to_string(),
        })
    }

    /// Check if this version matches the required version (major.minor)
    pub fn matches(&self, required: &str) -> bool {
        let parts: Vec<&str> = required.split('.').collect();
        if parts.len() < 2 {
            return false;
        }

        let req_major: u32 = parts[0].parse().unwrap_or(0);
        let req_minor: u32 = parts[1].parse().unwrap_or(0);

        self.major == req_major && self.minor == req_minor
    }

    /// Format as major.minor
    pub fn short_version(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

/// Represents a Swift toolchain installation
#[derive(Debug, Clone)]
pub struct SwiftToolchain {
    pub version: SwiftVersion,
    pub path: PathBuf,
    pub is_xcode: bool,
}

impl SwiftToolchain {
    /// Detect the currently active Swift toolchain
    pub fn detect_active() -> Result<Self> {
        let output = Command::new("swift")
            .arg("--version")
            .output()
            .map_err(|_| SwiftError::SwiftNotFound)?;

        if !output.status.success() {
            return Err(SwiftError::SwiftNotFound);
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = SwiftVersion::parse(&version_str)?;

        // Try to find the swift binary path
        let swift_path = which::which("swift")
            .map_err(|_| SwiftError::SwiftNotFound)?;

        let is_xcode = swift_path.to_string_lossy().contains("Xcode.app");

        Ok(Self {
            version,
            path: swift_path,
            is_xcode,
        })
    }

    /// Find Swift toolchain at specific path
    pub fn from_path(toolchain_path: &Path) -> Result<Self> {
        let swift_bin = toolchain_path.join("usr/bin/swift");
        
        if !swift_bin.exists() {
            return Err(SwiftError::ToolchainNotFound(
                toolchain_path.display().to_string(),
            ));
        }

        let output = Command::new(&swift_bin)
            .arg("--version")
            .output()
            .map_err(|e| SwiftError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(SwiftError::CommandFailed(
                "Failed to get Swift version".to_string(),
            ));
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = SwiftVersion::parse(&version_str)?;

        Ok(Self {
            version,
            path: swift_bin,
            is_xcode: false,
        })
    }

    /// List all available Swift toolchains on macOS
    pub fn list_available() -> Result<Vec<Self>> {
        let mut toolchains = Vec::new();

        // Check user toolchains
        let user_toolchains = dirs::home_dir()
            .map(|h| h.join("Library/Developer/Toolchains"))
            .filter(|p| p.exists());

        if let Some(toolchains_dir) = user_toolchains {
            if let Ok(entries) = std::fs::read_dir(toolchains_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("xctoolchain") {
                        if let Ok(toolchain) = Self::from_path(&entry.path()) {
                            toolchains.push(toolchain);
                        }
                    }
                }
            }
        }

        // Check system toolchains
        let system_toolchains = Path::new("/Library/Developer/Toolchains");
        if system_toolchains.exists() {
            if let Ok(entries) = std::fs::read_dir(system_toolchains) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("xctoolchain") {
                        if let Ok(toolchain) = Self::from_path(&entry.path()) {
                            toolchains.push(toolchain);
                        }
                    }
                }
            }
        }

        Ok(toolchains)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_swift_version() {
        let version = SwiftVersion::parse("Apple Swift version 6.3-dev (LLVM 478f55c39d6bc2c, Swift a15423cb66d4749)").unwrap();
        assert_eq!(version.major, 6);
        assert_eq!(version.minor, 3);
        assert!(version.is_dev);
    }

    #[test]
    fn test_parse_stable_version() {
        let version = SwiftVersion::parse("Apple Swift version 6.2.3 (swiftlang-6.2.3.3.21 clang-1700.6.3.2)").unwrap();
        assert_eq!(version.major, 6);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, Some(3));
        assert!(!version.is_dev);
    }

    #[test]
    fn test_version_matches() {
        let version = SwiftVersion::parse("6.3-dev").unwrap();
        assert!(version.matches("6.3"));
        assert!(!version.matches("6.2"));
    }
}
