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

/// The first eight bytes of a PNG file.
const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

#[test]
fn the_binary_prints_its_help() {
    binary("fs-image")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn the_binary_prints_its_version() {
    binary("fs-image")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn the_binary_refuses_a_command_it_does_not_have() {
    binary("fs-image").arg("not-a-command").assert().failure();
}

#[test]
fn the_binary_detects_a_png() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("image.png");
    std::fs::write(&path, PNG_MAGIC).expect("the file should be written");

    binary("fs-image")
        .args(["detect", path.to_str().expect("a path")])
        .assert()
        .success()
        .stdout(predicate::str::contains("Png"));
}

#[test]
fn the_binary_says_a_file_that_is_not_an_image_is_not_one() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("notes.txt");
    std::fs::write(&path, b"not an image").expect("the file should be written");

    binary("fs-image")
        .args(["detect", path.to_str().expect("a path")])
        .assert()
        .failure();
}
