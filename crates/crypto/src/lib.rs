//! Cryptographic utilities for FoodShare.
//!
//! This crate provides:
//! - HMAC signature generation and verification (SHA256, SHA1)
//! - Constant-time comparison for security against timing attacks
//! - RFC 6238 Time-based One-Time Password (TOTP / MFA) verification & QR URI generation
//! - Provider-specific webhook verification (Meta, Stripe, GitHub)

#![warn(missing_docs)]

mod error;
mod hmac_impl;
mod timing;
pub mod totp;

#[cfg(feature = "wasm")]
mod wasm;

pub use error::{CryptoError, Result};
pub use hmac_impl::{hmac_sha1, hmac_sha256, verify_signature};
pub use timing::constant_time_compare;
pub use totp::{build_otpauth_uri, generate_totp, verify_totp, DEFAULT_DIGITS, DEFAULT_TIME_STEP};
