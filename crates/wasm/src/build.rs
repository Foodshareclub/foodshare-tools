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
    let mut tasks = Vec::with_capacity(crates.len());
    for krate in crates {
        let krate = *krate;
        let layout = layout.clone();
        tasks.push(tokio::spawn(
            async move { build_one(&layout, &krate).await },
        ));
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
pub async fn build_one(layout: &Layout, krate: &Crate) -> BuildOutcome {
    let started = Instant::now();
    let out_dir = layout.out_dir(krate);
    let crate_dir = layout.tools_dir.join(krate.path);

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

    let output = Command::new("wasm-pack")
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
        .output()
        .await;

    let output = match output {
        Ok(output) => output,
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

    if !output.status.success() {
        let mut error = describe_exit(output.status);
        // The build output is the useful part of the failure.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: Vec<&str> = stderr.lines().rev().take(6).collect();
        if !tail.is_empty() {
            error.push_str(": ");
            error.push_str(&tail.into_iter().rev().collect::<Vec<_>>().join(" "));
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
        let outcome = build_one(&layout(), &loader::CRATES[0]).await;
        if !outcome.success {
            assert!(outcome.error.is_some());
        }
    }
}
