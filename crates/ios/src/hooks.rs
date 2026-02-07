//! Enterprise-grade Git Hooks for iOS Development
//!
//! This module provides safe, auditable hook operations with:
//! - **Preview mode**: See what will change before modifying files
//! - **Auto-backup**: Stash changes before formatting for safety
//! - **Diff summary**: Show exactly what was modified
//! - **Audit logging**: Track all hook operations
//! - **Recovery**: Easy rollback if something goes wrong

use crate::code_protection::{ProtectionConfig, SnapshotManager, SnapshotTrigger};
use crate::swift_tools;
use chrono::Local;
use foodshare_core::error::{exit_codes, Result};
use foodshare_core::git::GitRepo;
use foodshare_core::process::run_command;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// ============================================================================
// SAFE FORMAT - Enterprise-grade formatting with safety features
// ============================================================================

/// Configuration for safe formatting operations
#[derive(Debug, Clone)]
pub struct SafeFormatConfig {
    /// Show what would change without modifying files
    pub preview: bool,
    /// Create a stash backup before formatting
    pub backup: bool,
    /// Show detailed diff of changes
    pub show_diff: bool,
    /// Log operations to audit file
    pub audit: bool,
    /// Timeout for format operation
    pub timeout: Duration,
    /// Create a snapshot before formatting (Code Protection System)
    pub create_snapshot: bool,
}

impl Default for SafeFormatConfig {
    fn default() -> Self {
        Self {
            preview: false,
            backup: true, // Safe by default
            show_diff: true,
            audit: true,
            timeout: Duration::from_secs(120),
            create_snapshot: true, // Enable snapshots by default for maximum safety
        }
    }
}

/// Result of a safe format operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeFormatResult {
    /// Files that were formatted
    pub formatted_files: Vec<PathBuf>,
    /// Files that had no changes
    pub unchanged_files: Vec<PathBuf>,
    /// Files that failed to format
    pub failed_files: Vec<(PathBuf, String)>,
    /// Backup stash reference (if backup was enabled)
    pub backup_ref: Option<String>,
    /// Snapshot ID (if snapshot was created)
    pub snapshot_id: Option<String>,
    /// Total lines changed
    pub lines_changed: usize,
    /// Duration of the operation
    pub duration: Duration,
    /// Whether this was a preview (no actual changes)
    pub was_preview: bool,
    /// Diff summary per file
    pub diffs: HashMap<PathBuf, FileDiff>,
}

/// Diff information for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub insertions: usize,
    pub deletions: usize,
    pub hunks: Vec<String>,
}

/// Safe format executor with enterprise features
pub struct SafeFormat {
    config: SafeFormatConfig,
    repo: GitRepo,
}

impl SafeFormat {
    /// Create a new SafeFormat instance
    pub fn new(config: SafeFormatConfig) -> Result<Self> {
        let repo = GitRepo::open_current()?;
        Ok(Self { config, repo })
    }

    /// Format files with safety features
    pub fn format(&self, files: &[PathBuf]) -> Result<SafeFormatResult> {
        let start = Instant::now();
        let mut result = SafeFormatResult {
            formatted_files: Vec::new(),
            unchanged_files: Vec::new(),
            failed_files: Vec::new(),
            backup_ref: None,
            snapshot_id: None,
            lines_changed: 0,
            duration: Duration::ZERO,
            was_preview: self.config.preview,
            diffs: HashMap::new(),
        };

        // Filter to only Swift files
        let swift_files: Vec<_> = files
            .iter()
            .filter(|f| f.extension().map_or(false, |e| e == "swift"))
            .cloned()
            .collect();

        if swift_files.is_empty() {
            println!("  {} No Swift files to format", "ℹ".blue());
            result.duration = start.elapsed();
            return Ok(result);
        }

        // Step 0: Create snapshot if enabled (Code Protection System)
        if self.config.create_snapshot && !self.config.preview {
            if let Ok(manager) = SnapshotManager::new(ProtectionConfig::default()) {
                match manager.create_snapshot(
                    &swift_files,
                    SnapshotTrigger::PreFormat,
                    "Pre-format snapshot (auto)",
                ) {
                    Ok(snapshot) => {
                        result.snapshot_id = Some(snapshot.id.clone());
                        println!(
                            "  {} Snapshot created: {} ({} files)",
                            "📸".green(),
                            snapshot.id.dimmed(),
                            snapshot.files.len()
                        );
                    }
                    Err(e) => {
                        println!(
                            "  {} Snapshot failed (continuing): {}",
                            "⚠".yellow(),
                            e.to_string().dimmed()
                        );
                    }
                }
            }
        }

        // Step 1: Create stash backup if enabled
        if self.config.backup && !self.config.preview {
            result.backup_ref = self.create_backup()?;
            if let Some(ref backup) = result.backup_ref {
                println!(
                    "  {} Stash backup: {}",
                    "✓".green(),
                    backup.dimmed()
                );
            }
        }

        // Step 2: Capture original content for diff
        let mut original_contents: HashMap<PathBuf, String> = HashMap::new();
        for file in &swift_files {
            if file.exists() {
                if let Ok(content) = fs::read_to_string(file) {
                    original_contents.insert(file.clone(), content);
                }
            }
        }

        // Step 3: Preview or format
        if self.config.preview {
            println!("\n  {} Preview mode - no files will be modified\n", "👁".yellow());
            self.preview_format(&swift_files, &original_contents, &mut result)?;
        } else {
            self.execute_format(&swift_files, &original_contents, &mut result)?;
        }

        result.duration = start.elapsed();

        // Step 4: Audit log
        if self.config.audit {
            self.write_audit_log(&result)?;
        }

        Ok(result)
    }

