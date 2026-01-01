//! Terminal output utilities
//!
//! Provides consistent formatting for CLI output.

use owo_colors::OwoColorize;

/// Status message helpers
pub struct Status;

impl Status {
    /// Print a success message
    pub fn success(message: &str) {
        println!("{} {}", "✓".green(), message);
    }

    /// Print an error message
    pub fn error(message: &str) {
        eprintln!("{} {}", "✗".red(), message);
    }

    /// Print a warning message
    pub fn warning(message: &str) {
        eprintln!("{} {}", "⚠".yellow(), message);
    }

    /// Print an info message
    pub fn info(message: &str) {
        println!("{} {}", "ℹ".blue(), message);
    }

    /// Print a step message (for multi-step operations)
    pub fn step(step: usize, total: usize, message: &str) {
        println!(
            "{} {}",
            format!("[{}/{}]", step, total).dimmed(),
            message
        );
    }

    /// Print a header
    pub fn header(message: &str) {
        println!();
        println!("{}", message.bold());
        println!("{}", "─".repeat(message.len()));
    }

    /// Print a subheader
    pub fn subheader(message: &str) {
        println!();
        println!("{}", message.bold().dimmed());
    }
}

/// Format a duration for display
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs_f32();
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{:.1}s", secs)
    } else {
        let mins = (secs / 60.0).floor();
        let remaining_secs = secs % 60.0;
        format!("{}m {:.0}s", mins, remaining_secs)
    }
}

/// Format a file size for display
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format a count with singular/plural
pub fn format_count(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{} {}", count, singular)
    } else {
        format!("{} {}", count, plural)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_format_duration_ms() {
        let d = Duration::from_millis(500);
        assert_eq!(format_duration(d), "500ms");
    }

    #[test]
    fn test_format_duration_secs() {
        let d = Duration::from_secs_f32(5.5);
        assert_eq!(format_duration(d), "5.5s");
    }

    #[test]
    fn test_format_duration_mins() {
        let d = Duration::from_secs(125);
        assert_eq!(format_duration(d), "2m 5s");
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(500), "500 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(2048), "2.00 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(5 * 1024 * 1024), "5.00 MB");
    }

    #[test]
    fn test_format_count_singular() {
        assert_eq!(format_count(1, "file", "files"), "1 file");
    }

    #[test]
    fn test_format_count_plural() {
        assert_eq!(format_count(5, "file", "files"), "5 files");
    }
}
