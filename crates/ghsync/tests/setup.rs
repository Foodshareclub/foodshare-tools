//! T8 — setup: idempotent org provisioning.
//!
//! Every step is a find-or-create; a second run writes nothing. The live
//! run needs the org PAT (T0) and is blocked on it; these tests run the
//! whole flow against the fake with queued GraphQL responses, and the
//! GraphQL strings themselves are pinned by the unit test in `github.rs`.

use std::path::Path;

use ghsync::config::{Config, RepoConfig};
use ghsync::fakes::FakeGitHub;
use ghsync::setup::{SetupError, apply_report_to_config, run_setup};

fn cfg_with(path: &Path) -> Config {
    Config {
        repos: vec![RepoConfig {
            name: "foodshare-tools".into(),
            default_milestone: "Rust + TypeScript".into(),
            ..Default::default()
        }],
        token: Some("fake-token".into()),
        state_path: path.to_path_buf(),
        ..Config::default()
    }
}

/// The exact GraphQL responses a fresh-org setup receives, in call order:
/// find project (none) → create project → list fields (none) → create
/// Status + 4 options → create Assignee → create Phase + 3 options →
/// discussions (none, enabled) → create category → list views (none) →
/// create Board + Phases. The fake unwraps the `data` envelope, so each
/// entry is the inner data object wrapped in it.
fn fresh_org_responses() -> Vec<serde_json::Value> {
    let data = [
        serde_json::json!({
            "organization": {
                "projectsV2": {
                    "nodes": [],
                    "pageInfo": { "hasNextPage": false, "endCursor": null }
                }
            }
        }),
        serde_json::json!({
            "createProjectV2": { "project": { "id": "PV1_fake", "title": "Foodshare Board" } }
        }),
        serde_json::json!({ "node": { "fields": { "nodes": [] } } }),
        serde_json::json!({ "createProjectV2Field": { "field": { "id": "F_status" } } }),
        serde_json::json!({ "createProjectV2FieldSingleSelectOption": { "option": { "id": "opt_backlog", "name": "Backlog" } } }),
        serde_json::json!({ "createProjectV2FieldSingleSelectOption": { "option": { "id": "opt_blocked", "name": "Blocked" } } }),
        serde_json::json!({ "createProjectV2FieldSingleSelectOption": { "option": { "id": "opt_done", "name": "Done" } } }),
        serde_json::json!({ "createProjectV2FieldSingleSelectOption": { "option": { "id": "opt_inprog", "name": "In progress" } } }),
        serde_json::json!({ "createProjectV2Field": { "field": { "id": "F_assignee" } } }),
        serde_json::json!({ "createProjectV2Field": { "field": { "id": "F_phase" } } }),
        serde_json::json!({ "createProjectV2FieldSingleSelectOption": { "option": { "id": "ph_backlog", "name": "Backlog" } } }),
        serde_json::json!({ "createProjectV2FieldSingleSelectOption": { "option": { "id": "ph_done", "name": "Done" } } }),
        serde_json::json!({ "createProjectV2FieldSingleSelectOption": { "option": { "id": "ph_inprog", "name": "In progress" } } }),
        serde_json::json!({
            "repository": {
                "id": "R_1",
                "hasDiscussions": true,
                "discussionCategories": { "nodes": [] }
            }
        }),
        serde_json::json!({ "addDiscussionCategory": { "discussionCategory": { "id": "C_1", "name": "Other" } } }),
        serde_json::json!({ "node": { "views": { "nodes": [] } } }),
        serde_json::json!({ "createProjectV2View": { "view": { "id": "V_board" } } }),
        serde_json::json!({ "createProjectV2View": { "view": { "id": "V_phases" } } }),
    ];
    data.into_iter()
        .map(|d| serde_json::json!({ "data": d }))
        .collect()
}

#[tokio::test]
async fn fresh_org_setup_provisions_everything_and_records_ids() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    let mut cfg = cfg_with(&cfg_path);
    let mut gh = FakeGitHub::default();
    gh.graphql_responses = fresh_org_responses();
    // Discussion labels are supported on this repo (the probe finds the
    // labels endpoint); setup records both flags.
    gh.discussion_labels_supported
        .insert("foodshare-tools".to_string(), true);

    let report = run_setup(&mut gh, &cfg).await.unwrap();

    assert_eq!(report.user, "fakeuser");
    assert!(report.project_created);
    assert_eq!(report.project_id, "PV1_fake");
    assert_eq!(report.status_field, "F_status");
    assert_eq!(report.assignee_field, "F_assignee");
    assert_eq!(report.phase_field, "F_phase");
    // Every option of both single-select fields is recorded with its id.
    assert_eq!(report.status_option_ids.len(), 4);
    assert_eq!(
        report.status_option_ids.get("Backlog").map(String::as_str),
        Some("opt_backlog")
    );
    assert_eq!(
        report
            .status_option_ids
            .get("In progress")
            .map(String::as_str),
        Some("opt_inprog")
    );
    assert_eq!(report.phase_option_ids.len(), 3);
    assert_eq!(
        report
            .phase_option_ids
            .get("In progress")
            .map(String::as_str),
        Some("ph_inprog")
    );
    // Labels: ghsync, ghsync-<repo>, intake.
    assert_eq!(
        gh.created_labels,
        vec![
            ("foodshare-tools".to_string(), "ghsync".to_string()),
            (
                "foodshare-tools".to_string(),
                "ghsync-foodshare-tools".to_string()
            ),
            ("foodshare-tools".to_string(), "intake".to_string()),
        ]
    );
    // Milestone for the repo's default.
    assert_eq!(gh.milestones["foodshare-tools"].len(), 1);
    assert_eq!(
        gh.milestones["foodshare-tools"][0].title,
        "Rust + TypeScript"
    );
    // Handoffs category created and renamed.
    assert_eq!(gh.created_categories, vec!["R_1".to_string()]);
    assert_eq!(
        gh.renamed_categories,
        vec![(
            "foodshare-tools".to_string(),
            "C_1".to_string(),
            "Handoffs".to_string()
        )]
    );
    // The per-repo discussion flags are recorded for the config.
    assert_eq!(
        report.discussion_flags,
        vec![("foodshare-tools".to_string(), true, true)]
    );
    // No unused responses left: every queued response was consumed in
    // order (a wrong call order would have consumed a mismatched one).
    assert!(gh.graphql_responses.is_empty());

    // The ids land in the config file, which is re-loadable and keeps the
    // token.
    apply_report_to_config(&mut cfg, &report);
    cfg.save(&cfg_path).unwrap();
    let reloaded = Config::load(&cfg_path).unwrap();
    let p = reloaded.project.as_ref().unwrap();
    assert_eq!(p.project_id, "PV1_fake");
    assert_eq!(p.status_field, "F_status");
    assert_eq!(
        p.status_option_ids.get("Backlog").map(String::as_str),
        Some("opt_backlog")
    );
    assert_eq!(
        p.phase_option_ids.get("In progress").map(String::as_str),
        Some("ph_inprog")
    );
    assert_eq!(reloaded.token.as_deref(), Some("fake-token"));
    // The discussion flags round-trip into the config too.
    assert!(reloaded.repos[0].discussions_enabled);
    assert!(reloaded.repos[0].discussion_labels_enabled);
}

