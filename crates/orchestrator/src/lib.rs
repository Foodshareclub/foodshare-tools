//! Workspace orchestration.
//!
//! The orchestrator runs the workspace's verification domains — the Rust
//! workspace, the WASM bindings, the web app, the backend and the mobile
//! app — and keeps the state of the run as it goes.
//!
//! It is a library, not a service. The CLI, the TUI and the HTTP daemon
//! are three front-ends over the same implementation, so a domain behaves
//! the same however it is started, and none of them needs the others'
//! dependencies.
//!
//! # Example
//!
//! ```rust,no_run
//! use foodshare_orchestrator::{DomainKind, Orchestrator, Workspace};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let workspace = Workspace::discover()?;
//! let orchestrator = Orchestrator::new(workspace);
//!
//! let outcome = orchestrator.run_domain(DomainKind::Rust).await?;
//! println!("rust took {}ms", outcome.duration_ms);
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use serde::Serialize;
use tokio::sync::mpsc;

use foodshare_core::error::{Error, Result};

pub mod bin;
pub mod disk;
pub mod domain;
pub mod history;
pub mod plan;
pub mod runner;
pub mod state;
pub mod verify;
pub mod workspace;

pub use disk::{DiskStats, HealReport};
pub use domain::{DomainKind, Step};
pub use history::{DomainRecord, History, RunRecord};
pub use plan::{Options, Phase, Plan};
pub use runner::{LogLine, StepOutcome, Stream};
pub use state::{DomainState, State, Status};
pub use verify::{Stage, StageOutcome, Verifier, VerifyReport};
pub use workspace::{Repo, Workspace};

/// How a failed step is retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// How many extra attempts a failing step gets.
    pub max_retries: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_retries: 2 }
    }
}

/// How a domain's run ended.
#[derive(Debug, Clone, Serialize)]
pub struct DomainOutcome {
    /// Which domain ran.
    pub kind: DomainKind,
    /// Whether every step passed.
    pub success: bool,
    /// How long the domain took.
    pub duration_ms: u64,
    /// The steps that ran.
    pub steps: Vec<StepOutcome>,
}

/// How a whole plan's run ended.
#[derive(Debug, Clone, Serialize)]
pub struct PlanOutcome {
    /// Whether every domain passed.
    pub success: bool,
    /// How long the plan took.
    pub duration_ms: u64,
    /// The domains that ran.
    pub domains: Vec<DomainOutcome>,
}

/// What a step did, or why it could not run.
#[derive(Debug)]
enum StepResult {
    /// The step ran.
    Done(StepOutcome),
    /// The step could not be started.
    Failed {
        /// The domain the step belonged to.
        domain: DomainKind,
        /// Why it failed.
        error: Error,
    },
}

#[derive(Debug)]
struct Inner {
    state: RwLock<State>,
    cancelled: AtomicBool,
}

/// Runs the workspace's verification domains and tracks their state.
#[derive(Debug, Clone)]
pub struct Orchestrator {
    workspace: Workspace,
    plan: Plan,
    policy: RetryPolicy,
    history: Arc<History>,
    inner: Arc<Inner>,
}

impl Orchestrator {
    /// Build an orchestrator with the default plan and retry policy.
    #[must_use]
    pub fn new(workspace: Workspace) -> Self {
        Self::with_options(workspace, Options::default())
    }

    /// Build an orchestrator with a specific plan and retry policy.
    #[must_use]
    pub fn with_options(workspace: Workspace, options: Options) -> Self {
        Self::with_config(workspace, options, &plan::PlanConfig::default())
    }

    /// Build an orchestrator, with a configuration file's changes applied.
    #[must_use]
    pub fn with_config(workspace: Workspace, options: Options, config: &plan::PlanConfig) -> Self {
        let plan = plan::Plan::build_with(&workspace, options, config);
        Self {
            workspace,
            plan,
            policy: RetryPolicy::default(),
            history: Arc::new(History::default()),
            inner: Arc::new(Inner {
                state: RwLock::new(State::idle()),
                cancelled: AtomicBool::new(false),
            }),
        }
    }

    /// Give the orchestrator a history to keep.
    #[must_use]
    pub fn with_history(self, history: Arc<History>) -> Self {
        Self { history, ..self }
    }

    /// The history of finished runs.
    #[must_use]
    pub fn history(&self) -> Arc<History> {
        Arc::clone(&self.history)
    }

