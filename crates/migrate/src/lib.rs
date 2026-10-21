//! Secret migration library for self-hosted Supabase
//!
//! Provides utilities for migrating secrets between `.env.functions` files
//! and the Supabase vault, using the container runtime's `exec` for
//! database operations.

pub mod env_file;
pub mod error;
pub mod runtime;
pub mod secrets;
pub mod vault;

pub use runtime::Runtime;
