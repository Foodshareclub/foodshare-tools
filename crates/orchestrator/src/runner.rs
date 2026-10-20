//! Running a step and streaming its output.
//!
//! A step is one command in one directory. The runner spawns it, streams
//! stdout and stderr line by line to a channel, and reports how it ended.

use std::process::Stdio;
use std::time::{Duration, Instant};

use foodshare_core::error::{Error, Result};
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;

use crate::domain::Step;

/// Which stream a line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// One line of output from a running step.
#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    /// The stream the line came from.
    pub stream: Stream,
    /// The line itself, without its newline.
    pub text: String,
}

/// How a step ended.
#[derive(Debug, Clone, Serialize)]
pub struct StepOutcome {
    /// The domain the step belonged to.
    pub domain: crate::domain::DomainKind,
    /// The step's label.
    pub label: String,
    /// The exit code, or `-1` when the process was killed.
    pub exit_code: i32,
    /// Whether the step succeeded.
    pub success: bool,
    /// How long it took.
    pub duration_ms: u64,
}

/// Run a step, streaming its output to `tx`.
///
/// The step is not retried here; retrying belongs to the caller, which
/// also owns the disk healing that goes with it.
pub async fn run_step(step: &Step, tx: &UnboundedSender<LogLine>) -> Result<StepOutcome> {
    let started = Instant::now();

    let mut command = Command::new(&step.program);
    command
        .args(&step.args)
        .current_dir(&step.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for entry in &step.env {
        if let Some((key, value)) = entry.split_once('=') {
            command.env(key, value);
        }
    }

    let mut child = command.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::command_not_found(&step.program)
                .with_context(format!("running `{}` in {}", step.label, step.cwd))
        } else {
            Error::process(format!("Failed to spawn `{}`: {e}", step.program))
                .with_context(format!("running `{}` in {}", step.label, step.cwd))
        }
    })?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_task = stdout.map(|out| pump(out, tx.clone(), Stream::Stdout));
    let stderr_task = stderr.map(|err| pump(err, tx.clone(), Stream::Stderr));

    let status = child
        .wait()
        .await
        .map_err(|e| Error::process(format!("Failed to wait for `{}`: {e}", step.program)))?;

    if let Some(task) = stdout_task {
        let _ = task.await;
    }
    if let Some(task) = stderr_task {
        let _ = task.await;
    }

    let exit_code = status.code().unwrap_or(-1);
    Ok(StepOutcome {
        domain: step.domain,
        label: step.label.clone(),
        exit_code,
        success: status.success(),
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Read a pipe line by line, forwarding each line to the channel.
async fn pump<R>(reader: R, tx: UnboundedSender<LogLine>, stream: Stream)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if tx.send(LogLine { stream, text: line }).is_err() {
            // The receiver is gone; nobody is listening any more.
            break;
        }
    }
}

/// How long to wait between retry attempts.
#[must_use]
pub fn backoff(attempt: u32) -> Duration {
    Duration::from_secs(u64::from(attempt.max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::DomainKind;
    use tokio::sync::mpsc::unbounded_channel;

    fn step(program: &str, args: &[&str]) -> Step {
        Step::new(DomainKind::Rust, "test", program, args, ".")
    }

    #[tokio::test]
    async fn test_successful_step_reports_success() {
        let (tx, mut rx) = unbounded_channel();
        let outcome = run_step(&step("echo", &["hello"]), &tx)
            .await
            .expect("step runs");
        assert!(outcome.success);
        assert_eq!(outcome.exit_code, 0);

        let mut lines = Vec::new();
        while let Ok(line) = rx.try_recv() {
            lines.push(line.text);
        }
        assert_eq!(lines, vec!["hello"]);
    }

    #[tokio::test]
    async fn test_failing_step_reports_failure() {
        let (tx, _rx) = unbounded_channel();
        let outcome = run_step(&step("false", &[]), &tx).await.expect("step runs");
        assert!(!outcome.success);
        assert_eq!(outcome.exit_code, 1);
    }

    #[tokio::test]
    async fn test_stderr_is_streamed_separately() {
        let (tx, mut rx) = unbounded_channel();
        let outcome = run_step(&step("sh", &["-c", "echo out; echo err 1>&2"]), &tx)
            .await
            .expect("step runs");
        assert!(outcome.success);

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        while let Ok(line) = rx.try_recv() {
            match line.stream {
                Stream::Stdout => stdout.push(line.text),
                Stream::Stderr => stderr.push(line.text),
            }
        }
        assert_eq!(stdout, vec!["out"]);
        assert_eq!(stderr, vec!["err"]);
    }

    #[tokio::test]
    async fn test_missing_program_is_a_command_not_found_error() {
        let (tx, _rx) = unbounded_channel();
        let error = run_step(&step("definitely-not-a-real-program-xyz", &[]), &tx)
            .await
            .expect_err("spawn fails");
        assert_eq!(
            error.code,
            foodshare_core::error::ErrorCode::CommandNotFound
        );
    }

    #[tokio::test]
    async fn test_env_is_applied() {
        let (tx, mut rx) = unbounded_channel();
        let step =
            step("sh", &["-c", "echo $FOODSHARE_TEST_VAR"]).with_env("FOODSHARE_TEST_VAR", "42");
        run_step(&step, &tx).await.expect("step runs");
        let mut lines = Vec::new();
        while let Ok(line) = rx.try_recv() {
            lines.push(line.text);
        }
        assert_eq!(lines, vec!["42"]);
    }

    #[test]
    fn test_backoff_grows() {
        assert_eq!(backoff(1), Duration::from_secs(1));
        assert_eq!(backoff(3), Duration::from_secs(3));
        assert_eq!(backoff(0), Duration::from_secs(1));
    }
}
