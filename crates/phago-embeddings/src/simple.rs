//! Simple hash-based embedder (no external dependencies).
//!
//! This embedder uses a hash-based approach to create fixed-dimension vectors.
//! While not as semantically rich as neural embeddings, it provides a fast
//! baseline that works without any ML models.

use crate::{Embedder, EmbeddingError, EmbeddingResult};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Simple hash-based embedder.
///
/// Creates embeddings by hashing words into a fixed-dimension space.
/// Uses multiple hash functions for better distribution.
///
/// # Example
///
/// ```rust
/// use phago_embeddings::SimpleEmbedder;
/// use phago_embeddings::Embedder;
///
/// let embedder = SimpleEmbedder::new(128);
/// let vec = embedder.embed("hello world").unwrap();
/// assert_eq!(vec.len(), 128);
/// ```
pub struct SimpleEmbedder {
    dimension: usize,
    num_hashes: usize,
}

impl SimpleEmbedder {
    /// Create a new simple embedder with specified dimension.
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            num_hashes: 4, // Multiple hashes for better distribution
        }
    }

    /// Create with default dimension (256).
    pub fn default_dimension() -> Self {
        Self::new(256)
    }

    /// Tokenize text into words.
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }

    /// Hash a word with a seed to get an index.
    fn hash_with_seed(&self, word: &str, seed: u64) -> usize {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        word.hash(&mut hasher);
        (hasher.finish() as usize) % self.dimension
    }

    /// Hash a word with a seed to get a sign (+1 or -1).
    fn sign_hash(&self, word: &str, seed: u64) -> f32 {
        let mut hasher = DefaultHasher::new();
        (seed + 1000).hash(&mut hasher);
        word.hash(&mut hasher);
        if hasher.finish() % 2 == 0 { 1.0 } else { -1.0 }
    }
}

impl Default for SimpleEmbedder {
    fn default() -> Self {
        Self::default_dimension()
    }
}

impl Embedder for SimpleEmbedder {
    fn embed(&self, text: &str) -> EmbeddingResult<Vec<f32>> {
        if text.is_empty() {
            return Err(EmbeddingError::InvalidInput("Empty text".to_string()));
        }

        let tokens = self.tokenize(text);
        if tokens.is_empty() {
            // Return zero vector for text with no valid tokens
            return Ok(vec![0.0; self.dimension]);
        }

        let mut vector = vec![0.0f32; self.dimension];

        // Use multiple hash functions for each token
        for token in &tokens {
            for seed in 0..self.num_hashes as u64 {
                let idx = self.hash_with_seed(token, seed);
                let sign = self.sign_hash(token, seed);
                vector[idx] += sign;
            }
        }

        // Normalize by token count and number of hashes
        let scale = 1.0 / ((tokens.len() * self.num_hashes) as f32).sqrt();
        for v in &mut vector {
            *v *= scale;
        }

        // L2 normalize
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vector {
                *v /= norm;
            }
        }

        Ok(vector)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        "simple-hash"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_embedder() {
        let embedder = SimpleEmbedder::new(128);

        let v1 = embedder.embed("hello world").unwrap();
        let v2 = embedder.embed("hello world").unwrap();
        let v3 = embedder.embed("goodbye universe").unwrap();

        assert_eq!(v1.len(), 128);

        // Same text should produce same embedding
        let sim_same = embedder.similarity(&v1, &v2).unwrap();
        assert!((sim_same - 1.0).abs() < 0.001);

        // Different text should produce different embedding
        let sim_diff = embedder.similarity(&v1, &v3).unwrap();
        assert!(sim_diff < 0.9);
    }

    #[test]
    fn test_similar_texts() {
        let embedder = SimpleEmbedder::new(256);

        let v1 = embedder.embed("cell membrane transport").unwrap();
        let v2 = embedder.embed("membrane cell transport proteins").unwrap();
        let v3 = embedder.embed("quantum computing algorithms").unwrap();

        let sim_related = embedder.similarity(&v1, &v2).unwrap();
        let sim_unrelated = embedder.similarity(&v1, &v3).unwrap();

        // Related texts should have higher similarity
        assert!(sim_related > sim_unrelated);
    }
}
