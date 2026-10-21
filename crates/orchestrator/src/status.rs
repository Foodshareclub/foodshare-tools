//! The status the daemon serves.
//!
//! One type, built by the daemon and read by the TUI, so the two cannot
//! drift apart.

use serde::{Deserialize, Serialize};

use crate::domain::DomainKind;
use crate::state::{DomainState, State};

/// The status of the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    /// The service that served it.
    pub service: String,
    /// The version of the service.
    pub version: String,
    /// How long the service has been up.
    pub uptime_seconds: u64,
    /// Whether a run is in progress.
    pub running: bool,
    /// The state of every domain.
    pub domains: Vec<DomainState>,
    /// The metrics snapshot, for a client that wants the numbers.
    #[serde(default)]
    pub metrics: serde_json::Value,
}

impl StatusResponse {
    /// The state, in the shape the orchestrator works with.
    #[must_use]
    pub fn into_state(self) -> State {
        State {
            domains: self.domains,
            started_at_ms: None,
            running: self.running,
        }
    }

    /// The state of one domain.
    #[must_use]
    pub fn domain(&self, kind: DomainKind) -> Option<&DomainState> {
        self.domains.iter().find(|d| d.kind == kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Status;

    fn response() -> StatusResponse {
        StatusResponse {
            service: "fs-orchestrator".to_string(),
            version: "1.4.0".to_string(),
            uptime_seconds: 42,
            running: true,
            domains: vec![DomainState::new(DomainKind::Rust)],
            metrics: serde_json::json!({"counters": {}}),
        }
    }

    #[test]
    fn test_the_status_survives_the_trip_through_json() {
        let json = serde_json::to_string(&response()).expect("serialised");
        let parsed: StatusResponse = serde_json::from_str(&json).expect("parsed");

        assert_eq!(parsed.service, "fs-orchestrator");
        assert_eq!(parsed.uptime_seconds, 42);
        assert!(parsed.running);
        assert_eq!(parsed.domains.len(), 1);
        assert_eq!(parsed.domains[0].kind, DomainKind::Rust);
        assert_eq!(parsed.domains[0].status, Status::Idle);
    }

    #[test]
    fn test_the_state_keeps_the_domains() {
        let state = response().into_state();
        assert!(state.running);
        assert_eq!(state.domains.len(), 1);
        assert!(state.domain(DomainKind::Rust).is_some());
    }

    #[test]
    fn test_the_metrics_are_optional() {
        let json =
            r#"{"service":"x","version":"1","uptime_seconds":0,"running":false,"domains":[]}"#;
        let parsed: StatusResponse = serde_json::from_str(json).expect("parsed");
        assert!(parsed.metrics.is_null());
    }
}
