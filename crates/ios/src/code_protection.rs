//! Code Protection System - Enterprise-grade safety for code modifications
//!
//! This module provides comprehensive protection against accidental code loss:
//!
//! ## Safety Layers
//!
//! 1. **Snapshots**: Full file content backup before any operation
//! 2. **Verification**: Build check after modifications to catch breakage early
//! 3. **Interactive Approval**: Show diff and require explicit confirmation
//! 4. **Rollback**: One-command recovery to any previous state
//! 5. **Protected Paths**: Exclude critical files from auto-modification
//! 6. **Operation History**: Complete audit trail with undo capability
//! 7. **Commit Guard**: Verify exactly what will be committed
//! 8. **Push Guard**: Verify what will be pushed before it leaves local
//!
//! ## Recovery Commands
//!
//! ```bash
//! # List all snapshots
//! foodshare-ios protect list
//!
//! # Restore from latest snapshot
//! foodshare-ios protect restore --latest
//!
//! # Restore specific file from snapshot
//! foodshare-ios protect restore --snapshot <id> --file <path>
//!
//! # Show what would be committed
//! foodshare-ios protect commit-guard
//!
//! # Show what would be pushed
//! foodshare-ios protect push-guard
//! ```

use chrono::{DateTime, Local, Utc};
use foodshare_core::error::Result;
use foodshare_core::git::GitRepo;
use foodshare_core::process::run_command;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Code protection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionConfig {
    /// Enable snapshot creation before modifications
    pub snapshots_enabled: bool,
    /// Enable build verification after modifications
    pub verify_build: bool,
    /// Require interactive approval for changes
    pub interactive_approval: bool,
    /// Maximum number of snapshots to retain
    pub max_snapshots: usize,
    /// Paths that should never be auto-modified
    pub protected_paths: Vec<String>,
    /// Patterns to exclude from formatting
    pub exclude_patterns: Vec<String>,
    /// Directory for storing protection data
    pub data_dir: PathBuf,
}

impl Default for ProtectionConfig {
    fn default() -> Self {
        Self {
            snapshots_enabled: true,
            verify_build: true,
            interactive_approval: false, // Can be enabled for extra safety
            max_snapshots: 50,
            protected_paths: vec![
                "*.entitlements".to_string(),
                "Info.plist".to_string(),
                "*.xcconfig".to_string(),
                "project.pbxproj".to_string(),
            ],
            exclude_patterns: vec![
                "Generated".to_string(),
                "Derived".to_string(),
                ".build".to_string(),
            ],
            data_dir: PathBuf::from(".foodshare-hooks"),
        }
    }
}

// ============================================================================
// SNAPSHOT SYSTEM
// ============================================================================

/// A snapshot of file contents at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unique snapshot ID
    pub id: String,
    /// When the snapshot was created
    pub timestamp: DateTime<Utc>,
    /// What triggered this snapshot
    pub trigger: SnapshotTrigger,
    /// Description of the operation
    pub description: String,
    /// Files included in this snapshot
    pub files: Vec<FileSnapshot>,
    /// Git branch at time of snapshot
    pub branch: String,
    /// Git HEAD commit at time of snapshot
    pub commit: String,
}

/// What triggered a snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SnapshotTrigger {
    PreFormat,
    PreCommit,
    PrePush,
    PreRebase,
    Manual,
}

impl std::fmt::Display for SnapshotTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreFormat => write!(f, "pre-format"),
            Self::PreCommit => write!(f, "pre-commit"),
            Self::PrePush => write!(f, "pre-push"),
            Self::PreRebase => write!(f, "pre-rebase"),
            Self::Manual => write!(f, "manual"),
        }
    }
}

/// Snapshot of a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    /// Relative path to file
    pub path: PathBuf,
    /// SHA256 hash of content
    pub hash: String,
    /// File size in bytes
    pub size: u64,
    /// Content (stored separately in content store)
    #[serde(skip)]
    pub content: Option<String>,
}

