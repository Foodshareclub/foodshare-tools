//! WebAssembly builds for the published Foodshare crates.
//!
//! The five crates that ship to the browser are compiled with
//! `wasm-pack`, and the generated loaders are patched so Next.js can find
//! the `.wasm` files after bundling.
//!
//! # Example
//!
//! ```rust,no_run
//! use foodshare_wasm::{build, loader, Layout};
//! use std::path::PathBuf;
//!
//! # async fn run() {
//! let layout = Layout {
//!     tools_dir: PathBuf::from("foodshare-tools"),
//!     web_dir: PathBuf::from("foodshare-web"),
//! };
//!
//! let outcomes = build::build_all(&layout, loader::CRATES).await;
//! for outcome in outcomes {
//!     println!("{}: {}", outcome.crate_name, outcome.success);
//! }
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod build;
pub mod loader;

pub use build::{BuildOutcome, Layout};
pub use loader::Crate;
