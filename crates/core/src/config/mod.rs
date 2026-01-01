//! Configuration loading and schema definitions
//!
//! Shared configuration types used across all platforms.

mod loader;
mod schema;

pub use loader::Config;
pub use schema::*;
