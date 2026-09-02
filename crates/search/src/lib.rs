//! High-performance search and ranking algorithms for FoodShare.
//!
//! This crate provides:
//! - Multi-level relevance scoring
//! - Levenshtein edit distance & fuzzy matching
//! - Vector cosine similarity & L2 distance calculation
//! - Dimension normalization (384d / 1536d)
//! - Reciprocal Rank Fusion (RRF) for hybrid search
//! - Unicode-aware tokenization

#![warn(missing_docs)]

mod error;
mod fuzzy;
pub mod hybrid;
mod relevance;
pub mod rrf;
pub mod vector;

#[cfg(feature = "wasm")]
mod wasm;

pub use error::{Result, SearchError};
pub use fuzzy::{fuzzy_match, levenshtein_distance};
pub use hybrid::{HybridWeights, calculate_distance_decay, calculate_hybrid_score};
pub use relevance::{RelevanceScore, calculate_relevance};
pub use rrf::{DEFAULT_RRF_K, RankedResult, apply_rrf};
pub use vector::{cosine_similarity, l2_distance, l2_normalize, normalize_dimensions};

/// Search result with relevance score.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult<T> {
    /// The matched item
    pub item: T,
    /// Relevance score (higher is better)
    pub score: u32,
}
