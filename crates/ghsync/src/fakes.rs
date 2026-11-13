//! In-memory fakes for the two seams.
//!
//! `FakeKanban` and `FakeGitHub` stand in for the real clients in every
//! unit and integration test up to the live E2E. They mirror the real
//! behaviour: status transitions on the kanban side, numbering and
//! state on the GitHub side, and recording of every mutation so tests
//! can assert exactly what would have been sent.

use std::collections::HashMap;

use crate::github::{CheckRun, Comment, Discussion, GitHub, GitHubError, Issue, Milestone, Pull};
use crate::kanban::{Card, CardView, Kanban, KanbanError, links_from_comments};

/// In-memory kanban board.
#[derive(Default)]
pub struct FakeKanban {
    pub cards: Vec<Card>,
    pub comments: HashMap<String, Vec<String>>,
    /// Recorded `create` calls, in order.
    pub created: Vec<CreatedCard>,
    /// Recorded `block` calls: (task_id, kind, reason).
    pub blocked: Vec<(String, String, String)>,
    /// Recorded `unblock` calls: (task_id, reason).
    pub unblocked: Vec<(String, String)>,
    /// Recorded `complete` calls: (task_id, summary).
    pub completed: Vec<(String, String)>,
    next_id: u32,
}

/// A recorded card creation.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatedCard {
    pub id: String,
    pub title: String,
    pub body: String,
    pub assignee: String,
    pub idempotency_key: String,
    pub initial_status: String,
}

impl FakeKanban {
    fn find_mut(&mut self, task_id: &str) -> Option<&mut Card> {
        self.cards.iter_mut().find(|c| c.id == task_id)
    }
}

impl Kanban for FakeKanban {
    async fn list(&mut self) -> Result<Vec<Card>, KanbanError> {
        Ok(self.cards.clone())
    }

    async fn show(&mut self, task_id: &str) -> Result<CardView, KanbanError> {
        let card = self
            .cards
            .iter()
            .find(|c| c.id == task_id)
            .ok_or_else(|| KanbanError::NotFound(task_id.to_string(), "fake".to_string()))?;
        let comments = self.comments.get(task_id).cloned().unwrap_or_default();
        let mut card = card.clone();
        card.links = links_from_comments(&comments);
        Ok(CardView {
            card,
            comments,
            children: vec![],
            latest_summary: None,
        })
    }

    async fn create(
        &mut self,
        title: &str,
        body: &str,
        assignee: &str,
        idempotency_key: &str,
        initial_status: &str,
    ) -> Result<Card, KanbanError> {
        // Idempotency: a second create with the same key returns the
        // existing card.
        if let Some(existing) = self
            .created
            .iter()
            .find(|c| c.idempotency_key == idempotency_key)
        {
            return Ok(self
                .cards
                .iter()
                .find(|c| c.id == existing.id)
                .cloned()
                .unwrap_or_default());
        }
        self.next_id += 1;
        let card = Card {
            id: format!("t_fake{:04}", self.next_id),
            title: title.to_string(),
            body: body.to_string(),
            assignee: Some(assignee.to_string()),
            status: initial_status.to_string(),
            priority: 0,
            completion_contract: None,
            links: vec![],
            labels: vec![],
        };
        self.created.push(CreatedCard {
            id: card.id.clone(),
            title: card.title.clone(),
            body: card.body.clone(),
            assignee: assignee.to_string(),
            idempotency_key: idempotency_key.to_string(),
            initial_status: initial_status.to_string(),
        });
        self.cards.push(card.clone());
        Ok(card)
    }

    async fn comment(&mut self, task_id: &str, text: &str) -> Result<(), KanbanError> {
        if self.find_mut(task_id).is_none() {
            return Err(KanbanError::NotFound(
                task_id.to_string(),
                "fake".to_string(),
            ));
        }
        self.comments
            .entry(task_id.to_string())
            .or_default()
            .push(text.to_string());
        Ok(())
    }

