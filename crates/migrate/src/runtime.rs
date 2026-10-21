//! Choosing the container runtime that owns the database.
//!
//! The Supabase database runs in a container, and the container is reached
//! with the runtime's own command-line tool. Docker and Podman speak the
//! same commands for this — `exec` and `ps --format {{.Names}}` — so the
//! only question is which of them owns the running container.
//!
//! The choice is never a guess. A wrong runtime means a wrong database,
//! and the two look identical from the outside.

use std::fmt;

use crate::error::{MigrateError, Result};

/// A container runtime the tool can drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    /// Docker, the `docker` command.
    Docker,
    /// Podman, the `podman` command.
    Podman,
}

impl Runtime {
    /// The name of the command-line tool.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }

    /// Every runtime the tool knows how to drive.
    #[must_use]
    pub fn all() -> [Self; 2] {
        [Self::Docker, Self::Podman]
    }

    /// The runtime named `name`, if it is one the tool knows.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "docker" => Some(Self::Docker),
            "podman" => Some(Self::Podman),
            _ => None,
        }
    }
}

impl fmt::Display for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Decide which runtime to use.
///
/// `requested` is an explicit choice and it wins. Without one, the
/// runtime that owns the running container wins. If both own it, or
/// neither does, there is no answer to give: the tool would either talk
/// to the wrong database or to nothing at all.
fn choose(requested: Option<Runtime>, owners: &[(Runtime, bool)]) -> Result<Runtime> {
    if let Some(runtime) = requested {
        return Ok(runtime);
    }

    let running: Vec<Runtime> = owners
        .iter()
        .filter(|(_, running)| *running)
        .map(|(runtime, _)| *runtime)
        .collect();

    match running.as_slice() {
        [only] => Ok(*only),
        [] => Err(MigrateError::container(
            "no container runtime is running the database; install one, or name it with --runtime",
        )),
        [first, second, ..] => Err(MigrateError::container(format!(
            "the database runs under both {first} and {second}; name one with --runtime",
        ))),
    }
}

/// Resolve the runtime to drive.
///
/// The runtime is found by asking each one the tool knows whether it has
/// the container running. A runtime that is not installed is skipped, the
/// same as one that does not have the container.
///
/// `requested` comes from `--runtime` or the `CONTAINER_RUNTIME`
/// environment variable.
pub fn resolve(requested: Option<Runtime>, container: &str) -> Result<Runtime> {
    let owners: Vec<(Runtime, bool)> = Runtime::all()
        .into_iter()
        .map(|runtime| (runtime, owns_container(runtime, container)))
        .collect();
    choose(requested, &owners)
}

/// Whether `runtime` has the container running.
fn owns_container(runtime: Runtime, container: &str) -> bool {
    if !foodshare_core::process::command_exists(runtime.as_str()) {
        return false;
    }
    let result = foodshare_core::process::run_command(
        runtime.as_str(),
        &[
            "ps",
            "--format",
            "{{.Names}}",
            "--filter",
            &format!("name=^{container}$"),
        ],
    );
    match result {
        Ok(result) => result.stdout.trim() == container,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_runtime_wins() {
        let owners = [(Runtime::Docker, true), (Runtime::Podman, true)];
        assert_eq!(
            choose(Some(Runtime::Podman), &owners).expect("a runtime"),
            Runtime::Podman
        );
    }

    #[test]
    fn an_explicit_runtime_wins_even_when_nothing_owns_the_container() {
        let owners = [(Runtime::Docker, false), (Runtime::Podman, false)];
        assert_eq!(
            choose(Some(Runtime::Docker), &owners).expect("a runtime"),
            Runtime::Docker
        );
    }

    #[test]
    fn the_runtime_that_owns_the_container_wins() {
        let owners = [(Runtime::Docker, true), (Runtime::Podman, false)];
        assert_eq!(choose(None, &owners).expect("a runtime"), Runtime::Docker);

        let owners = [(Runtime::Docker, false), (Runtime::Podman, true)];
        assert_eq!(choose(None, &owners).expect("a runtime"), Runtime::Podman);
    }

    #[test]
    fn no_runtime_owns_the_container_is_an_error() {
        let owners = [(Runtime::Docker, false), (Runtime::Podman, false)];
        let error = choose(None, &owners).expect_err("no runtime");
        assert!(
            error.to_string().contains("no container runtime"),
            "got: {error}"
        );
    }

    #[test]
    fn two_runtimes_owning_the_container_is_an_error() {
        let owners = [(Runtime::Docker, true), (Runtime::Podman, true)];
        let error = choose(None, &owners).expect_err("no single runtime");
        assert!(error.to_string().contains("both"), "got: {error}");
    }

    #[test]
    fn runtime_names_round_trip() {
        assert_eq!(Runtime::parse("docker"), Some(Runtime::Docker));
        assert_eq!(Runtime::parse("PODMAN"), Some(Runtime::Podman));
        assert_eq!(Runtime::parse(" containerd "), None);
        assert_eq!(Runtime::Docker.as_str(), "docker");
        assert_eq!(Runtime::Podman.as_str(), "podman");
    }
}
