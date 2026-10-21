//! The report and the exit code have to agree, and the arguments have to
//! parse.
//!
//! The translation tool declared `-f` on three different arguments: the
//! global `--format`, the `--file` of `update`, and the `--file` of
//! `backfill`. Clap refuses to build a command with two arguments on one
//! short name, so the debug build panicked before it parsed anything —
//! and the debug build is what the documentation tells a reader to run.

use assert_cmd::Command;

#[test]
fn a_failed_deployment_in_json_mode_exits_with_a_failure() {
    let mut command = Command::cargo_bin("foodshare-i18n").expect("the binary should be built");

    // The environment check fails without the Supabase settings, which is
    // the first step of the deployment.
    command
        .env_remove("SUPABASE_URL")
        .env_remove("SUPABASE_SERVICE_ROLE_KEY")
        .args(["--format", "json", "deploy"]);

    let output = command.output().expect("the command should run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("the output should be JSON ({e}): {stdout}"));

    assert_eq!(
        parsed["success"],
        serde_json::json!(false),
        "the report should say the deployment failed"
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "the exit code should say the deployment failed too"
    );
}

/// Every subcommand has to be able to describe itself.
///
/// A command that panics while building its help is a command that
/// cannot be used at all, and the panic happens before the arguments
/// are even looked at.
#[test]
fn every_subcommand_can_describe_itself() {
    for subcommand in [
        "health",
        "test",
        "audit",
        "translate",
        "bench",
        "locales",
        "deploy",
        "update",
        "backfill",
        "generate-infoplist",
    ] {
        let output = Command::cargo_bin("foodshare-i18n")
            .expect("the binary should be built")
            .arg(subcommand)
            .arg("--help")
            .output()
            .expect("the command should run");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panicked"),
            "the subcommand {subcommand} panicked: {stderr}"
        );
        assert!(
            output.status.success(),
            "the subcommand {subcommand} could not describe itself: {stderr}"
        );
    }
}