    async fn block(&mut self, task_id: &str, kind: &str, reason: &str) -> Result<(), KanbanError> {
        let card = self
            .find_mut(task_id)
            .ok_or_else(|| KanbanError::NotFound(task_id.to_string(), "fake".to_string()))?;
        card.status = "blocked".into();
        // The CLI appends the reason as a comment; the fake mirrors that,
        // because push's unblock detection reads it back.
        self.comments
            .entry(task_id.to_string())
            .or_default()
            .push(reason.to_string());
        self.blocked
            .push((task_id.to_string(), kind.to_string(), reason.to_string()));
        Ok(())
    }

    async fn unblock(&mut self, task_id: &str, reason: &str) -> Result<(), KanbanError> {
        let card = self
            .find_mut(task_id)
            .ok_or_else(|| KanbanError::NotFound(task_id.to_string(), "fake".to_string()))?;
        card.status = "ready".into();
        self.unblocked
            .push((task_id.to_string(), reason.to_string()));
        Ok(())
    }

    async fn complete(&mut self, task_id: &str, summary: &str) -> Result<(), KanbanError> {
        let card = self
            .find_mut(task_id)
            .ok_or_else(|| KanbanError::NotFound(task_id.to_string(), "fake".to_string()))?;
        card.status = "done".into();
        self.completed
            .push((task_id.to_string(), summary.to_string()));
        Ok(())
    }
}

/// In-memory GitHub org.
#[derive(Default)]
pub struct FakeGitHub {
    pub issues: HashMap<(String, u32), Issue>,
    pub comments: HashMap<(String, u32), Vec<Comment>>,
    pub milestones: HashMap<String, Vec<Milestone>>,
    /// Recorded milestone assignments: (repo, issue) -> milestone number.
    pub milestone_set: HashMap<(String, u32), u32>,
    /// Recorded label sets: (repo, issue) -> labels.
    pub labels_set: HashMap<(String, u32), Vec<String>>,
    /// Recorded posted comments: (repo, issue, body), in order.
    pub comments_posted: Vec<(String, u32, String)>,
    /// Recorded `close_issue` calls: (repo, issue), in order.
    pub closed_calls: Vec<(String, u32)>,
    /// Recorded `set_issue_body` calls: (repo, issue) -> body.
    pub bodies_set: HashMap<(String, u32), String>,
    /// Project items: "repo#number" -> item id.
    pub project_items: HashMap<String, String>,
    /// Project field values: (item_id, field_id) -> value.
    pub project_fields: HashMap<(String, String), String>,
    /// Recorded project field reads: (item_id, field_id), in order.
    pub project_field_reads: Vec<(String, String)>,
    /// Recorded project field writes: (item_id, field_id, value input),
    /// in order.
    pub project_field_writes: Vec<(String, String, serde_json::Value)>,
    /// GraphQL calls in order: (query, variables).
    pub graphql_calls: Vec<(String, serde_json::Value)>,
    /// Queued GraphQL responses, popped one per call.
    pub graphql_responses: Vec<serde_json::Value>,
    /// Login returned by `get_user` (default "fakeuser").
    pub user_login: String,
    /// When true, `get_user` returns a 401 error.
    pub user_rejected: bool,
    /// Label names per repo.
    pub labels: HashMap<String, Vec<String>>,
    /// Recorded label creations: (repo, name), in order.
    pub created_labels: Vec<(String, String)>,
    /// Discussion state per repo: (enabled, category names). Serves both
    /// the `has_discussions` probe and the discussion-state query.
    pub discussion_state: HashMap<String, (bool, Vec<String>)>,
    /// Pull requests: (repo, number) -> Pull.
    pub pulls: HashMap<(String, u32), Pull>,
    /// Check runs keyed by commit sha.
    pub check_runs: HashMap<String, Vec<CheckRun>>,
    /// Discussions per repo.
    pub discussions: HashMap<String, Vec<Discussion>>,
    /// Recorded discussion creations: (repo, category, title), in order.
    pub created_discussions: Vec<(String, String, String)>,
    /// Recorded discussion comments: (discussion_id, body), in order.
    pub discussion_comments: Vec<(String, String)>,
    /// Recorded label removals: (repo, discussion_id, label_id), in order.
    pub removed_labels: Vec<(String, String, String)>,
    /// Discussion-label support per repo (the probe setup records).
    pub discussion_labels_supported: HashMap<String, bool>,
    next_issue_number: u32,
    next_milestone_number: u32,
    next_item_id: u32,
    next_comment_id: u64,
    next_discussion_number: u32,
    /// Category per discussion id (set by `create_discussion`).
    discussion_categories: HashMap<String, String>,
}

