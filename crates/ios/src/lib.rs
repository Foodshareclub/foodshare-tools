//! iOS-specific tools for Foodshare
//!
//! This crate provides iOS/Xcode-specific functionality:
//! - Xcode project manipulation
//! - Simulator management
//! - Swift tooling wrappers
//! - Build analysis
//! - Enterprise-grade git hooks
//! - Code protection system

#![warn(missing_docs)]

pub mod code_protection;
pub mod hooks;
pub mod simulator;
pub mod swift_tools;
pub mod xcode;
pub mod xcodeproj;
