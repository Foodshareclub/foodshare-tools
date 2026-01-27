//! Endpoint-specific API implementations
//!
//! Each module provides a typed interface for a specific set of backend endpoints.
//!
//! ## Mapping to foodshare-backend
//!
//! | Module | Backend Function | Description |
//! |--------|-----------------|-------------|
//! | `translations` | `get-translations` | Legacy standalone translation function |
//! | `localization` | `localization` | Consolidated localization service |
//! | `products` | `api-v1-products` | Products/listings CRUD API |
//! | `health` | `health`, `health-advanced` | Health check endpoints |
//! | `bff` | `bff` | Backend-for-frontend aggregation |

pub mod bff;
pub mod health;
pub mod localization;
pub mod products;
pub mod translations;

pub use bff::BffApi;
pub use health::HealthApi;
pub use localization::LocalizationApi;
pub use products::ProductsApi;
pub use translations::TranslationsApi;