    /// Build an orchestrator from the workspace's configuration file.
    ///
    /// A file that is not there is not an error: the built-in plan is
    /// the plan.
    pub fn from_workspace(workspace: Workspace, options: Options) -> Result<Self> {
        let config = plan::PlanConfig::load(workspace.root())?;
        Ok(Self::with_config(workspace, options, &config))
    }

    /// The workspace being orchestrated.
    #[must_use]
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// The plan that will be run.
    #[must_use]
    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// A snapshot of the current state.
    #[must_use]
    pub fn snapshot(&self) -> State {
        self.inner
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Ask the current run to stop at the next step boundary.
    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
    }

    /// Whether a cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// Clear a cancellation, so the orchestrator can run again.
    pub fn reset_cancellation(&self) {
        self.inner.cancelled.store(false, Ordering::SeqCst);
    }

    /// Run every domain in the plan, stopping at the first failure.
    pub async fn run_plan(&self) -> Result<PlanOutcome> {
        let started = Instant::now();
        let started_at = history::now();
        self.begin_run();
        let mut domains = Vec::new();
        let mut success = true;

        for phase in &self.plan.phases {
            if self.is_cancelled() {
                success = false;
                break;
            }
            let outcome = self.run_phase(phase, None).await?;
            let phase_ok = outcome.iter().all(|d| d.success);
            domains.extend(outcome);
            if !phase_ok {
                success = false;
                break;
            }
        }

        self.end_run();
        let outcome = PlanOutcome {
            success,
            duration_ms: started.elapsed().as_millis() as u64,
            domains,
        };
        self.record_run(
            &outcome.domains,
            outcome.success,
            outcome.duration_ms,
            started_at,
        );
        Ok(outcome)
    }

    /// Run one domain: every step that belongs to it.
    ///
    /// A phase can mix domains — the web type check and the backend tests
    /// run together — so running one domain runs only its own steps.
    pub async fn run_domain(&self, kind: DomainKind) -> Result<DomainOutcome> {
        let started = Instant::now();
        let started_at = history::now();
        self.begin_domain_run(kind);
        let mut steps = Vec::new();
        let mut success = true;

        for phase in &self.plan.phases {
            if self.is_cancelled() {
                success = false;
                break;
            }
            if !phase.domains().contains(&kind) {
                continue;
            }
            let outcomes = self.run_phase(phase, Some(kind)).await?;
            for outcome in outcomes {
                success &= outcome.success;
                steps.extend(outcome.steps);
            }
            if !success {
                break;
            }
        }

        self.end_run();
        let outcome = DomainOutcome {
            kind,
            success,
            duration_ms: started.elapsed().as_millis() as u64,
            steps,
        };
        self.record_run(
            std::slice::from_ref(&outcome),
            outcome.success,
            outcome.duration_ms,
            started_at,
        );
        Ok(outcome)
    }

    /// Add a run to the history.
    fn record_run(
        &self,
        domains: &[DomainOutcome],
        success: bool,
        duration_ms: u64,
        started_at: String,
    ) {
        let record = RunRecord {
            started_at,
            finished_at: history::now(),
            duration_ms,
            success,
            domains: domains
                .iter()
                .map(|d| DomainRecord {
                    domain: d.kind.name().to_string(),
                    success: d.success,
                    duration_ms: d.duration_ms,
                })
                .collect(),
        };
        if let Err(e) = self.history.record(record) {
            tracing::warn!(error = %e, "could not record the run in the history");
        }
    }

