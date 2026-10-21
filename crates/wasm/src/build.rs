//! The WebAssembly build engine.
//!
//! Compiles the published crates to WebAssembly with `wasm-pack`, then
//! patches the generated loaders so Next.js can find the `.wasm` files.
//!
//! The `nodejs` target is deliberate: the bridges in the web app are
//! server-only, so they load the module synchronously. Switching to
//! `bundler` or `web` would mean making those bridges async first.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;

use serde::Serialize;
use tokio::process::Command;

use crate::loader::{self, Crate};

/// The target `wasm-pack` builds for.
pub const TARGET: &str = "nodejs";

/// The `wasm-pack` version the generated modules are built with.
///
/// The generated modules are committed to the web repository, so the tool
/// that shapes them has to be pinned: a different `wasm-pack` emits a
/// different loader, and the difference looks like work in a diff rather
/// than like a version change. The version is checked before a build, and
/// the version that built a module is recorded inside it.
pub const WASM_PACK_VERSION: &str = "0.13.1";

/// The version of `wasm-pack` that is installed.
///
/// `wasm-pack --version` answers `wasm-pack 0.13.1`.
pub fn wasm_pack_version() -> Option<String> {
    let output = std::process::Command::new("wasm-pack")
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_version(&String::from_utf8_lossy(&output.stdout))
}

/// Read the version out of what `wasm-pack --version` answered.
fn parse_version(text: &str) -> Option<String> {
    let version = text.trim().strip_prefix("wasm-pack ")?;
    Some(version.trim().to_string())
}

/// Check that the installed `wasm-pack` is the one the modules are built with.
///
/// A build with a different `wasm-pack` produces different modules, and
/// those modules are committed to another repository. That is not a
/// difference to be discovered later, in a diff, by whoever commits it.
pub fn check_wasm_pack() -> Result<(), String> {
    let Some(installed) = wasm_pack_version() else {
        return Err(
            "wasm-pack is not installed, or it did not report a version; \
             the generated modules need it"
                .to_string(),
        );
    };
    check_version(&installed)
}

/// Compare an installed `wasm-pack` with the one the modules are built with.
fn check_version(installed: &str) -> Result<(), String> {
    if installed == WASM_PACK_VERSION {
        return Ok(());
    }
    Err(format!(
        "wasm-pack {installed} is installed, but the modules are built with {WASM_PACK_VERSION}; \
         the two produce different modules, and the modules are committed to the web repository"
    ))
}

/// How one crate's build ended.
#[derive(Debug, Clone, Serialize)]
pub struct BuildOutcome {
    /// The crate that was built.
    pub crate_name: String,
    /// Whether the build succeeded.
    pub success: bool,
    /// How long it took.
    pub duration_ms: u64,
    /// The size of the generated `.wasm` file, in kilobytes.
    pub size_kb: Option<f64>,
    /// Where the artefacts were written.
    pub out_dir: PathBuf,
    /// Why the build failed.
    pub error: Option<String>,
}

/// Where the build writes its artefacts.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The Rust workspace, which holds the crates.
    pub tools_dir: PathBuf,
    /// The web repository, which holds the generated modules.
    pub web_dir: PathBuf,
}

impl Layout {
    /// The directory the generated modules go into.
    #[must_use]
    pub fn out_root(&self) -> PathBuf {
        self.web_dir.join("src").join("wasm")
    }

    /// The output directory for one crate.
    #[must_use]
    pub fn out_dir(&self, krate: &Crate) -> PathBuf {
        self.out_root().join(krate.name)
    }
}

/// Build every crate.
///
/// The builds run at the same time: they share the Cargo target
/// directory, and running them one after another would serialise the
/// whole dependency compile.
pub async fn build_all(layout: &Layout, crates: &[Crate]) -> Vec<BuildOutcome> {
    build_all_with(layout, crates, false).await
}

/// Build every crate, sending the build output to standard error.
///
/// A caller that prints the results as JSON needs standard output to
/// hold nothing but the JSON.
pub async fn build_all_quiet(layout: &Layout, crates: &[Crate]) -> Vec<BuildOutcome> {
    build_all_with(layout, crates, true).await
}

async fn build_all_with(layout: &Layout, crates: &[Crate], quiet: bool) -> Vec<BuildOutcome> {
    let mut tasks = Vec::with_capacity(crates.len());
    for krate in crates {
        let krate = *krate;
        let layout = layout.clone();
        tasks.push(tokio::spawn(async move {
            build_one(&layout, &krate, quiet).await
        }));
    }

    let mut outcomes = Vec::with_capacity(crates.len());
    for task in tasks {
        match task.await {
            Ok(outcome) => outcomes.push(outcome),
            Err(e) => outcomes.push(BuildOutcome {
                crate_name: "unknown".to_string(),
                success: false,
                duration_ms: 0,
                size_kb: None,
                out_dir: PathBuf::new(),
                error: Some(format!("the build task failed: {e}")),
            }),
        }
    }
    outcomes
}

