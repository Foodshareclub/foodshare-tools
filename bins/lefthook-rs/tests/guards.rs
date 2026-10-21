//! The guards have to guard.
//!
//! A hook that reports a problem and then exits with success is a hook
//! that reports and does nothing. These tests are about the exit code,
//! because that is what the hook chain reads.

use assert_cmd::Command;
use std::path::Path;
use std::process::Command as StdCommand;

/// Make a directory into a git repository on the given branch.
fn repository_at(path: &Path, branch: &str) {
    let git = |args: &[&str]| {
        let status = StdCommand::new("git")
            .args(args)
            .current_dir(path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("git should run");
        assert!(status.success(), "git {args:?} should have succeeded");
    };

    git(&["init", "--initial-branch", branch]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "user.name", "Test"]);
    git(&["commit", "--allow-empty", "-m", "chore: an empty commit"]);
}

#[test]
fn a_protected_branch_stops_the_push() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    repository_at(dir.path(), "main");

    let mut command = Command::cargo_bin("lefthook-rs").expect("the binary should be built");
    command.current_dir(dir.path()).arg("protected-branch");

    let output = command.output().expect("the command should run");
    assert_eq!(
        output.status.code(),
        Some(1),
        "a push to a protected branch should be stopped"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("protected branch"),
        "the reason should be said: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_branch_of_your_own_lets_the_push_through() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    repository_at(dir.path(), "feature/a-change");

    let mut command = Command::cargo_bin("lefthook-rs").expect("the binary should be built");
    command.current_dir(dir.path()).arg("protected-branch");

    let output = command.output().expect("the command should run");
    assert_eq!(
        output.status.code(),
        Some(0),
        "a push to a branch of your own should be allowed"
    );
}

#[test]
fn a_directory_that_is_not_a_repository_is_an_error() {
    // A repository that cannot be read is not a repository with nothing
    // to protect. The difference is the difference between "there is
    // nothing to find" and "we could not look".
    let dir = tempfile::tempdir().expect("a temporary directory");

    let mut command = Command::cargo_bin("lefthook-rs").expect("the binary should be built");
    command.current_dir(dir.path()).arg("protected-branch");

    let output = command.output().expect("the command should run");
    assert_eq!(
        output.status.code(),
        Some(4),
        "a git error should be reported as a git error"
    );
}

#[test]
fn a_secret_scan_outside_a_repository_is_an_error() {
    // The scan that never ran must not report a clean bill.
    let dir = tempfile::tempdir().expect("a temporary directory");

    let mut command = Command::cargo_bin("lefthook-rs").expect("the binary should be built");
    command.current_dir(dir.path()).arg("security");

    let output = command.output().expect("the command should run");
    assert_eq!(
        output.status.code(),
        Some(4),
        "a scan that could not look should not report success"
    );
}

#[test]
fn a_bundle_over_the_threshold_stops_the_commit() {
    // The threshold is the reason the command takes one.
    let dir = tempfile::tempdir().expect("a temporary directory");
    let chunks = dir.path().join(".next/static/chunks");
    std::fs::create_dir_all(&chunks).expect("the directory should be created");

    // A chunk that is larger than the threshold it is given.
    let big = vec![b'x'; 4096];
    std::fs::write(chunks.join("big.js"), big).expect("the file should be written");

    let mut command = Command::cargo_bin("lefthook-rs").expect("the binary should be built");
    command
        .current_dir(dir.path())
        .args(["bundle-size", "--threshold", "1"]);

    let output = command.output().expect("the command should run");
    assert_eq!(
        output.status.code(),
        Some(1),
        "a bundle over the threshold should stop the commit"
    );
}

#[test]
fn a_bundle_under_the_threshold_lets_the_commit_through() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let chunks = dir.path().join(".next/static/chunks");
    std::fs::create_dir_all(&chunks).expect("the directory should be created");
    std::fs::write(chunks.join("small.js"), b"x".repeat(16)).expect("the file should be written");

    let mut command = Command::cargo_bin("lefthook-rs").expect("the binary should be built");
    command
        .current_dir(dir.path())
        .args(["bundle-size", "--threshold", "500"]);

    let output = command.output().expect("the command should run");
    assert_eq!(
        output.status.code(),
        Some(0),
        "a bundle under the threshold should be allowed"
    );
}

#[test]
fn a_secret_exits_with_the_security_code() {
    // A secret that was found is a security failure, not a general one.
    // A pipeline that can tell the two apart can treat them differently.
    let dir = tempfile::tempdir().expect("a temporary directory");
    repository_at(dir.path(), "feature/a-change");

    std::fs::write(
        dir.path().join("config.txt"),
        "HOOK_SECRET=whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw\n",
    )
    .expect("the file should be written");

    let git = |args: &[&str]| {
        let _ = StdCommand::new("git")
            .args(args)
            .current_dir(dir.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    };
    git(&["add", "config.txt"]);

    let mut command = Command::cargo_bin("lefthook-rs").expect("the binary should be built");
    command.current_dir(dir.path()).arg("security");

    let output = command.output().expect("the command should run");
    assert_eq!(
        output.status.code(),
        Some(5),
        "a secret should exit with the security code"
    );
}

#[test]
fn a_commit_message_that_does_not_pass_exits_with_the_validation_code() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let message = dir.path().join("COMMIT_MSG");
    std::fs::write(&message, "this is not a conventional commit\n")
        .expect("the file should be written");

    let mut command = Command::cargo_bin("lefthook-rs").expect("the binary should be built");
    command
        .current_dir(dir.path())
        .args(["conventional-commit", message.to_str().expect("a path")]);

    let output = command.output().expect("the command should run");
    assert_eq!(
        output.status.code(),
        Some(2),
        "a message that does not pass should exit with the validation code"
    );
}
