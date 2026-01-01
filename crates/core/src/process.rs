//! Process execution utilities
//!
//! Provides a unified interface for running external commands with:
//! - Output capture
//! - Directory context
//! - Environment variables
//! - Streaming output

use crate::error::{Error, Result};
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// Result of a command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Whether the command succeeded (exit code 0)
    pub success: bool,
    /// Exit code of the command
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
}

impl CommandResult {
    /// Create from std::process::Output
    pub fn from_output(output: Output) -> Self {
        Self {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    /// Get combined output (stdout + stderr)
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// Run a command and capture output
pub fn run_command(program: &str, args: &[&str]) -> Result<CommandResult> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Error::process(format!("Failed to execute {}: {}", program, e)))?;

    Ok(CommandResult::from_output(output))
}

/// Run a command in a specific directory
pub fn run_command_in_dir(program: &str, args: &[&str], dir: &Path) -> Result<CommandResult> {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Error::process(format!("Failed to execute {}: {}", program, e)))?;

    Ok(CommandResult::from_output(output))
}

/// Run a command with environment variables
pub fn run_command_with_env(
    program: &str,
    args: &[&str],
    env: &[(&str, &str)],
) -> Result<CommandResult> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, value) in env {
        cmd.env(key, value);
    }

    let output = cmd
        .output()
        .map_err(|e| Error::process(format!("Failed to execute {}: {}", program, e)))?;

    Ok(CommandResult::from_output(output))
}

/// Check if a command exists in PATH
pub fn command_exists(program: &str) -> bool {
    #[cfg(unix)]
    {
        Command::new("sh")
            .args(["-c", &format!("command -v {} >/dev/null 2>&1", program)])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(program)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// Get the path to a command
pub fn which_command(program: &str) -> Option<std::path::PathBuf> {
    #[cfg(unix)]
    {
        let output = Command::new("sh")
            .args(["-c", &format!("command -v {}", program)])
            .output()
            .ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(std::path::PathBuf::from(path));
            }
        }
        None
    }
    #[cfg(windows)]
    {
        let output = Command::new("where")
            .arg(program)
            .output()
            .ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()?
                .trim()
                .to_string();
            if !path.is_empty() {
                return Some(std::path::PathBuf::from(path));
            }
        }
        None
    }
}

/// Run a command and stream output to stdout/stderr (for interactive use)
pub fn run_command_streaming(program: &str, args: &[&str]) -> Result<i32> {
    let status = Command::new(program)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| Error::process(format!("Failed to execute {}: {}", program, e)))?;

    Ok(status.code().unwrap_or(-1))
}

/// Run a command and stream output in a specific directory
pub fn run_command_streaming_in_dir(program: &str, args: &[&str], dir: &Path) -> Result<i32> {
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| Error::process(format!("Failed to execute {}: {}", program, e)))?;

    Ok(status.code().unwrap_or(-1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_exists_echo() {
        assert!(command_exists("echo"));
    }

    #[test]
    fn test_command_exists_nonexistent() {
        assert!(!command_exists("nonexistent_command_12345"));
    }

    #[test]
    fn test_run_command_echo() {
        let result = run_command("echo", &["hello"]).unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("hello"));
    }

    #[test]
    fn test_command_result_combined_output() {
        let result = CommandResult {
            success: true,
            exit_code: 0,
            stdout: "out".to_string(),
            stderr: "err".to_string(),
        };
        assert!(result.combined_output().contains("out"));
        assert!(result.combined_output().contains("err"));
    }
}
