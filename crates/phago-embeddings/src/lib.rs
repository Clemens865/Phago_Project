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

mod embedder;
mod simple;
mod chunker;
mod normalize;

pub use embedder::{Embedder, EmbeddingError, EmbeddingResult};
pub use simple::SimpleEmbedder;
pub use chunker::{Chunker, ChunkConfig};
pub use normalize::{
    normalize_l2, normalize_l1, normalize_minmax, normalize_zscore,
    cosine_similarity, euclidean_distance, dot_product,
};

#[cfg(feature = "local")]
mod onnx;
#[cfg(feature = "local")]
pub use onnx::OnnxEmbedder;

#[cfg(feature = "api")]
mod api;
#[cfg(feature = "api")]
pub use api::{ApiEmbedder, ApiConfig};

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::{Embedder, EmbeddingError, EmbeddingResult};
    pub use crate::{SimpleEmbedder, Chunker, ChunkConfig};
    pub use crate::{normalize_l2, cosine_similarity};

    #[cfg(feature = "local")]
    pub use crate::OnnxEmbedder;

    #[cfg(feature = "api")]
    pub use crate::{ApiEmbedder, ApiConfig};
}