    /// Create a stash backup before formatting
    fn create_backup(&self) -> Result<Option<String>> {
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let stash_msg = format!("foodshare-hooks-backup-{}", timestamp);

        // Check if there are changes to stash
        if !self.repo.has_uncommitted_changes()? {
            return Ok(None);
        }

        // Create stash with --keep-index to preserve staged changes
        let result = run_command(
            "git",
            &["stash", "push", "-m", &stash_msg, "--keep-index"],
        )?;

        if result.success && !result.stdout.contains("No local changes") {
            Ok(Some(stash_msg))
        } else {
            Ok(None)
        }
    }

    /// Preview what formatting would change
    fn preview_format(
        &self,
        files: &[PathBuf],
        original_contents: &HashMap<PathBuf, String>,
        result: &mut SafeFormatResult,
    ) -> Result<()> {
        for file in files {
            let original = match original_contents.get(file) {
                Some(c) => c,
                None => continue,
            };

            // Run swiftformat with --lint to check for changes
            let cmd_result = run_command(
                "swiftformat",
                &[&file.to_string_lossy(), "--lint", "--lenient"],
            )?;

            if cmd_result.success {
                result.unchanged_files.push(file.clone());
                println!("  {} {} (no changes)", "○".dimmed(), file.display());
            } else {
                // Would have changes - get what they would be
                let formatted = self.get_formatted_content(file)?;
                let diff = self.compute_diff(original, &formatted);

                result.formatted_files.push(file.clone());
                result.lines_changed += diff.insertions + diff.deletions;

                println!(
                    "  {} {} ({} insertions, {} deletions)",
                    "●".yellow(),
                    file.display(),
                    format!("+{}", diff.insertions).green(),
                    format!("-{}", diff.deletions).red()
                );

                if self.config.show_diff && !diff.hunks.is_empty() {
                    for hunk in &diff.hunks {
                        println!("    {}", hunk.dimmed());
                    }
                }

                result.diffs.insert(file.clone(), diff);
            }
        }

        Ok(())
    }

