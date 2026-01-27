//! Centralized API client for Foodshare backend services
//!
//! This crate provides a unified, resilient HTTP client for interacting with
//! Foodshare backend services including Supabase Edge Functions and BFF endpoints.
//!
//! # Features
//!
//! - **Environment-based configuration**: Load URLs and keys from environment variables
//! - **Retry with exponential backoff**: Automatic retry for transient failures
//! - **Circuit breaker**: Prevent cascading failures during outages
//! - **Rate limiting**: Avoid hitting API throttling limits
//! - **Request correlation**: Track requests with unique IDs for debugging
//!
//! # Example
//!
//! ```rust,no_run
//! use foodshare_api_client::{FoodshareClient, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client with environment configuration
//!     let client = FoodshareClient::new()?;
//!
//!     // Check health
//!     let health = client.health().check().await?;
//!     println!("Service status: {}", health.status);
//!
//!     // Fetch translations
//!     let translations = client.translations().get("en").await?;
//!     println!("Got {} keys", translations.data.map(|d| d.messages).is_some());
//!
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod client;
pub mod config;
pub mod endpoints;
pub mod error;
pub mod middleware;

pub use client::FoodshareClient;
pub use config::{ClientConfig, Environment};
pub use error::{ApiError, ApiResult};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::client::FoodshareClient;
    pub use crate::config::{ClientConfig, Environment};
    pub use crate::endpoints::{BffApi, HealthApi, LocalizationApi, ProductsApi, TranslationsApi};
    pub use crate::error::{ApiError, ApiResult};
}