type Key = (String, u32);

impl FakeGitHub {
    fn key(&self, repo: &str, number: u32) -> Key {
        (repo.to_string(), number)
    }
}

impl GitHub for FakeGitHub {
    async fn create_issue(
        &mut self,
        repo: &str,
        title: &str,
        body: &str,
        labels: &[String],
    ) -> Result<Issue, GitHubError> {
        self.next_issue_number += 1;
        let issue = Issue {
            owner: "fakeorg".into(),
            repo: repo.to_string(),
            number: self.next_issue_number,
            title: title.to_string(),
            body: body.to_string(),
            node_id: format!("GH_ISSUE_{}", self.next_issue_number),
            state: "open".into(),
            labels: labels.to_vec(),
            milestone: None,
            url: format!(
                "https://github.com/fakeorg/{repo}/issues/{}",
                self.next_issue_number
            ),
        };
        self.issues
            .insert(self.key(repo, issue.number), issue.clone());
        Ok(issue)
    }

    async fn get_issue(&mut self, repo: &str, number: u32) -> Result<Issue, GitHubError> {
        self.issues
            .get(&self.key(repo, number))
            .cloned()
            .ok_or_else(|| GitHubError::Http {
                status: 404,
                url: format!("/{repo}/issues/{number}"),
                message: "not found".into(),
            })
    }

    async fn issue_comments(
        &mut self,
        repo: &str,
        number: u32,
    ) -> Result<Vec<Comment>, GitHubError> {
        Ok(self
            .comments
            .get(&self.key(repo, number))
            .cloned()
            .unwrap_or_default())
    }

    async fn create_milestone(
        &mut self,
        repo: &str,
        title: &str,
    ) -> Result<Milestone, GitHubError> {
        self.next_milestone_number += 1;
        let m = Milestone {
            number: self.next_milestone_number,
            title: title.to_string(),
            open_issues: 0,
            closed_issues: 0,
        };
        self.milestones
            .entry(repo.to_string())
            .or_default()
            .push(m.clone());
        Ok(m)
    }

    async fn list_milestones(&mut self, repo: &str) -> Result<Vec<Milestone>, GitHubError> {
        Ok(self.milestones.get(repo).cloned().unwrap_or_default())
    }

    async fn set_milestone(
        &mut self,
        repo: &str,
        number: u32,
        milestone: u32,
    ) -> Result<(), GitHubError> {
        self.milestone_set.insert(self.key(repo, number), milestone);
        if let Some(issue) = self.issues.get_mut(&self.key(repo, number)) {
            issue.milestone = self
                .milestones
                .get(repo)
                .and_then(|ms| ms.iter().find(|m| m.number == milestone))
                .map(|m| m.title.clone());
        }
        Ok(())
    }

    async fn set_labels(
        &mut self,
        repo: &str,
        number: u32,
        labels: &[String],
    ) -> Result<(), GitHubError> {
        if let Some(issue) = self.issues.get_mut(&self.key(repo, number)) {
            issue.labels = labels.to_vec();
        }
        self.labels_set
            .insert(self.key(repo, number), labels.to_vec());
        Ok(())
    }

    async fn close_issue(&mut self, repo: &str, number: u32) -> Result<(), GitHubError> {
        if let Some(issue) = self.issues.get_mut(&self.key(repo, number)) {
            issue.state = "closed".into();
        }
        self.closed_calls.push((repo.to_string(), number));
        Ok(())
    }

    async fn set_issue_body(
        &mut self,
        repo: &str,
        number: u32,
        body: &str,
    ) -> Result<(), GitHubError> {
        if let Some(issue) = self.issues.get_mut(&self.key(repo, number)) {
            issue.body = body.to_string();
        }
        self.bodies_set
            .insert(self.key(repo, number), body.to_string());
        Ok(())
    }