    /// Execute formatting
    fn execute_format(
        &self,
        files: &[PathBuf],
        original_contents: &HashMap<PathBuf, String>,
        result: &mut SafeFormatResult,
    ) -> Result<()> {
        for file in files {
            let original = match original_contents.get(file) {
                Some(c) => c.clone(),
                None => continue,
            };

            // Run swiftformat
            let cmd_result = run_command("swiftformat", &[&file.to_string_lossy()])?;

            if !cmd_result.success {
                result.failed_files.push((file.clone(), cmd_result.stderr.clone()));
                println!(
                    "  {} {} - {}",
                    "✗".red(),
                    file.display(),
                    cmd_result.stderr.lines().next().unwrap_or("Unknown error")
                );
                continue;
            }

            // Read the formatted content
            let formatted = fs::read_to_string(file).unwrap_or_default();

            if formatted == original {
                result.unchanged_files.push(file.clone());
                println!("  {} {} (no changes)", "○".dimmed(), file.display());
            } else {
                let diff = self.compute_diff(&original, &formatted);
                result.formatted_files.push(file.clone());
                result.lines_changed += diff.insertions + diff.deletions;

                println!(
                    "  {} {} ({} insertions, {} deletions)",
                    "✓".green(),
                    file.display(),
                    format!("+{}", diff.insertions).green(),
                    format!("-{}", diff.deletions).red()
                );

                if self.config.show_diff && !diff.hunks.is_empty() {
                    for hunk in diff.hunks.iter().take(3) {
                        println!("    {}", hunk.dimmed());
                    }
                    if diff.hunks.len() > 3 {
                        println!("    {} more changes...", format!("... {} ", diff.hunks.len() - 3).dimmed());
                    }
                }

                result.diffs.insert(file.clone(), diff);
            }
        }

        Ok(())
    }

    /// Get formatted content without modifying file
    fn get_formatted_content(&self, file: &Path) -> Result<String> {
        // Read original
        let original = fs::read_to_string(file)?;

        // Create temp file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("swiftformat-preview-{}.swift", std::process::id()));
        fs::write(&temp_file, &original)?;

        // Format temp file
        let _ = run_command("swiftformat", &[&temp_file.to_string_lossy()]);

        // Read formatted
        let formatted = fs::read_to_string(&temp_file).unwrap_or(original);

        // Cleanup
        let _ = fs::remove_file(&temp_file);

        Ok(formatted)
    }

    /// Compute diff between two strings
    fn compute_diff(&self, original: &str, formatted: &str) -> FileDiff {
        let original_lines: Vec<_> = original.lines().collect();
        let formatted_lines: Vec<_> = formatted.lines().collect();

        let mut insertions = 0;
        let mut deletions = 0;
        let mut hunks = Vec::new();

        // Simple line-by-line diff
        let max_lines = original_lines.len().max(formatted_lines.len());
        for i in 0..max_lines {
            let orig = original_lines.get(i).map(|s| *s);
            let fmt = formatted_lines.get(i).map(|s| *s);

            match (orig, fmt) {
                (Some(o), Some(f)) if o != f => {
                    deletions += 1;
                    insertions += 1;
                    if hunks.len() < 10 {
                        hunks.push(format!("-{}: {}", i + 1, truncate(o, 60)));
                        hunks.push(format!("+{}: {}", i + 1, truncate(f, 60)));
                    }
                }
                (Some(_), None) => {
                    deletions += 1;
                }
                (None, Some(_)) => {
                    insertions += 1;
                }
                _ => {}
            }
        }

        FileDiff {
            insertions,
            deletions,
            hunks,
        }
    }

    /// Write audit log
    fn write_audit_log(&self, result: &SafeFormatResult) -> Result<()> {
        let audit_dir = self.repo.workdir().join(".foodshare-hooks");
        fs::create_dir_all(&audit_dir)?;

        let audit_file = audit_dir.join("format-audit.log");
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

        let entry = format!(
            "[{}] FORMAT: {} files formatted, {} unchanged, {} failed, {} lines changed (backup: {})\n",
            timestamp,
            result.formatted_files.len(),
            result.unchanged_files.len(),
            result.failed_files.len(),
            result.lines_changed,
            result.backup_ref.as_deref().unwrap_or("none")
        );

        // Append to audit log
        let mut log = fs::read_to_string(&audit_file).unwrap_or_default();
        log.push_str(&entry);

        // Keep only last 1000 lines
        let lines: Vec<_> = log.lines().collect();
        let trimmed = if lines.len() > 1000 {
            lines[lines.len() - 1000..].join("\n")
        } else {
            log
        };

        fs::write(&audit_file, trimmed)?;

        Ok(())
    }

    /// Restore from backup
    pub fn restore_backup(backup_ref: &str) -> Result<bool> {
        // Find the stash by message
        let result = run_command("git", &["stash", "list"])?;

        for (idx, line) in result.stdout.lines().enumerate() {
            if line.contains(backup_ref) {
                let stash_ref = format!("stash@{{{}}}", idx);
                let pop_result = run_command("git", &["stash", "pop", &stash_ref])?;
                return Ok(pop_result.success);
            }
        }

        Ok(false)
    }
}

