//! HMAC implementations for various hash algorithms.

use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::Sha256;

use crate::{CryptoError, Result};

type HmacSha256 = Hmac<Sha256>;
type HmacSha1 = Hmac<Sha1>;

/// Generate HMAC-SHA256 signature.
///
/// # Arguments
/// * `key` - Secret key bytes
/// * `message` - Message to sign
///
/// # Returns
/// Signature as hex string
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(key)
        .expect("HMAC can take key of any size");
    mac.update(message);
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Generate HMAC-SHA1 signature.
///
/// # Arguments
/// * `key` - Secret key bytes
/// * `message` - Message to sign
///
/// # Returns
/// Signature as hex string
pub fn hmac_sha1(key: &[u8], message: &[u8]) -> String {
    let mut mac = HmacSha1::new_from_slice(key)
        .expect("HMAC can take key of any size");
    mac.update(message);
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Verify a signature against an expected value.
///
/// # Arguments
/// * `signature` - The signature to verify (hex-encoded)
/// * `expected` - The expected signature (hex-encoded)
///
/// # Returns
/// Ok(()) if signatures match, Err otherwise
pub fn verify_signature(signature: &str, expected: &str) -> Result<()> {
    if crate::constant_time_compare(signature.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(CryptoError::SignatureMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha256() {
        let key = b"secret";
        let message = b"hello world";
        let sig = hmac_sha256(key, message);

        // Verify it's a valid hex string of correct length (64 chars for SHA256)
        assert_eq!(sig.len(), 64);
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_hmac_sha1() {
        let key = b"secret";
        let message = b"hello world";
        let sig = hmac_sha1(key, message);

        // SHA1 produces 40 hex chars
        assert_eq!(sig.len(), 40);
    }

    #[test]
    fn test_verify_signature_match() {
        let sig = "abc123";
        assert!(verify_signature(sig, sig).is_ok());
    }

    #[test]
    fn test_verify_signature_mismatch() {
        assert!(verify_signature("abc123", "def456").is_err());
    }
}
