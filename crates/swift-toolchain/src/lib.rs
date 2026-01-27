//! Swift toolchain version management for Foodshare monorepo
//!
//! This crate provides functionality to:
//! - Detect installed Swift versions
//! - Verify Swift version consistency across Package.swift files
//! - Configure environment for specific Swift versions
//! - Migrate between Swift versions

pub mod config;
pub mod detect;
pub mod error;
pub mod migrate;
pub mod verify;

pub use config::SwiftConfig;
pub use detect::{SwiftToolchain, SwiftVersion};
pub use error::{Result, SwiftError};
pub use verify::VerificationReport;

/// Swift version requirements for the project
pub const REQUIRED_SWIFT_VERSION: &str = "6.3";
pub const SWIFT_TOOLS_VERSION: &str = "6.3";

/// Default toolchain path on macOS
pub const DEFAULT_TOOLCHAIN_PATH: &str =
    "/Users/organic/Library/Developer/Toolchains/swift-6.3-DEVELOPMENT-SNAPSHOT-2026-01-16-a.xctoolchain";
