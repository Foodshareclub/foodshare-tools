//! Configuration and the pure mappings between the two sides.
//!
//! Everything here is a constant or a pure derivation: the kanban status
//! table, the phase rule, the label set, the link-comment grammar, and the
//! config defaults. The only I/O is [`Config::load`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A kanban status as ghsync sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Todo,
    /// A card in the triage column (unspecified; a specifier fleshes it
    /// out). Projects as Backlog; it is not work in flight.
    Triage,
    Ready,
    Running,
    Blocked,
    Review,
    Done,
    /// A card parked in Scheduled (waiting on a time, not a human).
    /// Projects as Backlog; it is not work in flight.
    Scheduled,
    /// Terminal history. Projects as Done and its issue is closed.
    Archived,
}

impl Status {
    /// Parse a status string from `hermes kanban list --json`.
    pub fn from_kanban(raw: &str) -> Option<Self> {
        Some(match raw {
            "todo" => Self::Todo,
            "triage" => Self::Triage,
            "ready" => Self::Ready,
            "running" => Self::Running,
            "blocked" => Self::Blocked,
            "review" => Self::Review,
            "done" => Self::Done,
            "scheduled" => Self::Scheduled,
            "archived" => Self::Archived,
            _ => return None,
        })
    }

    /// The kebab-case status string, as used in the config file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Triage => "triage",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Blocked => "blocked",
            Self::Review => "review",
            Self::Done => "done",
            Self::Scheduled => "scheduled",
            Self::Archived => "archived",
        }
    }

    /// The Project v2 `Status` option for this status.
    pub fn project_status(self) -> &'static str {
        match self {
            Self::Todo | Self::Triage | Self::Ready | Self::Scheduled => "Backlog",
            Self::Running | Self::Review => "In progress",
            Self::Blocked => "Blocked",
            Self::Done | Self::Archived => "Done",
        }
    }

    /// Whether the linked issue must be open or closed for this status.
    pub fn issue_state(self) -> IssueState {
        match self {
            Self::Done | Self::Archived => IssueState::Closed,
            _ => IssueState::Open,
        }
    }
}

/// Whether the linked issue must be open or closed for a status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed,
}

/// Parse a kanban status string.
pub fn status_from_kanban(raw: &str) -> Option<Status> {
    Status::from_kanban(raw)
}

/// Whether the linked issue must be open or closed for a status.
pub fn issue_state_from_kanban(status: Status) -> IssueState {
    status.issue_state()
}

/// The `Phase` for a card: `in-progress` only for a claimed ready card
/// (ready + an assignee), `backlog` for everything else.
pub fn phase_from_card(status: Status, assignee: Option<&str>) -> &'static str {
    if status == Status::Ready && assignee.is_some() {
        "in-progress"
    } else {
        "backlog"
    }
}

/// The label every linked issue carries.
pub const GHSYNC_LABEL: &str = "ghsync";

/// The repo-scoped label: `ghsync-<repo>`.
pub fn repo_label(repo: &str) -> String {
    format!("ghsync-{repo}")
}

/// The full label set ghsync manages on a linked issue.
///
/// `intake` is deliberately not here: only the intake form's
/// `apply_labels` carries it, and the intake step removes it.
pub fn labels_for(repo: &str) -> Vec<String> {
    vec![GHSYNC_LABEL.to_string(), repo_label(repo)]
}

/// Prefix of the issue-side link comment.
pub const CARD_LINK_PREFIX: &str = "ghsync ";

/// Prefix of the card-side link footer.
pub const CARD_FOOTER_PREFIX: &str = "kanban:";

/// A GitHub issue reference: `owner/repo#number`.
pub use crate::kanban::LinkedIssue;

/// Alias kept for the mapping helpers in this module.
pub type IssueRef = LinkedIssue;

/// The issue-side link comment: `ghsync <owner>/<repo>#<n>`.
pub fn issue_link_comment(owner: &str, repo: &str, number: u32) -> String {
    format!("ghsync {owner}/{repo}#{number}")
}

/// The card-side link footer: `kanban:<board>:<task-id>`.
pub fn card_link_comment(board: &str, task_id: &str) -> String {
    format!("kanban:{board}:{task_id}")
}

/// Parse a link comment. The whole line must be the link; a line that
/// merely mentions one does not count.
pub fn parse_issue_link(line: &str) -> Option<IssueRef> {
    let rest = line.trim().strip_prefix(CARD_LINK_PREFIX)?;
    let (path, number) = rest.split_once('#')?;
    let (owner, repo) = path.split_once('/')?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    let number = number.parse().ok()?;
    Some(IssueRef {
        owner: owner.to_string(),
        repo: repo.to_string(),
        number,
    })
}