    async fn project_item_for_issue(
        &mut self,
        _project_id: &str,
        repo: &str,
        number: u32,
    ) -> Result<Option<String>, GitHubError> {
        Ok(self.project_items.get(&format!("{repo}#{number}")).cloned())
    }

    async fn add_project_item(
        &mut self,
        _project_id: &str,
        node_id: &str,
    ) -> Result<String, GitHubError> {
        let (repo, number) = self
            .issues
            .iter()
            .find(|(_, i)| i.node_id == node_id)
            .map(|((r, n), _)| (r.clone(), *n))
            .ok_or_else(|| GitHubError::Io(format!("fake: unknown issue node {node_id}")))?;
        self.next_item_id += 1;
        let id = format!("PRI_{}", self.next_item_id);
        self.project_items
            .insert(format!("{repo}#{number}"), id.clone());
        Ok(id)
    }

    async fn project_field_value(
        &mut self,
        item_id: &str,
        field_id: &str,
    ) -> Result<Option<String>, GitHubError> {
        self.project_field_reads
            .push((item_id.to_string(), field_id.to_string()));
        Ok(self
            .project_fields
            .get(&(item_id.to_string(), field_id.to_string()))
            .cloned())
    }

    async fn set_project_field(
        &mut self,
        _project_id: &str,
        item_id: &str,
        field_id: &str,
        value: &serde_json::Value,
    ) -> Result<(), GitHubError> {
        let stored = value
            .get("singleSelectOptionId")
            .and_then(|v| v.as_str())
            .or_else(|| value.get("text").and_then(|v| v.as_str()))
            .unwrap_or_default()
            .to_string();
        self.project_fields
            .insert((item_id.to_string(), field_id.to_string()), stored);
        self.project_field_writes
            .push((item_id.to_string(), field_id.to_string(), value.clone()));
        Ok(())
    }

    async fn get_user(&mut self) -> Result<String, GitHubError> {
        if self.user_rejected {
            return Err(GitHubError::Http {
                status: 401,
                url: "/user".into(),
                message: "Bad credentials".into(),
            });
        }
        Ok(if self.user_login.is_empty() {
            "fakeuser".to_string()
        } else {
            self.user_login.clone()
        })
    }

    async fn list_labels(&mut self, repo: &str) -> Result<Vec<String>, GitHubError> {
        Ok(self.labels.get(repo).cloned().unwrap_or_default())
    }

    async fn create_label(
        &mut self,
        repo: &str,
        name: &str,
        _color: &str,
    ) -> Result<(), GitHubError> {
        self.labels
            .entry(repo.to_string())
            .or_default()
            .push(name.to_string());
        self.created_labels
            .push((repo.to_string(), name.to_string()));
        Ok(())
    }

    async fn has_discussions(&mut self, repo: &str) -> Result<bool, GitHubError> {
        Ok(self
            .discussion_state
            .get(repo)
            .map(|(enabled, _)| *enabled)
            .unwrap_or(false))
    }

    async fn get_pull(&mut self, repo: &str, number: u32) -> Result<Pull, GitHubError> {
        self.pulls
            .get(&self.key(repo, number))
            .cloned()
            .ok_or_else(|| GitHubError::Http {
                status: 404,
                url: format!("/repos/{repo}/pulls/{number}"),
                message: "Not Found".into(),
            })
    }

    async fn check_runs(&mut self, _repo: &str, sha: &str) -> Result<Vec<CheckRun>, GitHubError> {
        Ok(self.check_runs.get(sha).cloned().unwrap_or_default())
    }

    async fn list_discussions(
        &mut self,
        repo: &str,
        category: Option<&str>,
        label: Option<&str>,
    ) -> Result<Vec<Discussion>, GitHubError> {
        let mut out: Vec<Discussion> = self.discussions.get(repo).cloned().unwrap_or_default();
        if let Some(label) = label {
            out.retain(|d| d.labels.iter().any(|l| l.name == label));
        }
        if let Some(category) = category {
            out.retain(|d| {
                self.discussion_categories
                    .get(&d.id)
                    .map(|c| c == category)
                    .unwrap_or(false)
            });
        }
        out.sort_by_key(|d| d.number);
        Ok(out)
    }

