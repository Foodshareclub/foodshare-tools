//! Finding the workspace's own binaries.
//!
//! The plan runs steps like the WASM build and the Maestro syntax check.
//! Those are Rust binaries in this workspace, so the plan has to say
//! where they are. A built binary is used when there is one; otherwise
//! `cargo run` builds and runs it, which always works but is slower.

use std::path::{Path, PathBuf};

/// A program to run, and the arguments to run it with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// The program to run.
    pub program: String,
    /// The arguments to pass.
    pub args: Vec<String>,
}

impl Invocation {
    /// The program, with its arguments.
    #[must_use]
    pub fn into_parts(self) -> (String, Vec<String>) {
        (self.program, self.args)
    }
}

/// Find a workspace binary.
///
/// The release build is preferred over the debug build, because a
/// verification run should not be measuring an unoptimised binary.
#[must_use]
pub fn find(tools_dir: &Path, name: &str) -> Invocation {
    for profile in ["release", "debug"] {
        let candidate = tools_dir.join("target").join(profile).join(name);
        if candidate.is_file() {
            return Invocation {
                program: candidate.display().to_string(),
                args: Vec::new(),
            };
        }
    }

    Invocation {
        program: "cargo".to_string(),
        args: vec!["run".to_string(), "--bin".to_string(), name.to_string()],
    }
}

/// Find a workspace binary and add arguments to it.
#[must_use]
pub fn find_with(tools_dir: &Path, name: &str, args: &[&str]) -> Invocation {
    let mut invocation = find(tools_dir, name);
    invocation
        .args
        .extend(args.iter().map(|a| (*a).to_string()));
    invocation
}

/// The path of a binary in the workspace, whether or not it is built.
#[must_use]
pub fn path(tools_dir: &Path, name: &str) -> PathBuf {
    tools_dir.join("target").join("release").join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finds_a_release_binary_first() {
        let dir = tempfile::tempdir().expect("temp dir");
        let release = dir.path().join("target/release");
        std::fs::create_dir_all(&release).expect("create");
        std::fs::write(release.join("fs-wasm"), "").expect("write");

        let invocation = find(dir.path(), "fs-wasm");
        assert_eq!(
            invocation.program,
            release.join("fs-wasm").display().to_string()
        );
        assert!(invocation.args.is_empty());
    }

    #[test]
    fn test_falls_back_to_a_debug_binary() {
        let dir = tempfile::tempdir().expect("temp dir");
        let debug = dir.path().join("target/debug");
        std::fs::create_dir_all(&debug).expect("create");
        std::fs::write(debug.join("fs-wasm"), "").expect("write");

        let invocation = find(dir.path(), "fs-wasm");
        assert_eq!(
            invocation.program,
            debug.join("fs-wasm").display().to_string()
        );
    }

    #[test]
    fn test_falls_back_to_cargo_run() {
        let dir = tempfile::tempdir().expect("temp dir");
        let invocation = find(dir.path(), "fs-wasm");
        assert_eq!(invocation.program, "cargo");
        assert_eq!(invocation.args, vec!["run", "--bin", "fs-wasm"]);
    }

    #[test]
    fn test_arguments_are_appended() {
        let dir = tempfile::tempdir().expect("temp dir");
        let invocation = find_with(dir.path(), "fs-wasm", &["--json"]);
        assert_eq!(invocation.program, "cargo");
        assert_eq!(invocation.args, vec!["run", "--bin", "fs-wasm", "--json"]);
    }
}
