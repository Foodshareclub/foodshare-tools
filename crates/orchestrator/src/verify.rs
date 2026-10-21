//! The full workspace verification.
//!
//! This is the cross-repository check: the toolchain policy in every
//! repository, the disk, the Rust workspace, the web build, the mobile
//! smoke tests and the backend edge function tests. It is a different
//! shape from the orchestrator's domain plan — it reaches into every
//! repository, and it runs the mobile *smoke* suite rather than the
//! mobile domain — so it is a plan of its own.

use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;

use crate::workspace::Workspace;

/// One stage of the verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    /// The Node.js guard, run in every repository.
    ///
    /// The policy is one — Bun is the only runtime — but the guard is
    /// per repository, because each one knows its own layout and its own
    /// carve-outs.
    NodeGuard {
        /// The repositories to check.
        repos: Vec<PathBuf>,
    },
    /// Heal the disk before the build.
    DiskHeal,
    /// Run a command in a directory.
    Command {
        /// What the stage is called.
        label: String,
        /// The program to run.
        program: String,
        /// The arguments to pass.
        args: Vec<String>,
        /// The directory to run in.
        cwd: PathBuf,
    },
}

impl Stage {
    /// The stage's label.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::NodeGuard { repos } => format!("Node.js guard ({} repositories)", repos.len()),
            Self::DiskHeal => "Disk health".to_string(),
            Self::Command { label, .. } => (*label).to_string(),
        }
    }
}

/// How one stage ended.
#[derive(Debug, Clone, Serialize)]
pub struct StageOutcome {
    /// What the stage was called.
    pub label: String,
    /// Whether it passed.
    pub success: bool,
    /// How long it took.
    pub duration_ms: u64,
    /// Why it failed.
    pub error: Option<String>,
}

/// How the verification ended.
#[derive(Debug, Clone, Serialize)]
pub struct VerifyReport {
    /// Whether every stage passed.
    pub success: bool,
    /// How long the whole verification took.
    pub duration_ms: u64,
    /// The stages that ran.
    pub stages: Vec<StageOutcome>,
}

/// The full workspace verification.
#[derive(Debug, Clone)]
pub struct Verifier {
    stages: Vec<Stage>,
}

impl Verifier {
    /// Build the verification for a workspace.
    #[must_use]
    pub fn new(workspace: &Workspace) -> Self {
        Self::build(workspace, &crate::plan::PlanConfig::default())
    }

    /// Build the verification, with a configuration file's changes
    /// applied.
    ///
    /// The toolchain policy check and the disk healing are the stages
    /// that come first, and they can be left out. The stages after them
    /// are the ones the file replaces.
    #[must_use]
    pub fn with_config(workspace: &Workspace, config: &crate::plan::PlanConfig) -> Self {
        Self::build(workspace, config)
    }

    fn build(workspace: &Workspace, config: &crate::plan::PlanConfig) -> Self {
        let tools = workspace.tools();
        let web = workspace.web();
        let backend = workspace.backend();
        let app = workspace.app();

        // The mobile smoke run is a Rust binary in this workspace.
        let sim = crate::bin::find(&tools, "fs-sim");
        let (program, args) = sim.into_parts();
        let sim_args: Vec<String> = args.into_iter().chain(["--smoke".to_string()]).collect();

        let mut stages = vec![
            Stage::NodeGuard {
                repos: vec![tools.clone(), app.clone(), web.clone(), backend.clone()],
            },
            Stage::DiskHeal,
            Stage::Command {
                label: "Rust workspace tests".to_string(),
                program: "cargo".to_string(),
                args: vec!["test".to_string(), "--workspace".to_string()],
                cwd: tools,
            },
            Stage::Command {
                label: "Web build (Turbopack)".to_string(),
                program: "bun".to_string(),
                args: vec!["run".to_string(), "build".to_string()],
                cwd: web,
            },
            Stage::Command {
                label: "Mobile smoke tests".to_string(),
                program,
                args: sim_args,
                cwd: app,
            },
            Stage::Command {
                label: "Backend edge function tests".to_string(),
                program: "deno".to_string(),
                args: vec![
                    "test".to_string(),
                    "--allow-all".to_string(),
                    "--config".to_string(),
                    "supabase/functions/deno.json".to_string(),
                    "supabase/functions/__tests__/".to_string(),
                ],
                cwd: backend,
            },
        ];

        // The built-in stages the file leaves out.
        for name in &config.verify_skip {
            match name.as_str() {
                "node-guard" => {
                    stages.retain(|stage| !matches!(stage, Stage::NodeGuard { .. }));
                }
                "disk" => {
                    stages.retain(|stage| !matches!(stage, Stage::DiskHeal));
                }
                _ => tracing::warn!(stage = name, "unknown verification stage to skip"),
            }
        }

        // The stages the file replaces.
        if !config.verify.is_empty() {
            let first_command = stages
                .iter()
                .position(|stage| matches!(stage, Stage::Command { .. }))
                .unwrap_or(stages.len());
            stages.retain(|stage| !matches!(stage, Stage::Command { .. }));
            let configured: Vec<Stage> = config
                .verify
                .iter()
                .map(|step| Stage::Command {
                    label: step.label.clone(),
                    program: step.program.clone(),
                    args: step.args.clone(),
                    cwd: workspace.root().join(&step.cwd),
                })
                .collect();
            let at = first_command.min(stages.len());
            stages.splice(at..at, configured);
        }

        Self { stages }
    }

