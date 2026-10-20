//! The verification domains and the steps that make them up.
//!
//! A *domain* is a slice of the workspace that is checked as a unit: the
//! Rust workspace, the WASM bindings, the web app, the backend, and the
//! mobile app. Each domain owns one or more *steps*, and a step is a
//! single command run in one directory.

use serde::{Deserialize, Serialize};

/// A slice of the workspace that is verified as a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DomainKind {
    /// The Rust workspace: `cargo test --workspace`.
    Rust,
    /// The WASM bindings built from the published crates.
    Wasm,
    /// The Next.js web app.
    Web,
    /// The Supabase edge functions.
    Backend,
    /// The Skip app: Maestro flows and Gradle unit tests.
    Mobile,
}

impl DomainKind {
    /// Every domain, in verification order.
    pub const ALL: [DomainKind; 5] = [
        DomainKind::Rust,
        DomainKind::Wasm,
        DomainKind::Web,
        DomainKind::Backend,
        DomainKind::Mobile,
    ];

    /// The stable name used in the status payload and on the command line.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Wasm => "wasm",
            Self::Web => "web",
            Self::Backend => "backend",
            Self::Mobile => "mobile",
        }
    }

    /// The label used in human-facing output.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Wasm => "WASM",
            Self::Web => "Web",
            Self::Backend => "Backend",
            Self::Mobile => "Mobile",
        }
    }

    /// The glyph used in the TUI.
    #[must_use]
    pub fn icon(self) -> &'static str {
        match self {
            Self::Rust => "🦀",
            Self::Wasm => "📦",
            Self::Web => "⚡",
            Self::Backend => "🛡️",
            Self::Mobile => "📱",
        }
    }

    /// A one-line description of what the domain checks.
    #[must_use]
    pub fn detail(self) -> &'static str {
        match self {
            Self::Rust => "cargo test --workspace, benches, clippy",
            Self::Wasm => "5 WASM modules: search/geo/crypto/compression/image",
            Self::Web => "Next.js Turbopack build, tsrs, oxlint, biome",
            Self::Backend => "Deno edge function tests, pgvector",
            Self::Mobile => "Skip transpile, Maestro flows, Gradle",
        }
    }

    /// Look a domain up by its stable name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.name() == name)
    }
}

impl std::fmt::Display for DomainKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// One command, run in one directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    /// The domain the step belongs to.
    pub domain: DomainKind,
    /// What the step is called, in human-facing output.
    pub label: String,
    /// The program to run.
    pub program: String,
    /// The arguments to pass.
    pub args: Vec<String>,
    /// The directory to run in.
    pub cwd: String,
    /// Extra environment variables, as `KEY=VALUE`.
    pub env: Vec<String>,
}

impl Step {
    /// Build a step in a domain.
    #[must_use]
    pub fn new(
        domain: DomainKind,
        label: impl Into<String>,
        program: impl Into<String>,
        args: &[&str],
        cwd: impl Into<String>,
    ) -> Self {
        Self {
            domain,
            label: label.into(),
            program: program.into(),
            args: args.iter().map(|a| (*a).to_string()).collect(),
            cwd: cwd.into(),
            env: Vec::new(),
        }
    }

    /// Add an environment variable.
    #[must_use]
    pub fn with_env(mut self, key: &str, value: impl Into<String>) -> Self {
        self.env.push(format!("{key}={}", value.into()));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_domains_are_in_verification_order() {
        let names: Vec<&str> = DomainKind::ALL.iter().map(|d| d.name()).collect();
        assert_eq!(names, vec!["rust", "wasm", "web", "backend", "mobile"]);
    }

    #[test]
    fn test_from_name_round_trips() {
        for domain in DomainKind::ALL {
            assert_eq!(DomainKind::from_name(domain.name()), Some(domain));
        }
    }

    #[test]
    fn test_from_name_rejects_unknown() {
        assert_eq!(DomainKind::from_name("nope"), None);
    }

    #[test]
    fn test_step_builds_args() {
        let step = Step::new(
            DomainKind::Rust,
            "Rust",
            "cargo",
            &["test", "--workspace"],
            "/tmp",
        );
        assert_eq!(step.domain, DomainKind::Rust);
        assert_eq!(step.args, vec!["test", "--workspace"]);
        assert_eq!(step.cwd, "/tmp");
        assert!(step.env.is_empty());
    }

    #[test]
    fn test_step_env_is_appended() {
        let step = Step::new(DomainKind::Mobile, "Gradle", "./gradlew", &[], "/tmp")
            .with_env("ANDROID_HOME", "/sdk");
        assert_eq!(step.env, vec!["ANDROID_HOME=/sdk"]);
    }
}
