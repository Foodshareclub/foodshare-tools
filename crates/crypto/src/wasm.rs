//! WASM bindings for crypto utilities.

use wasm_bindgen::prelude::*;

/// Generate HMAC-SHA256 signature and return as hex string.
#[wasm_bindgen]
pub fn hmac_sha256_hex(key: &str, message: &str) -> String {
    crate::hmac_sha256(key.as_bytes(), message.as_bytes())
}

/// Generate HMAC-SHA256 signature and return as base64 string.
#[wasm_bindgen]
pub fn hmac_sha256_base64(key: &str, message: &str) -> String {
    use base64::Engine;
    let signature_hex = crate::hmac_sha256(key.as_bytes(), message.as_bytes());
    // Convert hex to bytes then to base64
    if let Ok(bytes) = hex::decode(&signature_hex) {
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    } else {
        String::new()
    }
}

/// Generate HMAC-SHA1 signature and return as hex string.
#[wasm_bindgen]
pub fn hmac_sha1_hex(key: &str, message: &str) -> String {
    crate::hmac_sha1(key.as_bytes(), message.as_bytes())
}

/// Verify a webhook signature (constant-time comparison).
///
/// # Arguments
/// * `key` - The secret key
/// * `message` - The message/payload
/// * `signature_hex` - The expected signature in hex format
///
/// # Returns
/// true if signature matches, false otherwise
#[wasm_bindgen]
pub fn verify_webhook_sha256(key: &str, message: &str, signature_hex: &str) -> bool {
    let expected = crate::hmac_sha256(key.as_bytes(), message.as_bytes());
    crate::constant_time_compare(expected.as_bytes(), signature_hex.as_bytes())
}

/// Verify a signature with SHA1 (for legacy providers like GitHub).
#[wasm_bindgen]
pub fn verify_webhook_sha1(key: &str, message: &str, signature_hex: &str) -> bool {
    let expected = crate::hmac_sha1(key.as_bytes(), message.as_bytes());
    crate::constant_time_compare(expected.as_bytes(), signature_hex.as_bytes())
}

/// Constant-time comparison of two strings.
#[wasm_bindgen]
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    crate::constant_time_compare(a.as_bytes(), b.as_bytes())
}
