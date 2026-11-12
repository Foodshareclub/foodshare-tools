//! `ghsync setup`: idempotent org provisioning (plan T8).
//!
//! Every step is find-or-create, so a second run performs zero writes.
//! Steps: (1) verify the token; (2) per-repo labels; (3) the org Project
//! v2 board; (4) the project's fields and single-select options,
//! recording their ids; (5) per-repo default milestones; (6) discussions
//! report + the Handoffs category; (7) project grid views. The CLI then
//! writes the discovered ids back into the config
//! (`apply_report_to_config` + `Config::save`), preserving every key.
//!
//! The live run needs the org PAT (T0) and is blocked on it; the unit
//! tests drive this module through the fake with queued GraphQL
//! responses, and the GraphQL strings are snapshot-pinned in `github.rs`.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::config::Config;
use crate::github::{
    GHSYNC_CREATE_CATEGORY_MUTATION, GHSYNC_CREATE_KANBAN_VIEW_MUTATION,
    GHSYNC_CREATE_OPTION_MUTATION, GHSYNC_CREATE_PHASES_VIEW_MUTATION,
    GHSYNC_CREATE_PROJECT_MUTATION, GHSYNC_CREATE_SINGLE_SELECT_MUTATION,
    GHSYNC_CREATE_TEXT_FIELD_MUTATION, GHSYNC_DISCUSSIONS_QUERY, GHSYNC_FIELDS_QUERY,
    GHSYNC_LIST_PROJECTS_QUERY, GHSYNC_VIEWS_QUERY, GitHub, GitHubError,
};

/// The title of the org Project v2 board.
pub const PROJECT_TITLE: &str = "Foodshare Board";
/// The grid view covering the whole board, grouped by Status.
pub const VIEW_BOARD: &str = "Board";
/// The grid view grouped by Phase.
pub const VIEW_PHASES: &str = "Phases";
/// The discussion category that receives ghsync feed posts.
pub const HANDOFFS_CATEGORY: &str = "Handoffs";
/// Label color (decorative).
const LABEL_COLOR: &str = "0ea4a5";

/// A setup failure.
#[derive(Debug, Error)]
pub enum SetupError {
    /// The token was rejected (HTTP 401) — the PAT is missing or wrong.
    #[error(
        "ghsync: token rejected by GitHub (HTTP {status}): {message} — check the PAT in the config"
    )]
    Auth { status: u16, message: String },
    #[error(transparent)]
    GitHub(#[from] GitHubError),
}

/// What setup found/created; the ids are written back into the config.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SetupReport {
    /// The GitHub login the token authenticated as.
    pub user: String,
    /// (repo, labels created) per repo, in config order.
    pub labels_created: Vec<(String, u32)>,
    /// The project's GraphQL node id (e.g. `PV1_…`).
    pub project_id: String,
    /// True when the project itself was created this run.
    pub project_created: bool,
    /// The Status field id.
    pub status_field: String,
    /// The Assignee field id.
    pub assignee_field: String,
    /// The Phase field id.
    pub phase_field: String,
    /// Status option label -> option id.
    pub status_option_ids: BTreeMap<String, String>,
    /// Phase option label -> option id.
    pub phase_option_ids: BTreeMap<String, String>,
    /// (repo, milestones created) per repo, in config order.
    pub milestones_created: Vec<(String, u32)>,
    /// (repo, human-readable state line) per repo, in config order.
    pub discussions: Vec<(String, String)>,
    /// (repo, discussions enabled, labels supported) per repo, in config
    /// order — written back into the config by the CLI.
    pub discussion_flags: Vec<(String, bool, bool)>,
    /// Names of the views created this run.
    pub views_created: Vec<String>,
}

