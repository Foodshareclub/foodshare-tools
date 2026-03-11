//! App-specific tools for Foodshare
//!
//! This crate provides cross-platform App functionality:
//! - Xcode project manipulation
//! - Simulator and Emulator management
//! - Swift and Kotlin tooling wrappers
//! - Build analysis
//! - Enterprise-grade git hooks
//! - Code protection system

#![warn(missing_docs)]

// iOS modules
pub mod code_protection;
pub mod hooks;
pub mod simulator;
pub mod swift_tools;
pub mod xcode;
pub mod xcodeproj;

// Android modules
pub mod emulator;
pub mod gradle;
pub mod kotlin_tools;
pub mod swift_android;
pub mod swift_core;
