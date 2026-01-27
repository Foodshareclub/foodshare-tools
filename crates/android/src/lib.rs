//! Android-specific tools for Foodshare
//!
//! This crate provides Android-specific functionality:
//! - Gradle build system integration
//! - Emulator management
//! - Kotlin tooling wrappers
//! - Swift cross-compilation for Android
//! - FoodshareCore build scripts

#![warn(missing_docs)]

pub mod emulator;
pub mod gradle;
pub mod kotlin_tools;
pub mod swift_android;
pub mod swift_core;
