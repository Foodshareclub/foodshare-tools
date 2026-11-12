//! T2 — the pure config/mapping logic: the status table, phase rule,
//! labels, link-comment grammar and config defaults.
//!
//! These constants and derivations are what the whole tool hangs off, so
//! they get integration tests against the public API.

use std::path::Path;

use ghsync::config::{
    Config, IssueState, Status, issue_link_comment, issue_state_from_kanban, labels_for,
    parse_card_link, parse_issue_link, phase_from_card, status_from_kanban,
};

#[test]
fn status_round_trip() {
    let table = [
        ("todo", Status::Todo, "Backlog", IssueState::Open),
        ("ready", Status::Ready, "Backlog", IssueState::Open),
        ("running", Status::Running, "In progress", IssueState::Open),
        ("blocked", Status::Blocked, "Blocked", IssueState::Open),
        ("review", Status::Review, "In progress", IssueState::Open),
        ("done", Status::Done, "Done", IssueState::Closed),
    ];
    for (raw, status, project, issue) in table {
        assert_eq!(status_from_kanban(raw), Some(status), "parse {raw}");
        assert_eq!(Status::from_kanban(raw), Some(status), "inherent {raw}");
        assert_eq!(status.project_status(), project, "{raw}");
        assert_eq!(issue_state_from_kanban(status), issue, "{raw}");
        assert_eq!(status.issue_state(), issue, "{raw}");
    }
    assert_eq!(status_from_kanban("not-a-status"), None);
}

#[test]
fn issue_link_round_trip() {
    let s = issue_link_comment("Foodshareclub", "foodshare-app", 41);
    assert_eq!(s, "ghsync Foodshareclub/foodshare-app#41");
    let r = parse_issue_link(&s).unwrap();
    assert_eq!(r.owner, "Foodshareclub");
    assert_eq!(r.repo, "foodshare-app");
    assert_eq!(r.number, 41);
}

#[test]
fn non_link_comments_parse_to_none() {
    // A link is the whole line, nothing more.
    assert!(parse_issue_link("ghsync Foodshareclub/foodshare-app#41 and more").is_none());
    assert!(parse_issue_link("ghsync someone/said#abc").is_none());
    assert!(parse_issue_link("regular comment").is_none());
    assert!(parse_issue_link("kanban:foodshare:t_1").is_none());
    assert!(parse_card_link("ghsync Foodshareclub/foodshare-app#41").is_none());
}

#[test]
fn card_link_round_trip() {
    let s = ghsync::config::card_link_comment("foodshare", "t_849a21bb");
    assert_eq!(s, "kanban:foodshare:t_849a21bb");
    let (board, task_id) = parse_card_link(&s).unwrap();
    assert_eq!(board, "foodshare");
    assert_eq!(task_id, "t_849a21bb");
}

#[test]
fn labels_include_ghsync_and_repo() {
    assert_eq!(
        labels_for("foodshare-app"),
        vec!["ghsync", "ghsync-foodshare-app"]
    );
    // `intake` is never produced here: only the form's `apply_labels`
    // carries it.
    assert!(!labels_for("foodshare-app").contains(&"intake".to_string()));
}

#[test]
fn phase_is_backlog_by_default_and_in_progress_when_claimed_ready() {
    assert_eq!(phase_from_card(Status::Ready, None), "backlog");
    assert_eq!(phase_from_card(Status::Todo, Some("default")), "backlog");
    assert_eq!(
        phase_from_card(Status::Ready, Some("default")),
        "in-progress"
    );
    assert_eq!(phase_from_card(Status::Running, Some("default")), "backlog");
    assert_eq!(phase_from_card(Status::Blocked, Some("default")), "backlog");
}

