//! Fuzzy matching algorithms.

/// Calculate Levenshtein edit distance between two strings.
///
/// # Arguments
/// * `a` - First string
/// * `b` - Second string
///
/// # Returns
/// Number of single-character edits needed to transform a into b
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 { return n; }
    if n == 0 { return m; }

    // Use two rows for space optimization
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Check if text contains all characters of query in order.
///
/// This is a simple fuzzy match that checks if all query characters
/// appear in the text in the same order (but not necessarily consecutively).
///
/// # Arguments
/// * `text` - Text to search in
/// * `query` - Query characters to find
///
/// # Returns
/// true if all query characters are found in order
pub fn fuzzy_match(text: &str, query: &str) -> bool {
    let mut text_chars = text.chars().peekable();

    for query_char in query.chars() {
        loop {
            match text_chars.next() {
                Some(c) if c == query_char => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_same() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_one_edit() {
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
    }

    #[test]
    fn test_levenshtein_insert() {
        assert_eq!(levenshtein_distance("helo", "hello"), 1);
    }

    #[test]
    fn test_levenshtein_delete() {
        assert_eq!(levenshtein_distance("hello", "helo"), 1);
    }

    #[test]
    fn test_fuzzy_match_true() {
        assert!(fuzzy_match("hello world", "hwo"));
    }

    #[test]
    fn test_fuzzy_match_false() {
        assert!(!fuzzy_match("hello", "lhe"));
    }

    #[test]
    fn test_fuzzy_match_exact() {
        assert!(fuzzy_match("hello", "hello"));
    }
}
