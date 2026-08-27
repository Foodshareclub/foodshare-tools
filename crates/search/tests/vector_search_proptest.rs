//! Property-based testing for vector search and ranking algorithms.

use foodshare_search::{
    apply_rrf, cosine_similarity, l2_distance, l2_normalize, normalize_dimensions,
    DEFAULT_RRF_K,
};
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_cosine_similarity_bounds(
        a in prop::collection::vec(-100.0f32..100.0f32, 1..64),
        b in prop::collection::vec(-100.0f32..100.0f32, 1..64),
    ) {
        if a.len() == b.len() {
            let sim = cosine_similarity(&a, &b);
            // Floating point rounding can occasionally slightly exceed [-1.0, 1.0] by epsilon
            prop_assert!(sim >= -1.0001 && sim <= 1.0001);
        }
    }

    #[test]
    fn prop_l2_distance_non_negative(
        a in prop::collection::vec(-100.0f32..100.0f32, 1..64),
        b in prop::collection::vec(-100.0f32..100.0f32, 1..64),
    ) {
        if a.len() == b.len() {
            let dist = l2_distance(&a, &b);
            prop_assert!(dist >= 0.0);
        }
    }

    #[test]
    fn prop_normalize_dimensions_exact_length(
        v in prop::collection::vec(-10.0f32..10.0f32, 0..100),
        target_dim in 1usize..500,
    ) {
        let normalized = normalize_dimensions(&v, target_dim);
        prop_assert_eq!(normalized.len(), target_dim);
    }

    #[test]
    fn prop_l2_normalize_unit_length(
        mut v in prop::collection::vec(-50.0f32..50.0f32, 1..32),
    ) {
        let norm_sq: f32 = v.iter().map(|x| x * x).sum();
        if norm_sq > 0.0001 {
            l2_normalize(&mut v);
            let final_norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            prop_assert!((final_norm - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn prop_rrf_monotonicity(
        items in prop::collection::vec("[a-z0-9]{3,8}", 1..20),
    ) {
        let fused = apply_rrf(&[items], DEFAULT_RRF_K);
        for i in 1..fused.len() {
            prop_assert!(fused[i - 1].score >= fused[i].score);
        }
    }
}