#[tokio::test]
async fn second_run_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    let cfg = cfg_with(&cfg_path);
    // A fully provisioned org: project, all fields with options, both
    // views, discussions with Handoffs, all labels, milestone present.
    let mut gh = FakeGitHub::default();
    gh.labels.insert(
        "foodshare-tools".to_string(),
        vec![
            "ghsync".into(),
            "ghsync-foodshare-tools".into(),
            "intake".into(),
        ],
    );
    gh.milestones.insert(
        "foodshare-tools".to_string(),
        vec![ghsync::github::Milestone {
            number: 1,
            title: "Rust + TypeScript".into(),
            open_issues: 0,
            closed_issues: 0,
        }],
    );
    gh.graphql_responses = {
        let data = [
            serde_json::json!({
                "organization": {
                    "projectsV2": {
                        "nodes": [
                            { "id": "PV1_fake", "title": "Foodshare Board", "number": 1 }
                        ],
                        "pageInfo": { "hasNextPage": false, "endCursor": null }
                    }
                }
            }),
            serde_json::json!({
                "node": {
                    "fields": {
                        "nodes": [
                            {
                                "id": "F_status",
                                "name": "Status",
                                "options": [
                                    { "id": "opt_backlog", "name": "Backlog" },
                                    { "id": "opt_blocked", "name": "Blocked" },
                                    { "id": "opt_done", "name": "Done" },
                                    { "id": "opt_inprog", "name": "In progress" }
                                ]
                            },
                            { "id": "F_assignee", "name": "Assignee", "options": null },
                            {
                                "id": "F_phase",
                                "name": "Phase",
                                "options": [
                                    { "id": "ph_backlog", "name": "Backlog" },
                                    { "id": "ph_done", "name": "Done" },
                                    { "id": "ph_inprog", "name": "In progress" }
                                ]
                            }
                        ]
                    }
                }
            }),
            serde_json::json!({
                "repository": {
                    "id": "R_1",
                    "hasDiscussions": true,
                    "discussionCategories": { "nodes": [{ "id": "C_1", "name": "Handoffs" }] }
                }
            }),
            serde_json::json!({
                "node": {
                    "views": {
                        "nodes": [
                            { "id": "V_board", "name": "Board" },
                            { "id": "V_phases", "name": "Phases" }
                        ]
                    }
                }
            }),
        ];
        data.into_iter()
            .map(|d| serde_json::json!({ "data": d }))
            .collect::<Vec<_>>()
    };

    let report = run_setup(&mut gh, &cfg).await.unwrap();

    assert!(!report.project_created);
    assert_eq!(report.project_id, "PV1_fake");
    // Zero creates of any kind.
    assert!(gh.created_labels.is_empty());
    assert!(gh.created_categories.is_empty());
    assert!(gh.renamed_categories.is_empty());
    assert!(gh.milestones["foodshare-tools"].len() == 1);
    // Only the four read calls: find project, fields, discussions, views.
    let creates: Vec<&str> = gh
        .graphql_calls
        .iter()
        .map(|(q, _)| q.as_str())
        .filter(|q| q.starts_with("mutation"))
        .collect();
    assert!(creates.is_empty(), "unexpected mutations: {creates:?}");
    assert_eq!(gh.graphql_calls.len(), 4);
    // The ids are still reported, so the config stays correct.
    assert_eq!(report.status_option_ids.len(), 4);
    assert_eq!(report.phase_option_ids.len(), 3);
    // The flags are still probed and reported (probe is a read, not a
    // mutation; the fake returns false unless seeded).
    assert_eq!(
        report.discussion_flags,
        vec![("foodshare-tools".to_string(), true, false)]
    );
    let _ = &cfg_path;
}

#[tokio::test]
async fn rejected_token_fails_loud() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = cfg_with(&dir.path().join("config.toml"));
    let mut gh = FakeGitHub::default();
    gh.user_rejected = true;

    let err = run_setup(&mut gh, &cfg).await.unwrap_err();
    assert!(matches!(err, SetupError::Auth { .. }), "got: {err}");
    assert!(
        err.to_string().contains("401"),
        "error must name the failure: {err}"
    );
}
