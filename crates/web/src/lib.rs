//! Web-specific tools for Foodshare (Next.js/React)
//!
//! This crate provides web-specific functionality:
//! - Next.js security scanning (OWASP)
//! - Bundle size analysis
//! - Accessibility checks
//! - Import organization

#![warn(missing_docs)]

pub mod accessibility;
pub mod bundle_size;
pub mod nextjs_security;
