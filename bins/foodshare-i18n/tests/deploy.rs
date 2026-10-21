//! The report and the exit code have to agree.
//!
//! A caller that reads the exit code and a caller that reads the JSON must
//! not get different answers about whether the deployment passed.

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
