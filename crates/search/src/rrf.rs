//! Reciprocal Rank Fusion (RRF) for combining multiple ranked search results (semantic + keyword).

use std::collections::HashMap;
use std::hash::Hash;

/// Default RRF smoothing constant (standard value used in search engines).
pub const DEFAULT_RRF_K: f32 = 60.0;

/// Ranked item scored by RRF.
#[derive(Debug, Clone, PartialEq)]
pub struct RankedResult<T> {
    /// The underlying item or identifier
    pub item: T,
    /// Computed RRF score
    pub score: f32,
}

/// Apply Reciprocal Rank Fusion over multiple ranked result sets.
///
/// # Arguments
/// * `ranked_lists` - A slice of ranked result vectors (ordered best to worst)
/// * `k` - The RRF constant (default: 60.0)
///
/// # Returns
/// A single merged and descending-sorted vector of results with their aggregated RRF score.
pub fn apply_rrf<T: Clone + Eq + Hash>(ranked_lists: &[Vec<T>], k: f32) -> Vec<RankedResult<T>> {
    let mut scores: HashMap<T, f32> = HashMap::new();

    for list in ranked_lists {
        for (rank_idx, item) in list.iter().enumerate() {
            let rank = (rank_idx + 1) as f32;
            let rrf_score = 1.0 / (k + rank);
            *scores.entry(item.clone()).or_insert(0.0) += rrf_score;
        }
    }

    let mut results: Vec<RankedResult<T>> = scores
        .into_iter()
        .map(|(item, score)| RankedResult { item, score })
        .collect();

    // Sort descending by score
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_single_list() {
        let list = vec!["item1", "item2", "item3"];
        let fused = apply_rrf(&[list], DEFAULT_RRF_K);
        assert_eq!(fused.len(), 3);
        assert_eq!(fused[0].item, "item1");
        assert_eq!(fused[1].item, "item2");
        assert_eq!(fused[2].item, "item3");
        assert!(fused[0].score > fused[1].score);
    }

    #[test]
    fn test_rrf_multi_list_fusion() {
        // List A (semantic): item1 > item2 > item3
        let list_a = vec!["item1", "item2", "item3"];
        // List B (keyword): item2 > item1 > item4
        let list_b = vec!["item2", "item1", "item4"];

        let fused = apply_rrf(&[list_a, list_b], DEFAULT_RRF_K);

        // Both item1 and item2 appeared in top 2 across both lists, but item1: rank 1 + rank 2, item2: rank 2 + rank 1 => identical score
        assert_eq!(fused.len(), 4);
        assert!((fused[0].score - fused[1].score).abs() < 1e-5);
        // item3 and item4 were only in one list, so they rank lower
        assert!(fused[0].score > fused[2].score);
        assert!(fused[2].score > 0.0);
    }
}
