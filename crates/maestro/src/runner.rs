//! Running Maestro: syntax checks, single-device suites, and the
//! multi-device matrix.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

use crate::device::TargetDevice;

/// The result of a syntax check on one flow.
#[derive(Debug, Clone, Serialize)]
pub struct SyntaxResult {
    /// The flow that was checked.
    pub path: PathBuf,
    /// Whether the flow parsed.
    pub passed: bool,
}

/// The result of running a suite on one device.
#[derive(Debug, Clone, Serialize)]
pub struct TestOutcome {
    /// The device the suite ran on.
    pub device: TargetDevice,
    /// Whether the suite passed.
    pub passed: bool,
    /// How long it took.
    pub duration_ms: u64,
    /// The log file, when one was written.
    pub log_file: Option<PathBuf>,
    /// Why it failed.
    pub error: Option<String>,
}

/// Dump the UI hierarchy of a device.
///
/// The hierarchy is written by Maestro itself, so this is the one place
/// that knows how to ask for it: the arguments, the device flag and the
/// exit code all live here rather than in a caller.
pub async fn hierarchy(device_id: Option<&str>) -> std::io::Result<i32> {
    let mut command = Command::new("maestro");
    command.arg("hierarchy").stdin(Stdio::null());
    if let Some(id) = device_id {
        command.arg("--device").arg(id);
    }

    let status = command.status().await?;
    Ok(status.code().unwrap_or(1))
}

/// Check the syntax of every flow.
///
/// The flows are checked one at a time: `maestro check-syntax` is a
/// short-lived process, and a suite with many flows would otherwise fork
/// hundreds of them at once.
pub async fn check_syntax(flows: &[PathBuf]) -> Vec<SyntaxResult> {
    let mut results = Vec::with_capacity(flows.len());
    for flow in flows {
        let status = Command::new("maestro")
            .arg("check-syntax")
            .arg(flow)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
        let passed = status.map(|s| s.success()).unwrap_or(false);
        results.push(SyntaxResult {
            path: flow.clone(),
            passed,
        });
    }
    results
}

/// Run a Maestro suite on one device.
///
/// The output is written to `log_file` when one is given, and the process
/// exits with the suite's own code.
pub async fn run_test(
    device: &TargetDevice,
    extra_args: &[String],
    target: &Path,
    log_file: Option<&Path>,
) -> TestOutcome {
    let started = Instant::now();

    let mut command = Command::new("maestro");
    command
        .arg("--device")
        .arg(&device.id)
        .arg("test")
        .args(extra_args)
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            return TestOutcome {
                device: device.clone(),
                passed: false,
                duration_ms: 0,
                log_file: log_file.map(Path::to_path_buf),
                error: Some(format!("could not run maestro: {e}")),
            };
        }
    };

    // Both streams go to the same log, in the order they arrive.
    let log = match log_file {
        Some(path) => match tokio::fs::File::create(path).await {
            Ok(file) => Some(Arc::new(tokio::sync::Mutex::new(file))),
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "could not open the log file");
                None
            }
        },
        None => None,
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (out_task, err_task) = match (stdout, stderr, log) {
        (Some(out), Some(err), Some(file)) => {
            let out_file = Arc::clone(&file);
            let out_task = tokio::spawn(async move {
                copy_to_log(out, out_file).await;
            });
            let err_task = tokio::spawn(async move {
                copy_to_log(err, file).await;
            });
            (Some(out_task), Some(err_task))
        }
        _ => (None, None),
    };

    let status = child.wait().await;
    if let Some(task) = out_task {
        let _ = task.await;
    }
    if let Some(task) = err_task {
        let _ = task.await;
    }

    let (passed, error) = match status {
        Ok(status) if status.success() => (true, None),
        Ok(status) => (
            false,
            Some(format!("exit code {}", status.code().unwrap_or(-1))),
        ),
        Err(e) => (false, Some(format!("maestro failed: {e}"))),
    };

    TestOutcome {
        device: device.clone(),
        passed,
        duration_ms: started.elapsed().as_millis() as u64,
        log_file: log_file.map(Path::to_path_buf),
        error,
    }
}

/// Copy a process stream into the log file.
async fn copy_to_log<R>(stream: R, file: Arc<tokio::sync::Mutex<tokio::fs::File>>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut reader = tokio::io::BufReader::new(stream);
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let mut file = file.lock().await;
                if file.write_all(&buffer[..n]).await.is_err() {
                    break;
                }
            }
        }
    }
}

/// Run a Maestro suite on every device at once.
///
/// Each device gets its own log file, so a failure on one device does not
/// hide what happened on another.
pub async fn run_matrix(
    devices: &[TargetDevice],
    extra_args: &[String],
    target: &Path,
    reports_dir: &Path,
) -> Vec<TestOutcome> {
    let _ = tokio::fs::create_dir_all(reports_dir).await;

    let mut tasks = Vec::with_capacity(devices.len());
    for device in devices {
        let log_file = reports_dir.join(format!(
            "matrix_{}_{}.log",
            device.platform.name(),
            device.safe_id()
        ));
        let extra_args = extra_args.to_vec();
        let target = target.to_path_buf();
        let device = device.clone();
        tasks.push(tokio::spawn(async move {
            run_test(&device, &extra_args, &target, Some(&log_file)).await
        }));
    }

    let mut outcomes = Vec::with_capacity(devices.len());
    for task in tasks {
        match task.await {
            Ok(outcome) => outcomes.push(outcome),
            Err(e) => outcomes.push(TestOutcome {
                device: TargetDevice {
                    platform: crate::device::Platform::Ios,
                    id: "unknown".to_string(),
                    name: "unknown".to_string(),
                },
                passed: false,
                duration_ms: 0,
                log_file: None,
                error: Some(format!("the suite task failed: {e}")),
            }),
        }
    }
    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::Platform;

    fn device() -> TargetDevice {
        TargetDevice {
            platform: Platform::Ios,
            id: "test-id".to_string(),
            name: "Test Device".to_string(),
        }
    }

    #[tokio::test]
    async fn test_check_syntax_of_no_flows_is_empty() {
        assert!(check_syntax(&[]).await.is_empty());
    }

    #[tokio::test]
    async fn test_run_test_reports_a_missing_maestro() {
        // `maestro` is not installed everywhere; the outcome records the
        // failure rather than panicking.
        let outcome = run_test(&device(), &[], Path::new("."), None).await;
        if !outcome.passed {
            assert!(outcome.error.is_some());
        }
    }

    #[tokio::test]
    async fn test_run_matrix_of_no_devices_is_empty() {
        let dir = tempfile::tempdir().expect("temp dir");
        let outcomes = run_matrix(&[], &[], Path::new("."), dir.path()).await;
        assert!(outcomes.is_empty());
    }
}
