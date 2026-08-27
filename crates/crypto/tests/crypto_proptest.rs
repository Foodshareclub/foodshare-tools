//! Property-based testing for cryptography and TOTP algorithms.

use foodshare_crypto::{constant_time_compare, generate_totp, hmac_sha256, verify_totp};
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_constant_time_compare_reflexive(
        data in prop::collection::vec(any::<u8>(), 0..128),
    ) {
        prop_assert!(constant_time_compare(&data, &data));
    }

    #[test]
    fn prop_constant_time_compare_symmetry(
        a in prop::collection::vec(any::<u8>(), 0..64),
        b in prop::collection::vec(any::<u8>(), 0..64),
    ) {
        let cmp_ab = constant_time_compare(&a, &b);
        let cmp_ba = constant_time_compare(&b, &a);
        prop_assert_eq!(cmp_ab, cmp_ba);
    }

    #[test]
    fn prop_totp_code_length_and_digits(
        secret in prop::collection::vec(any::<u8>(), 10..32),
        timestamp in 0u64..2_000_000_000,
    ) {
        let code = generate_totp(&secret, timestamp, 30, 6).expect("valid totp");
        prop_assert_eq!(code.len(), 6);
        prop_assert!(code.chars().all(|c| c.is_ascii_digit()));

        // Verification must succeed at exact timestamp
        prop_assert!(verify_totp(&secret, &code, timestamp, 0));
    }

    #[test]
    fn prop_hmac_sha256_deterministic(
        key in prop::collection::vec(any::<u8>(), 1..64),
        msg in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let sig1 = hmac_sha256(&key, &msg);
        let sig2 = hmac_sha256(&key, &msg);
        prop_assert_eq!(sig1.clone(), sig2);
        prop_assert_eq!(sig1.len(), 64);
    }
}
