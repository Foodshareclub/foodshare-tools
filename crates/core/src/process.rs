//! Process execution utilities
//!
//! Provides a unified interface for running external commands with:
//! - Output capture
//! - Directory context
//! - Environment variables
//! - Streaming output

use crate::error::{Error, ErrorCode, Result};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};
use which::which as which_binary;

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
    /// Create from `std::process::Output`
    #[must_use]
    pub fn from_output(output: Output) -> Self {
        Self {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    /// Get combined output (stdout + stderr)
    #[must_use]
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
        .map_err(|e| Error::process(format!("Failed to execute {program}: {e}")))?;

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
        .map_err(|e| Error::process(format!("Failed to execute {program}: {e}")))?;

    Ok(CommandResult::from_output(output))
}

/// How often the timeout watcher looks to see whether the child has finished.
///
/// The child is polled rather than watched on a separate thread because the
/// watcher needs the child handle to be able to kill it, and a handle cannot
/// be shared across a thread boundary without a platform-specific API.
const TIMEOUT_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Run a command and capture output, giving up after `timeout`.
///
/// A command that hangs is a worse failure than a command that fails: the
/// caller waits forever and learns nothing. Here the child is killed when
/// the deadline passes, and the error names the program and the deadline.
///
/// # Errors
///
/// Returns an error when the program cannot be started, when the wait
/// fails, or when the deadline passes before the program finishes.
pub fn run_command_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<CommandResult> {
    let mut command = Command::new(program);
    command.args(args);
    run_spawned_with_timeout(command, program, args, timeout)
}

/// Run a command in a specific directory, giving up after `timeout`.
///
/// See [`run_command_with_timeout`].
///
/// # Errors
///
/// Returns an error when the program cannot be started, when the wait
/// fails, or when the deadline passes before the program finishes.
pub fn run_command_in_dir_with_timeout(
    program: &str,
    args: &[&str],
    dir: &Path,
    timeout: Duration,
) -> Result<CommandResult> {
    let mut command = Command::new(program);
    command.args(args).current_dir(dir);
    run_spawned_with_timeout(command, program, args, timeout)
}

/// Spawn a prepared command and wait for it, up to a deadline.
///
/// # Errors
///
/// Returns an error when the program cannot be started, when the wait
/// fails, or when the deadline passes before the program finishes.
fn run_spawned_with_timeout(
    mut command: Command,
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<CommandResult> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::process(format!("Failed to execute {program}: {e}")))?;

    // The pipes are drained on their own threads. A child that writes more
    // than one pipe buffer holds would block on the write while we wait for
    // it to exit, and the two would wait for each other.
    let stdout = child.stdout.take().map(read_pipe);
    let stderr = child.stderr.take().map(read_pipe);

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::process(format!("Failed to wait for {program}: {e}")));
            }
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            // The readers are joined so the pipes are closed before the
            // error is returned; a detached reader would keep the child's
            // output alive for as long as the process lives.
            let _ = stdout.map(|handle| handle.join());
            let _ = stderr.map(|handle| handle.join());
            return Err(Error::new(
                ErrorCode::ProcessTimeout,
                format!("{program} did not finish within {}s", timeout.as_secs()),
            )
            .with_context(format!("The command was: {program} {}", args.join(" ")))
            .with_suggestion(
                "Raise the timeout for this check, or run the command by hand to see what it waits for.",
            ));
        }

        std::thread::sleep(TIMEOUT_POLL_INTERVAL);
    };

    Ok(CommandResult {
        success: status.success(),
        exit_code: status.code().unwrap_or(-1),
        stdout: stdout
            .map(|handle| handle.join().unwrap_or_default())
            .unwrap_or_default(),
        stderr: stderr
            .map(|handle| handle.join().unwrap_or_default())
            .unwrap_or_default(),
    })
}

/// Read a pipe to the end on its own thread.
fn read_pipe(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buffer = String::new();
        let _ = pipe.read_to_string(&mut buffer);
        buffer
    })
}

/// Run a command with environment variables
pub fn run_command_with_env(
    program: &str,
    args: &[&str],
    env: &[(&str, &str)],
) -> Result<CommandResult> {
    let mut cmd = Command::new(program);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    for (key, value) in env {
        cmd.env(key, value);
    }

    let output = cmd
        .output()
        .map_err(|e| Error::process(format!("Failed to execute {program}: {e}")))?;

    Ok(CommandResult::from_output(output))
}

/// Characters that could enable shell injection attacks.
const SHELL_METACHARACTERS: &[char] = &[
    ';', '&', '|', '$', '`', '(', ')', '{', '}', '[', ']', '<', '>', '\n', '\r', '\'', '"', '\\',
    ' ', '\t',
];

