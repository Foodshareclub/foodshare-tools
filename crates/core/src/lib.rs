//! Core utilities for Foodshare development tools
//!
//! This crate provides shared functionality used across all platform-specific tools:
//! - Git operations (staging, branches, diffs)
//! - File scanning and filtering
//! - Process execution
//! - Configuration loading

pub mod config;
pub mod error;
pub mod file_scanner;
pub mod git;
pub mod process;

pub use error::{Error, Result};
