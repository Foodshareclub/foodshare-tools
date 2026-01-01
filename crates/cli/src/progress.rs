//! Progress indicators
//!
//! Provides progress bars and spinners for long-running operations.

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Create a spinner for indeterminate progress
pub fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a progress bar for determinate progress
pub fn progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("█▓░"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Create a progress bar for file processing
pub fn file_progress(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
            .unwrap()
            .progress_chars("█▓░"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Finish a progress bar with a success message
pub fn finish_success(pb: &ProgressBar, message: &str) {
    pb.finish_with_message(format!("✓ {}", message));
}

/// Finish a progress bar with an error message
pub fn finish_error(pb: &ProgressBar, message: &str) {
    pb.finish_with_message(format!("✗ {}", message));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_creation() {
        let pb = spinner("Testing...");
        pb.finish();
    }

    #[test]
    fn test_progress_bar_creation() {
        let pb = progress_bar(100, "Processing");
        pb.inc(50);
        pb.finish();
    }
}