/// Print format result summary
pub fn print_format_summary(result: &SafeFormatResult) {
    println!();

    let mode = if result.was_preview { " (preview)" } else { "" };

    if result.failed_files.is_empty() {
        print!("{} ", "✓".green());
    } else {
        print!("{} ", "⚠".yellow());
    }

    println!(
        "Format complete{}: {} formatted, {} unchanged, {} failed in {:.2}s",
        mode.yellow(),
        result.formatted_files.len(),
        result.unchanged_files.len(),
        result.failed_files.len(),
        result.duration.as_secs_f32()
    );

    if result.lines_changed > 0 {
        println!(
            "  {} lines changed across {} files",
            result.lines_changed,
            result.formatted_files.len()
        );
    }

    if let Some(ref snapshot) = result.snapshot_id {
        println!(
            "  {} Snapshot: foodshare-ios protect restore --snapshot {}",
            "📸".blue(),
            snapshot
        );
    }

    if let Some(ref backup) = result.backup_ref {
        println!(
            "  {} Stash: git stash apply (search for '{}')",
            "💾".blue(),
            backup
        );
    }

    if !result.failed_files.is_empty() {
        println!();
        println!("  {} Failed files:", "Errors:".red().bold());
        for (file, error) in &result.failed_files {
            println!("    {} {}: {}", "✗".red(), file.display(), error);
        }
    }
}

// ============================================================================
// PRE-PUSH CHECKS - Enterprise-grade validation before push
// ============================================================================

/// Pre-push check definition
#[derive(Debug, Clone)]
pub struct PrePushCheck {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
    pub timeout: Duration,
}

/// Pre-push check result
#[derive(Debug, Clone)]
pub struct PrePushCheckResult {
    pub name: String,
    pub success: bool,
    pub duration: Duration,
    pub output: Option<String>,
    pub skipped: bool,
    pub required: bool,
}

/// Configuration for pre-push checks
#[derive(Debug, Clone)]
pub struct PrePushConfig {
    /// Stop on first failure
    pub fail_fast: bool,
    /// Use release build
    pub release: bool,
    /// Run in quick mode (skip optional checks)
    pub quick_mode: bool,
    /// Checks to skip
    pub skip_checks: Vec<String>,
}

impl Default for PrePushConfig {
    fn default() -> Self {
        Self {
            fail_fast: true,
            release: false,
            quick_mode: false,
            skip_checks: Vec::new(),
        }
    }
}

/// Run pre-push validation checks
pub fn run_pre_push_checks(config: &PrePushConfig) -> Vec<PrePushCheckResult> {
    let mut results = Vec::new();

    println!();
    println!("{}", "Pre-push Validation".bold());
    println!("{}", "═".repeat(50));
    println!();

    // Define checks as a struct-based approach for type safety
    struct CheckDef {
        name: &'static str,
        description: &'static str,
        required: bool,
    }

    let checks = vec![
        CheckDef { name: "lint", description: "Swift lint check", required: true },
        CheckDef { name: "build", description: "Build validation", required: true },
        CheckDef { name: "test", description: "Unit tests", required: false },
    ];

    for check in checks {
        // Skip if in skip list
        if config.skip_checks.iter().any(|s| s == check.name) {
            results.push(PrePushCheckResult {
                name: check.name.to_string(),
                success: true,
                duration: Duration::ZERO,
                output: None,
                skipped: true,
                required: check.required,
            });
            println!("  {} {} {}", "⊘".dimmed(), check.name.dimmed(), "(skipped)".dimmed());
            continue;
        }

        // Skip non-required in quick mode
        if config.quick_mode && !check.required {
            results.push(PrePushCheckResult {
                name: check.name.to_string(),
                success: true,
                duration: Duration::ZERO,
                output: None,
                skipped: true,
                required: check.required,
            });
            println!("  {} {} {}", "⊘".dimmed(), check.name.dimmed(), "(quick mode)".dimmed());
            continue;
        }

        // Run the check
        print!("  {} {}...", "●".blue(), check.description);
        use std::io::Write;
        std::io::stdout().flush().ok();

        let start = Instant::now();
        let check_result = match check.name {
            "lint" => check_lint(config),
            "build" => check_build(config),
            "test" => check_tests(config),
            _ => Ok(()),
        };
        let duration = start.elapsed();

        let success = check_result.is_ok();
        let output = check_result.err();

        results.push(PrePushCheckResult {
            name: check.name.to_string(),
            success,
            duration,
            output: output.clone(),
            skipped: false,
            required: check.required,
        });

        // Clear line and print result
        print!("\r");
        if success {
            println!(
                "  {} {} {}",
                "✓".green(),
                check.description,
                format!("({:.1}s)", duration.as_secs_f32()).dimmed()
            );
        } else if !check.required {
            println!(
                "  {} {} {} {}",
                "⚠".yellow(),
                check.description.yellow(),
                format!("({:.1}s)", duration.as_secs_f32()).dimmed(),
                "(non-blocking)".dimmed()
            );

            if let Some(ref err) = output {
                for line in err.lines().take(3) {
                    println!("    {}", line.dimmed());
                }
            }
        } else {
            println!(
                "  {} {} {}",
                "✗".red(),
                check.description.red(),
                format!("({:.1}s)", duration.as_secs_f32()).dimmed()
            );

            if let Some(ref err) = output {
                // Show first few lines of error
                for line in err.lines().take(5) {
                    println!("    {}", line.dimmed());
                }
            }

            if config.fail_fast {
                break;
            }
        }
    }

    results
}

