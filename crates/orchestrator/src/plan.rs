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

use crate::domain::{DomainKind, Step};
use crate::workspace::Workspace;

/// Options that change the shape of the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Options {
    /// Skip the WASM build, on the assumption the artefacts are cached.
    pub quick: bool,
    /// Run the web and backend domains one after another instead of
    /// together.
    pub sequential: bool,
}

/// One phase of a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The phases, in order.
    pub phases: Vec<Phase>,
}

impl Plan {
    /// Build the plan for a workspace.
    #[must_use]
    pub fn build(workspace: &Workspace, options: Options) -> Self {
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
}
