//! Hybrid multi-modal ranking engine.
//!
//! Combines semantic vector similarity, keyword match, and geospatial proximity decay.

use crate::{cosine_similarity, fuzzy_match};
use serde::{Deserialize, Serialize};

/// Configuration weights for hybrid search ranking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridWeights {
    /// Weight for semantic embedding similarity (default: 0.45)
    pub vector_weight: f32,
    /// Weight for keyword/fuzzy match score (default: 0.35)
    pub text_weight: f32,
    /// Weight for geographic proximity decay (default: 0.20)
    pub geo_weight: f32,
    /// Distance half-life in kilometers for exponential decay (default: 10.0 km)
    pub distance_decay_half_life_km: f32,
}

impl Default for HybridWeights {
    fn default() -> Self {
        Self {
            vector_weight: 0.45,
            text_weight: 0.35,
            geo_weight: 0.20,
            distance_decay_half_life_km: 10.0,
        }
    }
}

/// Calculate exponential distance decay score in [0.0, 1.0].
///
/// Score = exp(-0.693147 * distance_km / half_life_km)
/// At distance = 0, score = 1.0.
/// At distance = half_life_km, score = 0.5.
#[inline]
pub fn calculate_distance_decay(distance_km: f32, half_life_km: f32) -> f32 {
    if distance_km < 0.0 || distance_km.is_infinite() {
        return 0.0;
    }
    let half_life = if half_life_km <= 0.0 {
        10.0
    } else {
        half_life_km
    };
    (-0.69314718f32 * (distance_km / half_life)).exp()
}

/// Calculate composite hybrid score for a listing.
#[inline]
pub fn calculate_hybrid_score(
    text_query: &str,
    target_text: &str,
    query_vector: Option<&[f32]>,
    item_vector: Option<&[f32]>,
    distance_km: Option<f32>,
    weights: &HybridWeights,
) -> f32 {
    let mut total_score = 0.0f32;
    let mut total_weight = 0.0f32;

    // 1. Vector similarity component
    if let (Some(qv), Some(iv)) = (query_vector, item_vector) {
        let sim = cosine_similarity(qv, iv).max(0.0).min(1.0);
        total_score += sim * weights.vector_weight;
        total_weight += weights.vector_weight;
    }

    // 2. Keyword/Fuzzy text match component
    if !text_query.is_empty() && !target_text.is_empty() {
        let text_score = if fuzzy_match(target_text, text_query) {
            1.0f32
        } else if target_text
            .to_lowercase()
            .contains(&text_query.to_lowercase())
        {
            0.8f32
        } else {
            0.0f32
        };
        total_score += text_score * weights.text_weight;
        total_weight += weights.text_weight;
    }

    // 3. Geographic proximity decay component
    if let Some(dist) = distance_km {
        let geo_score = calculate_distance_decay(dist, weights.distance_decay_half_life_km);
        total_score += geo_score * weights.geo_weight;
        total_weight += weights.geo_weight;
    }

    if total_weight > 0.0 {
        total_score / total_weight
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_decay_at_zero() {
        let decay = calculate_distance_decay(0.0, 10.0);
        assert!((decay - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_distance_decay_at_halflife() {
        let decay = calculate_distance_decay(10.0, 10.0);
        assert!((decay - 0.5).abs() < 1e-3);
    }

    #[test]
    fn test_hybrid_score_perfect_match() {
        let q_vec = vec![1.0, 0.0, 0.0];
        let weights = HybridWeights::default();
        let score = calculate_hybrid_score(
            "apples",
            "fresh apples",
            Some(&q_vec),
            Some(&q_vec),
            Some(0.0),
            &weights,
        );
        assert!(score > 0.95);
    }

    #[test]
    fn test_hybrid_score_far_away_lowers_score() {
        let q_vec = vec![1.0, 0.0, 0.0];
        let weights = HybridWeights::default();
        let score_near = calculate_hybrid_score(
            "apples",
            "apples",
            Some(&q_vec),
            Some(&q_vec),
            Some(1.0),
            &weights,
        );
        let score_far = calculate_hybrid_score(
            "apples",
            "apples",
            Some(&q_vec),
            Some(&q_vec),
            Some(100.0),
            &weights,
        );
        assert!(score_near > score_far);
    }
}
