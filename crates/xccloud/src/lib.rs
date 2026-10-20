//! Xcode Cloud build runs, logs and diagnostics.
//!
//! The tool talks to the App Store Connect API: the CI products of the
//! team, their build runs, the actions of a run, and the issues and
//! artefacts an action produced. When the credentials are not available
//! it falls back to the local build summaries CI leaves behind.
//!
//! # Example
//!
//! ```rust,no_run
//! use foodshare_xccloud::{auth, api};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let locations = auth::default_locations(std::path::Path::new("foodshare-app"));
//! let credentials = auth::resolve(&locations).expect("credentials");
//!
//! let client = api::Client::new(&credentials)?;
//! for product in client.products().await?.data {
//!     println!("{}", product.attributes.name.unwrap_or_default());
//! }
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod api;
pub mod auth;
pub mod diagnostics;
pub mod model;

pub use api::Client;
pub use auth::Credentials;
pub use model::{
    ArtifactAttributes, BuildAttributes, IssueAttributes, ListResponse, ProductAttributes,
};
