//! Talking to a running orchestrator daemon.
//!
//! The TUI can watch a daemon that is already running, rather than
//! starting an orchestrator of its own. A run started in one place is
//! then visible in the other, which is the point: one source of truth for
//! a run in progress.

use foodshare_orchestrator::{DomainKind, State, StatusResponse};

/// A client for a running daemon.
#[derive(Debug, Clone)]
pub struct DaemonClient {
    http: reqwest::Client,
    base_url: String,
}

impl DaemonClient {
    /// Build a client for a daemon.
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        Self {
            http: reqwest::Client::new(),
            base_url,
        }
    }

    /// The state of the run in progress.
    pub async fn status(&self) -> Option<State> {
        let response = self
            .http
            .get(format!("{}/status", self.base_url))
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        let status: StatusResponse = response.json().await.ok()?;
        Some(status.into_state())
    }

    /// Ask the daemon to start a domain.
    pub async fn run(&self, domain: DomainKind) -> bool {
        self.post(&format!("/run/{}", domain.name())).await
    }

    /// Ask the daemon to start the whole plan.
    pub async fn run_all(&self) -> bool {
        self.post("/run/all").await
    }

    /// Ask the daemon to stop the run in progress.
    pub async fn cancel(&self) -> bool {
        self.post("/cancel").await
    }

    /// Send a POST, and say whether it was accepted.
    async fn post(&self, path: &str) -> bool {
        let response = self
            .http
            .post(format!("{}{path}", self.base_url))
            .send()
            .await;
        matches!(response, Ok(response) if response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foodshare_orchestrator::{DomainKind, DomainState};

    /// The status the daemon serves.
    fn status_json() -> String {
        serde_json::to_string(&StatusResponse {
            service: "fs-orchestrator".to_string(),
            version: "1.4.0".to_string(),
            uptime_seconds: 42,
            running: true,
            domains: vec![DomainState::new(DomainKind::Rust)],
            metrics: serde_json::json!({}),
        })
        .expect("serialised")
    }

    #[test]
    fn test_a_status_response_becomes_a_state() {
        let response: StatusResponse = serde_json::from_str(&status_json()).expect("parsed");
        let state = response.into_state();
        assert!(state.running);
        assert_eq!(state.domains.len(), 1);
        assert_eq!(state.domains[0].kind, DomainKind::Rust);
    }

    #[test]
    fn test_the_status_carries_the_daemons_own_fields() {
        let response: StatusResponse = serde_json::from_str(&status_json()).expect("parsed");
        assert_eq!(response.service, "fs-orchestrator");
        assert_eq!(response.uptime_seconds, 42);
    }
}