/// Run every provisioning step against `gh` for `cfg`.
pub async fn run_setup(gh: &mut impl GitHub, cfg: &Config) -> Result<SetupReport, SetupError> {
    let mut report = SetupReport::default();

    // 1. The token.
    let user = gh.get_user().await.map_err(|e| match e {
        GitHubError::Http {
            status, message, ..
        } if status == 401 => SetupError::Auth { status, message },
        other => SetupError::GitHub(other),
    })?;
    report.user = user;

    // 2. Per-repo labels.
    for repo in &cfg.repos {
        let wanted = [
            "ghsync".to_string(),
            format!("ghsync-{}", repo.name),
            "intake".to_string(),
        ];
        let have = gh.list_labels(&repo.name).await?;
        let mut created = 0;
        for name in &wanted {
            if !have.iter().any(|h| h == name) {
                gh.create_label(&repo.name, name, LABEL_COLOR).await?;
                created += 1;
            }
        }
        report.labels_created.push((repo.name.clone(), created));
    }

    // 3. The project (found by title: no projectV2ByName in the GraphQL
    // schema — page through the org's boards, of which there are few).
    let mut project_id: Option<String> = None;
    let mut cursor: Option<String> = None;
    while project_id.is_none() {
        let data = gh
            .graphql(
                GHSYNC_LIST_PROJECTS_QUERY,
                serde_json::json!({ "org": cfg.org, "cursor": cursor }),
            )
            .await?;
        let pv = data
            .pointer("/organization/projectsV2")
            .ok_or_else(|| GitHubError::Io("projectsV2: missing from response".into()))?;
        let nodes = pv
            .pointer("/nodes")
            .and_then(|n| n.as_array())
            .cloned()
            .unwrap_or_default();
        for node in &nodes {
            if node.pointer("/title").and_then(|t| t.as_str()) == Some(PROJECT_TITLE) {
                project_id = node
                    .pointer("/id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                break;
            }
        }
        let more = pv
            .pointer("/pageInfo/hasNextPage")
            .and_then(|h| h.as_bool())
            .unwrap_or(false);
        let next = pv
            .pointer("/pageInfo/endCursor")
            .and_then(|c| c.as_str())
            .map(str::to_string);
        if !more || next.is_none() {
            break;
        }
        cursor = next;
    }
    let project_id = match project_id {
        Some(id) => id.to_string(),
        None => {
            let data = gh
                .graphql(
                    GHSYNC_CREATE_PROJECT_MUTATION,
                    serde_json::json!({
                        "input": {
                            "owner": cfg.org,
                            "title": PROJECT_TITLE,
                            "contentKind": "ORGANIZATION"
                        }
                    }),
                )
                .await?;
            report.project_created = true;
            data.pointer("/createProjectV2/project/id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    GitHubError::Io("createProjectV2: response missing project id".into())
                })?
        }
    };
    report.project_id = project_id.clone();

    // 4. Fields + options.
    let data = gh
        .graphql(
            GHSYNC_FIELDS_QUERY,
            serde_json::json!({ "projectId": project_id }),
        )
        .await?;
    let mut existing: BTreeMap<String, (String, Vec<(String, String)>)> = BTreeMap::new();
    if let Some(nodes) = data
        .pointer("/node/fields/nodes")
        .and_then(|n| n.as_array())
    {
        for n in nodes {
            let id = n
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let name = n
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let options = n
                .get("options")
                .and_then(|o| o.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|o| {
                            Some((
                                o.get("id")?.as_str()?.to_string(),
                                o.get("name")?.as_str()?.to_string(),
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default();
            existing.insert(name, (id, options));
        }
    }

    let proj = cfg.project.as_ref();
    let status_keys: Vec<String> = proj
        .map(|p| p.status_options.keys().cloned().collect())
        .unwrap_or_default();
    let phase_keys: Vec<String> = proj
        .map(|p| p.phase_options.keys().cloned().collect())
        .unwrap_or_default();

    let (status_field, status_option_ids) = ensure_single_select(
        gh,
        &project_id,
        "Status",
        &status_keys,
        existing.remove("Status"),
    )
    .await?;
    report.status_field = status_field.clone();
    report.status_option_ids = status_option_ids;

    let assignee_field = match existing.remove("Assignee") {
        Some((id, _)) => id,
        None => {
            let data = gh
                .graphql(
                    GHSYNC_CREATE_TEXT_FIELD_MUTATION,
                    serde_json::json!({ "projectId": project_id, "name": "Assignee" }),
                )
                .await?;
            data.pointer("/createProjectV2Field/field/id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    GitHubError::Io("createProjectV2Field(Assignee): missing id".into())
                })?
        }
    };
    report.assignee_field = assignee_field;

    let (phase_field, phase_option_ids) = ensure_single_select(
        gh,
        &project_id,
        "Phase",
        &phase_keys,
        existing.remove("Phase"),
    )
    .await?;
    report.phase_field = phase_field;
    report.phase_option_ids = phase_option_ids;

    // 5. Per-repo default milestones.
    for repo in &cfg.repos {
        let ms = gh.list_milestones(&repo.name).await?;
        if !ms.iter().any(|m| m.title == repo.default_milestone) {
            gh.create_milestone(&repo.name, &repo.default_milestone)
                .await?;
            report.milestones_created.push((repo.name.clone(), 1));
        } else {
            report.milestones_created.push((repo.name.clone(), 0));
        }
    }

    // 6. Discussions: report state, ensure the Handoffs category.
    for repo in &cfg.repos {
        let data = gh
            .graphql(
                GHSYNC_DISCUSSIONS_QUERY,
                serde_json::json!({ "owner": cfg.org, "name": &repo.name }),
            )
            .await?;
        let enabled = data
            .pointer("/repository/hasDiscussions")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !enabled {
            report.discussions.push((
                repo.name.clone(),
                "disabled — discussion intake inert".into(),
            ));
            report
                .discussion_flags
                .push((repo.name.clone(), false, false));
            continue;
        }
        let repo_id = data
            .pointer("/repository/id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let categories = data
            .pointer("/repository/discussionCategories/nodes")
            .and_then(|n| n.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.get("name").and_then(|v| v.as_str()))
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if categories.iter().any(|c| c == HANDOFFS_CATEGORY) {
            report.discussions.push((
                repo.name.clone(),
                format!("enabled — {HANDOFFS_CATEGORY} present"),
            ));
        } else {
            // GraphQL only creates fixed-enum categories; create OTHER and
            // rename it via REST.
            let data = gh
                .graphql(
                    GHSYNC_CREATE_CATEGORY_MUTATION,
                    serde_json::json!({ "repositoryId": repo_id }),
                )
                .await?;
            let category_id = data
                .pointer("/addDiscussionCategory/discussionCategory/id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| GitHubError::Io("addDiscussionCategory: missing id".into()))?;
            gh.rename_discussion_category(&repo.name, &category_id, HANDOFFS_CATEGORY)
                .await?;
            report.discussions.push((
                repo.name.clone(),
                format!("enabled — {HANDOFFS_CATEGORY} created"),
            ));
        }
        // Probe label support (404 = disabled) and record the flags; the
        // feed/intake steps gate on them.
        let labels_ok = gh.discussion_labels_supported(&repo.name).await?;
        report
            .discussion_flags
            .push((repo.name.clone(), true, labels_ok));
    }

    // 7. Project grid views.
    let data = gh
        .graphql(
            GHSYNC_VIEWS_QUERY,
            serde_json::json!({ "projectId": project_id }),
        )
        .await?;
    let views: Vec<String> = data
        .pointer("/node/views/nodes")
        .and_then(|n| n.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("name").and_then(|x| x.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if !views.iter().any(|v| v == VIEW_BOARD) {
        gh.graphql(
            GHSYNC_CREATE_KANBAN_VIEW_MUTATION,
            serde_json::json!({ "projectId": project_id, "name": VIEW_BOARD }),
        )
        .await?;
        report.views_created.push(VIEW_BOARD.into());
    }
    if !views.iter().any(|v| v == VIEW_PHASES) {
        gh.graphql(
            GHSYNC_CREATE_PHASES_VIEW_MUTATION,
            serde_json::json!({
                "projectId": project_id,
                "name": VIEW_PHASES,
                "fieldId": report.phase_field.as_str(),
            }),
        )
        .await?;
        report.views_created.push(VIEW_PHASES.into());
    }

    Ok(report)
}

/// Ensure a single-select field exists with all the wanted options;
/// returns (field id, option label -> option id).
async fn ensure_single_select(
    gh: &mut impl GitHub,
    project_id: &str,
    name: &str,
    option_names: &[String],
    existing: Option<(String, Vec<(String, String)>)>,
) -> Result<(String, BTreeMap<String, String>), GitHubError> {
    let (field_id, existing_options) = match existing {
        Some((id, opts)) => (id, opts),
        None => {
            let data = gh
                .graphql(
                    GHSYNC_CREATE_SINGLE_SELECT_MUTATION,
                    serde_json::json!({ "projectId": project_id, "name": name }),
                )
                .await?;
            let id = data
                .pointer("/createProjectV2Field/field/id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    GitHubError::Io(format!("createProjectV2Field({name}): missing id"))
                })?;
            (id, Vec::new())
        }
    };
    let mut ids = BTreeMap::new();
    for want in option_names {
        if let Some((oid, _)) = existing_options.iter().find(|(_, n)| n == want) {
            ids.insert(want.clone(), oid.clone());
        } else {
            let data = gh
                .graphql(
                    GHSYNC_CREATE_OPTION_MUTATION,
                    serde_json::json!({ "fieldId": field_id, "name": want }),
                )
                .await?;
            let oid = data
                .pointer("/createProjectV2FieldSingleSelectOption/option/id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    GitHubError::Io(format!(
                        "createProjectV2FieldSingleSelectOption({want}): missing id"
                    ))
                })?;
            ids.insert(want.clone(), oid);
        }
    }
    Ok((field_id, ids))
}

/// Copy the discovered ids into the config so the next `Config::load`
/// sees them (the caller saves the config afterwards).
pub fn apply_report_to_config(cfg: &mut Config, report: &SetupReport) {
    if let Some(p) = cfg.project.as_mut() {
        p.project_id = report.project_id.clone();
        p.status_field = report.status_field.clone();
        p.assignee_field = report.assignee_field.clone();
        p.phase_field = report.phase_field.clone();
        p.status_option_ids = report.status_option_ids.clone();
        p.phase_option_ids = report.phase_option_ids.clone();
    }
    for (repo, enabled, labels) in &report.discussion_flags {
        if let Some(r) = cfg.repos.iter_mut().find(|r| &r.name == repo) {
            r.discussions_enabled = *enabled;
            r.discussion_labels_enabled = *labels;
        }
    }
}
