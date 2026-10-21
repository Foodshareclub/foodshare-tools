//! The binaries have to start.
//!
//! Twelve of the fourteen binaries had no tests at all, so a binary that
//! could not print its own help would have been found by a user rather
//! than by the build. These tests are the floor: the binary starts, it
//! says its version, and it refuses a command it does not have.

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

#[test]
fn the_binary_refuses_a_command_it_does_not_have() {
    binary("fs-build").arg("not-a-command").assert().failure();
}
