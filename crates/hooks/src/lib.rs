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

pub use foodshare_core::error::{exit_codes, Result};

// Enterprise API exports
pub use secrets::{
    // Core types
    Finding,
    PatternCategory,
    PatternDef,
    ScanError,
    ScannerConfig,
    ScanOutput,
    ScanResult,
    SecretScanner,
    Severity,
    // Constants
    CONFIG_API_VERSION,
    PATTERN_VERSION,
    // Functions
    builtin_patterns,
};

// Legacy API exports (for backwards compatibility)
pub use secrets::{
    print_results,
    print_results_with_stats,
    scan_content,
    scan_content_with_entropy,
    scan_file,
    scan_files,
    scan_files_with_stats,
    ScanStats,
    SecretMatch,
};
