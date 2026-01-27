use crate::error::{Result, SwiftError};
use colored::Colorize;
use std::path::Path;
use walkdir::WalkDir;

pub struct SwiftMigrator {
    from_version: String,
    to_version: String,
    dry_run: bool,
}

impl SwiftMigrator {
    pub fn new(from_version: String, to_version: String, dry_run: bool) -> Self {
        Self {
            from_version,
            to_version,
            dry_run,
        }
    }

    /// Migrate all Package.swift files to new version
    pub fn migrate_package_files(&self, project_root: &Path) -> Result<Vec<String>> {
        let mut migrated = Vec::new();

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
                let path = entry.path();
                let content = std::fs::read_to_string(path)?;

                if content.contains(&format!("swift-tools-version: {}", self.from_version)) {
                    if !self.dry_run {
                        let new_content = content.replace(
                            &format!("swift-tools-version: {}", self.from_version),
                            &format!("swift-tools-version: {}", self.to_version),
                        );
                        std::fs::write(path, new_content)?;
                    }
                    migrated.push(path.display().to_string());
                }
            }
        }

        Ok(migrated)
    }

    /// Migrate Xcode project files
    pub fn migrate_xcode_projects(&self, project_root: &Path) -> Result<Vec<String>> {
        let mut migrated = Vec::new();

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
                let pbxproj = entry.path().join("project.pbxproj");
                if pbxproj.exists() {
                    let content = std::fs::read_to_string(&pbxproj)?;

                    if content.contains(&format!("SWIFT_VERSION = {};", self.from_version)) {
                        if !self.dry_run {
                            let new_content = content.replace(
                                &format!("SWIFT_VERSION = {};", self.from_version),
                                &format!("SWIFT_VERSION = {};", self.to_version),
                            );
                            std::fs::write(&pbxproj, new_content)?;
                        }
                        migrated.push(entry.path().display().to_string());
                    }
                }
            }
        }

        Ok(migrated)
    }

    /// Migrate documentation files
    pub fn migrate_documentation(&self, project_root: &Path) -> Result<Vec<String>> {
        let mut migrated = Vec::new();

        for entry in WalkDir::new(project_root)
            .max_depth(3)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Some(ext) = entry.path().extension() {
                if ext == "md" || ext == "sh" {
                    let path = entry.path();
                    let content = std::fs::read_to_string(path)?;

                    let patterns = vec![
                        format!("Swift {}", self.from_version),
                        format!("swift {}", self.from_version),
                        format!("swift-tools-version: {}", self.from_version),
                    ];

                    let mut needs_update = false;
                    for pattern in &patterns {
                        if content.contains(pattern) {
                            needs_update = true;
                            break;
                        }
                    }

                    if needs_update && !self.dry_run {
                        let mut new_content = content;
                        for pattern in &patterns {
                            let replacement = pattern.replace(&self.from_version, &self.to_version);
                            new_content = new_content.replace(pattern, &replacement);
                        }
                        std::fs::write(path, new_content)?;
                        migrated.push(path.display().to_string());
                    } else if needs_update {
                        migrated.push(path.display().to_string());
                    }
                }
            }
        }

        Ok(migrated)
    }

    /// Run full migration
    pub fn run(&self, project_root: &Path) -> Result<()> {
        println!(
            "\n{} {} → {}",
            "🔄 Migrating Swift version:".bold(),
            self.from_version.cyan(),
            self.to_version.green()
        );

        if self.dry_run {
            println!("{}", "  (Dry run - no files will be modified)".yellow());
        }
        println!();

        println!("{}", "📝 Migrating Package.swift files...".bold());
        let package_files = self.migrate_package_files(project_root)?;
        for file in &package_files {
            println!("  {} {}", "✓".green(), file);
        }
        println!("  {} files", package_files.len());
        println!();

        println!("{}", "🎯 Migrating Xcode projects...".bold());
        let xcode_projects = self.migrate_xcode_projects(project_root)?;
        for proj in &xcode_projects {
            println!("  {} {}", "✓".green(), proj);
        }
        println!("  {} projects", xcode_projects.len());
        println!();

        println!("{}", "📚 Migrating documentation...".bold());
        let docs = self.migrate_documentation(project_root)?;
        for doc in &docs {
            println!("  {} {}", "✓".green(), doc);
        }
        println!("  {} files", docs.len());
        println!();

        if self.dry_run {
            println!(
                "{}",
                "✅ Dry run complete! Run without --dry-run to apply changes.".green().bold()
            );
        } else {
            println!("{}", "✅ Migration complete!".green().bold());
            println!();
            println!("Next steps:");
            println!("  1. Clean build artifacts: rm -rf .build */build");
            println!("  2. Verify: foodshare-ios swift verify");
            println!("  3. Test builds: swift build");
        }

        Ok(())
    }
}