    /// Run one phase: its groups in order, the steps in a group together.
    ///
    /// When `only` is set, only that domain's steps run.
    async fn run_phase(
        &self,
        phase: &Phase,
        only: Option<DomainKind>,
    ) -> Result<Vec<DomainOutcome>> {
        let domains = match only {
            Some(kind) => vec![kind],
            None => phase.domains(),
        };
        for domain in &domains {
            self.set_status(*domain, Status::Running);
        }

        let mut steps: Vec<StepOutcome> = Vec::new();
        let mut errors: Vec<(DomainKind, Error)> = Vec::new();
        let mut failed = false;

        for group in &phase.groups {
            let selected: Vec<Step> = group
                .iter()
                .filter(|s| only.is_none_or(|kind| s.domain == kind))
                .cloned()
                .collect();
            if selected.is_empty() {
                continue;
            }
            self.set_current_steps(&selected);
            for result in self.run_group(&selected).await {
                match result {
                    StepResult::Done(outcome) => steps.push(outcome),
                    StepResult::Failed { domain, error } => errors.push((domain, error)),
                }
            }
            self.clear_current_steps(&selected);
            if !errors.is_empty() {
                failed = true;
                break;
            }
        }

        // Each domain is judged on its own steps.
        let mut outcomes = Vec::new();
        for domain in &domains {
            let domain_steps: Vec<&StepOutcome> =
                steps.iter().filter(|s| s.domain == *domain).collect();
            let domain_errors: Vec<&Error> = errors
                .iter()
                .filter(|(d, _)| d == domain)
                .map(|(_, e)| e)
                .collect();
            let success = !failed && domain_steps.iter().all(|s| s.success);
            let duration_ms = domain_steps.iter().map(|s| s.duration_ms).sum();
            // The message names the step that failed, not just the
            // domain: "the web domain failed" is not enough to act on.
            let last_error = domain_steps
                .iter()
                .find(|s| !s.success)
                .map(|s| format!("{} failed with exit code {}", s.label, s.exit_code))
                .or_else(|| domain_errors.first().map(|e| e.message.clone()));

            self.record_domain_result(*domain, success, duration_ms, last_error);
            outcomes.push(DomainOutcome {
                kind: *domain,
                success,
                duration_ms,
                steps: domain_steps.into_iter().cloned().collect(),
            });
        }

        foodshare_telemetry::metrics().increment(&format!(
            "orchestrator_domain_runs_total_{}",
            domain_key(&domains)
        ));
        if outcomes.iter().all(|d| d.success) {
            foodshare_telemetry::metrics().increment(&format!(
                "orchestrator_domain_pass_total_{}",
                domain_key(&domains)
            ));
        } else {
            foodshare_telemetry::metrics().increment(&format!(
                "orchestrator_domain_fail_total_{}",
                domain_key(&domains)
            ));
        }

        Ok(outcomes)
    }
    /// Run the steps of one group at the same time.
    async fn run_group(&self, steps: &[Step]) -> Vec<StepResult> {
        let mut set = tokio::task::JoinSet::new();
        for step in steps {
            let step = step.clone();
            let policy = self.policy;
            let heal_root: PathBuf = self.workspace.root().to_path_buf();
            let inner = Arc::clone(&self.inner);
            set.spawn(async move {
                let (tx, mut rx) = mpsc::unbounded_channel::<LogLine>();
                let domain = step.domain;
                let pump = tokio::spawn(async move {
                    while let Some(line) = rx.recv().await {
                        let mut state = inner.state.write().unwrap_or_else(|e| e.into_inner());
                        if let Some(entry) = state.domains.iter_mut().find(|d| d.kind == domain) {
                            entry.push_log(line);
                        }
                    }
                });
                let result = run_with_retry(step, tx, policy, heal_root).await;
                let _ = pump.await;
                result
            });
        }

        let mut results = Vec::new();
        while let Some(joined) = set.join_next().await {
            match joined {
                Ok(result) => results.push(result),
                Err(e) => results.push(StepResult::Failed {
                    domain: DomainKind::Rust,
                    error: Error::process(format!("A step task failed: {e}")),
                }),
            }
        }
        results
    }

    fn begin_run(&self) {
        let mut state = self.inner.state.write().unwrap_or_else(|e| e.into_inner());
        state.running = true;
        state.started_at_ms = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        );
        for domain in &mut state.domains {
            domain.reset();
        }
    }

    /// Start a single domain, leaving the others' results alone.
    fn begin_domain_run(&self, kind: DomainKind) {
        let mut state = self.inner.state.write().unwrap_or_else(|e| e.into_inner());
        state.running = true;
        state.started_at_ms = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        );
        if let Some(domain) = state.domains.iter_mut().find(|d| d.kind == kind) {
            domain.reset();
        }
    }

    fn end_run(&self) {
        let mut state = self.inner.state.write().unwrap_or_else(|e| e.into_inner());
        state.running = false;
    }

    /// Say which steps are running.
    fn set_current_steps(&self, steps: &[Step]) {
        let mut state = self.inner.state.write().unwrap_or_else(|e| e.into_inner());
        for step in steps {
            if let Some(domain) = state.domains.iter_mut().find(|d| d.kind == step.domain) {
                domain.current_step = Some(match domain.current_step.take() {
                    Some(current) => format!("{current}, {}", step.label),
                    None => step.label.clone(),
                });
            }
        }
    }

    /// Say that nothing is running.
    fn clear_current_steps(&self, steps: &[Step]) {
        let mut state = self.inner.state.write().unwrap_or_else(|e| e.into_inner());
        for step in steps {
            if let Some(domain) = state.domains.iter_mut().find(|d| d.kind == step.domain) {
                domain.current_step = None;
            }
        }
    }

    fn set_status(&self, kind: DomainKind, status: Status) {
        let mut state = self.inner.state.write().unwrap_or_else(|e| e.into_inner());
        if let Some(domain) = state.domains.iter_mut().find(|d| d.kind == kind) {
            domain.status = status;
        }
    }

    fn record_domain_result(
        &self,
        kind: DomainKind,
        success: bool,
        duration_ms: u64,
        last_error: Option<String>,
    ) {
        let mut state = self.inner.state.write().unwrap_or_else(|e| e.into_inner());
        if let Some(domain) = state.domains.iter_mut().find(|d| d.kind == kind) {
            domain.status = if success { Status::Pass } else { Status::Fail };
            domain.duration_ms = Some(duration_ms);
            if success {
                domain.pass += 1;
            } else {
                domain.fail += 1;
                domain.last_error = last_error.or_else(|| Some(format!("{} failed", kind.label())));
            }
        }
    }
}