/// Parse a card footer: `(board, task_id)`.
pub fn parse_card_link(line: &str) -> Option<(String, String)> {
    let rest = line.trim().strip_prefix(CARD_FOOTER_PREFIX)?;
    let (board, task_id) = rest.split_once(':')?;
    if board.is_empty() || task_id.is_empty() {
        return None;
    }
    Some((board.to_string(), task_id.to_string()))
}

/// A repository ghsync manages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoConfig {
    pub name: String,
    pub default_milestone: String,
}

/// The org Project v2 board.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjConfig {
    pub number: u32,
    /// The GraphQL node id of the Project v2 board (e.g. `PV1_…`),
    /// discovered by `ghsync setup` and written back into the config.
    /// Empty until setup has run; pushes report `setup-required` drift.
    pub project_id: String,
    pub status_field: String,
    pub assignee_field: String,
    pub phase_field: String,
    /// Project option label -> kanban status it maps to.
    pub status_options: BTreeMap<String, Status>,
    /// Project option label -> phase slug.
    pub phase_options: BTreeMap<String, String>,
    /// Project option label -> assignee profile name.
    pub assignee_options: BTreeMap<String, String>,
    /// Project option label -> single-select option id, for the Status
    /// field. Discovered by `ghsync setup` and written back into the
    /// config; empty until setup has run.
    pub status_option_ids: BTreeMap<String, String>,
    /// Project option label -> single-select option id, for the Phase
    /// field. Discovered by `ghsync setup`.
    pub phase_option_ids: BTreeMap<String, String>,
}

/// Where the ghsync state (cursor) file lives.
fn default_state_path() -> PathBuf {
    crate::cli::expand_tilde(&PathBuf::from("~/.config/ghsync/state.json"))
}

/// The ghsync configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub board: String,
    pub org: String,
    /// GitHub REST/GraphQL base URL.
    pub api: String,
    pub repos: Vec<RepoConfig>,
    pub default_assignee: String,
    pub project: Option<ProjConfig>,
    /// Agent profiles that an intake form may name; an unknown profile
    /// is a loud intake error naming this list.
    pub profiles: Vec<String>,
    pub token: Option<String>,
    pub state_path: PathBuf,
}

