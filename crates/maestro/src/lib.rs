//! Maestro E2E flows: discovery, syntax checking, and execution.
//!
//! This crate replaces the Bun runner that used to drive Maestro. It
//! covers the three things that runner did:
//!
//! - finding the flows in a suite,
//! - checking their syntax,
//! - running them, on one device or on every device at once.
//!
//! # Example
//!
//! ```rust,no_run
//! use foodshare_maestro::{device, flows, runner};
//! use std::path::Path;
//!
//! # async fn run() {
//! let maestro_dir = Path::new("foodshare-app/.maestro");
//!
//! let found = flows::discover(maestro_dir);
//! let results = runner::check_syntax(&found).await;
//!
//! let devices = device::discover();
//! if let Some(target) = devices.first() {
//!     runner::run_test(target, &[], maestro_dir, None).await;
//! }
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod device;
pub mod flows;
pub mod runner;

pub use device::{Platform, TargetDevice};
pub use flows::discover as discover_flows;
pub use runner::{SyntaxResult, TestOutcome};

/// The tags a suite can be filtered by.
pub const TAGS: &[&str] = &["smoke", "regression", "critical", "a11y"];

/// The directory that holds a Maestro suite, given the app directory.
#[must_use]
pub fn maestro_dir(app_dir: &std::path::Path) -> std::path::PathBuf {
    app_dir.join(".maestro")
}