/// Manages snapshots for code protection
pub struct SnapshotManager {
    config: ProtectionConfig,
    repo: GitRepo,
    snapshots_dir: PathBuf,
    content_dir: PathBuf,
    index_file: PathBuf,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(config: ProtectionConfig) -> Result<Self> {
        let repo = GitRepo::open_current()?;
        let data_dir = repo.workdir().join(&config.data_dir);
        let snapshots_dir = data_dir.join("snapshots");
        let content_dir = data_dir.join("content");
        let index_file = data_dir.join("snapshot-index.json");

        // Ensure directories exist
        fs::create_dir_all(&snapshots_dir)?;
        fs::create_dir_all(&content_dir)?;

        // Add to .gitignore if not already
        Self::ensure_gitignore(&repo, &config.data_dir)?;

        Ok(Self {
            config,
            repo,
            snapshots_dir,
            content_dir,
            index_file,
        })
    }

    /// Ensure the data directory is in .gitignore
    fn ensure_gitignore(repo: &GitRepo, data_dir: &Path) -> Result<()> {
        let gitignore = repo.workdir().join(".gitignore");
        let entry = format!("{}/", data_dir.display());

        if gitignore.exists() {
            let content = fs::read_to_string(&gitignore)?;
            if !content.contains(&entry) {
                let mut file = fs::OpenOptions::new().append(true).open(&gitignore)?;
                writeln!(file, "\n# Code protection data\n{}", entry)?;
            }
        }
        Ok(())
    }

    /// Create a snapshot of the given files
    pub fn create_snapshot(
        &self,
        files: &[PathBuf],
        trigger: SnapshotTrigger,
        description: &str,
    ) -> Result<Snapshot> {
        let id = generate_snapshot_id();
        let timestamp = Utc::now();
        let branch = self.repo.current_branch().unwrap_or_else(|_| "unknown".to_string());
        let commit = get_head_commit().unwrap_or_else(|_| "unknown".to_string());

        let mut file_snapshots = Vec::new();

        for file in files {
            let full_path = self.repo.workdir().join(file);
            if !full_path.exists() {
                continue;
            }

            let content = fs::read_to_string(&full_path)?;
            let hash = compute_hash(&content);
            let size = content.len() as u64;

            // Store content by hash (deduplication)
            let content_file = self.content_dir.join(&hash);
            if !content_file.exists() {
                fs::write(&content_file, &content)?;
            }

            file_snapshots.push(FileSnapshot {
                path: file.clone(),
                hash,
                size,
                content: None, // Don't store in snapshot struct
            });
        }

        let snapshot = Snapshot {
            id: id.clone(),
            timestamp,
            trigger,
            description: description.to_string(),
            files: file_snapshots,
            branch,
            commit,
        };

        // Save snapshot metadata
        let snapshot_file = self.snapshots_dir.join(format!("{}.json", id));
        let json = serde_json::to_string_pretty(&snapshot)?;
        fs::write(&snapshot_file, json)?;

        // Update index
        self.update_index(&snapshot)?;

        // Cleanup old snapshots
        self.cleanup_old_snapshots()?;

        Ok(snapshot)
    }

    /// Update the snapshot index
    fn update_index(&self, snapshot: &Snapshot) -> Result<()> {
        let mut index = self.load_index()?;
        index.push(SnapshotIndexEntry {
            id: snapshot.id.clone(),
            timestamp: snapshot.timestamp,
            trigger: snapshot.trigger.clone(),
            description: snapshot.description.clone(),
            file_count: snapshot.files.len(),
        });

        let json = serde_json::to_string_pretty(&index)?;
        fs::write(&self.index_file, json)?;
        Ok(())
    }

    /// Load the snapshot index
    fn load_index(&self) -> Result<Vec<SnapshotIndexEntry>> {
        if self.index_file.exists() {
            let content = fs::read_to_string(&self.index_file)?;
            Ok(serde_json::from_str(&content).unwrap_or_default())
        } else {
            Ok(Vec::new())
        }
    }