    async fn create_discussion(
        &mut self,
        repo: &str,
        category: &str,
        title: &str,
        body: &str,
    ) -> Result<Discussion, GitHubError> {
        self.next_discussion_number += 1;
        let number = self.next_discussion_number;
        let id = format!("D{number}");
        self.created_discussions
            .push((repo.to_string(), category.to_string(), title.to_string()));
        self.discussion_categories
            .insert(id.clone(), category.to_string());
        let d = Discussion {
            id: id.clone(),
            number,
            title: title.to_string(),
            body: body.to_string(),
            labels: Vec::new(),
            comments: Vec::new(),
        };
        self.discussions
            .entry(repo.to_string())
            .or_default()
            .push(d.clone());
        Ok(d)
    }

    async fn discussion_comment(
        &mut self,
        _repo: &str,
        discussion_id: &str,
        body: &str,
    ) -> Result<(), GitHubError> {
        self.discussion_comments
            .push((discussion_id.to_string(), body.to_string()));
        for ds in self.discussions.values_mut() {
            if let Some(d) = ds.iter_mut().find(|d| d.id == discussion_id) {
                d.comments.push(body.to_string());
            }
        }
        Ok(())
    }

    async fn remove_discussion_label(
        &mut self,
        repo: &str,
        discussion_id: &str,
        label_id: &str,
    ) -> Result<(), GitHubError> {
        self.removed_labels.push((
            repo.to_string(),
            discussion_id.to_string(),
            label_id.to_string(),
        ));
        if let Some(ds) = self.discussions.get_mut(repo)
            && let Some(d) = ds.iter_mut().find(|d| d.id == discussion_id)
        {
            d.labels.retain(|l| l.id != label_id);
        }
        Ok(())
    }

    async fn discussion_labels_supported(&mut self, repo: &str) -> Result<bool, GitHubError> {
        Ok(self
            .discussion_labels_supported
            .get(repo)
            .copied()
            .unwrap_or(false))
    }

    async fn issues_labeled(&mut self, repo: &str, label: &str) -> Result<Vec<Issue>, GitHubError> {
        let mut out: Vec<Issue> = self
            .issues
            .values()
            .filter(|i| i.repo == repo && i.labels.iter().any(|l| l == label))
            .cloned()
            .collect();
        out.sort_by_key(|i| i.number);
        Ok(out)
    }

    async fn comment(&mut self, repo: &str, number: u32, body: &str) -> Result<(), GitHubError> {
        self.next_comment_id += 1;
        self.comments
            .entry(self.key(repo, number))
            .or_default()
            .push(Comment {
                id: self.next_comment_id,
                user: "ghsync".into(),
                body: body.to_string(),
                created_at: "2026-11-11T00:00:00Z".into(),
            });
        self.comments_posted
            .push((repo.to_string(), number, body.to_string()));
        Ok(())
    }

    async fn graphql(
        &mut self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value, GitHubError> {
        self.graphql_calls
            .push((query.to_string(), variables.clone()));
        // The discussion-state query is a probe, not a provisioning step:
        // serve it from `discussion_state` instead of the queued responses
        // (mirrors the REST `has_discussions` probe).
        if query.contains("GhsyncDiscussions") {
            let repo = variables.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let (_, categories) = self
                .discussion_state
                .get(repo)
                .cloned()
                .unwrap_or((false, Vec::new()));
            let nodes: Vec<serde_json::Value> = categories
                .iter()
                .map(|c| serde_json::json!({ "id": format!("C_{repo}_{c}"), "name": c }))
                .collect();
            return Ok(serde_json::json!({
                "repository": {
                    "id": format!("R_{repo}"),
                    "discussionCategories": { "nodes": nodes }
                }
            }));
        }
        let v = self
            .graphql_responses
            .first()
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        if !self.graphql_responses.is_empty() {
            self.graphql_responses.remove(0);
        }
        if let Some(errors) = v.get("errors").and_then(|e| e.as_array()) {
            let messages: Vec<&str> = errors
                .iter()
                .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                .collect();
            return Err(GitHubError::Graphql {
                url: "/graphql".into(),
                message: messages.join("; "),
            });
        }
        Ok(v.get("data").cloned().unwrap_or(serde_json::Value::Null))
    }
}