fn check_lint(_config: &PrePushConfig) -> std::result::Result<(), String> {
    if !swift_tools::has_swiftlint() {
        return Ok(()); // Skip if not installed
    }

    let result = swift_tools::lint(&["FoodShare"], false, false)
        .map_err(|e| e.to_string())?;

    if result.success {
        Ok(())
    } else {
        Err(result.stderr)
    }
}

fn check_build(config: &PrePushConfig) -> std::result::Result<(), String> {
    let configuration = if config.release { "Release" } else { "Debug" };

    let result = crate::xcode::build(
        "FoodShare",
        configuration,
        "platform=iOS Simulator,name=iPhone 17 Pro Max",
        false,
    )
    .map_err(|e| e.to_string())?;

    if result.success {
        Ok(())
    } else {
        Err(result.stderr)
    }
}

fn check_tests(_config: &PrePushConfig) -> std::result::Result<(), String> {
    let result = crate::xcode::test(
        "FoodShare",
        "platform=iOS Simulator,name=iPhone 17 Pro Max",
        false,
    )
    .map_err(|e| e.to_string())?;

    if result.success {
        Ok(())
    } else {
        Err(result.stderr)
    }
}

/// Print pre-push summary
pub fn print_pre_push_summary(results: &[PrePushCheckResult]) -> i32 {
    println!();
    println!("{}", "─".repeat(50));

    let passed = results.iter().filter(|r| r.success && !r.skipped).count();
    let failed_required = results.iter().filter(|r| !r.success && r.required).count();
    let warned = results.iter().filter(|r| !r.success && !r.required).count();
    let skipped = results.iter().filter(|r| r.skipped).count();
    let total_time: Duration = results.iter().map(|r| r.duration).sum();

    if failed_required == 0 {
        if warned > 0 {
            println!(
                "{} Checks passed with {} warning(s) ({} passed, {} skipped) in {:.1}s",
                "✓".green().bold(),
                warned,
                passed,
                skipped,
                total_time.as_secs_f32()
            );
        } else {
            println!(
                "{} All checks passed ({} passed, {} skipped) in {:.1}s",
                "✓".green().bold(),
                passed,
                skipped,
                total_time.as_secs_f32()
            );
        }
        println!();
        exit_codes::SUCCESS
    } else {
        println!(
            "{} {} check(s) failed ({} passed, {} warned, {} skipped)",
            "✗".red().bold(),
            failed_required,
            passed,
            warned,
            skipped
        );
        println!();

        // Show recovery hint
        println!(
            "  {} To push anyway: {}",
            "ℹ".blue(),
            "git push --no-verify".yellow()
        );
        println!();

        exit_codes::FAILURE
    }
}

// ============================================================================
// UTILITIES
// ============================================================================

/// Truncate string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_format_config_default() {
        let config = SafeFormatConfig::default();
        assert!(!config.preview);
        assert!(config.backup);
        assert!(config.show_diff);
        assert!(config.audit);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_pre_push_config_default() {
        let config = PrePushConfig::default();
        assert!(config.fail_fast);
        assert!(!config.release);
        assert!(!config.quick_mode);
    }
}