    /// Cleanup old snapshots beyond the retention limit
    fn cleanup_old_snapshots(&self) -> Result<()> {
        let mut index = self.load_index()?;

        if index.len() <= self.config.max_snapshots {
            return Ok(());
        }

        // Sort by timestamp (oldest first)
        index.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Remove oldest snapshots
        let to_remove = index.len() - self.config.max_snapshots;
        for entry in index.iter().take(to_remove) {
            let snapshot_file = self.snapshots_dir.join(format!("{}.json", entry.id));
            let _ = fs::remove_file(snapshot_file);
        }

        // Update index
        let remaining: Vec<_> = index.into_iter().skip(to_remove).collect();
        let json = serde_json::to_string_pretty(&remaining)?;
        fs::write(&self.index_file, json)?;

        Ok(())
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotIndexEntry>> {
        let mut index = self.load_index()?;
        index.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Newest first
        Ok(index)
    }

    /// Get a specific snapshot
    pub fn get_snapshot(&self, id: &str) -> Result<Option<Snapshot>> {
        let snapshot_file = self.snapshots_dir.join(format!("{}.json", id));
        if snapshot_file.exists() {
            let content = fs::read_to_string(&snapshot_file)?;
            Ok(Some(serde_json::from_str(&content)?))
        } else {
            Ok(None)
        }
    }

    /// Get the latest snapshot
    pub fn get_latest_snapshot(&self) -> Result<Option<Snapshot>> {
        let index = self.list_snapshots()?;
        if let Some(entry) = index.first() {
            self.get_snapshot(&entry.id)
        } else {
            Ok(None)
        }
    }

    /// Restore files from a snapshot
    pub fn restore_snapshot(
        &self,
        snapshot: &Snapshot,
        files: Option<&[PathBuf]>,
        dry_run: bool,
    ) -> Result<RestoreResult> {
        let mut result = RestoreResult {
            restored_files: Vec::new(),
            skipped_files: Vec::new(),
            failed_files: Vec::new(),
            dry_run,
        };

        for file_snap in &snapshot.files {
            // Skip if specific files requested and this isn't one of them
            if let Some(requested) = files {
                if !requested.iter().any(|p| p == &file_snap.path) {
                    continue;
                }
            }

            let full_path = self.repo.workdir().join(&file_snap.path);
            let content_file = self.content_dir.join(&file_snap.hash);

            if !content_file.exists() {
                result.failed_files.push((
                    file_snap.path.clone(),
                    "Content not found in store".to_string(),
                ));
                continue;
            }

            let content = fs::read_to_string(&content_file)?;

            // Check if file has changed
            if full_path.exists() {
                let current = fs::read_to_string(&full_path)?;
                if current == content {
                    result.skipped_files.push(file_snap.path.clone());
                    continue;
                }
            }

            if dry_run {
                result.restored_files.push(file_snap.path.clone());
            } else {
                // Create backup before restore
                if full_path.exists() {
                    let backup_path = full_path.with_extension("swift.backup");
                    fs::copy(&full_path, &backup_path)?;
                }

                // Restore the file
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&full_path, &content)?;
                result.restored_files.push(file_snap.path.clone());
            }
        }

        Ok(result)
    }

    /// Get content for a file from a snapshot
    pub fn get_file_content(&self, snapshot: &Snapshot, path: &Path) -> Result<Option<String>> {
        for file_snap in &snapshot.files {
            if file_snap.path == path {
                let content_file = self.content_dir.join(&file_snap.hash);
                if content_file.exists() {
                    return Ok(Some(fs::read_to_string(&content_file)?));
                }
            }
        }
        Ok(None)
    }
}

/// Index entry for quick snapshot lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotIndexEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub trigger: SnapshotTrigger,
    pub description: String,
    pub file_count: usize,
}

/// Result of a restore operation
#[derive(Debug)]
pub struct RestoreResult {
    pub restored_files: Vec<PathBuf>,
    pub skipped_files: Vec<PathBuf>,
    pub failed_files: Vec<(PathBuf, String)>,
    pub dry_run: bool,
}

// ============================================================================
// BUILD VERIFICATION
// ============================================================================

