//! iOS-specific tools for Foodshare
//!
//! This crate provides iOS/Xcode-specific functionality:
//! - Xcode project manipulation
//! - Simulator management
//! - Swift tooling wrappers
//! - Build analysis

pub mod simulator;
pub mod swift_tools;
pub mod xcode;
pub mod xcodeproj;
