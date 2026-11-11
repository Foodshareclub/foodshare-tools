//! In-memory fakes for the two seams.
//!
//! `FakeKanban` and `FakeGitHub` stand in for the real clients in every
//! unit and integration test up to the live E2E. They mirror the real
//! behaviour: status transitions on the kanban side, numbering and
//! state on the GitHub side, and recording of every mutation so tests
//! can assert exactly what would have been sent.

use std::collections::HashMap;

use crate::github::{Comment, GitHub, GitHubError, Issue, Milestone};
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
    /// GraphQL calls in order: (query, variables).
    pub graphql_calls: Vec<(String, serde_json::Value)>,
    /// Queued GraphQL responses, popped one per call.
    pub graphql_responses: Vec<serde_json::Value>,
    next_issue_number: u32,
    next_milestone_number: u32,
    next_comment_id: u64,
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
            state: "open".into(),
            labels: labels.to_vec(),
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
        Ok(())
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
        self.graphql_calls.push((query.to_string(), variables));
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
