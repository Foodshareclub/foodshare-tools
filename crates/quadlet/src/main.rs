//! foodshare-quadlet — Quadlet validation and hygiene
//!
//! Validates `.container` files against the Podman Quadlet specification,
//! catching the failure modes that previously made every unit undeployable.
//! Provides a standalone `quadlet-hygiene` binary for CI usage.

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};

/// Error codes for Quadlet validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    FileReadError,
    FileNotFound,
    ValidationError,
    AbsoluteUsersVolume,
    TranscriptLeakage,
    UnexpandedTokens,
    BakedSecrets,
    MissingRequiredKeys,
    OperationalError,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::FileReadError => write!(f, "FileReadError"),
            ErrorCode::FileNotFound => write!(f, "FileNotFound"),
            ErrorCode::ValidationError => write!(f, "ValidationError"),
            ErrorCode::AbsoluteUsersVolume => write!(f, "AbsoluteUsersVolume"),
            ErrorCode::TranscriptLeakage => write!(f, "TranscriptLeakage"),
            ErrorCode::UnexpandedTokens => write!(f, "UnexpandedTokens"),
            ErrorCode::BakedSecrets => write!(f, "BakedSecrets"),
            ErrorCode::MissingRequiredKeys => write!(f, "MissingRequiredKeys"),
            ErrorCode::OperationalError => write!(f, "OperationalError"),
        }
    }
}

/// Enterprise error with code, message, and recovery suggestion.
#[derive(Debug, Clone)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    pub suggestion: String,
}

impl Error {
    pub fn new(code: ErrorCode, message: String, suggestion: String) -> Self {
        Error {
            code,
            message,
            suggestion,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:?}] {} (suggestion: {})",
            self.code, self.message, self.suggestion
        )
    }
}

impl std::error::Error for Error {}

/// Validation result type
pub type QuadletResult<T> = std::result::Result<T, Error>;

/// Check a single `.container` file for validity.
fn check_container_file(path: &Path) -> QuadletResult<()> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Error::new(
            ErrorCode::FileReadError,
            format!("Failed to read {}: {}", path.display(), e).to_string(),
            "Check file path and permissions".to_string(),
        )
    })?;

    let mut issues: Vec<String> = Vec::new();

    // 1. Check for absolute /Users/ paths
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.contains("Volume=/Users/") || trimmed.to_lowercase().contains("volume=/users/") {
            if let Some(eq_pos) = trimmed.find('=') {
                let after_eq = &trimmed[eq_pos + 1..];
                if after_eq.starts_with("/Users/") || after_eq.contains("/Users/") {
                    issues.push(
                        format!(
                            "{} : absolute /Users/ Volume directive (laptop-only)",
                            path.display()
                        )
                        .to_string(),
                    );
                    break;
                }
            }
        }
    }

    // 2. Check for transcript leakage
    let transcript_patterns = ["tool_call", "function=shell", "<parameter=command"];
    for line in content.lines() {
        let lower = line.to_lowercase();
        for pattern in &transcript_patterns {
            if lower.contains(pattern) {
                issues
                    .push(format!("{} : transcript leakage detected", path.display()).to_string());
                break;
            }
        }
    }

    // 3. Check for required keys: Image= and HealthCmd=
    if !content.contains("Image=") {
        issues.push(format!("{} : missing Image= key", path.display()).to_string());
    }
    if !content.contains("HealthCmd=") {
        issues.push(format!("{} : missing HealthCmd= key", path.display()).to_string());
    }

    if issues.is_empty() {
        Ok(())
    } else {
        let msg = issues.join("; ");
        Err(Error::new(
            ErrorCode::ValidationError,
            msg,
            "Fix the issues listed above".to_string(),
        ))
    }
}

/// Check multiple `.container` files.
fn check_files(paths: &[PathBuf]) -> QuadletResult<()> {
    if paths.is_empty() {
        return Err(Error::new(
            ErrorCode::ValidationError,
            "no files to check".to_string(),
            "provide at least one .container file path".to_string(),
        ));
    }

    let bar = ProgressBar::new(paths.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg} {bar:40.cyan/blue} {pos:>7}/{pos:7} {msg}")
            .unwrap(),
    );

    let mut failures = 0;
    for path in paths {
        bar.set_message(format!("Checking {}", path.display()));
        if let Err(e) = check_container_file(path) {
            eprintln!("{} :: FAIL: {}", path.display(), e);
            failures += 1;
        }
        bar.inc(1);
    }
    bar.finish();

    if failures > 0 {
        Err(Error::new(
            ErrorCode::ValidationError,
            format!("{} file(s) failed check", failures).to_string(),
            "see individual errors above".to_string(),
        ))
    } else {
        Ok(())
    }
}