#[test]
fn default_config_matches_plan() {
    let c = Config::default();
    assert_eq!(c.board, "foodshare");
    assert_eq!(c.org, "Foodshareclub");
    assert_eq!(c.api, "https://api.github.com");
    // Four org repos; catch-22 was removed (it is not a Foodshareclub
    // repo).
    assert_eq!(c.repos.len(), 4);
    assert!(c.repo("catch-22").is_none());
    assert_eq!(c.default_assignee, "default");

    // All four repos point at the org-wide monthly readiness milestone
    // (architect decision t_1cc489c3, §5 D6 — the name is month-bound;
    // the operator edits the config when the month rolls).
    let ms = "Prod Readiness — 2026-11";
    assert_eq!(c.repo("foodshare-app").unwrap().default_milestone, ms);
    assert_eq!(c.repo("foodshare-backend").unwrap().default_milestone, ms);
    assert_eq!(c.repo("foodshare-web").unwrap().default_milestone, ms);
    assert_eq!(c.repo("foodshare-tools").unwrap().default_milestone, ms);
    // The feed category defaults to "General" (setup verifies it among
    // the repo's pre-existing categories).
    assert_eq!(c.repo("foodshare-web").unwrap().feed_category, "General");
    assert!(c.repo("nope").is_none());

    let p = c.project.as_ref().unwrap();
    assert_eq!(p.number, 1);
    assert_eq!(p.status_field, "Status");
    assert_eq!(p.assignee_field, "Assignee");
    assert_eq!(p.phase_field, "Phase");
    assert_eq!(p.status_options.get("Backlog"), Some(&Status::Todo));
    assert_eq!(p.status_options.get("In progress"), Some(&Status::Running));
    assert_eq!(p.status_options.get("Done"), Some(&Status::Done));
    assert_eq!(p.status_options.get("Blocked"), Some(&Status::Blocked));
    assert_eq!(p.phase_options.get("Backlog"), Some(&"backlog".to_string()));
    assert_eq!(
        p.phase_options.get("In progress"),
        Some(&"in-progress".to_string())
    );
    assert_eq!(p.phase_options.get("Done"), Some(&"done".to_string()));
    // Six profiles: the board's agent profiles + human (architect
    // decision t_1cc489c3 — the old three-key map left architect/
    // reviewer/verifier cards unmapped, which could never be clean).
    assert_eq!(p.assignee_options.len(), 6);
    assert_eq!(
        p.assignee_options.get("agent-default"),
        Some(&"default".to_string())
    );
    assert_eq!(
        p.assignee_options.get("agent-architect"),
        Some(&"architect".to_string())
    );
    assert_eq!(
        p.assignee_options.get("agent-reviewer"),
        Some(&"reviewer".to_string())
    );
    assert_eq!(
        p.assignee_options.get("agent-security"),
        Some(&"security".to_string())
    );
    assert_eq!(
        p.assignee_options.get("agent-verifier"),
        Some(&"verifier".to_string())
    );
    assert_eq!(p.assignee_options.get("human"), Some(&"human".to_string()));
}

#[test]
fn load_partial_file_falls_back_to_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
board = "other"
repos = [ { name = "solo", default_milestone = "M1" } ]

[project]
number = 7
"#,
    )
    .unwrap();

    let c = Config::load(&path).unwrap();
    assert_eq!(c.board, "other");
    assert_eq!(c.repos.len(), 1);
    assert_eq!(c.repos[0].name, "solo");
    assert_eq!(c.repos[0].default_milestone, "M1");
    assert_eq!(c.project.as_ref().unwrap().number, 7);
    // Unset keys keep the defaults.
    assert_eq!(c.org, "Foodshareclub");
    assert_eq!(c.api, "https://api.github.com");
    assert_eq!(c.default_assignee, "default");
    assert_eq!(c.project.as_ref().unwrap().status_field, "Status");
    assert!(!c.project.as_ref().unwrap().status_options.is_empty());
    assert!(c.token.is_none());
}

#[test]
fn load_missing_file_errors() {
    let err = Config::load(Path::new("/nonexistent/ghsync/config.toml")).unwrap_err();
    assert!(
        err.to_string().contains("config not found"),
        "unexpected error: {err}"
    );
}

#[test]
fn load_rejects_unknown_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "boardard = \"typo\"\n").unwrap();
    let err = Config::load(&path).unwrap_err();
    assert!(
        err.to_string().contains("unknown field"),
        "unexpected: {err}"
    );
}

#[test]
fn triage_status_maps_to_backlog_and_open_issue() {
    // Triage is a kanban-only column (cards in triage..done are projected).
    use ghsync::config::IssueState;
    assert!(matches!(
        Status::from_kanban("triage"),
        Some(Status::Triage)
    ));
    assert_eq!(Status::Triage.project_status(), "Backlog");
    assert_eq!(Status::Triage.issue_state(), IssueState::Open);
    // The triage column has no phase of its own.
    assert_eq!(phase_from_card(Status::Triage, Some("default")), "backlog");
}

#[test]
fn load_project_option_ids() {
    // setup (T8) writes the discovered option ids back into the config.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        "[project]\n\
         number = 1\n\
         status_option_ids = { Backlog = \"opt-backlog\", \"In progress\" = \"opt-inprog\" }\n\
         phase_option_ids = { Backlog = \"ph-backlog\" }\n",
    )
    .unwrap();
    let c = Config::load(&path).unwrap();
    let p = c.project.unwrap();
    assert_eq!(
        p.status_option_ids.get("Backlog").map(String::as_str),
        Some("opt-backlog")
    );
    assert_eq!(
        p.status_option_ids.get("In progress").map(String::as_str),
        Some("opt-inprog")
    );
    assert_eq!(
        p.phase_option_ids.get("Backlog").map(String::as_str),
        Some("ph-backlog")
    );
    // Absent maps default to empty.
    assert!(!p.status_option_ids.contains_key("Done"));
    assert_eq!(p.phase_option_ids.len(), 1);
}