/// Verify that code still builds after modifications
pub fn verify_build(quick: bool) -> Result<BuildVerification> {
    let start = Instant::now();

    print!("  {} Verifying build...", "●".blue());
    io::stdout().flush().ok();

    let args = if quick {
        // Quick syntax check only
        vec![
            "-scheme", "FoodShare",
            "-destination", "platform=iOS Simulator,name=iPhone 17 Pro Max",
            "build",
            "-quiet",
            "ONLY_ACTIVE_ARCH=YES",
            "BUILD_ACTIVE_RESOURCES_ONLY=YES",
        ]
    } else {
        // Full build
        vec![
            "-scheme", "FoodShare",
            "-destination", "platform=iOS Simulator,name=iPhone 17 Pro Max",
            "build",
        ]
    };

    let result = run_command("xcodebuild", &args)?;
    let duration = start.elapsed();

    print!("\r");

    if result.success {
        println!(
            "  {} Build verification passed {}",
            "✓".green(),
            format!("({:.1}s)", duration.as_secs_f32()).dimmed()
        );
        Ok(BuildVerification {
            success: true,
            duration,
            errors: Vec::new(),
        })
    } else {
        println!(
            "  {} Build verification FAILED {}",
            "✗".red(),
            format!("({:.1}s)", duration.as_secs_f32()).dimmed()
        );

        // Extract errors
        let errors: Vec<String> = result
            .stderr
            .lines()
            .filter(|l| l.contains("error:"))
            .map(String::from)
            .collect();

        Ok(BuildVerification {
            success: false,
            duration,
            errors,
        })
    }
}

/// Result of build verification
#[derive(Debug)]
pub struct BuildVerification {
    pub success: bool,
    pub duration: Duration,
    pub errors: Vec<String>,
}

// ============================================================================
// INTERACTIVE APPROVAL
// ============================================================================

/// Show changes and get user approval
pub fn get_interactive_approval(changes: &[FileChange]) -> Result<ApprovalDecision> {
    println!();
    println!("{}", "═".repeat(60));
    println!("{}", "Changes requiring approval:".bold());
    println!("{}", "═".repeat(60));
    println!();

    for change in changes {
        let change_marker = match change.change_type {
            ChangeType::Modified => "M".yellow().to_string(),
            ChangeType::Added => "A".green().to_string(),
            ChangeType::Deleted => "D".red().to_string(),
        };
        println!(
            "  {} {} ({} → {})",
            change_marker,
            change.path.display(),
            format!("+{}", change.insertions).green(),
            format!("-{}", change.deletions).red()
        );

        // Show preview of changes
        if !change.preview.is_empty() {
            for line in change.preview.iter().take(5) {
                if line.starts_with('+') {
                    println!("    {}", line.green());
                } else if line.starts_with('-') {
                    println!("    {}", line.red());
                } else {
                    println!("    {}", line.dimmed());
                }
            }
            if change.preview.len() > 5 {
                println!("    {} more lines...", format!("... {} ", change.preview.len() - 5).dimmed());
            }
        }
        println!();
    }

    println!("{}", "─".repeat(60));
    print!(
        "{}",
        "Apply these changes? [y]es / [n]o / [p]review all / [r]evert: ".bold()
    );
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => Ok(ApprovalDecision::Approve),
        "n" | "no" => Ok(ApprovalDecision::Reject),
        "p" | "preview" => Ok(ApprovalDecision::PreviewAll),
        "r" | "revert" => Ok(ApprovalDecision::Revert),
        _ => Ok(ApprovalDecision::Reject),
    }
}

/// A file change for approval
#[derive(Debug)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: ChangeType,
    pub insertions: usize,
    pub deletions: usize,
    pub preview: Vec<String>,
}

/// Type of change
#[derive(Debug)]
pub enum ChangeType {
    Modified,
    Added,
    Deleted,
}

/// User's decision on changes
#[derive(Debug, PartialEq)]
pub enum ApprovalDecision {
    Approve,
    Reject,
    PreviewAll,
    Revert,
}

// ============================================================================
// COMMIT GUARD
// ============================================================================

/// Guard that shows exactly what will be committed
pub struct CommitGuard {
    repo: GitRepo,
}

impl CommitGuard {
    pub fn new() -> Result<Self> {
        Ok(Self {
            repo: GitRepo::open_current()?,
        })
    }

