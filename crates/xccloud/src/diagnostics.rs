//! The local build summaries.
//!
//! CI runs leave a Markdown summary of each Xcode Cloud build in the
//! app repository. They are what a developer reads when the API is not
//! reachable, or when the credentials are not set.

use std::path::{Path, PathBuf};

/// The directory the summaries are written to.
#[must_use]
pub fn logs_dir(app_dir: &Path) -> PathBuf {
    app_dir.join("logs").join("xcodecloud")
}

/// The summaries in a directory, newest first.
///
/// The files are ordered by name, which the CI names after the build, so
/// the order is the build order.
#[must_use]
pub fn summaries(app_dir: &Path) -> Vec<PathBuf> {
    let dir = logs_dir(app_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    files.sort();
    files.reverse();
    files
}

/// The summaries to show, and their contents.
///
/// Each summary is truncated, because a full build log is a wall of text
/// and the point is the headline.
#[must_use]
pub fn read_summaries(app_dir: &Path, count: usize, max_chars: usize) -> Vec<(PathBuf, String)> {
    summaries(app_dir)
        .into_iter()
        .take(count)
        .filter_map(|path| {
            let contents = std::fs::read_to_string(&path).ok()?;
            let truncated: String = contents.chars().take(max_chars).collect();
            Some((path, truncated))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summaries_of_a_missing_directory_is_empty() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(summaries(dir.path()).is_empty());
    }

    #[test]
    fn test_summaries_are_found_and_truncated() {
        let dir = tempfile::tempdir().expect("temp dir");
        let logs = logs_dir(dir.path());
        std::fs::create_dir_all(&logs).expect("create");
        std::fs::write(logs.join("build-1.md"), "abcdefghij").expect("write");
        std::fs::write(logs.join("build-2.md"), "klmnopqrst").expect("write");
        std::fs::write(logs.join("notes.txt"), "ignored").expect("write");

        let summaries = summaries(dir.path());
        assert_eq!(summaries.len(), 2);
        // The newest first.
        assert!(summaries[0].ends_with("build-2.md"));

        let read = read_summaries(dir.path(), 1, 4);
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].1, "klmn");
    }
}
