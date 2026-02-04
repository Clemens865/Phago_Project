//! Semantic similarity utilities for vector embeddings.
//!
//! Provides cosine similarity computation and semantic wiring logic
//! for the knowledge graph. When nodes have embeddings, edge weights
//! can be modulated by semantic similarity.

/// Compute cosine similarity between two vectors.
///
/// Returns a value in [-1, 1] where:
/// - 1.0 = identical direction
/// - 0.0 = orthogonal
/// - -1.0 = opposite direction
///
/// Returns None if vectors have different lengths or are empty/zero-norm.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f64> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }

    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;

    for (&ai, &bi) in a.iter().zip(b.iter()) {
        dot += ai as f64 * bi as f64;
        norm_a += (ai as f64) * (ai as f64);
        norm_b += (bi as f64) * (bi as f64);
    }

    let norm = (norm_a * norm_b).sqrt();
    if norm == 0.0 {
        return None;
    }

    Some(dot / norm)
}

/// Compute similarity between two embeddings, normalized to [0, 1].
///
/// Uses cosine similarity internally but maps the result from [-1, 1] to [0, 1]
/// using: `(cosine + 1) / 2`
///
/// This is more suitable for edge weights which should be non-negative.
pub fn normalized_similarity(a: &[f32], b: &[f32]) -> Option<f64> {
    cosine_similarity(a, b).map(|cos| (cos + 1.0) / 2.0)
}

/// Configuration for semantic wiring.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticWiringConfig {
    /// Minimum similarity threshold for creating/strengthening edges.
    /// Edges between concepts with similarity below this are not created.
    pub min_similarity: f64,

    /// Weight multiplier for semantic similarity.
    /// Final weight = base_weight * (1 + similarity_influence * similarity)
    pub similarity_influence: f64,

    /// Whether to require both nodes to have embeddings.
    /// If false, edges between nodes without embeddings use base weight only.
    pub require_embeddings: bool,
}

impl Default for SemanticWiringConfig {
    fn default() -> Self {
        Self {
            min_similarity: 0.0,
            similarity_influence: 0.5,
            require_embeddings: false,
        }
    }
}

impl SemanticWiringConfig {
    /// Create a strict configuration that only wires semantically similar concepts.
    pub fn strict() -> Self {
        Self {
            min_similarity: 0.3,
            similarity_influence: 1.0,
            require_embeddings: true,
        }
    }

    /// Create a relaxed configuration that uses similarity as a boost.
    pub fn relaxed() -> Self {
        Self {
            min_similarity: 0.0,
            similarity_influence: 0.3,
            require_embeddings: false,
        }
    }
}

/// Compute the edge weight based on base weight and semantic similarity.
///
/// If both nodes have embeddings and similarity meets the threshold:
/// `weight = base_weight * (1 + similarity_influence * similarity)`
///
/// If embeddings are missing and `require_embeddings` is false:
/// `weight = base_weight`
///
/// If embeddings are missing and `require_embeddings` is true:
/// Returns None (edge should not be created).
pub fn compute_semantic_weight(
    base_weight: f64,
    embedding_a: Option<&[f32]>,
    embedding_b: Option<&[f32]>,
    config: &SemanticWiringConfig,
) -> Option<f64> {
    match (embedding_a, embedding_b) {
        (Some(a), Some(b)) => {
            let similarity = normalized_similarity(a, b)?;
            if similarity < config.min_similarity {
                if config.require_embeddings {
                    return None;
                }
                // Below threshold but not requiring similarity — use base weight
                return Some(base_weight);
            }
            // Boost weight based on similarity
            let boosted = base_weight * (1.0 + config.similarity_influence * similarity);
            Some(boosted.min(1.0))
        }
        _ => {
            if config.require_embeddings {
                None
            } else {
                Some(base_weight)
            }
        }
    }
}

/// Compute L2 distance between two vectors.
pub fn l2_distance(a: &[f32], b: &[f32]) -> Option<f64> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }

    let sum: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(&ai, &bi)| {
            let diff = ai as f64 - bi as f64;
            diff * diff
        })
        .sum();

    Some(sum.sqrt())
}

/// Compute dot product between two vectors.
pub fn dot_product(a: &[f32], b: &[f32]) -> Option<f64> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }

    let dot: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(&ai, &bi)| ai as f64 * bi as f64)
        .sum();

    Some(dot)
}

/// L2 normalize a vector in place.
pub fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// L2 normalize a vector, returning a new vector.
pub fn l2_normalized(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter().map(|x| x / norm).collect()
    } else {
        v.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!((sim + 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!(cosine_similarity(&a, &b).is_none());
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!(cosine_similarity(&a, &b).is_none());
    }

    #[test]
    fn normalized_similarity_maps_to_zero_one() {
        // Identical = 1.0
        let a = vec![1.0, 2.0, 3.0];
        assert!((normalized_similarity(&a, &a).unwrap() - 1.0).abs() < 1e-6);

        // Opposite = 0.0
        let b = vec![-1.0, -2.0, -3.0];
        assert!(normalized_similarity(&a, &b).unwrap().abs() < 1e-6);

        // Orthogonal = 0.5
        let c = vec![1.0, 0.0];
        let d = vec![0.0, 1.0];
        assert!((normalized_similarity(&c, &d).unwrap() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn semantic_weight_with_embeddings() {
        let config = SemanticWiringConfig::default();
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.9, 0.1, 0.0]; // Similar to a

        let weight = compute_semantic_weight(0.1, Some(&a), Some(&b), &config).unwrap();
        // Similarity is high, so weight should be boosted
        assert!(weight > 0.1);
    }

    #[test]
    fn semantic_weight_without_embeddings_relaxed() {
        let config = SemanticWiringConfig::relaxed();
        let weight = compute_semantic_weight(0.1, None, None, &config).unwrap();
        assert!((weight - 0.1).abs() < 1e-6);
    }

    #[test]
    fn semantic_weight_without_embeddings_strict() {
        let config = SemanticWiringConfig::strict();
        let weight = compute_semantic_weight(0.1, None, None, &config);
        assert!(weight.is_none());
    }

    #[test]
    fn semantic_weight_below_threshold() {
        let config = SemanticWiringConfig {
            min_similarity: 0.9,
            similarity_influence: 1.0,
            require_embeddings: true,
        };
        // Orthogonal vectors have similarity 0.5
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let weight = compute_semantic_weight(0.1, Some(&a), Some(&b), &config);
        assert!(weight.is_none());
    }

    #[test]
    fn l2_distance_works() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((l2_distance(&a, &b).unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn l2_normalize_works() {
        let mut v = vec![3.0, 4.0];
        l2_normalize(&mut v);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn dot_product_works() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        assert!((dot_product(&a, &b).unwrap() - 32.0).abs() < 1e-6);
    }
}
