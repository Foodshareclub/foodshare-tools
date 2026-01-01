//! Git hooks for Foodshare development tools
//!
//! This crate provides shared git hook implementations:
//! - Conventional commit validation
//! - Secret scanning
//! - Migration checks
//! - Pre-push validation

pub mod commit_msg;
pub mod migrations;
pub mod pre_push;
pub mod secrets;

pub use foodshare_core::error::{exit_codes, Result};