/// Build one crate and patch its loader.
pub async fn build_one(layout: &Layout, krate: &Crate, quiet: bool) -> BuildOutcome {
    let started = Instant::now();
    let out_dir = layout.out_dir(krate);
    let crate_dir = layout.tools_dir.join(krate.path);

    // The tool is checked before anything is written. A build with the
    // wrong `wasm-pack` would overwrite the modules the web repository
    // tracks with different ones, and the difference would only be
    // noticed when somebody committed it.
    if let Err(e) = check_wasm_pack() {
        return BuildOutcome {
            crate_name: krate.name.to_string(),
            success: false,
            duration_ms: started.elapsed().as_millis() as u64,
            size_kb: None,
            out_dir,
            error: Some(e),
        };
    }

    if let Err(e) = tokio::fs::create_dir_all(&out_dir).await {
        let error = format!("could not create {}: {e}", out_dir.display());
        return BuildOutcome {
            crate_name: krate.name.to_string(),
            success: false,
            duration_ms: 0,
            size_kb: None,
            out_dir,
            error: Some(error),
        };
    }

    let mut command = Command::new("wasm-pack");
    command
        .arg("build")
        .arg(&crate_dir)
        .arg("--target")
        .arg(TARGET)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--release")
        .arg("--")
        .arg("--locked")
        .arg("--features")
        .arg("wasm")
        .current_dir(&layout.tools_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            return BuildOutcome {
                crate_name: krate.name.to_string(),
                success: false,
                duration_ms: started.elapsed().as_millis() as u64,
                size_kb: None,
                out_dir,
                error: Some(format!("could not run wasm-pack: {e}")),
            };
        }
    };

    // The build output goes to the terminal as it happens, and the last
    // few lines are kept for the failure message.
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_task = stdout.map(|stream| tokio::spawn(pump(stream, Stream::Stdout, quiet)));
    let err_task = stderr.map(|stream| tokio::spawn(pump(stream, Stream::Stderr, quiet)));

    let status = child.wait().await;
    let out_tail = match out_task {
        Some(task) => task.await.unwrap_or_default(),
        None => String::new(),
    };
    let err_tail = match err_task {
        Some(task) => task.await.unwrap_or_default(),
        None => String::new(),
    };

    let status = match status {
        Ok(status) => status,
        Err(e) => {
            return BuildOutcome {
                crate_name: krate.name.to_string(),
                success: false,
                duration_ms: started.elapsed().as_millis() as u64,
                size_kb: None,
                out_dir,
                error: Some(format!("wasm-pack failed: {e}")),
            };
        }
    };

    if !status.success() {
        let mut error = describe_exit(status);
        // The build output is the useful part of the failure.
        let tail = if err_tail.is_empty() {
            out_tail
        } else {
            err_tail
        };
        if !tail.is_empty() {
            error.push_str(": ");
            error.push_str(&tail);
        }
        return BuildOutcome {
            crate_name: krate.name.to_string(),
            success: false,
            duration_ms: started.elapsed().as_millis() as u64,
            size_kb: None,
            out_dir,
            error: Some(error),
        };
    }

    // The loader is patched after the build, because `wasm-pack` rewrites
    // the file on every build.
    patch_loaded_module(&out_dir, krate).await;
    restore_vendored_gitignore(&out_dir);

    let wasm_file = out_dir.join(krate.wasm_name());
    let size_kb = std::fs::metadata(&wasm_file)
        .map(|m| (m.len() as f64 / 1024.0 * 10.0).round() / 10.0)
        .ok();

    BuildOutcome {
        crate_name: krate.name.to_string(),
        success: true,
        duration_ms: started.elapsed().as_millis() as u64,
        size_kb,
        out_dir,
        error: None,
    }
}

/// How many lines of output are kept for the failure message.
const TAIL_LINES: usize = 6;

