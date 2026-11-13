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
/// Status (with its options) → create Assignee → create Phase (with its
/// options) → list fields again (option ids are only known after the
/// create) → list views (none) → create Board + Phases. The discussion
/// state is NOT queued: the fake serves the category-list query from
/// `discussion_state` (mirroring the REST `has_discussions` probe) and
/// categories cannot be created via API, so nothing is provisioned there.
fn fresh_org_responses() -> Vec<serde_json::Value> {
    let fields_full = serde_json::json!({
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
    });
    let data = [
        serde_json::json!({
            "organization": {
                "id": "ORG_fake",
                "projectsV2": {
                    "nodes": [],
                    "pageInfo": { "hasNextPage": false, "endCursor": null }
                }
            }
        }),
        serde_json::json!({
            "createProjectV2": { "projectV2": { "id": "PV1_fake", "title": "Foodshare Board" } }
        }),
        serde_json::json!({ "node": { "fields": { "nodes": [] } } }),
        // Call order: the Assignee text field is created before the
        // single-select ensures run.
        serde_json::json!({
            "createProjectV2Field": { "projectV2Field": { "id": "F_assignee" } }
        }),
        serde_json::json!({
            "createProjectV2Field": { "projectV2Field": { "id": "F_status" } }
        }),
        serde_json::json!({
            "createProjectV2Field": { "projectV2Field": { "id": "F_phase" } }
        }),
        fields_full,
        serde_json::json!({ "node": { "views": { "nodes": [] } } }),
        serde_json::json!({
            "createProjectV2View": { "projectV2View": { "id": "V_board" } }
        }),
        serde_json::json!({
            "createProjectV2View": { "projectV2View": { "id": "V_phases" } }
        }),
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
    // Discussions are enabled on this repo, and the configured feed
    // category (default "General") pre-exists. Categories cannot be
    // created via API (live-verified): setup only verifies the feed
    // category among the pre-existing ones.
    gh.discussion_state.insert(
        "foodshare-tools".to_string(),
        (
            true,
            vec!["Announcements".into(), "General".into(), "Q&A".into()],
        ),
    );
    // Discussion labels are supported on this repo (the probe finds the
    // labels endpoint); setup records both flags.
    gh.discussion_labels_supported
        .insert("foodshare-tools".to_string(), true);

    let report = run_setup(&mut gh, &cfg).await.unwrap();

    assert_eq!(report.user, "fakeuser");
    assert!(report.project_created);
    // The create call must use the live input shape {ownerId, title}:
    // `owner` and `contentKind` do not exist on CreateProjectV2Input
    // (live-verified 2026-11-12 — the old shape was rejected by the API).
    let create_call = gh
        .graphql_calls
        .iter()
        .find(|(q, _)| q.contains("GhsyncCreateProject"))
        .expect("the create call was made");
    let input = &create_call.1["input"];
    assert_eq!(input["ownerId"], "ORG_fake");
    assert_eq!(input["title"], "Foodshare Board");
    assert!(input.get("owner").is_none(), "stale input key: owner");
    assert!(
        input.get("contentKind").is_none(),
        "stale input key: contentKind"
    );
    assert_eq!(report.project_id, "PV1_fake");
    assert_eq!(report.status_field, "F_status");
    assert_eq!(report.assignee_field, "F_assignee");
    assert_eq!(report.phase_field, "F_phase");
    // Options are created WITH the field (no separate option-create
    // mutation exists in the live schema): the Status create call carries
    // all four options, each with the required name/color/description.
    // (The options ride on the query string as `singleSelectOptions:
    // $options`; the variables are top-level.)
    let status_create = gh
        .graphql_calls
        .iter()
        .find(|(q, v)| q.contains("GhsyncCreateSingleSelect") && v["name"] == "Status")
        .expect("the Status create was made");
    let options = &status_create.1["options"];
    let names: Vec<&str> = options
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["name"].as_str().unwrap())
        .collect();
    // BTreeMap key order: alphabetical.
    assert_eq!(names, vec!["Backlog", "Blocked", "Done", "In progress"]);
    for o in options.as_array().unwrap() {
        assert!(o.get("color").and_then(|c| c.as_str()).is_some());
        assert!(o.get("description").and_then(|c| c.as_str()).is_some());
    }
    // No per-option create calls of any kind.
    assert!(
        !gh.graphql_calls
            .iter()
            .any(|(q, _)| q.contains("GhsyncCreateOption")),
        "stale option-create call"
    );
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
    // The feed category is verified, never created: categories cannot be
    // created via API (live-verified), so setup only checks the
    // configured one among the pre-existing ones.
    assert_eq!(
        report.discussions,
        vec![(
            "foodshare-tools".to_string(),
            "enabled — feed category General present".to_string()
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
    // views, discussions with the feed category, all labels, milestone
    // present.
    let mut gh = FakeGitHub::default();
    gh.discussion_state.insert(
        "foodshare-tools".to_string(),
        (true, vec!["General".into()]),
    );
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
        let fields_full = serde_json::json!({
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
        });
        let data = [
            serde_json::json!({
                "organization": {
                    "id": "ORG_fake",
                    "projectsV2": {
                        "nodes": [
                            { "id": "PV1_fake", "title": "Foodshare Board", "number": 1 }
                        ],
                        "pageInfo": { "hasNextPage": false, "endCursor": null }
                    }
                }
            }),
            fields_full.clone(),
            // Option ids are collected from a second fields read (the
            // ensures run first, no mutations expected on a complete
            // board).
            fields_full,
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
    // Zero creates of any kind (no category creations possible: they
    // cannot be created via API at all).
    assert!(gh.created_labels.is_empty());
    assert!(gh.milestones["foodshare-tools"].len() == 1);
    // Only the five read calls: find project, fields (ensure), fields
    // (option ids), discussion categories (state-answered by the fake,
    // not the queue), views.
    let creates: Vec<&str> = gh
        .graphql_calls
        .iter()
        .map(|(q, _)| q.as_str())
        .filter(|q| q.starts_with("mutation"))
        .collect();
    assert!(creates.is_empty(), "unexpected mutations: {creates:?}");
    assert_eq!(gh.graphql_calls.len(), 5);
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
async fn field_missing_an_option_gets_one_update_with_merged_list() {
    // The live schema has no option-create mutation: a single-select
    // field missing configured options is fixed with ONE
    // `updateProjectV2Field` carrying the FULL option list — existing
    // options keep their ids (so in-project items are not orphaned),
    // new ones are appended without ids.
    let dir = tempfile::tempdir().unwrap();
    let cfg = cfg_with(&dir.path().join("config.toml"));
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
    gh.discussion_state.insert(
        "foodshare-tools".to_string(),
        (true, vec!["General".into()]),
    );
    // Status exists but only has two of the four configured options.
    let fields_partial = serde_json::json!({
        "node": {
            "fields": {
                "nodes": [
                    {
                        "id": "F_status",
                        "name": "Status",
                        "options": [
                            { "id": "opt_backlog", "name": "Backlog" },
                            { "id": "opt_blocked", "name": "Blocked" }
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
    });
    let fields_full = serde_json::json!({
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
    });
    gh.graphql_responses = [
        serde_json::json!({
            "organization": {
                "id": "ORG_fake",
                "projectsV2": {
                    "nodes": [
                        { "id": "PV1_fake", "title": "Foodshare Board", "number": 1 }
                    ],
                    "pageInfo": { "hasNextPage": false, "endCursor": null }
                }
            }
        }),
        fields_partial,
        serde_json::json!({
            "updateProjectV2Field": { "projectV2Field": { "id": "F_status" } }
        }),
        fields_full,
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
    ]
    .into_iter()
    .map(|d| serde_json::json!({ "data": d }))
    .collect::<Vec<_>>();

    let report = run_setup(&mut gh, &cfg).await.unwrap();

    // Exactly one mutation, on the Status field.
    let mutations: Vec<_> = gh
        .graphql_calls
        .iter()
        .filter(|(q, _)| q.starts_with("mutation"))
        .collect();
    assert_eq!(mutations.len(), 1, "unexpected mutations: {mutations:?}");
    let (query, vars) = &mutations[0];
    assert!(query.contains("GhsyncSetFieldOptions"));
    assert_eq!(vars["fieldId"], "F_status");
    let options = &vars["options"];
    let arr = options.as_array().unwrap();
    assert_eq!(arr.len(), 4);
    // Existing options keep their ids; new ones are appended without.
    assert_eq!(arr[0]["id"], "opt_backlog");
    assert_eq!(arr[1]["id"], "opt_blocked");
    assert!(arr[2].get("id").is_none());
    assert_eq!(arr[2]["name"], "Done");
    assert!(arr[3].get("id").is_none());
    assert_eq!(arr[3]["name"], "In progress");
    // The report carries all four option ids from the final read.
    assert_eq!(report.status_option_ids.len(), 4);
    assert_eq!(
        report.status_option_ids.get("Done").map(String::as_str),
        Some("opt_done")
    );
    assert!(gh.graphql_responses.is_empty());
}

#[tokio::test]
async fn missing_feed_category_fails_loud_and_stops_before_views() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = cfg_with(&dir.path().join("config.toml"));
    let mut gh = FakeGitHub::default();
    gh.graphql_responses = fresh_org_responses();
    // Discussions enabled, but the configured feed category (the default
    // "General") is not among the pre-existing categories.
    gh.discussion_state.insert(
        "foodshare-tools".to_string(),
        (true, vec!["Announcements".into(), "Q&A".into()]),
    );

    let err = run_setup(&mut gh, &cfg).await.unwrap_err();
    let msg = err.to_string();
    let SetupError::FeedCategory {
        repo,
        wanted,
        available,
    } = err
    else {
        panic!("expected FeedCategory, got: {msg}");
    };
    assert_eq!(repo, "foodshare-tools");
    assert_eq!(wanted, "General");
    assert_eq!(
        available,
        vec!["Announcements".to_string(), "Q&A".to_string()]
    );
    // The error must be actionable: name the wanted category and the
    // available ones so the operator can fix the config.
    assert!(msg.contains("General"), "{msg}");
    assert!(msg.contains("Announcements"), "{msg}");
    // The final step (views) never ran: its three queued responses are
    // untouched.
    assert_eq!(gh.graphql_responses.len(), 3);
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