    /// Leave out the toolchain policy check.
    #[must_use]
    pub fn without_node_guard(mut self) -> Self {
        self.stages
            .retain(|stage| !matches!(stage, Stage::NodeGuard { .. }));
        self
    }

    /// Keep only the toolchain policy check.
    #[must_use]
    pub fn only_node_guard(mut self) -> Self {
        self.stages
            .retain(|stage| matches!(stage, Stage::NodeGuard { .. }));
        self
    }

    /// Leave out the mobile smoke tests.
    ///
    /// They need a booted simulator, so they do not run everywhere.
    #[must_use]
    pub fn without_mobile(mut self) -> Self {
        self.stages.retain(
            |stage| !matches!(stage, Stage::Command { label, .. } if label == "Mobile smoke tests"),
        );
        self
    }

    /// The stages that will run.
    #[must_use]
    pub fn stages(&self) -> &[Stage] {
        &self.stages
    }

    /// Run the verification, stopping at the first failure.
    pub async fn run(&self, root: &std::path::Path) -> VerifyReport {
        let started = Instant::now();
        let mut outcomes = Vec::with_capacity(self.stages.len());

        for stage in &self.stages {
            let stage_started = Instant::now();
            let result = self.run_stage(stage, root).await;
            let (success, error) = match result {
                Ok(()) => (true, None),
                Err(e) => (false, Some(e)),
            };
            outcomes.push(StageOutcome {
                label: stage.label(),
                success,
                duration_ms: stage_started.elapsed().as_millis() as u64,
                error,
            });
            if !success {
                break;
            }
        }

        let success = outcomes.iter().all(|o| o.success);
        VerifyReport {
            success,
            duration_ms: started.elapsed().as_millis() as u64,
            stages: outcomes,
        }
    }

    /// Run one stage.
    async fn run_stage(&self, stage: &Stage, root: &std::path::Path) -> Result<(), String> {
        match stage {
            Stage::NodeGuard { repos } => run_node_guard(repos).await,
            Stage::DiskHeal => {
                let report = crate::disk::heal(root, crate::disk::MIN_FREE_GB, false)
                    .await
                    .map_err(|e| e.to_string())?;
                for action in &report.actions {
                    println!("  ✓ {action}");
                }
                Ok(())
            }
            Stage::Command {
                label,
                program,
                args,
                cwd,
            } => run_command(label, program, args, cwd).await,
        }
    }
}

/// Run the Node.js guard in every repository.
async fn run_node_guard(repos: &[PathBuf]) -> Result<(), String> {
    for repo in repos {
        let script = repo.join("scripts").join("check-no-node.ts");
        if !script.is_file() {
            return Err(format!(
                "{}: scripts/check-no-node.ts is missing",
                repo.display()
            ));
        }
        run_command(
            "Node.js guard",
            "bun",
            &["scripts/check-no-node.ts".to_string()],
            repo,
        )
        .await?;
    }
    Ok(())
}

