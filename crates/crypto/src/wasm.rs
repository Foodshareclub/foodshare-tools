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

/// Generate a 6-digit TOTP MFA token from raw or base32 secret.
#[wasm_bindgen]
pub fn generate_totp_code(secret: &str, time_seconds: Option<u64>) -> Result<String, JsValue> {
    let now = time_seconds.unwrap_or_else(|| {
        (js_sys::Date::now() / 1000.0) as u64
    });
    crate::generate_totp(secret.as_bytes(), now, crate::DEFAULT_TIME_STEP, crate::DEFAULT_DIGITS)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Verify a user-entered TOTP token with time drift window.
#[wasm_bindgen]
pub fn verify_totp_code(
    secret: &str,
    code: &str,
    time_seconds: Option<u64>,
    window_steps: Option<i64>,
) -> bool {
    let now = time_seconds.unwrap_or_else(|| {
        (js_sys::Date::now() / 1000.0) as u64
    });
    let window = window_steps.unwrap_or(1);
    crate::verify_totp(secret.as_bytes(), code, now, window)
}

/// Generate standard otpauth URI for MFA QR Code generation.
#[wasm_bindgen]
pub fn build_totp_uri(account_name: &str, issuer: &str, base32_secret: &str) -> String {
    crate::build_otpauth_uri(account_name, issuer, base32_secret)
}
