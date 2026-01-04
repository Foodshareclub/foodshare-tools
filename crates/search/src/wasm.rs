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