    /// Show what will be committed
    pub fn show_pending_commit(&self) -> Result<PendingCommit> {
        let staged = self.repo.staged_files()?;
        let mut files = Vec::new();

        for path in &staged {
            let diff = self.get_staged_diff(path)?;
            files.push(StagedFile {
                path: path.clone(),
                insertions: diff.insertions,
                deletions: diff.deletions,
                is_new: diff.is_new,
            });
        }

        let total_insertions: usize = files.iter().map(|f| f.insertions).sum();
        let total_deletions: usize = files.iter().map(|f| f.deletions).sum();

        Ok(PendingCommit {
            files,
            total_insertions,
            total_deletions,
            branch: self.repo.current_branch()?,
        })
    }

    fn get_staged_diff(&self, path: &Path) -> Result<FileDiffStats> {
        let result = run_command(
            "git",
            &["diff", "--cached", "--numstat", &path.to_string_lossy()],
        )?;

        let mut insertions = 0;
        let mut deletions = 0;

        for line in result.stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                insertions = parts[0].parse().unwrap_or(0);
                deletions = parts[1].parse().unwrap_or(0);
            }
        }

        // Check if it's a new file
        let status = run_command("git", &["status", "--porcelain", &path.to_string_lossy()])?;
        let is_new = status.stdout.starts_with("A ");

        Ok(FileDiffStats {
            insertions,
            deletions,
            is_new,
        })
    }

    /// Verify the commit after it's made
    pub fn verify_last_commit(&self) -> Result<CommitVerification> {
        let result = run_command("git", &["log", "-1", "--stat", "--format=%H%n%s"])?;
        let lines: Vec<&str> = result.stdout.lines().collect();

        let hash = lines.first().map(|s| s.to_string()).unwrap_or_default();
        let message = lines.get(1).map(|s| s.to_string()).unwrap_or_default();

        // Get files in last commit
        let files_result = run_command("git", &["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"])?;
        let files: Vec<PathBuf> = files_result
            .stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect();

        Ok(CommitVerification {
            hash,
            message,
            files,
        })
    }
}

#[derive(Debug)]
pub struct PendingCommit {
    pub files: Vec<StagedFile>,
    pub total_insertions: usize,
    pub total_deletions: usize,
    pub branch: String,
}

#[derive(Debug)]
pub struct StagedFile {
    pub path: PathBuf,
    pub insertions: usize,
    pub deletions: usize,
    pub is_new: bool,
}

#[derive(Debug)]
struct FileDiffStats {
    insertions: usize,
    deletions: usize,
    is_new: bool,
}

#[derive(Debug)]
pub struct CommitVerification {
    pub hash: String,
    pub message: String,
    pub files: Vec<PathBuf>,
}

// ============================================================================
// PUSH GUARD
// ============================================================================

/// Guard that shows exactly what will be pushed
pub struct PushGuard {
    repo: GitRepo,
}

impl PushGuard {
    pub fn new() -> Result<Self> {
        Ok(Self {
            repo: GitRepo::open_current()?,
        })
    }

    /// Show what will be pushed
    pub fn show_pending_push(&self, remote: &str, branch: &str) -> Result<PendingPush> {
        // Get commits that will be pushed
        let result = run_command(
            "git",
            &["log", &format!("{}/{}..HEAD", remote, branch), "--oneline"],
        )?;

        let commits: Vec<CommitSummary> = result
            .stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                CommitSummary {
                    hash: parts.first().map(|s| s.to_string()).unwrap_or_default(),
                    message: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
                }
            })
            .collect();

        // Get total diff stats
        let diff_result = run_command(
            "git",
            &["diff", "--stat", &format!("{}/{}..HEAD", remote, branch)],
        )?;

        let stats = parse_diff_stats(&diff_result.stdout);

        Ok(PendingPush {
            remote: remote.to_string(),
            branch: branch.to_string(),
            commits,
            files_changed: stats.files_changed,
            insertions: stats.insertions,
            deletions: stats.deletions,
        })
    }
}

#[derive(Debug)]
pub struct PendingPush {
    pub remote: String,
    pub branch: String,
    pub commits: Vec<CommitSummary>,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug)]
pub struct CommitSummary {
    pub hash: String,
    pub message: String,
}

