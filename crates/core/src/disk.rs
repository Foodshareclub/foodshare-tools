//! Disk space health, auto-healing, and artefact cleanup.
//!
//! A build that fails because the disk is full is a failure the tools can
//! prevent: before a retry, the caches that grow without bound are pruned
//! and the space is checked again.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::process::run_command;

/// The directories that are pruned when a run is asked to clean up.
///
/// These are the build outputs of every repository in the workspace.
pub const ARTIFACT_DIRS: &[&str] = &[
    "foodshare-tools/target",
    "foodshare-app/.build",
    "foodshare-app/Android/app/build",
    "foodshare-app/Android/.gradle",
    "foodshare-app/Android/.skip-stub",
    "foodshare-app/Android/.skip-plugin-marker",
    "foodshare-web/.next",
    "foodshare-web/node_modules/.cache",
    "foodshare-web/.next/cache",
    "foodshare-backend/supabase/.temp",
];

/// How much free space, in gigabytes, the tools want before a build.
pub const MIN_FREE_GB: f64 = 4.0;

/// Free space on the volume holding a path.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DiskStats {
    /// Total size of the volume, in gigabytes.
    pub total_gb: f64,
    /// Used space, in gigabytes.
    pub used_gb: f64,
    /// Free space, in gigabytes.
    pub avail_gb: f64,
    /// Free space as a percentage of the total.
    pub avail_percent: u64,
}

/// What a heal pass did.
#[derive(Debug, Clone, Default, Serialize)]
pub struct HealReport {
    /// Whether anything was pruned.
    pub performed: bool,
    /// How much space the pass reclaimed, in gigabytes.
    pub freed_gb: f64,
    /// What was pruned, one line per action.
    pub actions: Vec<String>,
}

/// Read the free space of the volume holding `path`.
pub fn stats(path: &Path) -> Result<DiskStats> {
    let output = run_command("df", &["-k", &path.to_string_lossy()])?;
    parse_df(&output.stdout).ok_or_else(|| {
        Error::process(format!(
            "Could not read disk statistics for {}",
            path.display()
        ))
        .with_suggestion("Check that `df` is available on this machine")
    })
}

/// Parse the output of `df -k`.
///
/// The header is skipped by trying every line and keeping the first one
/// whose size columns are numbers; `df` output differs between macOS and
/// Linux, and a wrapped header would break a positional parse.
fn parse_df(output: &str) -> Option<DiskStats> {
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let (Ok(total), Ok(used), Ok(avail)) = (
            parts[1].parse::<u64>(),
            parts[2].parse::<u64>(),
            parts[3].parse::<u64>(),
        ) else {
            continue;
        };
        let to_gb = |kb: u64| (kb as f64 / 1024.0 / 1024.0 * 10.0).round() / 10.0;
        return Some(DiskStats {
            total_gb: to_gb(total),
            used_gb: to_gb(used),
            avail_gb: to_gb(avail),
            avail_percent: if total > 0 {
                (avail as f64 / total as f64 * 100.0).round() as u64
            } else {
                0
            },
        });
    }
    None
}

/// Free space on the volume holding the workspace root.
pub fn workspace_stats(root: &Path) -> Result<DiskStats> {
    stats(root)
}

