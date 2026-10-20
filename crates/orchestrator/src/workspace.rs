//! Locating the workspace and the repositories inside it.
//!
//! The workspace is the directory that holds `foodshare-app`, `foodshare-web`,
//! `foodshare-backend` and `foodshare-tools`. Every repository is a sibling,
//! so one root locates all of them.

use std::path::{Path, PathBuf};

use foodshare_core::error::{Error, Result};

/// Environment variable that overrides workspace discovery.
pub const ROOT_ENV: &str = "FOODSHARE_WORKSPACE_ROOT";

/// A repository in the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Repo {
    /// The tooling repository. This one holds the Rust workspace.
    Tools,
    /// The cross-platform app.
    App,
    /// The web app.
    Web,
    /// The backend.
    Backend,
}

impl Repo {
    /// The directory name of the repository inside the workspace.
    #[must_use]
    pub fn dir_name(self) -> &'static str {
        match self {
            Self::Tools => "foodshare-tools",
            Self::App => "foodshare-app",
            Self::Web => "foodshare-web",
            Self::Backend => "foodshare-backend",
        }
    }
}

/// The workspace root and the repositories it contains.
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// Build a workspace from an explicit root.
    #[must_use]
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Find the workspace root.
    ///
    /// `FOODSHARE_WORKSPACE_ROOT` wins if it is set. Otherwise the current
    /// directory is used when it holds `foodshare-app`, and its parent when
    /// that holds it. The current directory is the fallback, so a tools-only
    /// checkout still resolves to something usable.
    pub fn discover() -> Result<Self> {
        if let Some(root) = std::env::var_os(ROOT_ENV) {
            let root = PathBuf::from(root);
            if root.is_dir() {
                return Ok(Self::with_root(root));
            }
            return Err(Error::config(format!(
                "{ROOT_ENV} points at {}, which is not a directory",
                root.display()
            ))
            .with_suggestion("Point it at the directory that contains foodshare-app"));
        }

        let cwd = std::env::current_dir()
            .map_err(|e| Error::io(format!("Failed to read the current directory: {e}")))?;

        if cwd.join("foodshare-app").is_dir() {
            return Ok(Self::with_root(cwd));
        }
        if let Some(parent) = cwd.parent() {
            if parent.join("foodshare-app").is_dir() {
                return Ok(Self::with_root(parent));
            }
        }
        Ok(Self::with_root(cwd))
    }

    /// The workspace root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The directory of a repository.
    #[must_use]
    pub fn repo(&self, repo: Repo) -> PathBuf {
        self.root.join(repo.dir_name())
    }

    /// The tools repository, which holds the Rust workspace.
    #[must_use]
    pub fn tools(&self) -> PathBuf {
        self.repo(Repo::Tools)
    }

    /// The app repository.
    #[must_use]
    pub fn app(&self) -> PathBuf {
        self.repo(Repo::App)
    }

    /// The web repository.
    #[must_use]
    pub fn web(&self) -> PathBuf {
        self.repo(Repo::Web)
    }

    /// The backend repository.
    #[must_use]
    pub fn backend(&self) -> PathBuf {
        self.repo(Repo::Backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_dir_names() {
        assert_eq!(Repo::Tools.dir_name(), "foodshare-tools");
        assert_eq!(Repo::App.dir_name(), "foodshare-app");
        assert_eq!(Repo::Web.dir_name(), "foodshare-web");
        assert_eq!(Repo::Backend.dir_name(), "foodshare-backend");
    }

    #[test]
    fn test_workspace_paths_are_siblings() {
        let workspace = Workspace::with_root("/tmp/ws");
        assert_eq!(workspace.root(), Path::new("/tmp/ws"));
        assert_eq!(workspace.tools(), Path::new("/tmp/ws/foodshare-tools"));
        assert_eq!(workspace.app(), Path::new("/tmp/ws/foodshare-app"));
        assert_eq!(workspace.web(), Path::new("/tmp/ws/foodshare-web"));
        assert_eq!(workspace.backend(), Path::new("/tmp/ws/foodshare-backend"));
    }

    #[test]
    fn test_discover_honours_the_override() {
        let dir = tempfile::tempdir().expect("temp dir");
        // SAFETY: single-threaded test body; the variable is restored below.
        unsafe { std::env::set_var(ROOT_ENV, dir.path()) };
        let workspace = Workspace::discover().expect("discover");
        assert_eq!(workspace.root(), dir.path());
        unsafe { std::env::remove_var(ROOT_ENV) };
    }
}