/// Which of a process's streams is being read.
#[derive(Debug, Clone, Copy)]
enum Stream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// Copy a process's output to the terminal, keeping the last few lines.
///
/// The build takes a while, and its progress is worth seeing while it
/// happens. The tail is what the failure message quotes.
async fn pump<R>(stream: R, kind: Stream, quiet: bool) -> String
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    use std::collections::VecDeque;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let mut lines = BufReader::new(stream).lines();
    let mut tail: VecDeque<String> = VecDeque::with_capacity(TAIL_LINES);
    // A quiet build sends its output to standard error, so standard
    // output holds nothing but the results.
    let mut terminal: Box<dyn tokio::io::AsyncWrite + Send + Unpin> = match (kind, quiet) {
        (Stream::Stdout, true) => Box::new(tokio::io::stderr()),
        (Stream::Stdout, false) => Box::new(tokio::io::stdout()),
        (Stream::Stderr, _) => Box::new(tokio::io::stderr()),
    };

    while let Ok(Some(line)) = lines.next_line().await {
        if tail.len() == TAIL_LINES {
            tail.pop_front();
        }
        tail.push_back(line.clone());
        let _ = terminal.write_all(format!("{line}\n").as_bytes()).await;
    }
    let _ = terminal.flush().await;

    tail.into_iter().collect::<Vec<_>>().join(" ")
}

/// Restore the `.gitignore` the vendored module directory needs.
///
/// `wasm-pack` writes a plain `*`, which ignores the type definitions
/// and the package manifest that the web repository tracks on purpose.
fn restore_vendored_gitignore(out_dir: &Path) {
    let path = out_dir.join(".gitignore");
    let wanted = crate::loader::vendored_gitignore();
    if std::fs::read_to_string(&path).ok().as_deref() == Some(wanted.as_str()) {
        return;
    }
    if let Err(e) = std::fs::write(&path, &wanted) {
        tracing::warn!(path = %path.display(), error = %e, "could not restore the .gitignore");
    }
}

/// Describe how a process ended.
///
/// A process killed by a signal has no exit code, and saying so is more
/// useful than "-1".
fn describe_exit(status: std::process::ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        match (status.code(), status.signal()) {
            (Some(code), _) => format!("wasm-pack exited with {code}"),
            (None, Some(signal)) => format!("wasm-pack was killed by signal {signal}"),
            (None, None) => "wasm-pack ended without an exit code".to_string(),
        }
    }
    #[cfg(not(unix))]
    {
        match status.code() {
            Some(code) => format!("wasm-pack exited with {code}"),
            None => "wasm-pack ended without an exit code".to_string(),
        }
    }
}

/// Patch the generated loader in place.
async fn patch_loaded_module(out_dir: &Path, krate: &Crate) {
    let js_path = out_dir.join(loader::js_file_name(krate));
    let Ok(source) = tokio::fs::read_to_string(&js_path).await else {
        tracing::warn!(path = %js_path.display(), "no generated loader to patch");
        return;
    };
    let Some(patched) = loader::patch_loader(&source, krate) else {
        tracing::debug!(path = %js_path.display(), "the loader needs no patching");
        return;
    };
    if let Err(e) = tokio::fs::write(&js_path, patched).await {
        tracing::warn!(path = %js_path.display(), error = %e, "could not patch the loader");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout {
            tools_dir: PathBuf::from("/tmp/ws/foodshare-tools"),
            web_dir: PathBuf::from("/tmp/ws/foodshare-web"),
        }
    }

    #[test]
    fn test_out_dir_is_under_the_web_source() {
        let layout = layout();
        let out = layout.out_dir(&loader::CRATES[0]);
        assert!(out.ends_with("foodshare-web/src/wasm/foodshare-geo"));
    }

    #[tokio::test]
    async fn test_build_without_wasm_pack_fails_cleanly() {
        // `wasm-pack` is not installed everywhere; the outcome records the
        // failure rather than panicking.
        let outcome = build_one(&layout(), &loader::CRATES[0], false).await;
        if !outcome.success {
            assert!(outcome.error.is_some());
        }
    }

    #[test]
    fn test_the_version_is_read_out_of_the_answer() {
        assert_eq!(
            parse_version("wasm-pack 0.13.1"),
            Some("0.13.1".to_string())
        );
        assert_eq!(
            parse_version("wasm-pack 0.13.1\n"),
            Some("0.13.1".to_string())
        );
        // Anything else is not an answer this understands.
        assert_eq!(parse_version("0.13.1"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn test_the_installed_version_has_to_match() {
        assert!(check_version(WASM_PACK_VERSION).is_ok());

        let error = check_version("0.15.0").expect_err("a different version");
        assert!(
            error.contains("0.15.0") && error.contains(WASM_PACK_VERSION),
            "got: {error}"
        );
        // The message has to say why it matters, or it reads like a
        // preference rather than a rule.
        assert!(error.contains("web repository"), "got: {error}");
    }
}