/// Check if a command exists in PATH.
///
/// Returns `false` for:
/// - Empty program names
/// - Names containing shell metacharacters
/// - Programs not found in PATH
///
/// # Example
/// ```
/// use foodshare_core::process::command_exists;
/// assert!(command_exists("ls"));
/// assert!(!command_exists("nonexistent_cmd_xyz"));
/// assert!(!command_exists("ls; rm -rf /")); // Rejected: shell metacharacters
/// ```
#[must_use]
pub fn command_exists(program: &str) -> bool {
    is_safe_program_name(program) && which_binary(program).is_ok()
}

/// Get the absolute path to a command.
///
/// Returns `None` for invalid program names or if not found in PATH.
#[must_use]
pub fn which_command(program: &str) -> Option<PathBuf> {
    is_safe_program_name(program)
        .then(|| which_binary(program).ok())
        .flatten()
}

/// Validate program name is safe (non-empty, no shell metacharacters).
#[inline]
fn is_safe_program_name(program: &str) -> bool {
    !program.is_empty() && !program.contains(SHELL_METACHARACTERS)
}

/// Run a command and stream output to stdout/stderr (for interactive use)
pub fn run_command_streaming(program: &str, args: &[&str]) -> Result<i32> {
    let status = Command::new(program)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| Error::process(format!("Failed to execute {program}: {e}")))?;

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
        .map_err(|e| Error::process(format!("Failed to execute {program}: {e}")))?;

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
    fn test_command_exists_common_tools() {
        // Test common Unix tools that should exist
        assert!(command_exists("ls"));
        assert!(command_exists("cat"));
    }

    #[test]
    fn test_command_exists_prevents_injection() {
        // These should return false, not execute malicious commands
        assert!(!command_exists("test; echo pwned"));
        assert!(!command_exists("test && echo pwned"));
        assert!(!command_exists("test || echo pwned"));
        assert!(!command_exists("$(echo pwned)"));
        assert!(!command_exists("`echo pwned`"));
    }

    #[test]
    fn test_which_command_returns_path() {
        let path = which_command("echo");
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("echo"));
    }

    #[test]
    fn test_which_command_nonexistent() {
        assert!(which_command("nonexistent_command_12345").is_none());
    }

    #[test]
    fn test_which_command_prevents_injection() {
        // These should return None, not execute malicious commands
        assert!(which_command("test; echo pwned").is_none());
        assert!(which_command("test && echo pwned").is_none());
        assert!(which_command("test || echo pwned").is_none());
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

    #[test]
    fn test_command_result_combined_output_empty_stderr() {
        let result = CommandResult {
            success: true,
            exit_code: 0,
            stdout: "only stdout".to_string(),
            stderr: String::new(),
        };
        assert_eq!(result.combined_output(), "only stdout");
    }

    #[test]
    fn test_command_result_combined_output_empty_stdout() {
        let result = CommandResult {
            success: true,
            exit_code: 0,
            stdout: String::new(),
            stderr: "only stderr".to_string(),
        };
        assert_eq!(result.combined_output(), "only stderr");
    }

    #[test]
    fn test_run_command_with_timeout_returns_output() {
        let result = run_command_with_timeout("echo", &["hello"], Duration::from_secs(10))
            .expect("the command should have finished");
        assert!(result.success);
        assert!(result.stdout.contains("hello"));
    }

    #[test]
    fn test_run_command_with_timeout_gives_up() {
        let started = Instant::now();
        let error = run_command_with_timeout("sleep", &["30"], Duration::from_millis(200))
            .expect_err("the command should have been given up on");
        assert_eq!(error.code, ErrorCode::ProcessTimeout);
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the wait should end at the deadline, not at the command's own"
        );
    }

    #[test]
    fn test_run_command_in_dir_with_timeout_returns_output() {
        let result = run_command_in_dir_with_timeout(
            "echo",
            &["in-dir"],
            Path::new("."),
            Duration::from_secs(10),
        )
        .expect("the command should have finished");
        assert!(result.success);
        assert!(result.stdout.contains("in-dir"));
    }

    #[test]
    fn test_run_command_with_timeout_reports_a_failing_command() {
        // A command that fails is a result, not an error: the caller wants
        // to know the exit code, not to have it swallowed.
        let result = run_command_with_timeout("false", &[], Duration::from_secs(10))
            .expect("the command should have run");
        assert!(!result.success);
        assert_ne!(result.exit_code, 0);
    }
}
