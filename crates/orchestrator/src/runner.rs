//! Running a step and streaming its output.
//!
//! A step is one command in one directory. The runner spawns it, streams
//! stdout and stderr line by line to a channel, and reports how it ended.

use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use foodshare_core::error::{Error, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;

use crate::domain::Step;

pub use crate::state::{LogLine, Stream};

/// How a step ended.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// How long a step may take before it is stopped.
///
/// A build can legitimately take a long time, so the default is
/// generous. It is there so a hung process fails the step instead of
/// hanging the orchestrator, and with it the daemon.
pub const DEFAULT_STEP_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// The environment variable that overrides the step timeout.
pub const STEP_TIMEOUT_ENV: &str = "FOODSHARE_STEP_TIMEOUT_SECS";

/// How long a step may take.
#[must_use]
pub fn step_timeout() -> Duration {
    parse_step_timeout(std::env::var(STEP_TIMEOUT_ENV).ok().as_deref())
}

/// The step timeout, from a setting.
///
/// A value of zero means "no limit", which is not what an operator who
/// sets the variable means, so it falls back to the default.
#[must_use]
pub fn parse_step_timeout(value: Option<&str>) -> Duration {
    match value.and_then(|v| v.trim().parse::<u64>().ok()) {
        Some(0) | None => DEFAULT_STEP_TIMEOUT,
        Some(seconds) => Duration::from_secs(seconds),
    }
}

/// How a command ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    /// The exit code, or `-1` when the process was killed.
    pub exit_code: i32,
    /// Whether the command succeeded.
    pub success: bool,
    /// How long it took.
    pub duration_ms: u64,
}