impl Default for Config {
    /// The plan defaults.
    fn default() -> Self {
        Self {
            board: "foodshare".into(),
            org: "Foodshareclub".into(),
            api: "https://api.github.com".into(),
            repos: vec![
                RepoConfig {
                    name: "foodshare-app".into(),
                    default_milestone: "Next.js 16 + Bun 1.4.4".into(),
                },
                RepoConfig {
                    name: "foodshare-backend".into(),
                    default_milestone: "Rust".into(),
                },
                RepoConfig {
                    name: "foodshare-web".into(),
                    default_milestone: "Next.js 16 + Bun 1.4.4".into(),
                },
                RepoConfig {
                    name: "foodshare-tools".into(),
                    default_milestone: "Rust + TypeScript".into(),
                },
                RepoConfig {
                    name: "catch-22".into(),
                    default_milestone: "Next.js 16 + Bun 1.4.4".into(),
                },
            ],
            default_assignee: "default".into(),
            profiles: vec!["default".into()],
            project: Some(ProjConfig {
                number: 1,
                project_id: String::new(),
                status_field: "Status".into(),
                assignee_field: "Assignee".into(),
                phase_field: "Phase".into(),
                status_options: [
                    ("Backlog", Status::Todo),
                    ("In progress", Status::Running),
                    ("Done", Status::Done),
                    ("Blocked", Status::Blocked),
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
                phase_options: [
                    ("Backlog", "backlog"),
                    ("In progress", "in-progress"),
                    ("Done", "done"),
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
                assignee_options: [
                    ("agent-default", "default"),
                    ("agent-security", "security"),
                    ("human", "human"),
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
                status_option_ids: BTreeMap::new(),
                phase_option_ids: BTreeMap::new(),
            }),
            token: None,
            state_path: default_state_path(),
        }
    }
}

impl Config {
    /// The repo entry for `name`, if configured.
    pub fn repo(&self, name: &str) -> Option<&RepoConfig> {
        self.repos.iter().find(|r| r.name == name)
    }

    /// Load the TOML config at `path`; missing keys fall back to the
    /// defaults above.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text =
            std::fs::read_to_string(path).map_err(|_| ConfigError::NotFound(path.to_path_buf()))?;
        let raw: Raw = toml::from_str(&text).map_err(|e| ConfigError::Parse(e.to_string()))?;
        Self::from_raw(raw)
    }

    /// Write the config back to `path`, preserving every key (including
    /// the token). Atomic like the cursor: temp file + rename.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let text = toml::to_string_pretty(&Raw::from(self))
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        if let Some(dir) = path.parent()
            && !dir.as_os_str().is_empty()
        {
            std::fs::create_dir_all(dir).map_err(|e| ConfigError::Parse(e.to_string()))?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &text).map_err(|e| ConfigError::Parse(e.to_string()))?;
        std::fs::rename(&tmp, path).map_err(|e| ConfigError::Parse(e.to_string()))?;
        Ok(())
    }
}

/// Failure loading the config file.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config not found at {0}; copy ghsync.example.toml there and fill in the token")]
    NotFound(PathBuf),
    #[error("config parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct Raw {
    #[serde(skip_serializing_if = "Option::is_none")]
    board: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    org: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api: Option<String>,
    repos: Vec<RawRepo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_assignee: Option<String>,
    profiles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<RawProj>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct RawRepo {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_milestone: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct RawProj {
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assignee_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phase_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_options: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phase_options: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assignee_options: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_option_ids: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phase_option_ids: Option<BTreeMap<String, String>>,
}

impl From<&Config> for Raw {
    fn from(c: &Config) -> Self {
        Self {
            board: Some(c.board.clone()),
            org: Some(c.org.clone()),
            api: Some(c.api.clone()),
            repos: c
                .repos
                .iter()
                .map(|r| RawRepo {
                    name: Some(r.name.clone()),
                    default_milestone: Some(r.default_milestone.clone()),
                })
                .collect(),
            default_assignee: Some(c.default_assignee.clone()),
            profiles: c.profiles.clone(),
            project: c.project.as_ref().map(|p| RawProj {
                number: Some(p.number),
                project_id: Some(p.project_id.clone()),
                status_field: Some(p.status_field.clone()),
                assignee_field: Some(p.assignee_field.clone()),
                phase_field: Some(p.phase_field.clone()),
                status_options: Some(
                    p.status_options
                        .iter()
                        .map(|(k, v)| (k.clone(), v.as_str().to_string()))
                        .collect(),
                ),
                phase_options: Some(p.phase_options.clone()),
                assignee_options: Some(p.assignee_options.clone()),
                status_option_ids: Some(p.status_option_ids.clone()),
                phase_option_ids: Some(p.phase_option_ids.clone()),
            }),
            token: c.token.clone(),
            state_path: Some(c.state_path.clone()),
        }
    }
}

impl Config {
    fn from_raw(raw: Raw) -> Result<Self, ConfigError> {
        let d = Self::default();
        let project = match raw.project {
            Some(p) => {
                let dp = d.project.as_ref().expect("default project");
                let status_options = match p.status_options {
                    Some(options) => {
                        let mut map = BTreeMap::new();
                        for (label, status) in options {
                            let status = Status::from_kanban(&status).ok_or_else(|| {
                                ConfigError::Parse(format!(
                                    "unknown kanban status `{status}` for project option `{label}`"
                                ))
                            })?;
                            map.insert(label, status);
                        }
                        map
                    }
                    None => dp.status_options.clone(),
                };
                Some(ProjConfig {
                    number: p.number.unwrap_or(dp.number),
                    project_id: p.project_id.unwrap_or(dp.project_id.clone()),
                    status_field: p.status_field.unwrap_or(dp.status_field.clone()),
                    assignee_field: p.assignee_field.unwrap_or(dp.assignee_field.clone()),
                    phase_field: p.phase_field.unwrap_or(dp.phase_field.clone()),
                    status_options,
                    phase_options: p.phase_options.unwrap_or_else(|| dp.phase_options.clone()),
                    assignee_options: p
                        .assignee_options
                        .unwrap_or_else(|| dp.assignee_options.clone()),
                    status_option_ids: p.status_option_ids.unwrap_or_default(),
                    phase_option_ids: p.phase_option_ids.unwrap_or_default(),
                })
            }
            None => d.project.clone(),
        };
        Ok(Self {
            board: raw.board.unwrap_or(d.board),
            org: raw.org.unwrap_or(d.org),
            api: raw.api.unwrap_or(d.api),
            repos: if raw.repos.is_empty() {
                d.repos
            } else {
                raw.repos
                    .into_iter()
                    .map(|r| {
                        Ok(RepoConfig {
                            name: r.name.ok_or_else(|| {
                                ConfigError::Parse("every repo entry needs a `name`".into())
                            })?,
                            default_milestone: r.default_milestone.unwrap_or_default(),
                        })
                    })
                    .collect::<Result<Vec<RepoConfig>, ConfigError>>()?
            },
            default_assignee: raw.default_assignee.unwrap_or(d.default_assignee),
            profiles: if raw.profiles.is_empty() {
                d.profiles
            } else {
                raw.profiles
            },
            project,
            token: raw.token,
            state_path: raw
                .state_path
                .map(|p| crate::cli::expand_tilde(&p))
                .unwrap_or(d.state_path),
        })
    }
}
