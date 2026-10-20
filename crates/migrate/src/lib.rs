//! Secret migration library for self-hosted Supabase
//!
//! Provides utilities for migrating secrets between `.env.functions` files
//! and the Supabase vault, using `docker exec psql` for database operations.

pub mod env_file;
pub mod error;
pub mod secrets;
pub mod vault;
