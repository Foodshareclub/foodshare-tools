//! Counting the markers in a source tree.
//!
//! The counts are the ones the configuration's thresholds are about: the
//! TODOs, the FIXMEs, and the force unwraps. They are a ratchet, so the
//! counting has to be simple and repeatable rather than clever.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// What the scan found.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Analysis {
    /// The files that were looked at.
    pub files_scanned: usize,
    /// The lines that were looked at.
    pub lines_scanned: usize,
    /// The TODO markers.
    pub todos: usize,
    /// The FIXME markers.
    pub fixmes: usize,
    /// The force unwraps.
    pub force_unwraps: usize,
}

/// The files a scan looks at.
///
/// A `.swift` or a `.kt` file is source; everything else is not.
fn source_files(path: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != "node_modules" && name != ".build"
        })
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry
            .path()
            .extension()
            .is_some_and(|e| e == "swift" || e == "kt" || e == "kts")
        {
            found.push(entry.path().to_path_buf());
        }
    }

    found.sort();
    found
}

/// Count the markers in the source files below a path.
pub fn analyze(path: &Path) -> Analysis {
    let mut analysis = Analysis::default();

    for file in source_files(path) {
        let Ok(content) = std::fs::read_to_string(&file) else {
            continue;
        };

        analysis.files_scanned += 1;

        for line in content.lines() {
            analysis.lines_scanned += 1;

            let upper = line.to_uppercase();
            if upper.contains("TODO") {
                analysis.todos += 1;
            }
            if upper.contains("FIXME") {
                analysis.fixmes += 1;
            }
            analysis.force_unwraps += count_force_unwraps(line);
        }
    }

    analysis
}

/// Count the force unwraps in one line.
///
/// A Swift force unwrap is a `!` that is not `!=`, and a Kotlin one is
/// a `!!`. The count is a heuristic, and it says so: it is a ratchet,
/// not a compiler.
fn count_force_unwraps(line: &str) -> usize {
    let mut count = 0;
    let characters: Vec<char> = line.chars().collect();
    let mut index = 0;

    while index < characters.len() {
        if characters[index] == '!' {
            // `!=` is a comparison, not an unwrap.
            if characters.get(index + 1) == Some(&'=') {
                index += 2;
                continue;
            }
            // A `!` in a string is not an unwrap.
            if in_string(&line, index) {
                index += 1;
                continue;
            }
            count += 1;
            // `!!` is one force unwrap, not two.
            if characters.get(index + 1) == Some(&'!') {
                index += 1;
            }
        }
        index += 1;
    }

    count
}

/// Whether a character is inside a string literal.
fn in_string(line: &str, at: usize) -> bool {
    let mut inside = false;
    let mut escaped = false;

    for (index, character) in line.chars().enumerate() {
        if index >= at {
            break;
        }
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => inside = !inside,
            _ => {}
        }
    }

    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a_todo_is_counted() {
        let analysis = analyze_text("let x = 1 // TODO: finish this\n");
        assert_eq!(analysis.todos, 1);
    }

    #[test]
    fn test_a_fixme_is_counted() {
        let analysis = analyze_text("// FIXME: this is wrong\n");
        assert_eq!(analysis.fixmes, 1);
    }

    #[test]
    fn test_a_force_unwrap_is_counted() {
        let analysis = analyze_text("let value = optional!\n");
        assert_eq!(analysis.force_unwraps, 1);
    }

    #[test]
    fn test_a_comparison_is_not_a_force_unwrap() {
        let analysis = analyze_text("if a != b { }\n");
        assert_eq!(analysis.force_unwraps, 0);
    }

    #[test]
    fn test_a_bang_in_a_string_is_not_a_force_unwrap() {
        let analysis = analyze_text("let text = \"hello!\"\n");
        assert_eq!(analysis.force_unwraps, 0);
    }

    #[test]
    fn test_a_double_bang_is_one_force_unwrap() {
        let analysis = analyze_text("let value = optional!!\n");
        assert_eq!(analysis.force_unwraps, 1);
    }

    #[test]
    fn test_a_clean_line_counts_nothing() {
        let analysis = analyze_text("let x = 1\n");
        assert_eq!(analysis.todos, 0);
        assert_eq!(analysis.fixmes, 0);
        assert_eq!(analysis.force_unwraps, 0);
    }

    /// Analyze a piece of text as if it were one file.
    fn analyze_text(text: &str) -> Analysis {
        let mut analysis = Analysis::default();
        analysis.files_scanned = 1;

        for line in text.lines() {
            analysis.lines_scanned += 1;
            let upper = line.to_uppercase();
            if upper.contains("TODO") {
                analysis.todos += 1;
            }
            if upper.contains("FIXME") {
                analysis.fixmes += 1;
            }
            analysis.force_unwraps += count_force_unwraps(line);
        }

        analysis
    }
}
