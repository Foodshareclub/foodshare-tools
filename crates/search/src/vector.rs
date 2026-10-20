//! Vector similarity and normalization utilities for pgvector & semantic search.

/// Compute cosine similarity between two float vectors.
///
/// Returns a value between -1.0 and 1.0 (or 0.0 to 1.0 for normalized embeddings).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    let chunks_a = a.chunks_exact(8);
    let chunks_b = b.chunks_exact(8);
    let rem_a = chunks_a.remainder();
    let rem_b = chunks_b.remainder();

    for (ca, cb) in chunks_a.zip(chunks_b) {
        for i in 0..8 {
            let x = ca[i];
            let y = cb[i];
            dot += x * y;
            norm_a += x * x;
            norm_b += y * y;
        }
    }

    for (&x, &y) in rem_a.iter().zip(rem_b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        dot / denominator
    }
}

/// Compute Euclidean (L2) distance between two float vectors.
pub fn l2_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return f32::MAX;
    }

    let mut sum_sq = 0.0f32;
    let chunks_a = a.chunks_exact(8);
    let chunks_b = b.chunks_exact(8);
    let rem_a = chunks_a.remainder();
    let rem_b = chunks_b.remainder();

    for (ca, cb) in chunks_a.zip(chunks_b) {
        for i in 0..8 {
            let diff = ca[i] - cb[i];
            sum_sq += diff * diff;
        }
    }

    for (&x, &y) in rem_a.iter().zip(rem_b.iter()) {
        let diff = x - y;
        sum_sq += diff * diff;
    }

    sum_sq.sqrt()
}

/// Normalize a float vector to unit length (L2 norm = 1.0).
pub fn l2_normalize(v: &mut [f32]) {
    let norm_sq: f32 = v.iter().map(|x| x * x).sum();
    let norm = norm_sq.sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Pad with zeros or truncate vector to match target dimensions (e.g. 384 for gte-small / pgvector).
pub fn normalize_dimensions(v: &[f32], target_dim: usize) -> Vec<f32> {
    if v.len() == target_dim {
        v.to_vec()
    } else if v.len() > target_dim {
        v[..target_dim].to_vec()
    } else {
        let mut padded = v.to_vec();
        padded.resize(target_dim, 0.0);
        padded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 2.0];
        let b = vec![-1.0, -2.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-5);
    }

    #[test]
    fn test_l2_normalize() {
        let mut v = vec![3.0, 4.0];
        l2_normalize(&mut v);
        assert!((v[0] - 0.6).abs() < 1e-5);
        assert!((v[1] - 0.8).abs() < 1e-5);
        let norm = (v[0] * v[0] + v[1] * v[1]).sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_normalize_dimensions() {
        let v = vec![1.0, 2.0, 3.0, 4.0];
        // Truncate to 2
        let truncated = normalize_dimensions(&v, 2);
        assert_eq!(truncated, vec![1.0, 2.0]);

        // Pad to 6
        let padded = normalize_dimensions(&v, 6);
        assert_eq!(padded, vec![1.0, 2.0, 3.0, 4.0, 0.0, 0.0]);
    }
}
