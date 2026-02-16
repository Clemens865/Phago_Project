//! # Phago Embeddings
//!
//! Embedding backends for Phago semantic intelligence.
//!
//! This crate provides vector embeddings for semantic understanding:
//! - Text → vector conversion
//! - Similarity computation
//! - Chunking and normalization
//!
//! ## Features
//!
//! - `local`: ONNX-based local embeddings (no API needed)
//! - `api`: API-based embeddings (OpenAI, Voyage, etc.)
//! - `full`: Both local and API support
//!
//! ## Usage
//!
//! ```rust,ignore
//! use phago_embeddings::{Embedder, SimpleEmbedder};
//!
//! let embedder = SimpleEmbedder::new();
//! let vector = embedder.embed("cell membrane transport");
//! let similarity = embedder.cosine_similarity(&v1, &v2);
//! ```

mod chunker;
mod embedder;
mod normalize;
mod simple;

pub use chunker::{ChunkConfig, Chunker};
pub use embedder::{Embedder, EmbeddingError, EmbeddingResult};
pub use normalize::{
    cosine_similarity, dot_product, euclidean_distance, normalize_l1, normalize_l2,
    normalize_minmax, normalize_zscore,
};
pub use simple::SimpleEmbedder;

#[cfg(feature = "local")]
mod onnx;
#[cfg(feature = "local")]
pub use onnx::OnnxEmbedder;

#[cfg(feature = "api")]
mod api;
#[cfg(feature = "api")]
pub use api::{ApiConfig, ApiEmbedder};

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::{cosine_similarity, normalize_l2};
    pub use crate::{ChunkConfig, Chunker, SimpleEmbedder};
    pub use crate::{Embedder, EmbeddingError, EmbeddingResult};

    #[cfg(feature = "local")]
    pub use crate::OnnxEmbedder;

    #[cfg(feature = "api")]
    pub use crate::{ApiConfig, ApiEmbedder};
}