#[derive(Parser)]
#[command(name = "quadlet")]
#[command(about = "Quadlet validation and hygiene for Podman GitHub runners")]
struct Cli {
    /// Check specific .container files or directories
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let default_paths: Vec<PathBuf> = vec![
        PathBuf::from("quadlet/foodshare-runner-p01.container"),
        PathBuf::from("quadlet/foodshare-runner-p02.container"),
        PathBuf::from("quadlet/foodshare-runner-p03.container"),
        PathBuf::from("quadlet/foodshare-runner-p04.container"),
        PathBuf::from("quadlet/foodshare-runner-p05.container"),
        PathBuf::from("quadlet/foodshare-runner-p06.container"),
        PathBuf::from("quadlet/foodshare-runner-p07.container"),
        PathBuf::from("quadlet/foodshare-runner-p08.container"),
        PathBuf::from("quadlet/foodshare-org.container"),
        PathBuf::from("quadlet/foodshare-network.service"),
        PathBuf::from("quadlet/foodshare-web.container"),
        PathBuf::from("quadlet/foodshare-cloudflared.container"),
        PathBuf::from("quadlet/foodshare-runner-base.container"),
    ];

    let paths_to_check = if cli.files.is_empty() {
        default_paths.clone()
    } else {
        cli.files.clone()
    };

    match check_files(&paths_to_check) {
        Ok(()) => {
            println!("All {} hygiene checks passed.", paths_to_check.len());
        }
        Err(e) => {
            eprintln!("quadlet hygiene FAILED: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A container file that passes every check.
    const GOOD: &str = "\
[Container]
Image=quay.io/fedora/fedora:41
HealthCmd=/usr/bin/true
Volume=/home/runner/work:/home/runner/work
";

    fn write(content: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("unit.container");
        std::fs::write(&path, content).expect("the file should be written");
        (dir, path)
    }

    #[test]
    fn a_good_container_file_passes() {
        let (_dir, path) = write(GOOD);
        assert!(check_container_file(&path).is_ok());
    }

    #[test]
    fn a_missing_image_key_is_an_error() {
        let (_dir, path) = write("HealthCmd=/usr/bin/true\n");
        let error = check_container_file(&path).expect_err("the file should fail");
        assert_eq!(error.code, ErrorCode::ValidationError);
        assert!(
            error.message.contains("missing Image="),
            "the reason should be named: {}",
            error.message
        );
    }

    #[test]
    fn a_missing_health_command_is_an_error() {
        let (_dir, path) = write("Image=quay.io/fedora/fedora:41\n");
        let error = check_container_file(&path).expect_err("the file should fail");
        assert!(
            error.message.contains("missing HealthCmd="),
            "the reason should be named: {}",
            error.message
        );
    }

    #[test]
    fn an_absolute_users_volume_is_an_error() {
        // The path only works on the machine that wrote it, which is the
        // failure this validator exists to catch.
        let (_dir, path) = write(&format!("{GOOD}Volume=/Users/someone/work:/work\n"));
        let error = check_container_file(&path).expect_err("the file should fail");
        assert!(
            error.message.contains("absolute /Users/"),
            "the reason should be named: {}",
            error.message
        );
    }

    #[test]
    fn a_transcript_leak_is_an_error() {
        let (_dir, path) = write(&format!("{GOOD}# tool_call in the notes\n"));
        let error = check_container_file(&path).expect_err("the file should fail");
        assert!(
            error.message.contains("transcript leakage"),
            "the reason should be named: {}",
            error.message
        );
    }

    #[test]
    fn a_file_that_is_not_there_is_an_error() {
        let error = check_container_file(Path::new("/not/a/file.container"))
            .expect_err("the file should fail");
        assert_eq!(error.code, ErrorCode::FileReadError);
    }

    #[test]
    fn no_files_at_all_is_an_error() {
        let error = check_files(&[]).expect_err("the check should fail");
        assert_eq!(error.code, ErrorCode::ValidationError);
    }
}
