//! T3 — seam tests: fakes, pure helpers, and the real clients' plumbing.
//!
//! The real clients are exercised live only in the E2E (T14). Here:
//! - the fakes behave like their real counterparts (in memory),
//! - the REST path builders produce exact strings,
//! - `RealKanban` refuses in a fenced (delegate-child) context,
//! - `RealKanban`'s CLI plumbing is verified end to end against a fake
//!   `hermes` binary that records its argv (board flag first, `--json`
//!   last, exit status and JSON parsing).

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;

use ghsync::fakes::{FakeGitHub, FakeKanban};
use ghsync::github::{
    GitHub, path_check_runs, path_discussion_category, path_issue, path_issue_comments,
    path_labels, path_milestones, path_pull, path_repo,
};
use ghsync::kanban::{Card, Kanban, KanbanError, RealKanban, links_from_comments};

// ---------- FakeKanban ----------

#[tokio::test]
async fn fake_kanban_create_lists_and_records() {
    let mut kb = FakeKanban::default();
    let card = kb
        .create("Title", "Body", "default", "key-1", "ready")
        .await
        .unwrap();
    assert!(card.id.starts_with("t_"));
    assert_eq!(card.status, "ready");
    assert_eq!(card.assignee.as_deref(), Some("default"));

    let cards = kb.list().await.unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].title, "Title");
    assert_eq!(kb.created.len(), 1);
    assert_eq!(kb.created[0].idempotency_key, "key-1");
    assert_eq!(kb.created[0].initial_status, "ready");
}

#[tokio::test]
async fn fake_kanban_lifecycle_records_and_updates_status() {
    let mut kb = FakeKanban::default();
    let card = kb.create("T", "B", "default", "k", "ready").await.unwrap();

    kb.comment(&card.id, "ghsync Foodshareclub/foodshare-app#41")
        .await
        .unwrap();
    let view = kb.show(&card.id).await.unwrap();
    assert_eq!(view.comments.len(), 1);
    assert_eq!(view.card.links.len(), 1);
    assert_eq!(view.card.links[0].number, 41);

    kb.block(&card.id, "needs_input", "waiting").await.unwrap();
    assert_eq!(kb.list().await.unwrap()[0].status, "blocked");
    assert_eq!(kb.blocked[0].2, "waiting");

    kb.unblock(&card.id, "go").await.unwrap();
    assert_eq!(kb.list().await.unwrap()[0].status, "ready");
    assert_eq!(kb.unblocked[0].1, "go");

    kb.complete(&card.id, "done it").await.unwrap();
    assert_eq!(kb.list().await.unwrap()[0].status, "done");
    assert_eq!(kb.completed[0].1, "done it");
}

#[tokio::test]
async fn fake_kanban_show_missing_errors() {
    let mut kb = FakeKanban::default();
    assert!(matches!(
        kb.show("t_nope").await.unwrap_err(),
        KanbanError::NotFound(_, _)
    ));
}

// ---------- FakeGitHub ----------

#[tokio::test]
async fn fake_github_issue_lifecycle() {
    let mut gh = FakeGitHub::default();
    let issue = gh
        .create_issue("foodshare-app", "Title", "Body", &["ghsync".into()])
        .await
        .unwrap();
    assert_eq!(issue.number, 1);
    assert_eq!(issue.state, "open");
    assert_eq!(issue.labels, vec!["ghsync".to_string()]);
    assert_eq!(issue.repo, "foodshare-app");

    let got = gh.get_issue("foodshare-app", 1).await.unwrap();
    assert_eq!(got.title, "Title");
    assert_eq!(got.body, "Body");

    gh.close_issue("foodshare-app", 1).await.unwrap();
    assert_eq!(
        gh.get_issue("foodshare-app", 1).await.unwrap().state,
        "closed"
    );

    gh.comment("foodshare-app", 1, "a comment").await.unwrap();
    let cs = gh.issue_comments("foodshare-app", 1).await.unwrap();
    assert_eq!(cs.len(), 1);
    assert_eq!(cs[0].body, "a comment");

    assert!(gh.get_issue("foodshare-app", 99).await.is_err());
}

