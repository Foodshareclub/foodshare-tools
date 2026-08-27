//! WASM bindings for search utilities.

use wasm_bindgen::prelude::*;

/// Calculate relevance score for a query against text.
///
/// # Arguments
/// * `query` - Search query
/// * `text` - Text to match against
///
/// # Returns
/// Relevance score (0-50, higher is better)
#[wasm_bindgen]
pub fn relevance_score(query: &str, text: &str) -> u32 {
    crate::calculate_relevance(text, query)
}

/// Check if text contains a fuzzy match for query.
///
/// Returns true if all characters in query appear in text in order.
#[wasm_bindgen]
pub fn fuzzy_contains(query: &str, text: &str) -> bool {
    crate::fuzzy_match(text, query)
}

/// Calculate Levenshtein edit distance between two strings.
#[wasm_bindgen]
pub fn edit_distance(a: &str, b: &str) -> usize {
    crate::levenshtein_distance(a, b)
}

/// Search items and return sorted results as JSON.
///
/// # Arguments
/// * `query` - Search query
/// * `items_json` - JSON array of items with `id` and `text` fields
/// * `max_results` - Maximum results to return (0 for all)
///
/// # Returns
/// JSON array of results with `id` and `score` fields, sorted by score
#[wasm_bindgen]
pub fn search_items(query: &str, items_json: &str, max_results: usize) -> String {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    struct Item {
        id: String,
        text: String,
    }

    #[derive(Serialize)]
    struct Result {
        id: String,
        score: u32,
    }

    let items: Vec<Item> = match serde_json::from_str(items_json) {
        Ok(items) => items,
        Err(_) => return "[]".to_string(),
    };

    let mut results: Vec<Result> = items
        .into_iter()
        .map(|item| {
            let score = crate::calculate_relevance(&item.text, query);
            Result { id: item.id, score }
        })
        .filter(|r| r.score > 0)
        .collect();

    results.sort_by(|a, b| b.score.cmp(&a.score));

    if max_results > 0 {
        results.truncate(max_results);
    }

    serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
}

/// Calculate cosine similarity between two float arrays in WebAssembly.
#[wasm_bindgen]
pub fn vector_cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    crate::cosine_similarity(a, b)
}

/// Calculate Euclidean (L2) distance between two float arrays in WebAssembly.
#[wasm_bindgen]
pub fn vector_l2_distance(a: &[f32], b: &[f32]) -> f32 {
    crate::l2_distance(a, b)
}

/// Pad or truncate vector to exact target dimensions (e.g. 384 for gte-small).
#[wasm_bindgen]
pub fn vector_normalize_dimensions(v: &[f32], target_dim: usize) -> Vec<f32> {
    crate::normalize_dimensions(v, target_dim)
}

/// Merge ranked result lists using Reciprocal Rank Fusion in WebAssembly.
///
/// # Arguments
/// * `lists_json` - JSON 2D array of item IDs: `[["item1", "item2"], ["item2", "item3"]]`
/// * `k` - RRF smoothing parameter (default 60.0)
#[wasm_bindgen]
pub fn rrf_merge(lists_json: &str, k: Option<f32>) -> String {
    let lists: Vec<Vec<String>> = match serde_json::from_str(lists_json) {
        Ok(l) => l,
        Err(_) => return "[]".to_string(),
    };

    let k_val = k.unwrap_or(crate::DEFAULT_RRF_K);
    let ranked = crate::apply_rrf(&lists, k_val);

    #[derive(serde::Serialize)]
    struct RrfOutput {
        id: String,
        score: f32,
    }

    let output: Vec<RrfOutput> = ranked
        .into_iter()
        .map(|r| RrfOutput {
            id: r.item,
            score: r.score,
        })
        .collect();

    serde_json::to_string(&output).unwrap_or_else(|_| "[]".to_string())
}

/// Calculate multi-modal hybrid score in WebAssembly.
#[wasm_bindgen]
pub fn hybrid_score(
    text_query: &str,
    target_text: &str,
    query_vector: Option<Vec<f32>>,
    item_vector: Option<Vec<f32>>,
    distance_km: Option<f32>,
    vector_weight: Option<f32>,
    text_weight: Option<f32>,
    geo_weight: Option<f32>,
    half_life_km: Option<f32>,
) -> f32 {
    let weights = crate::HybridWeights {
        vector_weight: vector_weight.unwrap_or(0.45),
        text_weight: text_weight.unwrap_or(0.35),
        geo_weight: geo_weight.unwrap_or(0.20),
        distance_decay_half_life_km: half_life_km.unwrap_or(10.0),
    };

    crate::calculate_hybrid_score(
        text_query,
        target_text,
        query_vector.as_deref(),
        item_vector.as_deref(),
        distance_km,
        &weights,
    )
}

/// Calculate exponential geospatial proximity decay in WebAssembly.
#[wasm_bindgen]
pub fn distance_decay(distance_km: f32, half_life_km: Option<f32>) -> f32 {
    crate::calculate_distance_decay(distance_km, half_life_km.unwrap_or(10.0))
}
