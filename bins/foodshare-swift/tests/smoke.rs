//! The binaries have to start, and the ones that can be asked something
//! have to answer it.
//!
//! Twelve of the fourteen binaries had no tests at all, so a binary that
//! could not print its own help would have been found by a user rather
//! than by the build.

use assert_cmd::Command;
use predicates::prelude::*;

fn binary(name: &str) -> Command {
    Command::cargo_bin(name).expect("the binary should be built")
}

#[test]
fn the_binary_prints_its_help() {
    binary("foodshare-swift")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn the_binary_prints_its_version() {
    binary("foodshare-swift")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn the_binary_refuses_a_command_it_does_not_have() {
    binary("foodshare-swift")
        .arg("not-a-command")
        .assert()
        .failure();
}

#[test]
fn the_binary_detects_the_toolchain() {
    // The detection reads the machine, so it answers whatever is there.
    // The answer has to be a report and not a crash.
    binary("foodshare-swift")
        .arg("detect")
        .assert()
        .success()
        .stdout(predicate::str::contains("Swift"));
}
