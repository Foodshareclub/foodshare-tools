//! High-performance fuzzy search for FoodShare.
//!
//! This crate provides:
//! - Multi-level relevance scoring
//! - Levenshtein edit distance
//! - Unicode-aware tokenization
//! - Thread-safe caching

#![warn(missing_docs)]

mod error;
mod fuzzy;
mod relevance;

#[cfg(feature = "wasm")]
mod wasm;

pub use error::{Result, SearchError};
pub use fuzzy::{fuzzy_match, levenshtein_distance};
pub use relevance::{RelevanceScore, calculate_relevance};

/// Search result with relevance score.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult<T> {
    /// The matched item
    pub item: T,
    /// Relevance score (higher is better)
    pub score: u32,
}
