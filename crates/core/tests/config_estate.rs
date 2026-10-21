//! The configuration the estate writes, and what this build does with it.
//!
//! The repositories in the estate write a `.foodshare-hooks.toml` next to
//! their code. `foodshare-app` writes the largest one. These tests load
//! that shape and check two things: that the keys the tools act on are
//! read, and that the keys they do not act on are reported rather than
//! dropped in silence.

use foodshare_core::config::{Config, ConfigSchema};
use std::path::Path;

/// The shape `foodshare-app/.foodshare-hooks.toml` has.
///
/// The values are placeholders: the file is about which sections and keys
/// exist, not about what they say. The real file is checked by
/// `the_real_estate_file_is_reported_honestly` when the repository is
/// next to this one.
const ESTATE_SHAPE: &str = r#"
[general]
swift_version = "6.1"
scheme = "FoodShare"
destination = "platform=iOS Simulator,name=iPhone 17 Pro Max"
device = "iPhone 17 Pro Max"
bundle_id = "com.flutterflow.foodshare"
package_name = "com.foodshare"
exclude_paths = ["**/Build/**", "**/DerivedData/**"]

[format]
stage_fixed = true
paths = ["Sources/FoodShare"]

[lint]
strict = false
reporter = "default"
paths = ["Sources/FoodShare"]

[analyze]
threshold_todos = 10
threshold_fixmes = 5
threshold_force = 20
paths = ["Sources/FoodShare"]

[secrets]
patterns = ["(?i)token\\s*[=:]\\s*[\"'][^\"']{8,}[\"']"]
exclude_patterns = ["your_token_here"]
exclude_files = [".lefthook.yml"]

[commit_msg]
types = ["feat", "fix", "docs"]
max_length = 72
min_length = 10
skip_merge = true
skip_revert = true

[test]
coverage_threshold = 70
unit_target = "FoodShareTests"

[migrations]
migrations_dir = "supabase/migrations"

[pre_push]
fail_fast = true
configuration = "Release"
timeout = 1800
quick_checks = ["validate", "memory"]

[pre_push.timeouts]
validate = 120
build = 1800

[pre_push.guidance]
enabled = true

[version]
project_file = "Darwin/FoodShare.xcodeproj/project.pbxproj"

[build]
clean_first = false

[deps]
resolved_file = "Package.resolved"

[docs]
output_dir = "docs/api"

[clean]
derived_data = "~/Library/Developer/Xcode/DerivedData/FoodShare-*"

[android]
module = "app"
build_variant = "debug"
"#;

fn schema_of(content: &str) -> ConfigSchema {
    toml::from_str(content).expect("the configuration should parse")
}

#[test]
fn the_keys_the_tools_act_on_are_read() {
    let schema = schema_of(ESTATE_SHAPE);

    assert_eq!(schema.general.scheme.as_deref(), Some("FoodShare"));
    assert_eq!(
        schema.general.destination.as_deref(),
        Some("platform=iOS Simulator,name=iPhone 17 Pro Max")
    );
    assert_eq!(schema.general.device.as_deref(), Some("iPhone 17 Pro Max"));
    assert_eq!(
        schema.general.bundle_id.as_deref(),
        Some("com.flutterflow.foodshare")
    );
    assert_eq!(
        schema.general.package_name.as_deref(),
        Some("com.foodshare")
    );
    assert_eq!(schema.general.exclude_paths.len(), 2);

    assert!(schema.format.stage_fixed);
    assert_eq!(schema.format.paths, vec!["Sources/FoodShare".to_string()]);

    assert!(!schema.lint.strict);
    assert_eq!(schema.lint.reporter.as_deref(), Some("default"));

    assert_eq!(schema.analyze.paths, vec!["Sources/FoodShare".to_string()]);

    assert_eq!(schema.secrets.additional_patterns.len(), 1);
    assert_eq!(schema.secrets.exclude_patterns.len(), 1);

    assert!(schema.pre_push.fail_fast);
    assert_eq!(schema.pre_push.configuration.as_deref(), Some("Release"));
    assert_eq!(schema.pre_push.timeout, Some(1800));
    assert_eq!(schema.pre_push.timeouts.get("build"), Some(&1800));
}

