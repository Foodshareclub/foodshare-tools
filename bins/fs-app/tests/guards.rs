//! The guards have to guard, and the formats have to be the format they say.
//!
//! These tests are about the exit code and the output, because that is
//! what a caller reads.

use assert_cmd::Command;
use std::path::{Path, PathBuf};

/// Put a script on the PATH that records how it was called.
///
/// The lint command runs `swiftlint`. A machine that has it and a machine
/// that does not are two different machines, so the test brings its own.
fn fake_swiftlint(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("the directory should be created");

    let script = bin.join("swiftlint");
    std::fs::write(
        &script,
        "#!/bin/sh\nfor arg in \"$@\"; do echo \"$arg\" >> \"$SWIFTLINT_CALLS\"; done\nexit 0\n",
    )
    .expect("the script should be written");
    make_executable(&script);

    bin
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)
            .expect("the file should exist")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("the mode should be set");
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[test]
fn the_doctor_speaks_json_when_it_is_asked_to() {
    let mut command = Command::cargo_bin("fs-app").expect("the binary should be built");
    command.args(["doctor", "--json"]);

    let output = command.output().expect("the command should run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The flag that says JSON has to produce JSON. A flag that prints text
    // is a flag that lies about the format.
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("the output should be JSON ({e}): {stdout}"));

    assert!(
        parsed.get("xcode").is_some(),
        "the report should name Xcode"
    );
    assert!(
        parsed.get("swift").is_some(),
        "the report should name Swift"
    );
    assert!(
        parsed.get("missing").is_some(),
        "the report should name what is missing"
    );
}

#[test]
fn a_secret_scan_outside_a_repository_is_an_error() {
    let dir = tempfile::tempdir().expect("a temporary directory");

    let mut command = Command::cargo_bin("fs-app").expect("the binary should be built");
    command.current_dir(dir.path()).arg("secrets");

    let output = command.output().expect("the command should run");
    assert_eq!(
        output.status.code(),
        Some(4),
        "a scan that could not look should not report success"
    );
}

#[test]
fn a_file_is_linted_as_a_file() {
    // The command takes files. A file that is treated as a directory is
    // linted inside, which finds nothing and says nothing.
    let dir = tempfile::tempdir().expect("a temporary directory");
    let bin = fake_swiftlint(dir.path());
    let calls = dir.path().join("calls.txt");

    let source = dir.path().join("Example.swift");
    std::fs::write(&source, "let x = 1\n").expect("the file should be written");

    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let mut command = Command::cargo_bin("fs-app").expect("the binary should be built");
    command
        .current_dir(dir.path())
        .env("PATH", path)
        .env("SWIFTLINT_CALLS", &calls)
        .args(["lint", "Example.swift"]);

    let _ = command.output().expect("the command should run");

    let recorded = std::fs::read_to_string(&calls).unwrap_or_default();
    assert!(
        recorded.contains("Example.swift"),
        "the file should have been passed to the linter: {recorded:?}"
    );
}

#[test]
fn the_configuration_keys_the_tool_does_not_read_are_reported() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = dir.path().join(".foodshare-hooks.toml");
    std::fs::write(
        &config,
        "[docs]\noutput_dir = \"docs/api\"\n\n[secrets]\npatterns = []\n",
    )
    .expect("the file should be written");

    let mut command = Command::cargo_bin("fs-app").expect("the binary should be built");
    command
        .current_dir(dir.path())
        .args(["--config", config.to_str().expect("a path"), "secrets"]);

    let output = command.output().expect("the command should run");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("docs"),
        "the ignored section should be reported: {stderr}"
    );
}