fn parse_diff_stats(output: &str) -> DiffStats {
    let mut stats = DiffStats {
        files_changed: 0,
        insertions: 0,
        deletions: 0,
    };

    for line in output.lines() {
        if line.contains("files changed") || line.contains("file changed") {
            let parts: Vec<&str> = line.split(',').collect();
            for part in parts {
                let part = part.trim();
                if part.contains("file") {
                    if let Some(num) = part.split_whitespace().next() {
                        stats.files_changed = num.parse().unwrap_or(0);
                    }
                } else if part.contains("insertion") {
                    if let Some(num) = part.split_whitespace().next() {
                        stats.insertions = num.parse().unwrap_or(0);
                    }
                } else if part.contains("deletion") {
                    if let Some(num) = part.split_whitespace().next() {
                        stats.deletions = num.parse().unwrap_or(0);
                    }
                }
            }
        }
    }

    stats
}

#[derive(Debug)]
struct DiffStats {
    files_changed: usize,
    insertions: usize,
    deletions: usize,
}

// ============================================================================
// OPERATION HISTORY
// ============================================================================

/// Tracks all hook operations for audit and undo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub operation: OperationType,
    pub affected_files: Vec<PathBuf>,
    pub snapshot_id: Option<String>,
    pub success: bool,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Format,
    Lint,
    Commit,
    Push,
    Restore,
    Rollback,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Format => write!(f, "format"),
            Self::Lint => write!(f, "lint"),
            Self::Commit => write!(f, "commit"),
            Self::Push => write!(f, "push"),
            Self::Restore => write!(f, "restore"),
            Self::Rollback => write!(f, "rollback"),
        }
    }
}

/// Manages operation history
pub struct OperationHistory {
    history_file: PathBuf,
}

impl OperationHistory {
    pub fn new(data_dir: &Path) -> Result<Self> {
        let history_file = data_dir.join("operation-history.json");
        fs::create_dir_all(data_dir)?;
        Ok(Self { history_file })
    }

    /// Record an operation
    pub fn record(&self, record: OperationRecord) -> Result<()> {
        let mut history = self.load()?;
        history.push(record);

        // Keep only last 1000 operations
        let history = if history.len() > 1000 {
            let skip_count = history.len() - 1000;
            history.into_iter().skip(skip_count).collect()
        } else {
            history
        };

        let json = serde_json::to_string_pretty(&history)?;
        fs::write(&self.history_file, json)?;
        Ok(())
    }

    /// Load operation history
    pub fn load(&self) -> Result<Vec<OperationRecord>> {
        if self.history_file.exists() {
            let content = fs::read_to_string(&self.history_file)?;
            Ok(serde_json::from_str(&content).unwrap_or_default())
        } else {
            Ok(Vec::new())
        }
    }

    /// Get recent operations
    pub fn recent(&self, count: usize) -> Result<Vec<OperationRecord>> {
        let history = self.load()?;
        Ok(history.into_iter().rev().take(count).collect())
    }
}

// ============================================================================
// UTILITIES
// ============================================================================

/// Generate a unique snapshot ID
fn generate_snapshot_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("snap-{:x}", timestamp)
}

/// Compute SHA256 hash of content
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Get HEAD commit hash
fn get_head_commit() -> Result<String> {
    let result = run_command("git", &["rev-parse", "HEAD"])?;
    Ok(result.stdout.trim().to_string())
}

// ============================================================================
// PRINT HELPERS
// ============================================================================

/// Print pending commit info
pub fn print_pending_commit(pending: &PendingCommit) {
    println!();
    println!("{}", "═".repeat(60));
    println!("{}", "COMMIT GUARD - What will be committed:".bold());
    println!("{}", "═".repeat(60));
    println!();
    println!("  Branch: {}", pending.branch.cyan());
    println!();

    for file in &pending.files {
        let status_marker = if file.is_new {
            "A".green().to_string()
        } else {
            "M".yellow().to_string()
        };
        println!(
            "  {} {} ({}, {})",
            status_marker,
            file.path.display(),
            format!("+{}", file.insertions).green(),
            format!("-{}", file.deletions).red()
        );
    }

    println!();
    println!(
        "  Total: {} files, {}, {}",
        pending.files.len(),
        format!("+{}", pending.total_insertions).green(),
        format!("-{}", pending.total_deletions).red()
    );
    println!("{}", "═".repeat(60));
}

