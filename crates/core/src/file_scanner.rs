//! File scanning utilities
//!
//! Provides efficient file discovery and filtering across the codebase.

use crate::error::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// File scanner with configurable filters
pub struct FileScanner {
    root: PathBuf,
    extensions: Vec<String>,
    exclude_patterns: Vec<String>,
    respect_gitignore: bool,
}

impl FileScanner {
    /// Create a new file scanner rooted at the given path
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            extensions: Vec::new(),
            exclude_patterns: Vec::new(),
            respect_gitignore: true,
        }
    }

    /// Filter by file extensions (e.g., "swift", "kt", "ts")
    pub fn with_extensions(mut self, extensions: &[&str]) -> Self {
        self.extensions = extensions.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add patterns to exclude (glob patterns)
    pub fn exclude(mut self, patterns: &[&str]) -> Self {
        self.exclude_patterns = patterns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Whether to respect .gitignore files
    pub fn respect_gitignore(mut self, respect: bool) -> Self {
        self.respect_gitignore = respect;
        self
    }

    /// Scan and return matching files
    pub fn scan(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_entry(|e| !self.is_hidden(e.path()))
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            // Check extension filter
            if !self.extensions.is_empty() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                if !self.extensions.iter().any(|e| e == ext) {
                    continue;
                }
            }

            // Check exclude patterns
            let path_str = path.to_string_lossy();
            if self.should_exclude(&path_str) {
                continue;
            }

            files.push(path.to_path_buf());
        }

        Ok(files)
    }

    fn is_hidden(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.') && n != "." && n != "..")
            .unwrap_or(false)
    }

    fn should_exclude(&self, path_str: &str) -> bool {
        for pattern in &self.exclude_patterns {
            // Simple glob matching
            if pattern.contains("**") {
                let parts: Vec<&str> = pattern.split("**").collect();
                if parts.len() == 2 {
                    let suffix = parts[1].trim_start_matches('/');
                    if path_str.contains(suffix) {
                        return true;
                    }
                }
            } else if let Ok(pat) = glob::Pattern::new(pattern) {
                if pat.matches(path_str) {
                    return true;
                }
            }
        }
        false
    }
}

/// Scan for Swift files in a directory
pub fn scan_swift_files(root: &Path) -> Result<Vec<PathBuf>> {
    FileScanner::new(root)
        .with_extensions(&["swift"])
        .exclude(&["**/Tests/**", "**/DerivedData/**", "**/.build/**"])
        .scan()
}

/// Scan for Kotlin files in a directory
pub fn scan_kotlin_files(root: &Path) -> Result<Vec<PathBuf>> {
    FileScanner::new(root)
        .with_extensions(&["kt", "kts"])
        .exclude(&["**/build/**", "**/generated/**"])
        .scan()
}

/// Scan for TypeScript/JavaScript files in a directory
pub fn scan_ts_files(root: &Path) -> Result<Vec<PathBuf>> {
    FileScanner::new(root)
        .with_extensions(&["ts", "tsx", "js", "jsx"])
        .exclude(&["**/node_modules/**", "**/dist/**", "**/.next/**"])
        .scan()
}

/// Count lines of code in a file
pub fn count_lines(path: &Path) -> Result<usize> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.lines().count())
}

/// Get file size in bytes
pub fn file_size(path: &Path) -> Result<u64> {
    Ok(std::fs::metadata(path)?.len())
}

/// Find files matching a pattern recursively (simple walkdir-based)
pub fn find_files(root: &Path, pattern: &str) -> Vec<PathBuf> {
    let glob_pattern = glob::Pattern::new(pattern).ok();

    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            glob_pattern
                .as_ref()
                .map_or(true, |p| p.matches_path(e.path()))
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_scanner_new() {
        let scanner = FileScanner::new("/tmp");
        assert_eq!(scanner.root, PathBuf::from("/tmp"));
        assert!(scanner.extensions.is_empty());
    }

    #[test]
    fn test_file_scanner_with_extensions() {
        let scanner = FileScanner::new("/tmp").with_extensions(&["swift", "kt"]);
        assert_eq!(scanner.extensions, vec!["swift", "kt"]);
    }

    #[test]
    fn test_file_scanner_exclude() {
        let scanner = FileScanner::new("/tmp").exclude(&["**/build/**"]);
        assert_eq!(scanner.exclude_patterns, vec!["**/build/**"]);
    }
}
