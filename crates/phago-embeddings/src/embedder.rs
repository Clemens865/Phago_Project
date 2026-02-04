//! Core embedder trait and types.

use thiserror::Error;

/// Embedding error types.
#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),

    #[error("Tokenization failed: {0}")]
    TokenizationFailed(String),

    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for embedding operations.
pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

/// Core trait for embedding providers.
///
/// Implementors convert text to dense vectors for semantic similarity.
pub trait Embedder: Send + Sync {
    /// Embed a single text string.
    fn embed(&self, text: &str) -> EmbeddingResult<Vec<f32>>;

    /// Embed multiple texts in a batch (more efficient).
    fn embed_batch(&self, texts: &[&str]) -> EmbeddingResult<Vec<Vec<f32>>> {
        // Default implementation: embed one by one
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Get the embedding dimension.
    fn dimension(&self) -> usize;

    /// Get the model name/identifier.
    fn model_name(&self) -> &str;

    /// Compute cosine similarity between two vectors.
    fn similarity(&self, a: &[f32], b: &[f32]) -> EmbeddingResult<f32> {
        if a.len() != b.len() {
            return Err(EmbeddingError::DimensionMismatch {
                expected: a.len(),
                got: b.len(),
            });
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }

        Ok(dot / (norm_a * norm_b))
    }

    /// Find the most similar text from a list.
    fn most_similar<'a>(
        &self,
        query: &str,
        candidates: &[&'a str],
    ) -> EmbeddingResult<Option<(&'a str, f32)>> {
        if candidates.is_empty() {
            return Ok(None);
        }

        let query_vec = self.embed(query)?;
        let candidate_vecs = self.embed_batch(candidates)?;

        let mut best: Option<(usize, f32)> = None;
        for (i, vec) in candidate_vecs.iter().enumerate() {
            let sim = self.similarity(&query_vec, vec)?;
            if best.is_none() || sim > best.unwrap().1 {
                best = Some((i, sim));
            }
        }

        Ok(best.map(|(i, sim)| (candidates[i], sim)))
    }

    /// Find top-k most similar texts.
    fn top_k_similar<'a>(
        &self,
        query: &str,
        candidates: &[&'a str],
        k: usize,
    ) -> EmbeddingResult<Vec<(&'a str, f32)>> {
        if candidates.is_empty() || k == 0 {
            return Ok(vec![]);
        }

        let query_vec = self.embed(query)?;
        let candidate_vecs = self.embed_batch(candidates)?;

        let mut scores: Vec<(usize, f32)> = candidate_vecs
            .iter()
            .enumerate()
            .map(|(i, vec)| {
                let sim = self.similarity(&query_vec, vec).unwrap_or(0.0);
                (i, sim)
            })
            .collect();

        // Sort by similarity descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scores
            .into_iter()
            .take(k)
            .map(|(i, sim)| (candidates[i], sim))
            .collect())
    }
}

/// Embedding with metadata.
#[derive(Debug, Clone)]
pub struct Embedding {
    /// The vector representation.
    pub vector: Vec<f32>,
    /// Original text (optional).
    pub text: Option<String>,
    /// Token count.
    pub tokens: usize,
}

impl Embedding {
    /// Create a new embedding.
    pub fn new(vector: Vec<f32>) -> Self {
        Self {
            vector,
            text: None,
            tokens: 0,
        }
    }

    /// Create with text.
    pub fn with_text(vector: Vec<f32>, text: String, tokens: usize) -> Self {
        Self {
            vector,
            text: Some(text),
            tokens,
        }
    }

    /// Get dimension.
    pub fn dimension(&self) -> usize {
        self.vector.len()
    }
}
