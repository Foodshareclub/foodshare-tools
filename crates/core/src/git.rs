//! Git operations using command-line git
//!
//! Provides a unified interface for git operations across all platforms.
//! Uses command-line git to avoid dependency issues with git2/libgit2.

use crate::error::{Error, Result};
use crate::process::run_command_in_dir;
use std::path::{Path, PathBuf};

/// Git repository wrapper
pub struct GitRepo {
    workdir: PathBuf,
}

impl GitRepo {
    /// Open a git repository at the given path
    pub fn open(path: &Path) -> Result<Self> {
        // Verify it's a git repo
        let result = run_command_in_dir("git", &["rev-parse", "--git-dir"], path)?;
        if !result.success {
            return Err(Error::not_a_git_repo());
        }

        // Get the working directory root
        let result = run_command_in_dir("git", &["rev-parse", "--show-toplevel"], path)?;
        let workdir = PathBuf::from(result.stdout.trim());

        Ok(Self { workdir })
    }

    /// Open the repository in the current directory
    pub fn open_current() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        Self::open(&current_dir)
    }

    /// Get the repository working directory
    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    /// Get staged files (files in the index that differ from HEAD)
    pub fn staged_files(&self) -> Result<Vec<PathBuf>> {
        let result = run_command_in_dir(
            "git",
            &["diff", "--cached", "--name-only", "--diff-filter=ACMR"],
            &self.workdir,
        )?;

        Ok(result
            .stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect())
    }

    /// Get staged files filtered by extension
    pub fn staged_files_with_extension(&self, extensions: &[&str]) -> Result<Vec<PathBuf>> {
        let files = self.staged_files()?;
        Ok(files
            .into_iter()
            .filter(|f| {
                f.extension()
                    .and_then(|e| e.to_str())
                    .map_or(false, |ext| extensions.contains(&ext))
            })
            .collect())
    }

    /// Get staged Swift files
    pub fn staged_swift_files(&self) -> Result<Vec<PathBuf>> {
        self.staged_files_with_extension(&["swift"])
    }

    /// Get staged Kotlin files
    pub fn staged_kotlin_files(&self) -> Result<Vec<PathBuf>> {
        self.staged_files_with_extension(&["kt", "kts"])
    }

    /// Get staged TypeScript/JavaScript files
    pub fn staged_ts_files(&self) -> Result<Vec<PathBuf>> {
        self.staged_files_with_extension(&["ts", "tsx", "js", "jsx"])
    }

    /// Get modified files (not yet staged)
    pub fn modified_files(&self) -> Result<Vec<PathBuf>> {
        let result = run_command_in_dir(
            "git",
            &["diff", "--name-only"],
            &self.workdir,
        )?;

        Ok(result
            .stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect())
    }

    /// Get untracked files
    pub fn untracked_files(&self) -> Result<Vec<PathBuf>> {
        let result = run_command_in_dir(
            "git",
            &["ls-files", "--others", "--exclude-standard"],
            &self.workdir,
        )?;

        Ok(result
            .stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect())
    }

    /// Stage a file
    pub fn stage_file(&self, path: &Path) -> Result<()> {
        let result = run_command_in_dir(
            "git",
            &["add", &path.to_string_lossy()],
            &self.workdir,
        )?;

        if result.success {
            Ok(())
        } else {
            Err(Error::git(format!("Failed to stage file: {}", result.stderr)))
        }
    }

    /// Stage multiple files
    pub fn stage_files(&self, paths: &[PathBuf]) -> Result<()> {
        for path in paths {
            self.stage_file(path)?;
        }
        Ok(())
    }

    /// Get the current branch name
    pub fn current_branch(&self) -> Result<String> {
        let result = run_command_in_dir(
            "git",
            &["rev-parse", "--abbrev-ref", "HEAD"],
            &self.workdir,
        )?;

        Ok(result.stdout.trim().to_string())
    }

    /// Get the latest tag
    pub fn latest_tag(&self) -> Result<Option<String>> {
        let result = run_command_in_dir(
            "git",
            &["describe", "--tags", "--abbrev=0"],
            &self.workdir,
        )?;

        if result.success && !result.stdout.trim().is_empty() {
            Ok(Some(result.stdout.trim().to_string()))
        } else {
            Ok(None)
        }
    }

    /// Get commits since a specific tag or ref
    pub fn commits_since(&self, since: &str) -> Result<Vec<String>> {
        let result = run_command_in_dir(
            "git",
            &["log", &format!("{}..HEAD", since), "--oneline", "--format=%s"],
            &self.workdir,
        )?;

        Ok(result
            .stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }

    /// Check if there are uncommitted changes
    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        let result = run_command_in_dir(
            "git",
            &["status", "--porcelain"],
            &self.workdir,
        )?;

        Ok(!result.stdout.trim().is_empty())
    }

    /// Get diff statistics
    pub fn diff_stats(&self) -> Result<DiffStats> {
        let result = run_command_in_dir(
            "git",
            &["diff", "--stat", "--shortstat"],
            &self.workdir,
        )?;

        // Parse shortstat output: " 3 files changed, 10 insertions(+), 5 deletions(-)"
        let mut stats = DiffStats {
            files_changed: 0,
            insertions: 0,
            deletions: 0,
        };

        for line in result.stdout.lines() {
            if line.contains("files changed") || line.contains("file changed") {
                // Parse the numbers
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

        Ok(stats)
    }

    /// Check if a path is ignored by git
    pub fn is_ignored(&self, path: &Path) -> bool {
        let result = run_command_in_dir(
            "git",
            &["check-ignore", "-q", &path.to_string_lossy()],
            &self.workdir,
        );

        result.map(|r| r.success).unwrap_or(false)
    }

    /// Get uncommitted files (both staged and unstaged)
    pub fn uncommitted_files(&self) -> Result<Vec<PathBuf>> {
        let result = run_command_in_dir(
            "git",
            &["status", "--porcelain"],
            &self.workdir,
        )?;

        Ok(result
            .stdout
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|l| {
                // Format: "XY filename" where X is index status, Y is worktree status
                if l.len() > 3 {
                    Some(PathBuf::from(l[3..].trim()))
                } else {
                    None
                }
            })
            .collect())
    }
}

/// Statistics from a git diff
#[derive(Debug, Clone)]
pub struct DiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

/// Check if we're in a git repository
pub fn is_git_repo(path: &Path) -> bool {
    run_command_in_dir("git", &["rev-parse", "--git-dir"], path)
        .map(|r| r.success)
        .unwrap_or(false)
}

/// Get the git root directory
pub fn git_root(path: &Path) -> Option<PathBuf> {
    run_command_in_dir("git", &["rev-parse", "--show-toplevel"], path)
        .ok()
        .filter(|r| r.success)
        .map(|r| PathBuf::from(r.stdout.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_is_git_repo() {
        // Test with a path that might or might not be a git repo
        // This test just verifies the function doesn't panic
        let current = env::current_dir().unwrap();
        let _ = is_git_repo(&current); // Just verify it runs without panic
    }

    #[test]
    fn test_diff_stats_clone() {
        let stats = DiffStats {
            files_changed: 3,
            insertions: 20,
            deletions: 10,
        };
        let cloned = stats.clone();
        assert_eq!(cloned.files_changed, 3);
        assert_eq!(cloned.insertions, 20);
        assert_eq!(cloned.deletions, 10);
    }
}