/// Print pending push info
pub fn print_pending_push(pending: &PendingPush) {
    println!();
    println!("{}", "═".repeat(60));
    println!("{}", "PUSH GUARD - What will be pushed:".bold());
    println!("{}", "═".repeat(60));
    println!();
    println!(
        "  Target: {}/{}",
        pending.remote.cyan(),
        pending.branch.cyan()
    );
    println!("  Commits: {}", pending.commits.len());
    println!();

    for commit in pending.commits.iter().take(10) {
        let short_hash = if commit.hash.len() >= 7 {
            &commit.hash[..7]
        } else {
            &commit.hash
        };
        println!("    {} {}", short_hash.yellow(), commit.message);
    }
    if pending.commits.len() > 10 {
        println!("    ... and {} more commits", pending.commits.len() - 10);
    }

    println!();
    println!(
        "  Total: {} files, {}, {}",
        pending.files_changed,
        format!("+{}", pending.insertions).green(),
        format!("-{}", pending.deletions).red()
    );
    println!("{}", "═".repeat(60));
}

/// Print snapshot list
pub fn print_snapshot_list(snapshots: &[SnapshotIndexEntry]) {
    println!();
    println!("{}", "═".repeat(70));
    println!("{}", "CODE PROTECTION SNAPSHOTS".bold());
    println!("{}", "═".repeat(70));
    println!();

    if snapshots.is_empty() {
        println!("  No snapshots found.");
        println!();
        return;
    }

    println!(
        "  {:<20} {:<12} {:<8} {}",
        "ID".bold(),
        "TRIGGER".bold(),
        "FILES".bold(),
        "TIMESTAMP".bold()
    );
    println!("  {}", "─".repeat(66));

    for snap in snapshots.iter().take(20) {
        let local_time: DateTime<Local> = snap.timestamp.into();
        println!(
            "  {:<20} {:<12} {:<8} {}",
            snap.id.cyan(),
            snap.trigger.to_string(),
            snap.file_count,
            local_time.format("%Y-%m-%d %H:%M:%S")
        );
    }

    if snapshots.len() > 20 {
        println!("  ... and {} more snapshots", snapshots.len() - 20);
    }

    println!();
    println!("  {}", "Recovery commands:".bold());
    println!("    foodshare-ios protect restore --latest");
    println!("    foodshare-ios protect restore --snapshot <ID>");
    println!("{}", "═".repeat(70));
}

/// Print restore result
pub fn print_restore_result(result: &RestoreResult) {
    println!();

    if result.dry_run {
        println!("{}", "DRY RUN - No files were modified".yellow().bold());
        println!();
    }

    if !result.restored_files.is_empty() {
        println!(
            "{} {} file(s):",
            if result.dry_run { "Would restore" } else { "Restored" },
            result.restored_files.len()
        );
        for file in &result.restored_files {
            println!("  {} {}", "✓".green(), file.display());
        }
    }

    if !result.skipped_files.is_empty() {
        println!();
        println!("Skipped {} file(s) (no changes):", result.skipped_files.len());
        for file in &result.skipped_files {
            println!("  {} {}", "○".dimmed(), file.display());
        }
    }

    if !result.failed_files.is_empty() {
        println!();
        println!("{} {} file(s):", "Failed".red(), result.failed_files.len());
        for (file, error) in &result.failed_files {
            println!("  {} {} - {}", "✗".red(), file.display(), error);
        }
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let hash = compute_hash("hello world");
        assert_eq!(hash.len(), 64); // SHA256 produces 64 hex chars
    }

    #[test]
    fn test_generate_snapshot_id() {
        let id = generate_snapshot_id();
        assert!(id.starts_with("snap-"));
    }

    #[test]
    fn test_protection_config_default() {
        let config = ProtectionConfig::default();
        assert!(config.snapshots_enabled);
        assert!(config.verify_build);
        assert!(!config.interactive_approval);
    }
}
