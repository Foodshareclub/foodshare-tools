//! The verification plan.
//!
//! A plan is an ordered list of phases. Each phase holds one or more
//! *groups* of steps, and the steps in a group run at the same time. The
//! shape mirrors the order the Bun orchestrator used, so the sequence a
//! developer sees does not change when the plan moves to Rust.
//!
//! Every step belongs to exactly one domain. A phase may mix domains —
//! the web type check and the backend tests run together — and the state
//! of each domain is still tracked separately.

use serde::{Deserialize, Serialize};

use crate::domain::{DomainKind, Step};
use crate::workspace::Workspace;

/// The configuration file that may change the plan.
pub const CONFIG_FILE: &str = ".foodshare-verify.toml";

/// Options that change the shape of the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Options {
    /// Skip the WASM build, on the assumption the artefacts are cached.
    pub quick: bool,
    /// Run the web and backend domains one after another instead of
    /// together.
    pub sequential: bool,
}

/// The plan, as a configuration file may change it.
///
/// A file changes two things: which domains run, and what a domain runs.
/// Everything else keeps the built-in shape, so a file with one line in
/// it does not have to restate the whole plan.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PlanConfig {
    /// The domains to leave out.
    #[serde(default)]
    pub skip: Vec<String>,
    /// The domains whose steps the file replaces.
    #[serde(default)]
    pub domain: Vec<DomainConfig>,
    /// The stages of the full verification, when the file gives them.
    ///
    /// The built-in stages — the toolchain policy check and the disk
    /// healing — come first and are not replaced; these are the stages
    /// after them.
    #[serde(default, rename = "verify")]
    pub verify: Vec<StepConfig>,
    /// The built-in verification stages to leave out: `node-guard` and
    /// `disk`.
    #[serde(default, rename = "verify_skip")]
    pub verify_skip: Vec<String>,
}

/// A domain whose steps a configuration file replaces.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DomainConfig {
    /// The domain, by its name.
    pub name: String,
    /// The steps that replace the domain's built-in ones. They run in
    /// order.
    #[serde(default, rename = "step")]
    pub steps: Vec<StepConfig>,
}

/// A step, as a configuration file describes it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StepConfig {
    /// What the step is called.
    pub label: String,
    /// The program to run.
    pub program: String,
    /// The arguments to pass.
    #[serde(default)]
    pub args: Vec<String>,
    /// The directory to run in, relative to the workspace root.
    pub cwd: String,
    /// Extra environment variables, as `KEY=VALUE`.
    #[serde(default)]
    pub env: Vec<String>,
}

impl PlanConfig {
    /// Read the configuration file, if there is one.
    ///
    /// A file that is not there is not an error: the built-in plan is
    /// the plan.
    pub fn load(root: &std::path::Path) -> crate::Result<Self> {
        Self::load_from(&root.join(CONFIG_FILE))
    }

    /// Read a configuration file.
    pub fn load_from(path: &std::path::Path) -> crate::Result<Self> {
        if !path.is_file() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path).map_err(|e| {
            foodshare_core::error::Error::config(format!(
                "Failed to read the plan configuration {}: {e}",
                path.display()
            ))
        })?;
        toml::from_str(&content).map_err(|e| {
            foodshare_core::error::Error::config(format!(
                "Failed to parse the plan configuration {}: {e}",
                path.display()
            ))
        })
    }

    /// The domains the file leaves out.
    pub fn skipped(&self) -> Vec<DomainKind> {
        self.skip
            .iter()
            .filter_map(|name| DomainKind::from_name(name))
            .collect()
    }

    /// The steps the file gives a domain, if it gives it any.
    pub fn steps_for(&self, kind: DomainKind) -> Option<&[StepConfig]> {
        self.domain
            .iter()
            .find(|d| d.name == kind.name())
            .map(|d| d.steps.as_slice())
    }
}

/// One phase of a plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Phase {
    /// What the phase is called.
    pub label: String,
    /// The groups of steps. Steps in a group run in parallel.
    pub groups: Vec<Vec<Step>>,
}

impl Phase {
    /// The domains the phase touches, in the order they appear.
    #[must_use]
    pub fn domains(&self) -> Vec<DomainKind> {
        let mut domains = Vec::new();
        for step in self.groups.iter().flatten() {
            if !domains.contains(&step.domain) {
                domains.push(step.domain);
            }
        }
        domains
    }
}

