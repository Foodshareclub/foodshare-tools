//! Cryptographic utilities for FoodShare.
//!
//! This crate provides:
//! - HMAC signature generation and verification
//! - Constant-time comparison for security
//! - Provider-specific webhook verification (Meta, Stripe, GitHub)

#![warn(missing_docs)]

mod hmac_impl;
mod timing;
mod error;

#[cfg(feature = "wasm")]
mod wasm;

pub use hmac_impl::{hmac_sha256, hmac_sha1, verify_signature};
pub use timing::constant_time_compare;
pub use error::{CryptoError, Result};
