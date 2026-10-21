//! The build engine has to accept the flags it documents.
//!
//! It used to declare them as trailing arguments and read them by hand,
//! so every flag it advertised — `--release`, `--ios`, `--android`,
//! `--all`, `--swift`, `--clean`, `--test` — was rejected by the
//! argument parser before it reached the code. `fs-build --release`
//! exited 2 with "unexpected argument".

use assert_cmd::Command;
use predicates::prelude::*;

fn binary(name: &str) -> Command {
    Command::cargo_bin(name).expect("the binary should be built")
}

#[test]
fn the_binary_prints_its_help() {
    binary("fs-build")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn the_binary_prints_its_version() {
    binary("fs-build")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

/// Every flag the engine documents has to be accepted.
///
/// The engine's help is the list of what it can do, and a flag that is
/// in the help and not in the parser is a flag that does not work.
#[test]
fn every_documented_flag_is_accepted() {
    for flag in [
        "--android",
        "--ios",
        "--all",
        "--swift",
        "--release",
        "--clean",
        "--test",
    ] {
        let output = binary("fs-build")
            .arg(flag)
            .arg("--help")
            .output()
            .expect("the command should run");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "the flag {flag} was rejected: {stderr}"
        );
    }
}

/// Every suite the engine documents has to be a subcommand.
#[test]
fn every_documented_suite_is_a_subcommand() {
    for suite in ["swift", "kotlin", "bridge", "test"] {
        let output = binary("fs-build")
            .arg(suite)
            .arg("--help")
            .output()
            .expect("the command should run");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "the suite {suite} was rejected: {stderr}"
        );
    }
}

#[test]
fn the_binary_refuses_a_command_it_does_not_have() {
    binary("fs-build").arg("not-a-command").assert().failure();
}