/// A whole verification plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Plan {
    /// The phases, in order.
    pub phases: Vec<Phase>,
}

impl Plan {
    /// Build the plan for a workspace.
    #[must_use]
    pub fn build(workspace: &Workspace, options: Options) -> Self {
        Self::build_with(workspace, options, &PlanConfig::default())
    }

    /// Build the plan for a workspace, with a configuration file's changes
    /// applied.
    ///
    /// A domain the file skips leaves the plan. A domain the file gives
    /// steps to has those steps instead of its built-in ones. Every other
    /// domain keeps the built-in shape.
    #[must_use]
    pub fn build_with(workspace: &Workspace, options: Options, config: &PlanConfig) -> Self {
        let mut plan = Self::built_in(workspace, options);

        let skipped = config.skipped();
        if !skipped.is_empty() {
            plan.remove_domains(&skipped);
        }

        for kind in DomainKind::ALL {
            let Some(steps) = config.steps_for(kind) else {
                continue;
            };
            plan.replace_domain(workspace, kind, steps);
        }

        plan
    }

    /// The built-in plan.
    #[must_use]
    fn built_in(workspace: &Workspace, options: Options) -> Self {
        let tools = workspace.tools();
        let web = workspace.web();
        let backend = workspace.backend();
        let app = workspace.app();

        let mut phases = Vec::new();

        phases.push(Phase {
            label: "Rust".to_string(),
            groups: vec![vec![Step::new(
                DomainKind::Rust,
                "Rust (cargo test --workspace)",
                "cargo",
                &["test", "--workspace"],
                tools.display().to_string(),
            )]],
        });

        if !options.quick {
            // The WASM build is a Rust binary in this workspace.
            let wasm = crate::bin::find_with(
                &tools,
                "fs-wasm",
                &[
                    "--tools-dir",
                    &tools.display().to_string(),
                    "--web-dir",
                    &web.display().to_string(),
                ],
            );
            let (program, args) = wasm.into_parts();
            phases.push(Phase {
                label: "WASM".to_string(),
                groups: vec![vec![Step::new(
                    DomainKind::Wasm,
                    "WASM (wasm-pack, 5 crates)",
                    program,
                    &args.iter().map(String::as_str).collect::<Vec<_>>(),
                    tools.display().to_string(),
                )]],
            });
        }

        let web_type_check = Step::new(
            DomainKind::Web,
            "Web type-check",
            "bun",
            &["run", "type-check"],
            web.display().to_string(),
        );
        let web_build = Step::new(
            DomainKind::Web,
            "Web build (Turbopack)",
            "bun",
            &["run", "build"],
            web.display().to_string(),
        );
        let backend_tests = Step::new(
            DomainKind::Backend,
            "Backend (deno test)",
            "deno",
            &[
                "test",
                "--allow-all",
                "--config",
                "supabase/functions/deno.json",
                "supabase/functions/__tests__/",
            ],
            backend.display().to_string(),
        );

        if options.sequential {
            phases.push(Phase {
                label: "Web".to_string(),
                groups: vec![vec![web_type_check], vec![web_build]],
            });
            phases.push(Phase {
                label: "Backend".to_string(),
                groups: vec![vec![backend_tests]],
            });
        } else {
            phases.push(Phase {
                label: "Web + Backend".to_string(),
                groups: vec![vec![web_type_check, backend_tests], vec![web_build]],
            });
        }

        // The Maestro syntax check is a Rust binary in this workspace.
        let maestro = crate::bin::find_with(
            &tools,
            "fs-maestro",
            &["syntax", "--app-dir", &app.display().to_string()],
        );
        let (program, args) = maestro.into_parts();
        let gradle = Step::new(
            DomainKind::Mobile,
            "Mobile Gradle :app:testDebugUnitTest",
            "./gradlew",
            &[
                ":app:testDebugUnitTest",
                "--build-cache",
                "--parallel",
                "--no-configuration-cache",
            ],
            app.join("Android").display().to_string(),
        )
        .with_env(
            "ANDROID_HOME",
            std::env::var("ANDROID_HOME")
                .unwrap_or_else(|_| "/Users/organic/Library/Android/sdk".to_string()),
        );

        phases.push(Phase {
            label: "Mobile".to_string(),
            groups: vec![
                vec![Step::new(
                    DomainKind::Mobile,
                    "Mobile Maestro syntax",
                    program,
                    &args.iter().map(String::as_str).collect::<Vec<_>>(),
                    app.display().to_string(),
                )],
                vec![gradle],
            ],
        });

        Self { phases }
    }

