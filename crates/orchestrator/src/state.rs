//! The state of a verification run.
//!
//! The state is what the TUI renders and what the status endpoint
//! serves. It is deliberately cheap to snapshot: the orchestrator holds it
//! behind a lock and hands out copies.

use serde::{Deserialize, Serialize};

use crate::domain::DomainKind;

/// The most log lines kept per domain.
///
/// Older lines are dropped in blocks so a long run does not spend its
/// time trimming one line at a time.
pub const MAX_LOGS: usize = 200;

/// How many lines are dropped when the log is full.
pub const LOG_DROP: usize = 50;

/// Where a domain is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Never run, or reset.
    Idle,
    /// Running right now.
    Running,
    /// Finished green.
    Pass,
    /// Finished red.
    Fail,
}

impl Status {
    /// The label shown in the TUI.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::Running => "RUN",
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
        }
    }
}

/// The state of one domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainState {
    /// Which domain this is.
    pub kind: DomainKind,
    /// Where it is in its lifecycle.
    pub status: Status,
    /// Steps that passed.
    pub pass: u64,
    /// Steps that failed.
    pub fail: u64,
    /// How long the last run took.
    pub duration_ms: Option<u64>,
    /// The step that is running right now, when there is one.
    pub current_step: Option<String>,
    /// The tail of the run's output.
    pub logs: Vec<String>,
    /// The last failure message.
    pub last_error: Option<String>,
}

impl DomainState {
    /// A fresh, idle domain.
    #[must_use]
    pub fn new(kind: DomainKind) -> Self {
        Self {
            kind,
            status: Status::Idle,
            pass: 0,
            fail: 0,
            duration_ms: None,
            current_step: None,
            logs: Vec::new(),
            last_error: None,
        }
    }

    /// Append a line to the log, trimming the tail when it gets long.
    pub fn push_log(&mut self, line: impl Into<String>) {
        self.logs.push(line.into());
        if self.logs.len() > MAX_LOGS {
            self.logs.drain(0..LOG_DROP);
        }
    }

    /// Reset the run counters, keeping the log.
    pub fn reset(&mut self) {
        self.status = Status::Idle;
        self.pass = 0;
        self.fail = 0;
        self.duration_ms = None;
        self.last_error = None;
    }
}

/// A snapshot of the whole run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// The state of every domain, in verification order.
    pub domains: Vec<DomainState>,
    /// When the current run started.
    pub started_at_ms: Option<u64>,
    /// Whether a run is in progress.
    pub running: bool,
}

impl State {
    /// A snapshot with every domain idle.
    #[must_use]
    pub fn idle() -> Self {
        Self {
            domains: DomainKind::ALL.into_iter().map(DomainState::new).collect(),
            started_at_ms: None,
            running: false,
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

    #[test]
    fn test_idle_state_has_every_domain() {
        let state = State::idle();
        assert_eq!(state.domains.len(), DomainKind::ALL.len());
        assert!(state.domains.iter().all(|d| d.status == Status::Idle));
        assert!(!state.running);
    }

    #[test]
    fn test_domain_lookup() {
        let state = State::idle();
        assert_eq!(
            state.domain(DomainKind::Rust).map(|d| d.kind),
            Some(DomainKind::Rust)
        );
        assert!(state.domain(DomainKind::Rust).is_some());
    }

    #[test]
    fn test_log_is_trimmed_in_blocks() {
        let mut domain = DomainState::new(DomainKind::Rust);
        for index in 0..(MAX_LOGS + 10) {
            domain.push_log(format!("line {index}"));
        }
        assert_eq!(domain.logs.len(), MAX_LOGS - LOG_DROP + 10);
        // The oldest lines are the ones dropped.
        assert_eq!(domain.logs[0], format!("line {LOG_DROP}"));
    }

    #[test]
    fn test_reset_keeps_the_log() {
        let mut domain = DomainState::new(DomainKind::Rust);
        domain.push_log("kept");
        domain.status = Status::Fail;
        domain.fail = 2;
        domain.reset();
        assert_eq!(domain.status, Status::Idle);
        assert_eq!(domain.fail, 0);
        assert_eq!(domain.logs, vec!["kept"]);
    }

    #[test]
    fn test_status_labels() {
        assert_eq!(Status::Idle.label(), "IDLE");
        assert_eq!(Status::Running.label(), "RUN");
        assert_eq!(Status::Pass.label(), "PASS");
        assert_eq!(Status::Fail.label(), "FAIL");
    }
}