/// Run a command, streaming its output to `tx`.
///
/// This is the one place a command is run: the plan's steps and the
/// verification's stages both come through here, so both get the same
/// timeout, the same cancellation and the same errors.
pub async fn run_command(
    label: &str,
    program: &str,
    args: &[String],
    cwd: &std::path::Path,
    env: &[(String, String)],
    tx: &UnboundedSender<LogLine>,
    cancelled: &AtomicBool,
    timeout: Duration,
) -> Result<CommandOutcome> {
    let started = Instant::now();

    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // The child leads its own process group, so stopping it stops
    // everything it started. `cargo test` leaves the test binaries
    // running otherwise, and they keep the build directory locked.
    #[cfg(unix)]
    command.process_group(0);
    for (key, value) in env {
        command.env(key, value);
    }

    let mut child = command.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::command_not_found(program)
                .with_context(format!("running `{label}` in {}", cwd.display()))
        } else {
            Error::process(format!("Failed to spawn `{program}`: {e}"))
                .with_context(format!("running `{label}` in {}", cwd.display()))
        }
    })?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_task = stdout.map(|out| pump(out, tx.clone(), Stream::Stdout));
    let stderr_task = stderr.map(|err| pump(err, tx.clone(), Stream::Stderr));

    let status = match tokio::time::timeout(timeout, wait_for(label, &mut child, cancelled)).await {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            // The command is still running, so it is stopped rather than
            // left behind.
            stop_process(&mut child).await;
            return Err(Error::process(format!(
                "`{label}` did not finish within {}s and was stopped",
                timeout.as_secs()
            )));
        }
    };

    if let Some(task) = stdout_task {
        let _ = task.await;
    }
    if let Some(task) = stderr_task {
        let _ = task.await;
    }

    Ok(CommandOutcome {
        exit_code: status.code().unwrap_or(-1),
        success: status.success(),
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Run a step, streaming its output to `tx`.
///
/// The step is not retried here; retrying belongs to the caller, which
/// also owns the disk healing that goes with it. A step that is
/// cancelled is stopped, along with everything it started.
pub async fn run_step(
    step: &Step,
    tx: &UnboundedSender<LogLine>,
    cancelled: &AtomicBool,
) -> Result<StepOutcome> {
    run_step_with_timeout(step, tx, step_timeout(), cancelled).await
}

/// Run a step, with an explicit timeout.
pub async fn run_step_with_timeout(
    step: &Step,
    tx: &UnboundedSender<LogLine>,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<StepOutcome> {
    let env: Vec<(String, String)> = step
        .env
        .iter()
        .filter_map(|entry| {
            entry
                .split_once('=')
                .map(|(key, value)| (key.to_string(), value.to_string()))
        })
        .collect();

    let outcome = run_command(
        &step.label,
        &step.program,
        &step.args,
        std::path::Path::new(&step.cwd),
        &env,
        tx,
        cancelled,
        timeout,
    )
    .await?;

    Ok(StepOutcome {
        domain: step.domain,
        label: step.label.clone(),
        exit_code: outcome.exit_code,
        success: outcome.success,
        duration_ms: outcome.duration_ms,
    })
}

/// Copy a process's output to a channel, line by line.
async fn pump<R>(stream: R, tx: UnboundedSender<LogLine>, kind: Stream)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    use tokio::io::{AsyncBufReadExt, BufReader};

    let mut lines = BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if tx
            .send(LogLine {
                stream: kind,
                text: line,
            })
            .is_err()
        {
            // The receiver is gone; nobody is listening any more.
            break;
        }
    }
}

/// Wait for a process, watching for a cancellation.
async fn wait_for(
    label: &str,
    child: &mut tokio::process::Child,
    cancelled: &AtomicBool,
) -> Result<std::process::ExitStatus> {
    loop {
        if cancelled.load(Ordering::SeqCst) {
            stop_process(child).await;
            return Err(Error::process(format!("`{label}` was cancelled")));
        }
        match tokio::time::timeout(CANCEL_POLL, child.wait()).await {
            Ok(Ok(status)) => return Ok(status),
            Ok(Err(e)) => {
                return Err(Error::process(format!("Failed to wait for `{label}`: {e}")));
            }
            // Still running: look again.
            Err(_) => continue,
        }
    }
}

/// Stop a process and everything it started.
///
/// The child leads its own process group, so the whole tree goes with it.
/// `cargo test` leaves the test binaries running otherwise, and they
/// would keep the build directory locked.
///
/// The group is signalled with `kill(2)` rather than with the `kill`
/// utility. The utility takes a process group as a *negative* argument
/// and has to tell that apart from a signal name, and on Linux it does
/// not always: `kill -KILL -<pgid>` reported success, yet only the group
/// leader was signalled, and everything the leader had started kept
/// running in the same group. The system call has no such ambiguity.
async fn stop_process(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        // SAFETY: `kill` takes a process id and a signal, reads no memory
        // and touches no Rust state. A negative id names a process group.
        // The id comes from a child this process started, so the group is
        // its own and nothing else can be in it.
        let sent = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        if sent != 0 {
            tracing::debug!(
                "the process group {pid} could not be signalled: {}",
                std::io::Error::last_os_error()
            );
        }
    }
    let _ = child.start_kill();
    let _ = child.wait().await;
}