/// Run a command, with its output on the terminal.
async fn run_command(
    label: &str,
    program: &str,
    args: &[String],
    cwd: &std::path::Path,
) -> Result<(), String> {
    let program = program.to_string();
    let args = args.to_vec();
    let cwd = cwd.to_path_buf();

    // The build is long and the output is wanted on the terminal, so it
    // runs on a blocking thread rather than holding the runtime.
    let outcome = tokio::task::spawn_blocking(move || {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        foodshare_core::process::run_command_streaming_in_dir(&program, &args, &cwd)
    })
    .await
    .map_err(|e| format!("{label}: the task failed: {e}"))?;

    let result = outcome.map_err(|e| format!("{label}: {e}"))?;
    if result == 0 {
        Ok(())
    } else {
        Err(format!("{label} failed with exit code {result}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> Workspace {
        Workspace::with_root("/tmp/ws")
    }

    #[test]
    fn test_the_stages_cover_every_repository() {
        let verifier = Verifier::new(&workspace());
        let labels: Vec<String> = verifier.stages().iter().map(Stage::label).collect();
        assert_eq!(
            labels,
            vec![
                "Node.js guard (4 repositories)",
                "Disk health",
                "Rust workspace tests",
                "Web build (Turbopack)",
                "Mobile smoke tests",
                "Backend edge function tests",
            ]
        );
    }

    #[test]
    fn test_the_node_guard_covers_every_repository() {
        let verifier = Verifier::new(&workspace());
        let Some(Stage::NodeGuard { repos }) = verifier.stages().first() else {
            panic!("the first stage is the node guard");
        };
        assert_eq!(repos.len(), 4);
        assert!(repos.iter().all(|r| r.ends_with("foodshare-app")
            || r.ends_with("foodshare-web")
            || r.ends_with("foodshare-backend")
            || r.ends_with("foodshare-tools")));
    }

    #[test]
    fn test_the_node_guard_can_be_left_out() {
        let verifier = Verifier::new(&workspace()).without_node_guard();
        assert!(
            !verifier
                .stages()
                .iter()
                .any(|s| matches!(s, Stage::NodeGuard { .. }))
        );
        assert_eq!(verifier.stages().len(), 5);
    }

    #[test]
    fn test_the_node_guard_can_be_run_on_its_own() {
        let verifier = Verifier::new(&workspace()).only_node_guard();
        assert_eq!(verifier.stages().len(), 1);
        assert!(matches!(
            verifier.stages().first(),
            Some(Stage::NodeGuard { .. })
        ));
    }

    #[test]
    fn test_the_mobile_smoke_tests_can_be_left_out() {
        let verifier = Verifier::new(&workspace()).without_mobile();
        assert!(
            !verifier.stages().iter().any(
                |s| matches!(s, Stage::Command { label, .. } if *label == "Mobile smoke tests")
            )
        );
    }

    #[test]
    fn test_a_configuration_file_can_replace_the_stages() {
        let config = crate::plan::PlanConfig {
            verify: vec![crate::plan::StepConfig {
                label: "Only the Rust tests".to_string(),
                program: "cargo".to_string(),
                args: vec!["test".to_string(), "--workspace".to_string()],
                cwd: "foodshare-tools".to_string(),
                env: Vec::new(),
            }],
            ..crate::plan::PlanConfig::default()
        };
        let verifier = Verifier::with_config(&workspace(), &config);

        let commands: Vec<&Stage> = verifier
            .stages()
            .iter()
            .filter(|s| matches!(s, Stage::Command { .. }))
            .collect();
        assert_eq!(commands.len(), 1);
        let Stage::Command {
            label,
            program,
            cwd,
            ..
        } = commands[0]
        else {
            panic!("it is a command");
        };
        assert_eq!(label, "Only the Rust tests");
        assert_eq!(program, "cargo");
        assert!(cwd.ends_with("foodshare-tools"));
    }

    #[test]
    fn test_a_configuration_file_can_leave_out_a_built_in_stage() {
        let config = crate::plan::PlanConfig {
            verify_skip: vec!["node-guard".to_string(), "disk".to_string()],
            ..crate::plan::PlanConfig::default()
        };
        let verifier = Verifier::with_config(&workspace(), &config);
        assert!(
            !verifier
                .stages()
                .iter()
                .any(|s| matches!(s, Stage::NodeGuard { .. }))
        );
        assert!(
            !verifier
                .stages()
                .iter()
                .any(|s| matches!(s, Stage::DiskHeal))
        );
        // The command stages are still there.
        assert!(
            verifier
                .stages()
                .iter()
                .any(|s| matches!(s, Stage::Command { .. }))
        );
    }

    #[test]
    fn test_the_rust_stage_runs_the_workspace_tests() {
        let verifier = Verifier::new(&workspace());
        let Some(Stage::Command {
            program, args, cwd, ..
        }) = verifier.stages().iter().find(
            |s| matches!(s, Stage::Command { label, .. } if *label == "Rust workspace tests"),
        )
        else {
            panic!("the rust stage is there");
        };
        assert_eq!(program, "cargo");
        assert_eq!(args, &["test".to_string(), "--workspace".to_string()]);
        assert!(cwd.ends_with("foodshare-tools"));
    }
}
