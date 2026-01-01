//! Core utilities for Foodshare development tools
//!
//! This crate provides shared functionality used across all platform-specific tools:
//!
//! - **Error handling**: Enterprise-grade errors with codes, context, and recovery suggestions
//! - **Git operations**: Staging, branches, diffs using command-line git
//! - **File scanning**: Efficient file discovery with filtering
//! - **Process execution**: Safe command execution with timeouts
//! - **Configuration**: TOML-based configuration with validation
//! - **Health checks**: Verify tool dependencies and environment
//!
//! # Example
//!
//! ```rust,no_run
//! use foodshare_core::{git::GitRepo, health::HealthChecker};
//!
//! // Check environment health
//! let report = HealthChecker::new()
//!     .with_standard_checks()
//!     .run();
//!
//! if !report.is_healthy() {
//!     eprintln!("Environment issues detected!");
//! }
//!
//! // Work with git
//! let repo = GitRepo::open_current().expect("Not a git repo");
//! let staged = repo.staged_files().expect("Failed to get staged files");
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod audit;
pub mod cache;
pub mod config;
pub mod error;
pub mod feature_flags;
pub mod file_scanner;
pub mod git;
pub mod health;
pub mod process;
pub mod rate_limit;
pub mod retry;
pub mod validation;

pub use error::{Error, ErrorCode, Result, ResultExt};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::audit::{AuditAction, AuditEvent, AuditLog};
    pub use crate::cache::{Cache, CacheConfig};
    pub use crate::error::{exit_codes, Error, ErrorCode, Result, ResultExt};
    pub use crate::feature_flags::{FeatureFlags, Flag, FlagValue};
    pub use crate::git::GitRepo;
    pub use crate::health::{HealthChecker, HealthReport, HealthStatus};
    pub use crate::rate_limit::{RateLimitConfig, RateLimiter};
    pub use crate::retry::{retry, CircuitBreaker, RetryConfig};
    pub use crate::validation::{ValidationResult, Validator};
}