/// How often a running step is checked for cancellation.
const CANCEL_POLL: Duration = Duration::from_millis(200);

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

    /// A flag that is never set.
    fn not_cancelled() -> AtomicBool {
        AtomicBool::new(false)
    }

    #[tokio::test]
    async fn test_successful_step_reports_success() {
        let (tx, mut rx) = unbounded_channel();
        let outcome = run_step(&step("echo", &["hello"]), &tx, &not_cancelled())
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
        let outcome = run_step(&step("false", &[]), &tx, &not_cancelled())
            .await
            .expect("step runs");
        assert!(!outcome.success);
        assert_eq!(outcome.exit_code, 1);
    }

    #[tokio::test]
    async fn test_stderr_is_streamed_separately() {
        let (tx, mut rx) = unbounded_channel();
        let outcome = run_step(
            &step("sh", &["-c", "echo out; echo err 1>&2"]),
            &tx,
            &not_cancelled(),
        )
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
        let error = run_step(
            &step("definitely-not-a-real-program-xyz", &[]),
            &tx,
            &not_cancelled(),
        )
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
        run_step(&step, &tx, &not_cancelled())
            .await
            .expect("step runs");
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

    #[test]
    fn test_the_step_timeout_falls_back_to_the_default() {
        assert_eq!(parse_step_timeout(None), DEFAULT_STEP_TIMEOUT);
        assert_eq!(parse_step_timeout(Some("")), DEFAULT_STEP_TIMEOUT);
        assert_eq!(
            parse_step_timeout(Some("not a number")),
            DEFAULT_STEP_TIMEOUT
        );
        // Zero would mean "no limit", which is not what was asked for.
        assert_eq!(parse_step_timeout(Some("0")), DEFAULT_STEP_TIMEOUT);
        assert_eq!(parse_step_timeout(Some(" 90 ")), Duration::from_secs(90));
    }

    #[tokio::test]
    async fn test_a_step_that_overruns_its_timeout_is_stopped() {
        let (tx, _rx) = unbounded_channel();
        let step = Step::new(DomainKind::Rust, "sleeper", "sh", &["-c", "sleep 30"], ".");
        let error = run_step_with_timeout(&step, &tx, Duration::from_millis(200), &not_cancelled())
            .await
            .expect_err("the step times out");
        assert!(
            error.to_string().contains("did not finish within"),
            "got: {error}"
        );
    }

    #[tokio::test]
    async fn test_a_cancelled_step_is_stopped() {
        let (tx, _rx) = unbounded_channel();
        let cancelled = std::sync::Arc::new(AtomicBool::new(false));
        let step = Step::new(DomainKind::Rust, "sleeper", "sh", &["-c", "sleep 30"], ".");

        // Set the flag once the step is under way.
        let flag = cancelled.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            flag.store(true, Ordering::SeqCst);
        });

        let started = Instant::now();
        let error = run_step(&step, &tx, &cancelled)
            .await
            .expect_err("the step is cancelled");
        assert!(error.to_string().contains("cancelled"), "got: {error}");
        // It did not wait for the sleep to finish.
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    /// The processes that are still running a command line holding `marker`.
    ///
    /// `ps` is asked rather than `pgrep`, and it is asked directly rather
    /// than through a shell: on Linux `pgrep -f` matches the command line
    /// of the process that asks it, so `pgrep -f 'sleep 37'` counts the
    /// command that asked, and the count is never zero.
    ///
    /// A killed process that has not been reaped yet is a zombie, and a
    /// zombie is gone for good — it cannot run anything, and it does not
    /// count.
    #[cfg(unix)]
    fn still_running(marker: &str) -> Vec<String> {
        let listing = std::process::Command::new("ps")
            .arg("-A")
            .arg("-o")
            .arg("pid=,stat=,args=")
            .output();
        let Ok(listing) = listing else {
            return Vec::new();
        };
        String::from_utf8_lossy(&listing.stdout)
            .lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                let _pid = fields.next()?;
                let state = fields.next()?;
                let args = fields.collect::<Vec<_>>().join(" ");
                if args.contains(marker) && !state.starts_with('Z') {
                    Some(args)
                } else {
                    None
                }
            })
            .collect()
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_a_cancelled_step_stops_what_it_started() {
        let (tx, _rx) = unbounded_channel();
        let cancelled = std::sync::Arc::new(AtomicBool::new(false));
        // The step starts a child of its own, the way `cargo test` does.
        // The marker is its own, so this test does not count the
        // processes of the test above it: they run in parallel.
        let step = Step::new(
            DomainKind::Rust,
            "parent",
            "sh",
            &["-c", "sleep 37 & sleep 37"],
            ".",
        );

        let flag = cancelled.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            flag.store(true, Ordering::SeqCst);
        });

        let _ = run_step(&step, &tx, &cancelled).await;
        // Give the group a moment to be signalled.
        tokio::time::sleep(Duration::from_millis(800)).await;

        // Nothing of the step is left running.
        let leftover = still_running("sleep 37");
        assert!(
            leftover.is_empty(),
            "the step left processes behind: {leftover:?}"
        );
    }
}