    /// Take the domains out of the plan.
    fn remove_domains(&mut self, kinds: &[DomainKind]) {
        for phase in &mut self.phases {
            for group in &mut phase.groups {
                group.retain(|step| !kinds.contains(&step.domain));
            }
        }
        // A phase with nothing left in it is not a phase.
        self.phases
            .retain(|phase| !phase.groups.iter().all(|group| group.is_empty()));
    }

    /// Give a domain the steps from the configuration file.
    fn replace_domain(&mut self, workspace: &Workspace, kind: DomainKind, steps: &[StepConfig]) {
        self.remove_domains(&[kind]);

        let configured: Vec<Step> = steps
            .iter()
            .map(|step| {
                let cwd = workspace.root().join(&step.cwd);
                let mut built = Step::new(
                    kind,
                    step.label.clone(),
                    step.program.clone(),
                    &step.args.iter().map(String::as_str).collect::<Vec<_>>(),
                    cwd.display().to_string(),
                );
                for entry in &step.env {
                    if let Some((key, value)) = entry.split_once('=') {
                        built = built.with_env(key, value);
                    }
                }
                built
            })
            .collect();

        if configured.is_empty() {
            return;
        }

        // The steps run in order, so each is a group of its own.
        self.phases.push(Phase {
            label: kind.label().to_string(),
            groups: configured.into_iter().map(|step| vec![step]).collect(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> Workspace {
        Workspace::with_root("/tmp/ws")
    }

    #[test]
    fn test_plan_has_the_expected_phases() {
        let plan = Plan::build(&workspace(), Options::default());
        let labels: Vec<&str> = plan.phases.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(labels, vec!["Rust", "WASM", "Web + Backend", "Mobile"]);
    }

    #[test]
    fn test_quick_skips_wasm() {
        let plan = Plan::build(
            &workspace(),
            Options {
                quick: true,
                ..Options::default()
            },
        );
        let labels: Vec<&str> = plan.phases.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(labels, vec!["Rust", "Web + Backend", "Mobile"]);
    }

    #[test]
    fn test_sequential_splits_web_and_backend() {
        let plan = Plan::build(
            &workspace(),
            Options {
                sequential: true,
                ..Options::default()
            },
        );
        let labels: Vec<&str> = plan.phases.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(labels, vec!["Rust", "WASM", "Web", "Backend", "Mobile"]);
    }

    #[test]
    fn test_wasm_step_uses_the_rust_binary() {
        let plan = Plan::build(&workspace(), Options::default());
        let wasm = &plan.phases[1].groups[0][0];
        // Either the built binary, or `cargo run` to build it.
        assert!(
            wasm.program.ends_with("fs-wasm") || wasm.program == "cargo",
            "got: {}",
            wasm.program
        );
        assert!(wasm.args.iter().any(|a| a == "fs-wasm") || wasm.program.ends_with("fs-wasm"));
    }

    #[test]
    fn test_maestro_step_uses_the_rust_binary() {
        let plan = Plan::build(&workspace(), Options::default());
        let mobile = &plan.phases[3];
        let maestro = &mobile.groups[0][0];
        assert!(
            maestro.program.ends_with("fs-maestro") || maestro.program == "cargo",
            "got: {}",
            maestro.program
        );
        assert!(maestro.args.iter().any(|a| a == "syntax"));
    }

    #[test]
    fn test_web_and_backend_run_together() {
        let plan = Plan::build(&workspace(), Options::default());
        let phase = &plan.phases[2];
        assert_eq!(phase.groups.len(), 2);
        assert_eq!(phase.groups[0].len(), 2);
        assert_eq!(phase.groups[1].len(), 1);
    }

    #[test]
    fn test_a_mixed_phase_reports_both_domains() {
        let plan = Plan::build(&workspace(), Options::default());
        let phase = &plan.phases[2];
        assert_eq!(phase.domains(), vec![DomainKind::Web, DomainKind::Backend]);
    }

    #[test]
    fn test_every_domain_is_covered_by_the_plan() {
        let plan = Plan::build(&workspace(), Options::default());
        let mut covered: Vec<DomainKind> = Vec::new();
        for phase in &plan.phases {
            for domain in phase.domains() {
                if !covered.contains(&domain) {
                    covered.push(domain);
                }
            }
        }
        for domain in DomainKind::ALL {
            assert!(
                covered.contains(&domain),
                "{domain} is not covered by the plan"
            );
        }
    }

    #[test]
    fn test_quick_leaves_out_the_wasm_domain() {
        let plan = Plan::build(
            &workspace(),
            Options {
                quick: true,
                ..Options::default()
            },
        );
        let covered: Vec<DomainKind> = plan
            .phases
            .iter()
            .flat_map(|phase| phase.domains())
            .collect();
        assert!(!covered.contains(&DomainKind::Wasm));
        assert!(covered.contains(&DomainKind::Rust));
        assert!(covered.contains(&DomainKind::Web));
        assert!(covered.contains(&DomainKind::Backend));
        assert!(covered.contains(&DomainKind::Mobile));
    }

    #[test]
    fn test_steps_carry_their_domain() {
        let plan = Plan::build(&workspace(), Options::default());
        let rust = &plan.phases[0].groups[0][0];
        assert_eq!(rust.domain, DomainKind::Rust);
        let web = &plan.phases[2].groups[0][0];
        assert_eq!(web.domain, DomainKind::Web);
    }

    #[test]
    fn test_a_configuration_file_can_skip_a_domain() {
        let config = PlanConfig {
            skip: vec!["wasm".to_string()],
            ..PlanConfig::default()
        };
        let plan = Plan::build_with(&workspace(), Options::default(), &config);
        assert!(
            !plan
                .phases
                .iter()
                .flat_map(|p| p.groups.iter().flatten())
                .any(|s| s.domain == DomainKind::Wasm)
        );
        // The other domains are untouched.
        assert!(
            plan.phases
                .iter()
                .flat_map(|p| p.groups.iter().flatten())
                .any(|s| s.domain == DomainKind::Rust)
        );
    }

    #[test]
    fn test_a_configuration_file_can_replace_a_domain() {
        let config = PlanConfig {
            domain: vec![DomainConfig {
                name: "web".to_string(),
                steps: vec![StepConfig {
                    label: "Web lint".to_string(),
                    program: "bun".to_string(),
                    args: vec!["run".to_string(), "lint".to_string()],
                    cwd: "foodshare-web".to_string(),
                    env: Vec::new(),
                }],
            }],
            ..PlanConfig::default()
        };
        let plan = Plan::build_with(&workspace(), Options::default(), &config);

        let web_steps: Vec<&Step> = plan
            .phases
            .iter()
            .flat_map(|p| p.groups.iter().flatten())
            .filter(|s| s.domain == DomainKind::Web)
            .collect();
        assert_eq!(web_steps.len(), 1);
        assert_eq!(web_steps[0].label, "Web lint");
        assert_eq!(web_steps[0].program, "bun");
        assert!(web_steps[0].cwd.ends_with("foodshare-web"));
    }

    #[test]
    fn test_a_configuration_file_is_read() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(".foodshare-verify.toml");
        std::fs::write(
            &path,
            "skip = [\"wasm\"]\n\n[[domain]]\nname = \"web\"\n\n[[domain.step]]\nlabel = \"Web lint\"\nprogram = \"bun\"\nargs = [\"run\", \"lint\"]\ncwd = \"foodshare-web\"\n",
        )
        .expect("write");

        let config = PlanConfig::load_from(&path).expect("parsed");
        assert_eq!(config.skipped(), vec![DomainKind::Wasm]);
        let steps = config.steps_for(DomainKind::Web).expect("web steps");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].label, "Web lint");
    }

    #[test]
    fn test_a_missing_configuration_file_is_the_built_in_plan() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config = PlanConfig::load(dir.path()).expect("loaded");
        assert!(config.skip.is_empty());
        assert!(config.domain.is_empty());
    }

    #[test]
    fn test_a_broken_configuration_file_is_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(".foodshare-verify.toml");
        std::fs::write(&path, "skip = [").expect("write");
        assert!(PlanConfig::load_from(&path).is_err());
    }
}
