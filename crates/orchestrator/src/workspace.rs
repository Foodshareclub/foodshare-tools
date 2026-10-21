//! The workspace and the repositories in it.
//!
//! The workspace is the directory that holds `foodshare-app`,
//! `foodshare-web`, `foodshare-backend` and `foodshare-tools`. Every
//! repository is a sibling, so one root locates all of them.
//!
//! This is the one place that answers "where am I?", because the tools
//! are run from all sorts of directories and the answer has to be the
//! same every time.

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
    /// The app repository, when it is somewhere else.
    app: Option<PathBuf>,
}

impl Workspace {
    /// Build a workspace from an explicit root.
    #[must_use]
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            app: None,
        }
    }

    /// Build a workspace whose app repository is somewhere else.
    ///
    /// The other repositories are the app repository's siblings, so the
    /// root is its parent.
    #[must_use]
    pub fn with_app_dir(app: impl Into<PathBuf>) -> Self {
        let app = app.into();
        let root = app
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| app.clone());
        Self {
            root,
            app: Some(app),
        }
    }

    /// Find the workspace root.
    ///
    /// `FOODSHARE_WORKSPACE_ROOT` wins if it is set. Otherwise the
    /// current directory is walked up until a directory holding the app
    /// repository is found, which is the same answer whether the tools
    /// are run from the workspace, from a repository in it, or from a
    /// directory inside one.
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
        Ok(Self::discover_from(&cwd))
    }

    /// Find the workspace root, starting from a directory.
    ///
    /// The environment override is not read here: this is the part that
    /// can be tested without changing the state of the process.
    #[must_use]
    pub fn discover_from(cwd: &Path) -> Self {
        for dir in cwd.ancestors() {
            // A directory that holds the app repository is the root.
            if dir.join(Repo::App.dir_name()).is_dir() {
                return Self::with_root(dir);
            }
            // The current directory is inside the app repository, so its
            // parent is the root.
            if dir.file_name().and_then(|n| n.to_str()) == Some(Repo::App.dir_name()) {
                return Self::with_root(dir.parent().unwrap_or(dir));
            }
        }

        Self::with_root(cwd)
    }

    /// The workspace root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The directory of a repository.
    #[must_use]
    pub fn repo(&self, repo: Repo) -> PathBuf {
        match (repo, &self.app) {
            (Repo::App, Some(app)) => app.clone(),
            (repo, _) => self.root.join(repo.dir_name()),
        }
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
    fn test_the_repositories_are_siblings() {
        let workspace = Workspace::with_root("/tmp/ws");
        assert_eq!(workspace.root(), Path::new("/tmp/ws"));
        assert_eq!(workspace.tools(), Path::new("/tmp/ws/foodshare-tools"));
        assert_eq!(workspace.app(), Path::new("/tmp/ws/foodshare-app"));
        assert_eq!(workspace.web(), Path::new("/tmp/ws/foodshare-web"));
        assert_eq!(workspace.backend(), Path::new("/tmp/ws/foodshare-backend"));
    }

    #[test]
    fn test_an_explicit_app_directory_overrides_the_layout() {
        let workspace = Workspace::with_app_dir("/elsewhere/foodshare-app");
        assert_eq!(workspace.app(), Path::new("/elsewhere/foodshare-app"));
        // The other repositories are its siblings.
        assert_eq!(workspace.root(), Path::new("/elsewhere"));
        assert_eq!(workspace.web(), Path::new("/elsewhere/foodshare-web"));
    }

    #[test]
    fn test_discovery_honours_the_override() {
        let dir = tempfile::tempdir().expect("temp dir");
        // SAFETY: single-threaded test body; the variable is restored below.
        unsafe { std::env::set_var(ROOT_ENV, dir.path()) };
        let workspace = Workspace::discover().expect("discover");
        assert_eq!(workspace.root(), dir.path());
        unsafe { std::env::remove_var(ROOT_ENV) };
    }

    #[test]
    fn test_discovery_finds_the_root_from_a_subdirectory() {
        let dir = tempfile::tempdir().expect("temp dir");
        let app = dir.path().join("foodshare-app");
        let inside = app.join("Darwin").join("FoodShare.xcodeproj");
        std::fs::create_dir_all(&inside).expect("create");

        // The discovery starts from inside the app repository. The
        // process's own directory is left alone: it is global state, and
        // the tests beside this one would see it change.
        let workspace = Workspace::discover_from(&app);

        assert_eq!(workspace.root(), dir.path());
        assert_eq!(workspace.app(), app);
    }

    #[test]
    fn test_discovery_from_a_directory_without_the_app_repository() {
        let dir = tempfile::tempdir().expect("temp dir");
        let workspace = Workspace::discover_from(dir.path());
        assert_eq!(workspace.root(), dir.path());
    }
}
