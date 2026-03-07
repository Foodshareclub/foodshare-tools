//! Cryptographic utilities for FoodShare.
//!
//! This crate provides:
//! - HMAC signature generation and verification
//! - Constant-time comparison for security
//! - Provider-specific webhook verification (Meta, Stripe, GitHub)

#![warn(missing_docs)]

mod error;
mod hmac_impl;
mod timing;

#[cfg(feature = "wasm")]
mod wasm;

pub use error::{CryptoError, Result};
pub use hmac_impl::{hmac_sha1, hmac_sha256, verify_signature};
pub use timing::constant_time_compare;
