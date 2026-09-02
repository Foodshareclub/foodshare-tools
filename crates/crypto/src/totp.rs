//! Time-based One-Time Password (TOTP) algorithm conforming to RFC 6238.
//!
//! Used for Multi-Factor Authentication (MFA) enrollment and verification.

use crate::error::{CryptoError, Result};
use crate::hmac_impl::hmac_sha1_raw;
use crate::timing::constant_time_compare;

/// Default time step in seconds (RFC 6238 standard)
pub const DEFAULT_TIME_STEP: u64 = 30;

/// Default number of OTP digits
pub const DEFAULT_DIGITS: u32 = 6;

/// Generate a 6-digit TOTP code for a given secret key and unix timestamp.
///
/// # Arguments
/// * `secret` - Raw secret bytes (decoded from Base32)
/// * `time_seconds` - Current Unix timestamp in seconds
/// * `time_step` - Step interval in seconds (default: 30)
/// * `digits` - Number of digits (default: 6)
pub fn generate_totp(
    secret: &[u8],
    time_seconds: u64,
    time_step: u64,
    digits: u32,
) -> Result<String> {
    if secret.is_empty() {
        return Err(CryptoError::InvalidKey(
            "Secret cannot be empty".to_string(),
        ));
    }

    let counter = time_seconds / time_step;
    let counter_bytes = counter.to_be_bytes();

    let hmac_result = hmac_sha1_raw(secret, &counter_bytes);

    // Dynamic truncation (RFC 4226 section 5.4)
    let offset = (hmac_result[hmac_result.len() - 1] & 0x0f) as usize;
    if offset + 4 > hmac_result.len() {
        return Err(CryptoError::SignatureMismatch);
    }

    let binary = ((u32::from(hmac_result[offset] & 0x7f)) << 24)
        | ((u32::from(hmac_result[offset + 1])) << 16)
        | ((u32::from(hmac_result[offset + 2])) << 8)
        | (u32::from(hmac_result[offset + 3]));

    let modulus = 10u32.pow(digits);
    let code = binary % modulus;

    Ok(format!("{:0>width$}", code, width = digits as usize))
}

/// Verify a TOTP code with time drift window tolerance (± window steps).
///
/// # Arguments
/// * `secret` - Raw secret bytes
/// * `code` - User-provided code string (e.g. "123456")
/// * `time_seconds` - Current Unix timestamp
/// * `window_steps` - Number of steps to look behind and ahead (e.g. 1 allows ±30s)
pub fn verify_totp(secret: &[u8], code: &str, time_seconds: u64, window_steps: i64) -> bool {
    let clean_code = code.trim().replace(' ', "");
    if clean_code.len() != DEFAULT_DIGITS as usize {
        return false;
    }

    for step_offset in -window_steps..=window_steps {
        let offset_seconds = (step_offset * DEFAULT_TIME_STEP as i64) as i128;
        let candidate_time = (time_seconds as i128 + offset_seconds).max(0) as u64;

        if let Ok(expected) =
            generate_totp(secret, candidate_time, DEFAULT_TIME_STEP, DEFAULT_DIGITS)
        {
            if constant_time_compare(clean_code.as_bytes(), expected.as_bytes()) {
                return true;
            }
        }
    }

    false
}

/// Build standard otpauth URI for QR code generation (Google Authenticator / 1Password / Apple Keychain).
pub fn build_otpauth_uri(account_name: &str, issuer: &str, base32_secret: &str) -> String {
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
        issuer, account_name, base32_secret, issuer, DEFAULT_DIGITS, DEFAULT_TIME_STEP
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Standard RFC 6238 test vectors with secret "12345678901234567890" (ASCII bytes)
    const RFC_SECRET: &[u8] = b"12345678901234567890";

    #[test]
    fn test_rfc6238_vector_59s() {
        // T = 59s -> Code should be "287082"
        let code = generate_totp(RFC_SECRET, 59, 30, 6).unwrap();
        assert_eq!(code, "287082");
    }

    #[test]
    fn test_rfc6238_vector_1111111109s() {
        // T = 1111111109s -> Code should be "081804"
        let code = generate_totp(RFC_SECRET, 1111111109, 30, 6).unwrap();
        assert_eq!(code, "081804");
    }

    #[test]
    fn test_rfc6238_vector_1111111111s() {
        // T = 1111111111s -> Code should be "050471"
        let code = generate_totp(RFC_SECRET, 1111111111, 30, 6).unwrap();
        assert_eq!(code, "050471");
    }

    #[test]
    fn test_verify_totp_with_drift() {
        let code_now = generate_totp(RFC_SECRET, 1000, 30, 6).unwrap();

        // Exact time
        assert!(verify_totp(RFC_SECRET, &code_now, 1000, 1));
        // Within +30s drift window
        assert!(verify_totp(RFC_SECRET, &code_now, 1025, 1));
        // Within -30s drift window
        assert!(verify_totp(RFC_SECRET, &code_now, 975, 1));
        // Invalid code
        assert!(!verify_totp(RFC_SECRET, "000000", 1000, 1));
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_totp_length_and_digits(
            secret in prop::collection::vec(any::<u8>(), 1..64),
            time_seconds in 0u64..10_000_000_000,
        ) {
            let code = generate_totp(&secret, time_seconds, 30, 6).unwrap();
            prop_assert_eq!(code.len(), 6);
            prop_assert!(code.chars().all(|c| c.is_ascii_digit()));
        }

        #[test]
        fn prop_totp_deterministic(
            secret in prop::collection::vec(any::<u8>(), 1..64),
            time_seconds in 0u64..10_000_000_000,
        ) {
            let code1 = generate_totp(&secret, time_seconds, 30, 6).unwrap();
            let code2 = generate_totp(&secret, time_seconds, 30, 6).unwrap();
            prop_assert_eq!(code1, code2);
        }

        #[test]
        fn prop_totp_exact_verification(
            secret in prop::collection::vec(any::<u8>(), 1..64),
            time_seconds in 0u64..10_000_000_000,
        ) {
            let code = generate_totp(&secret, time_seconds, 30, 6).unwrap();
            prop_assert!(verify_totp(&secret, &code, time_seconds, 0));
        }
    }
}
