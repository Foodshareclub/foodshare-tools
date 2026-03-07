//! Git hooks for Foodshare development tools
//!
//! This crate provides shared git hook implementations:
//! - Conventional commit validation
//! - Secret scanning (enterprise-grade)
//! - Migration checks
//! - Pre-push validation
//!
//! # Secret Scanning
//!
//! The secret scanning module provides enterprise-grade detection with:
//! - 19 built-in patterns for common secret types
//! - Configuration-driven pattern management
//! - Allowlisting and fingerprint suppression
//! - Parallel file scanning
//! - Entropy-based detection
//!
//! See [`secrets`] module for full documentation.

#![warn(missing_docs)]

pub mod commit_msg;
pub mod migrations;
pub mod pre_push;
pub mod secrets;

pub use foodshare_core::error::{Result, exit_codes};

// Enterprise API exports
pub use secrets::{
    // Constants
    CONFIG_API_VERSION,
    // Core types
    Finding,
    PATTERN_VERSION,
    PatternCategory,
    PatternDef,
    ScanError,
    ScanOutput,
    ScanResult,
    ScannerConfig,
    SecretScanner,
    Severity,
    // Functions
    builtin_patterns,
};

// Legacy API exports (for backwards compatibility)
pub use secrets::{
    ScanStats, SecretMatch, print_results, print_results_with_stats, scan_content,
    scan_content_with_entropy, scan_file, scan_files, scan_files_with_stats,
};