/// Prune caches until there is enough free space.
///
/// The pass is skipped when there is already enough room, unless `force`
/// is set. Whatever it prunes is reported, so the caller can say what
/// happened.
pub fn heal(root: &Path, min_free_gb: f64, force: bool) -> Result<HealReport> {
    let before = stats(root)?;
    if !force && before.avail_gb >= min_free_gb {
        return Ok(HealReport {
            performed: false,
            freed_gb: 0.0,
            actions: Vec::new(),
        });
    }

    let mut actions = Vec::new();
    let critical = before.avail_gb < min_free_gb;

    // 1. Maestro test recordings.
    if let Some(home) = home_dir() {
        let maestro = home.join(".maestro").join("tests");
        if maestro.is_dir() {
            let entries = std::fs::read_dir(&maestro).map(|d| d.count()).unwrap_or(0);
            if remove_dir_all(&maestro) {
                actions.push(format!(
                    "Cleaned {entries} Maestro test recording(s) from {}",
                    maestro.display()
                ));
            }
        }

        // 2. The XCTest cache.
        let xctest = home
            .join("Library")
            .join("Caches")
            .join("com.apple.dt.XCTest");
        if xctest.exists() && remove_dir_all(&xctest) {
            actions.push("Cleaned the XCTest cache".to_string());
        }

        // 3. The Bun install cache, which is large and safe to drop.
        let bun_cache = home.join(".bun").join("install").join("cache");
        if bun_cache.exists() && (force || before.avail_gb < 2.0) && remove_dir_all(&bun_cache) {
            actions.push("Pruned the Bun install cache".to_string());
        }

        // 4. Xcode DerivedData for this project.
        let derived = home
            .join("Library")
            .join("Developer")
            .join("Xcode")
            .join("DerivedData");
        if derived.is_dir() && (force || critical) {
            if let Ok(entries) = std::fs::read_dir(&derived) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains("foodshare")
                        || name.contains("FoodShare")
                        || name.contains("Project-")
                    {
                        let path = entry.path();
                        if remove_dir_all(&path) {
                            actions.push(format!("Pruned Xcode DerivedData: {name}"));
                        }
                    }
                }
            }
        }
    }

    // 5. The app's own DerivedData logs.
    let derived_logs = root
        .join("foodshare-app")
        .join(".build")
        .join("DerivedData")
        .join("Logs");
    if derived_logs.exists() && remove_dir_all(&derived_logs) {
        actions.push("Pruned the Xcode compilation logs".to_string());
    }

    // 6. Cargo's incremental cache, which grows with every edit.
    let incremental = root
        .join("foodshare-tools")
        .join("target")
        .join("debug")
        .join("incremental");
    if incremental.exists() && (force || critical) && remove_dir_all(&incremental) {
        actions.push("Pruned the Cargo incremental cache".to_string());
    }

    let after = stats(root)?;
    let freed_gb = ((after.avail_gb - before.avail_gb) * 10.0).round() / 10.0;

    Ok(HealReport {
        performed: !actions.is_empty(),
        freed_gb: freed_gb.max(0.0),
        actions,
    })
}

/// Remove every build artefact in the workspace.
///
/// Returns the paths that were removed.
pub fn clean_artifacts(root: &Path) -> Vec<PathBuf> {
    let mut removed = Vec::new();
    for relative in ARTIFACT_DIRS {
        let path = root.join(relative);
        if path.exists() && remove_dir_all(&path) {
            removed.push(path);
        }
    }
    removed
}

/// Remove a path, whether it is a file or a directory.
fn remove_dir_all(path: &Path) -> bool {
    let result = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    result.is_ok()
}

/// The user's home directory.
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_df_reads_the_size_columns() {
        let output = "Filesystem 1024-blocks      Used Available Capacity iused      ifree %iused  Mounted on\n/dev/disk3s5s1  1048576000  104857600  943718400     10%  1234  5678    1%   /\n";
        let stats = parse_df(output).expect("parsed");
        assert_eq!(stats.total_gb, 1000.0);
        assert_eq!(stats.used_gb, 100.0);
        assert_eq!(stats.avail_gb, 900.0);
        assert_eq!(stats.avail_percent, 90);
    }

    #[test]
    fn test_parse_df_skips_the_header() {
        let output =
            "Filesystem 1024-blocks Used Available Capacity iused ifree %iused Mounted on\n";
        assert!(parse_df(output).is_none());
    }

    #[test]
    fn test_parse_df_of_nothing_is_none() {
        assert!(parse_df("").is_none());
        assert!(parse_df("not a table").is_none());
    }

    #[test]
    fn test_clean_artifacts_removes_the_listed_paths() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        let target = root.join("foodshare-tools/target");
        std::fs::create_dir_all(&target).expect("create target");
        std::fs::write(target.join("marker"), "x").expect("write marker");

        let removed = clean_artifacts(root);
        assert_eq!(removed.len(), 1);
        assert!(!target.exists());
    }

    #[test]
    fn test_clean_artifacts_ignores_missing_paths() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(clean_artifacts(dir.path()).is_empty());
    }

    #[test]
    fn test_heal_is_skipped_when_there_is_room() {
        let dir = tempfile::tempdir().expect("temp dir");
        let report = heal(dir.path(), 0.0, false).expect("heal");
        assert!(!report.performed);
        assert!(report.actions.is_empty());
    }
}