/// The metric name suffix for a set of domains.
fn domain_key(domains: &[DomainKind]) -> String {
    if domains.len() == 1 {
        domains[0].name().to_string()
    } else {
        domains
            .iter()
            .map(|d| d.name())
            .collect::<Vec<_>>()
            .join("_and_")
    }
}

/// Run a step, retrying it and healing the disk between attempts.
///
/// A step that could not be started is not retried: a missing program
/// will still be missing on the second attempt.
async fn run_with_retry(
    step: Step,
    tx: mpsc::UnboundedSender<LogLine>,
    policy: RetryPolicy,
    heal_root: PathBuf,
) -> StepResult {
    let domain = step.domain;
    let mut attempt = 0u32;
    loop {
        match runner::run_step(&step, &tx).await {
            Ok(outcome) if outcome.success => return StepResult::Done(outcome),
            Ok(outcome) => {
                if attempt >= policy.max_retries {
                    return StepResult::Done(outcome);
                }
                attempt += 1;

                let _ = tx.send(LogLine {
                    stream: Stream::Stderr,
                    text: format!(
                        "⚠ {} failed (exit {}), attempt {attempt}/{} — healing disk and retrying",
                        step.label,
                        outcome.exit_code,
                        policy.max_retries + 1
                    ),
                });
                let _ = disk::heal(&heal_root, disk::MIN_FREE_GB, false).await;
                tokio::time::sleep(runner::backoff(attempt)).await;
            }
            Err(error) => return StepResult::Failed { domain, error },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn orchestrator() -> Orchestrator {
        Orchestrator::new(Workspace::with_root("/tmp/ws"))
    }

    #[test]
    fn test_snapshot_starts_idle() {
        let orchestrator = orchestrator();
        let state = orchestrator.snapshot();
        assert!(!state.running);
        assert_eq!(state.domains.len(), DomainKind::ALL.len());
    }

    #[test]
    fn test_cancellation_is_observable() {
        let orchestrator = orchestrator();
        assert!(!orchestrator.is_cancelled());
        orchestrator.cancel();
        assert!(orchestrator.is_cancelled());
        orchestrator.reset_cancellation();
        assert!(!orchestrator.is_cancelled());
    }

    #[test]
    fn test_clone_shares_the_state() {
        let orchestrator = orchestrator();
        let clone = orchestrator.clone();
        orchestrator.cancel();
        assert!(clone.is_cancelled());
    }

    #[test]
    fn test_domain_key_names_one_domain() {
        assert_eq!(domain_key(&[DomainKind::Rust]), "rust");
        assert_eq!(
            domain_key(&[DomainKind::Web, DomainKind::Backend]),
            "web_and_backend"
        );
    }

    #[tokio::test]
    async fn test_run_domain_reports_a_failure() {
        // The working directory does not exist, so the step fails rather
        // than hanging.
        let orchestrator = Orchestrator::with_options(
            Workspace::with_root("/tmp/ws-that-does-not-exist"),
            Options {
                quick: true,
                sequential: true,
            },
        );
        let outcome = orchestrator
            .run_domain(DomainKind::Rust)
            .await
            .expect("run finishes");
        assert!(!outcome.success);
        assert!(outcome.duration_ms < 60_000);

        let state = orchestrator.snapshot();
        let rust = state.domain(DomainKind::Rust).expect("rust state");
        assert_eq!(rust.status, Status::Fail);
        assert!(rust.last_error.is_some());
    }
}
