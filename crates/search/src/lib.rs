//! High-performance fuzzy search for FoodShare.
//!
//! This crate provides:
//! - Multi-level relevance scoring
//! - Levenshtein edit distance
//! - Unicode-aware tokenization
//! - Thread-safe caching

mod relevance;
mod fuzzy;
mod error;

#[cfg(feature = "wasm")]
mod wasm;

pub use relevance::{calculate_relevance, RelevanceScore};
pub use fuzzy::{fuzzy_match, levenshtein_distance};
pub use error::{SearchError, Result};

/// Search result with relevance score.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult<T> {
    /// The matched item
    pub item: T,
    /// Relevance score (higher is better)
    pub score: u32,
}