#[test]
fn the_keys_the_tools_do_not_act_on_are_reported() {
    let schema = schema_of(ESTATE_SHAPE);
    let unknown = foodshare_core::config::unknown_keys_in(ESTATE_SHAPE, &schema)
        .expect("the configuration should be inspectable");

    // The sections no tool reads.
    for section in ["android", "build", "clean", "deps", "docs", "version"] {
        assert!(
            unknown.contains(&section.to_string()),
            "the section {section} was not reported: {unknown:?}"
        );
    }

    // The keys inside a section the tool does read.
    assert!(
        unknown.contains(&"pre_push.guidance".to_string()),
        "the guidance table was not reported: {unknown:?}"
    );
    assert!(
        unknown.contains(&"pre_push.quick_checks".to_string()),
        "the check lists were not reported: {unknown:?}"
    );
    assert!(
        unknown.contains(&"test.unit_target".to_string()),
        "the test target was not reported: {unknown:?}"
    );
}

#[test]
fn the_report_does_not_name_the_keys_that_are_read() {
    let schema = schema_of(ESTATE_SHAPE);
    let unknown = foodshare_core::config::unknown_keys_in(ESTATE_SHAPE, &schema)
        .expect("the configuration should be inspectable");

    for key in [
        "general.scheme",
        "general.destination",
        "format.paths",
        "lint.strict",
        "analyze.paths",
        "secrets.patterns",
        "commit_msg.max_length",
        "test.coverage_threshold",
        "migrations.migrations_dir",
        "pre_push.timeout",
        "pre_push.timeouts",
    ] {
        assert!(
            !unknown.contains(&key.to_string()),
            "the key {key} was reported as unknown: {unknown:?}"
        );
    }
}

#[test]
fn a_complete_configuration_reports_nothing() {
    let content = r#"
[secrets]
patterns = ["a-secret-shape"]
exclude_files = ["compose.yaml"]
"#;
    let schema = schema_of(content);
    let unknown = foodshare_core::config::unknown_keys_in(content, &schema)
        .expect("the configuration should be inspectable");
    assert!(
        unknown.is_empty(),
        "nothing should have been reported: {unknown:?}"
    );
}

/// The real file, when the estate is next to this repository.
///
/// The copy of the estate's file in the tests is a shape, not the file. A
/// developer with the workspace checked out gets the real one checked, so
/// a change to the estate's configuration shows up here rather than in
/// production.
#[test]
fn the_real_estate_file_is_reported_honestly() {
    let path = Path::new("../foodshare-app/.foodshare-hooks.toml");
    if !path.exists() {
        eprintln!("the estate is not next to this repository; skipping");
        return;
    }

    let content = std::fs::read_to_string(path).expect("the configuration should be readable");
    let schema = schema_of(&content);
    let unknown = foodshare_core::config::unknown_keys_in(&content, &schema)
        .expect("the configuration should be inspectable");

    // The keys the tools act on must not be reported.
    for key in [
        "general.scheme",
        "general.destination",
        "secrets.patterns",
        "pre_push.timeout",
    ] {
        assert!(
            !unknown.contains(&key.to_string()),
            "the key {key} was reported as unknown: {unknown:?}"
        );
    }

    // The sections no tool reads must be reported, or the report is not
    // telling the truth.
    for section in ["android", "build", "clean", "deps", "docs", "version"] {
        assert!(
            unknown.contains(&section.to_string()),
            "the section {section} was not reported: {unknown:?}"
        );
    }
}

#[test]
fn the_report_names_the_file_it_came_from() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join(".foodshare-hooks.toml");
    std::fs::write(&path, "[docs]\noutput_dir = \"docs/api\"\n")
        .expect("the file should be written");

    let config =
        Config::load(Some(path.to_str().expect("a path"))).expect("the config should load");
    assert_eq!(config.unknown_keys, vec!["docs".to_string()]);

    let report = config.unknown_keys_report();
    assert!(
        report.contains("docs"),
        "the report should name the key: {report}"
    );
    assert!(
        report.contains(path.to_str().expect("a path")),
        "the report should name the file: {report}"
    );
}

#[test]
fn a_configuration_with_nothing_to_report_says_nothing() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join(".foodshare-hooks.toml");
    std::fs::write(&path, "[secrets]\npatterns = []\n").expect("the file should be written");

    let config =
        Config::load(Some(path.to_str().expect("a path"))).expect("the config should load");
    assert!(config.unknown_keys.is_empty());
    assert!(config.unknown_keys_report().is_empty());
}
