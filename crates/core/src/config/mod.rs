//! Configuration loading and schema definitions
//!
//! Shared configuration types used across all platforms.

mod loader;
mod schema;
mod unknown;

pub use loader::Config;
pub use schema::*;
pub use unknown::{unknown_keys, unknown_keys_in};
