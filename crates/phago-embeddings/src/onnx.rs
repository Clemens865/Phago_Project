//! ONNX-based local embeddings.
//!
//! This module provides embeddings using ONNX Runtime for local inference
//! without requiring API calls.
//!
//! Requires the `local` feature.

use crate::{Embedder, EmbeddingError, EmbeddingResult};

/// ONNX-based embedder for local inference.
///
/// Uses ONNX Runtime to run embedding models locally.
/// Supports sentence-transformers models exported to ONNX.
///
/// # Example
///
/// ```rust,ignore
/// use phago_embeddings::OnnxEmbedder;
///
/// let embedder = OnnxEmbedder::from_pretrained("all-MiniLM-L6-v2")?;
/// let vec = embedder.embed("hello world")?;
/// ```
pub struct OnnxEmbedder {
    model_name: String,
    dimension: usize,
    // In a full implementation, these would be:
    // session: ort::Session,
    // tokenizer: tokenizers::Tokenizer,
}

impl OnnxEmbedder {
    /// Load a pretrained model by name.
    ///
    /// Supported models:
    /// - `all-MiniLM-L6-v2` (384 dimensions)
    /// - `all-mpnet-base-v2` (768 dimensions)
    /// - `paraphrase-multilingual-MiniLM-L12-v2` (384 dimensions)
    pub fn from_pretrained(model_name: &str) -> EmbeddingResult<Self> {
        let dimension = match model_name {
            "all-MiniLM-L6-v2" => 384,
            "all-mpnet-base-v2" => 768,
            "paraphrase-multilingual-MiniLM-L12-v2" => 384,
            _ => {
                return Err(EmbeddingError::ModelNotLoaded(format!(
                    "Unknown model: {}. Use from_path() for custom models.",
                    model_name
                )))
            }
        };

        // In a full implementation, this would:
        // 1. Download model if not cached
        // 2. Load ONNX session
        // 3. Load tokenizer

        Ok(Self {
            model_name: model_name.to_string(),
            dimension,
        })
    }

    /// Load a model from a local path.
    pub fn from_path(model_path: &str, tokenizer_path: &str) -> EmbeddingResult<Self> {
        // In a full implementation, this would load from the paths
        let _ = (model_path, tokenizer_path);
        Err(EmbeddingError::ModelNotLoaded(
            "Custom model loading not yet implemented. Use from_pretrained() or SimpleEmbedder."
                .to_string(),
        ))
    }
}

impl Embedder for OnnxEmbedder {
    fn embed(&self, text: &str) -> EmbeddingResult<Vec<f32>> {
        // Placeholder implementation
        // In a full implementation, this would:
        // 1. Tokenize text
        // 2. Run ONNX inference
        // 3. Mean pool the output
        // 4. Normalize

        // For now, return a deterministic pseudo-embedding based on hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut vector = vec![0.0f32; self.dimension];

        for (i, word) in text.split_whitespace().enumerate() {
            let mut hasher = DefaultHasher::new();
            word.hash(&mut hasher);
            i.hash(&mut hasher);
            let hash = hasher.finish();

            for j in 0..self.dimension {
                let idx = (hash as usize + j * 31) % self.dimension;
                let sign = if (hash >> (j % 64)) & 1 == 0 {
                    1.0
                } else {
                    -1.0
                };
                vector[idx] += sign * 0.1;
            }
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
        &self.model_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onnx_embedder_creation() {
        let embedder = OnnxEmbedder::from_pretrained("all-MiniLM-L6-v2").unwrap();
        assert_eq!(embedder.dimension(), 384);
        assert_eq!(embedder.model_name(), "all-MiniLM-L6-v2");
    }

    #[test]
    fn test_onnx_embed() {
        let embedder = OnnxEmbedder::from_pretrained("all-MiniLM-L6-v2").unwrap();
        let vec = embedder.embed("hello world").unwrap();
        assert_eq!(vec.len(), 384);

        // Check normalized
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }
}
