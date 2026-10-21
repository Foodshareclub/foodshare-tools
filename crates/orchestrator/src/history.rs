//! The history of finished runs.
//!
//! A daemon that only knows about the run in progress cannot say what
//! happened before it started. The history is the answer: the last few
//! runs, kept on disk so they survive a restart.

use std::path::PathBuf;
use std::sync::RwLock;

use foodshare_core::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// How many runs are kept.
pub const DEFAULT_LIMIT: usize = 20;

/// The name of the file, under the state directory.
pub const FILE_NAME: &str = "orchestrator-runs.json";

/// What one part of a run did.
///
/// A part is a domain of a plan, or a stage of a verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunPart {
    /// The part's name.
    pub name: String,
    /// Whether it passed.
    pub success: bool,
    /// How long it took.
    pub duration_ms: u64,
}

/// One finished run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRecord {
    /// When the run started.
    pub started_at: String,
    /// When the run finished.
    pub finished_at: String,
    /// How long the run took.
    pub duration_ms: u64,
    /// Whether every domain passed.
    pub success: bool,
    /// What each part of the run did.
    pub parts: Vec<RunPart>,
}

/// The history of finished runs.
#[derive(Debug)]
pub struct History {
    path: Option<PathBuf>,
    limit: usize,
    records: RwLock<Vec<RunRecord>>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            path: None,
            limit: DEFAULT_LIMIT,
            records: RwLock::new(Vec::new()),
        }
    }
}

impl History {
    /// Read the history, if there is one.
    ///
    /// A file that is not there is not an error: the history starts
    /// empty. A file that cannot be read *is* an error, because silently
    /// starting again would lose the record of what happened.
    pub fn load(path: Option<PathBuf>, limit: usize) -> Result<Self> {
        let records = match &path {
            Some(path) if path.is_file() => {
                let content = std::fs::read_to_string(path).map_err(|e| {
                    Error::io(format!(
                        "Failed to read the run history {}: {e}",
                        path.display()
                    ))
                })?;
                serde_json::from_str(&content).map_err(|e| {
                    Error::io(format!(
                        "Failed to parse the run history {}: {e}",
                        path.display()
                    ))
                })?
            }
            _ => Vec::new(),
        };

        Ok(Self {
            path,
            limit,
            records: RwLock::new(records),
        })
    }

    /// The runs, newest first.
    #[must_use]
    pub fn records(&self) -> Vec<RunRecord> {
        self.records
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// How many runs are kept.
    #[must_use]
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Add a run, and write the history out.
    pub fn record(&self, record: RunRecord) -> Result<()> {
        {
            let mut records = self.records.write().unwrap_or_else(|e| e.into_inner());
            records.insert(0, record);
            records.truncate(self.limit);
        }
        self.save()
    }

    /// Write the history out.
    fn save(&self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::io(format!(
                    "Failed to create the history directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let content = serde_json::to_string_pretty(&self.records())
            .map_err(|e| Error::io(format!("Failed to serialise the run history: {e}")))?;
        std::fs::write(path, content).map_err(|e| {
            Error::io(format!(
                "Failed to write the run history {}: {e}",
                path.display()
            ))
        })
    }
}

/// Where the history is kept, when no path is given.
///
/// The state directory, rather than the workspace: the history belongs to
/// the machine, not to the repository.
#[must_use]
pub fn default_path() -> Option<PathBuf> {
    dirs::state_dir().map(|dir| dir.join("foodshare").join(FILE_NAME))
}

/// The current time, as the history records it.
#[must_use]
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(success: bool) -> RunRecord {
        RunRecord {
            started_at: "2026-10-21T00:00:00Z".to_string(),
            finished_at: "2026-10-21T00:01:00Z".to_string(),
            duration_ms: 60_000,
            success,
            parts: vec![RunPart {
                name: "rust".to_string(),
                success,
                duration_ms: 60_000,
            }],
        }
    }

    #[test]
    fn test_a_missing_file_is_an_empty_history() {
        let dir = tempfile::tempdir().expect("temp dir");
        let history = History::load(Some(dir.path().join("runs.json")), 5).expect("loaded");
        assert!(history.records().is_empty());
    }

    #[test]
    fn test_a_run_is_kept_and_written() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("runs.json");
        let history = History::load(Some(path.clone()), 5).expect("loaded");
        history.record(record(true)).expect("recorded");

        assert_eq!(history.records().len(), 1);
        assert!(path.is_file());

        // And it survives a restart.
        let reloaded = History::load(Some(path), 5).expect("loaded");
        assert_eq!(reloaded.records().len(), 1);
        assert!(reloaded.records()[0].success);
    }

    #[test]
    fn test_the_newest_run_comes_first() {
        let dir = tempfile::tempdir().expect("temp dir");
        let history = History::load(Some(dir.path().join("runs.json")), 5).expect("loaded");
        history.record(record(true)).expect("recorded");
        history.record(record(false)).expect("recorded");

        let records = history.records();
        assert_eq!(records.len(), 2);
        assert!(!records[0].success);
        assert!(records[1].success);
    }

    #[test]
    fn test_the_history_is_trimmed_to_the_limit() {
        let dir = tempfile::tempdir().expect("temp dir");
        let history = History::load(Some(dir.path().join("runs.json")), 2).expect("loaded");
        for _ in 0..5 {
            history.record(record(true)).expect("recorded");
        }
        assert_eq!(history.records().len(), 2);
    }

    #[test]
    fn test_a_broken_history_is_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("runs.json");
        std::fs::write(&path, "not json").expect("write");
        assert!(History::load(Some(path), 5).is_err());
    }

    #[test]
    fn test_a_history_without_a_path_is_not_written() {
        let history = History::load(None, 5).expect("loaded");
        history.record(record(true)).expect("recorded");
        assert_eq!(history.records().len(), 1);
    }
}
