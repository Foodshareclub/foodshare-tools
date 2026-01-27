use crate::detect::{SwiftToolchain, SwiftVersion};
use crate::error::{Result, SwiftError};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSwiftInfo {
    pub path: PathBuf,
    pub tools_version: String,
    pub matches_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XcodeProjectInfo {
    pub path: PathBuf,
    pub swift_version: String,
    pub configuration_count: usize,
    pub matches_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub installed_version: String,
    pub required_version: String,
    pub active_toolchain: String,
    pub package_files: Vec<PackageSwiftInfo>,
    pub xcode_projects: Vec<XcodeProjectInfo>,
    pub all_match: bool,
    pub issues: Vec<String>,
}

impl VerificationReport {
    /// Verify Swift version consistency across the project
    pub fn generate(project_root: &Path, required_version: &str) -> Result<Self> {
        let toolchain = SwiftToolchain::detect_active()?;
        let installed_version = toolchain.version.short_version();
        let active_toolchain = toolchain.path.display().to_string();

        let mut package_files = Vec::new();
        let mut xcode_projects = Vec::new();
        let mut issues = Vec::new();

        // Find all Package.swift files
        for entry in WalkDir::new(project_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.')
                    && name != "build"
                    && name != "SourcePackages"
                    && name != "swift-android-contributions"
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == "Package.swift" {
                if let Ok(info) = Self::check_package_swift(entry.path(), required_version) {
                    if !info.matches_required {
                        issues.push(format!(
                            "Package.swift at {} uses version {} (expected {})",
                            entry.path().display(),
                            info.tools_version,
                            required_version
                        ));
                    }
                    package_files.push(info);
                }
            }
        }

        // Find Xcode projects
        for entry in WalkDir::new(project_root)
            .max_depth(3)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                == Some("xcodeproj")
            {
                if let Ok(info) = Self::check_xcode_project(entry.path(), required_version) {
                    if !info.matches_required {
                        issues.push(format!(
                            "Xcode project at {} uses version {} (expected {})",
                            entry.path().display(),
                            info.swift_version,
                            required_version
                        ));
                    }
                    xcode_projects.push(info);
                }
            }
        }

        // Check if installed version matches required
        if !toolchain.version.matches(required_version) {
            issues.push(format!(
                "Installed Swift version {} does not match required version {}",
                installed_version, required_version
            ));
        }

        let all_match = issues.is_empty();

        Ok(Self {
            installed_version,
            required_version: required_version.to_string(),
            active_toolchain,
            package_files,
            xcode_projects,
            all_match,
            issues,
        })
    }

    fn check_package_swift(path: &Path, required_version: &str) -> Result<PackageSwiftInfo> {
        let content = std::fs::read_to_string(path)?;
        
        // Extract swift-tools-version from first line
        let tools_version = content
            .lines()
            .next()
            .and_then(|line| {
                if line.contains("swift-tools-version") {
                    line.split(':')
                        .nth(1)
                        .map(|v| v.trim().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "unknown".to_string());

        let matches_required = tools_version.starts_with(required_version);

        Ok(PackageSwiftInfo {
            path: path.to_path_buf(),
            tools_version,
            matches_required,
        })
    }

    fn check_xcode_project(path: &Path, required_version: &str) -> Result<XcodeProjectInfo> {
        let pbxproj = path.join("project.pbxproj");
        if !pbxproj.exists() {
            return Err(SwiftError::InvalidPackageFile(
                "project.pbxproj not found".to_string(),
            ));
        }

        let content = std::fs::read_to_string(&pbxproj)?;
        
        // Count SWIFT_VERSION occurrences
        let mut swift_versions = Vec::new();
        for line in content.lines() {
            if line.contains("SWIFT_VERSION = ") {
                if let Some(version) = line.split("SWIFT_VERSION = ").nth(1) {
                    let version = version.trim_end_matches(';').trim();
                    swift_versions.push(version.to_string());
                }
            }
        }

        let configuration_count = swift_versions.len();
        let swift_version = swift_versions.first().cloned().unwrap_or_else(|| "unknown".to_string());
        let matches_required = swift_version.starts_with(required_version);

        Ok(XcodeProjectInfo {
            path: path.to_path_buf(),
            swift_version,
            configuration_count,
            matches_required,
        })
    }

    /// Print the verification report to console
    pub fn print(&self) {
        println!("\n{}", "🔍 Swift Version Verification".bold());
        println!("{}", "==============================".bold());
        println!();

        println!("📦 Installed Swift: {}", self.installed_version.cyan());
        println!("🎯 Required Version: {}", self.required_version.cyan());
        println!("🔧 Active Toolchain: {}", self.active_toolchain.dimmed());
        println!();

        println!("{}", "📄 Package.swift Files:".bold());
        for pkg in &self.package_files {
            let status = if pkg.matches_required {
                "✓".green()
            } else {
                "✗".red()
            };
            println!(
                "  {} {} ({})",
                status,
                pkg.path.display(),
                pkg.tools_version
            );
        }
        println!();

        if !self.xcode_projects.is_empty() {
            println!("{}", "🎯 Xcode Projects:".bold());
            for proj in &self.xcode_projects {
                let status = if proj.matches_required {
                    "✓".green()
                } else {
                    "✗".red()
                };
                println!(
                    "  {} {} ({}, {} configurations)",
                    status,
                    proj.path.display(),
                    proj.swift_version,
                    proj.configuration_count
                );
            }
            println!();
        }

        if self.all_match {
            println!("{}", "✅ All versions match!".green().bold());
        } else {
            println!("{}", "❌ Version mismatches detected:".red().bold());
            for issue in &self.issues {
                println!("  • {}", issue.red());
            }
        }
        println!();
    }

    /// Export report as JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SwiftError::Config(e.to_string()))
    }
}