#[tokio::test]
async fn fake_github_milestones_and_fields() {
    let mut gh = FakeGitHub::default();
    let m = gh
        .create_milestone("foodshare-app", "Next.js 16 + Bun 1.4.4")
        .await
        .unwrap();
    assert_eq!(m.number, 1);
    assert_eq!(m.title, "Next.js 16 + Bun 1.4.4");
    let ms = gh.list_milestones("foodshare-app").await.unwrap();
    assert_eq!(ms.len(), 1);

    gh.set_milestone("foodshare-app", 1, 1).await.unwrap();
    gh.set_labels(
        "foodshare-app",
        1,
        &["ghsync".into(), "ghsync-foodshare-app".into()],
    )
    .await
    .unwrap();
    assert_eq!(
        gh.milestone_set.get(&("foodshare-app".to_string(), 1)),
        Some(&1)
    );
    assert_eq!(
        gh.labels_set
            .get(&("foodshare-app".to_string(), 1))
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn fake_github_graphql_queue_surfaces_errors() {
    let gh = FakeGitHub::default();
    let mut gh = with_graphql(
        gh,
        &[
            "{\"data\":{\"ok\":true}}",
            "{\"errors\":[{\"message\":\"nope\"}]}",
        ],
    );
    let ok = gh
        .graphql("query { x }", serde_json::json!({"a": 1}))
        .await
        .unwrap();
    // The graphql seam returns the `data` object directly.
    assert_eq!(ok["ok"], true);
    let err = gh
        .graphql("query { y }", serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("nope"));
    assert_eq!(gh.graphql_calls.len(), 2);
    assert_eq!(gh.graphql_calls[0].0, "query { x }");
    assert_eq!(gh.graphql_calls[0].1, serde_json::json!({"a": 1}));
}

fn with_graphql(gh: FakeGitHub, responses: &[&str]) -> FakeGitHub {
    let mut gh = gh;
    for r in responses {
        gh.graphql_responses.push(serde_json::from_str(r).unwrap());
    }
    gh
}

// ---------- REST path builders ----------

#[test]
fn rest_paths_are_exact() {
    assert_eq!(
        path_repo("Foodshareclub", "foodshare-app"),
        "/repos/Foodshareclub/foodshare-app"
    );
    assert_eq!(
        path_issue("Foodshareclub", "foodshare-app", 41),
        "/repos/Foodshareclub/foodshare-app/issues/41"
    );
    assert_eq!(
        path_issue_comments("Foodshareclub", "foodshare-app", 41, 2),
        "/repos/Foodshareclub/foodshare-app/issues/41/comments?per_page=100&page=2"
    );
    assert_eq!(
        path_milestones("Foodshareclub", "foodshare-app"),
        "/repos/Foodshareclub/foodshare-app/milestones?state=all&per_page=100"
    );
    assert_eq!(
        path_pull("Foodshareclub", "foodshare-tools", 42),
        "/repos/Foodshareclub/foodshare-tools/pulls/42"
    );
    assert_eq!(
        path_check_runs("Foodshareclub", "foodshare-tools", "abc123"),
        "/repos/Foodshareclub/foodshare-tools/commits/abc123/check-runs?per_page=100"
    );
    assert_eq!(
        path_labels("Foodshareclub", "foodshare-app"),
        "/repos/Foodshareclub/foodshare-app/labels"
    );
    assert_eq!(
        path_discussion_category("Foodshareclub", "foodshare-tools", "DC_id42"),
        "/repos/Foodshareclub/foodshare-tools/discussions/categories/DC_id42"
    );
}

// ---------- RealKanban plumbing ----------

fn write_fake_hermes(
    dir: &std::path::Path,
    json: &str,
    args_file: &std::path::Path,
) -> std::path::PathBuf {
    let json_file = dir.join("out.json");
    std::fs::write(&json_file, json).unwrap();
    let bin = dir.join("fake-hermes");
    std::fs::write(
        &bin,
        format!(
            "#!/bin/sh\necho \"$@\" > {}\ncat {}\n",
            args_file.display(),
            json_file.display()
        ),
    )
    .unwrap();
    let mut p = std::fs::metadata(&bin).unwrap().permissions();
    p.set_mode(0o755);
    std::fs::set_permissions(&bin, p).unwrap();
    bin
}

#[tokio::test]
async fn real_kanban_shells_to_cli_with_board_flag_first() {
    let dir = tempfile::tempdir().unwrap();
    let args_file = dir.path().join("args.txt");
    let bin = write_fake_hermes(
        dir.path(),
        r#"[{"id":"t_abc","title":"A","body":"b","assignee":"default","status":"ready","priority":0,"completion_contract":"local-only"}]"#,
        &args_file,
    );
    let mut kb = RealKanban::with_marker("foodshare", None);
    kb.bin = bin;

    let cards = kb.list().await.unwrap();
    assert_eq!(cards[0].id, "t_abc");
    assert_eq!(cards[0].status, "ready");

    let args = std::fs::read_to_string(args_file).unwrap();
    // The board flag goes BEFORE the subcommand; --json is present.
    assert!(
        args.starts_with("kanban --board foodshare list"),
        "argv was: {args}"
    );
    assert!(args.contains("--json"), "argv was: {args}");
}

#[tokio::test]
async fn real_kanban_create_parses_task_json() {
    let dir = tempfile::tempdir().unwrap();
    let args_file = dir.path().join("args.txt");
    let bin = write_fake_hermes(
        dir.path(),
        r#"{"id":"t_new","title":"New card","body":"b","assignee":"default","status":"ready","priority":0,"completion_contract":"local-only"}"#,
        &args_file,
    );
    let mut kb = RealKanban::with_marker("foodshare", None);
    kb.bin = bin;

    let card = kb
        .create("New card", "b", "default", "key", "ready")
        .await
        .unwrap();
    assert_eq!(card.id, "t_new");

    let args = std::fs::read_to_string(&args_file).unwrap();
    assert!(
        args.starts_with("kanban --board foodshare create"),
        "argv was: {args}"
    );
    assert!(args.contains("--idempotency-key"), "argv was: {args}");
    // Q1: intake cards default to `ready`, which is the board default —
    // the CLI has no `--initial-status ready`, so ready omits the flag.
    assert!(
        !args.contains("--initial-status"),
        "ready must omit --initial-status; argv was: {args}"
    );

    // A non-default initial status is passed through.
    kb.create("New card", "b", "default", "key2", "running")
        .await
        .unwrap();
    let args = std::fs::read_to_string(&args_file).unwrap();
    assert!(
        args.contains("--initial-status running"),
        "argv was: {args}"
    );
}

#[tokio::test]
async fn real_kanban_maps_cli_failure_to_error() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("failing-hermes");
    std::fs::write(&bin, "#!/bin/sh\necho 'boom' >&2\nexit 2\n").unwrap();
    let mut p = std::fs::metadata(&bin).unwrap().permissions();
    p.set_mode(0o755);
    std::fs::set_permissions(&bin, p).unwrap();
    let mut kb = RealKanban::with_marker("foodshare", None);
    kb.bin = bin;

    let err = kb.list().await.unwrap_err();
    match err {
        KanbanError::Command { code, stderr, .. } => {
            assert_eq!(code, 2);
            assert!(stderr.contains("boom"));
        }
        other => panic!("expected Command error, got {other:?}"),
    }
}

#[tokio::test]
async fn real_kanban_refuses_in_fenced_context() {
    // A delegate_task child (marker set) must fail fast, before spawning.
    let mut kb = RealKanban::with_marker("foodshare", Some("/Users/x/.hermes"));
    assert!(matches!(
        kb.list().await.unwrap_err(),
        KanbanError::Fenced { .. }
    ));
    assert!(matches!(
        kb.create("t", "b", "default", "k", "ready")
            .await
            .unwrap_err(),
        KanbanError::Fenced { .. }
    ));
}

// ---------- helpers ----------

#[test]
fn links_from_comments_parses_only_link_lines() {
    let comments = vec![
        "some chatter".to_string(),
        "ghsync Foodshareclub/foodshare-app#41".to_string(),
        "ghsync Foodshareclub/foodshare-backend#7".to_string(),
        "ghsync mentioned in passing 41".to_string(),
    ];
    let links = links_from_comments(&comments);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].repo, "foodshare-app");
    assert_eq!(links[1].repo, "foodshare-backend");
    assert!(
        !HashMap::<String, Vec<ghsync::kanban::LinkedIssue>>::new().contains_key("x"),
        "sanity: HashMap usable"
    );
}

#[test]
fn card_derives_status_and_phase() {
    let card = Card {
        id: "t_1".into(),
        title: "t".into(),
        body: String::new(),
        assignee: Some("default".into()),
        status: "ready".into(),
        priority: 0,
        completion_contract: None,
        links: vec![],
        labels: vec![],
    };
    assert_eq!(card.status_enum(), Some(ghsync::config::Status::Ready));
    assert_eq!(card.phase(), "in-progress");
}
