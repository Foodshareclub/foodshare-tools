//! Relevance scoring for search results.

/// Relevance score levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelevanceScore {
    /// No match
    None = 0,
    /// Fuzzy match
    Fuzzy = 10,
    /// Contains substring
    Contains = 20,
    /// Word boundary match
    WordBoundary = 30,
    /// Starts with query
    StartsWith = 40,
    /// Exact match
    Exact = 50,
}

/// Calculate relevance score for a text against a query.
///
/// # Arguments
/// * `text` - The text to score
/// * `query` - The search query
///
/// # Returns
/// Relevance score (higher is better)
pub fn calculate_relevance(text: &str, query: &str) -> u32 {
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    // Exact match
    if text_lower == query_lower {
        return RelevanceScore::Exact as u32;
    }

    // Starts with
    if text_lower.starts_with(&query_lower) {
        return RelevanceScore::StartsWith as u32;
    }

    // Word boundary match
    for word in text_lower.split_whitespace() {
        if word.starts_with(&query_lower) {
            return RelevanceScore::WordBoundary as u32;
        }
    }

    // Contains
    if text_lower.contains(&query_lower) {
        return RelevanceScore::Contains as u32;
    }

    // Fuzzy match (simple character matching)
    if crate::fuzzy_match(&text_lower, &query_lower) {
        return RelevanceScore::Fuzzy as u32;
    }

    RelevanceScore::None as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert_eq!(calculate_relevance("Hello", "hello"), RelevanceScore::Exact as u32);
    }

    #[test]
    fn test_starts_with() {
        assert_eq!(calculate_relevance("Hello World", "hello"), RelevanceScore::StartsWith as u32);
    }

    #[test]
    fn test_word_boundary() {
        assert_eq!(calculate_relevance("Say Hello", "hello"), RelevanceScore::WordBoundary as u32);
    }

    #[test]
    fn test_contains() {
        assert_eq!(calculate_relevance("SayHelloWorld", "hello"), RelevanceScore::Contains as u32);
    }
}
