//! # Phago Vectors
//!
//! Vector database adapters for Phago biological computing framework.
//!
//! This crate provides a unified interface for storing and searching embeddings
//! across different vector database backends.
//!
//! ## Supported Backends
//!
//! | Backend | Feature Flag | Description |
//! |---------|--------------|-------------|
//! | In-Memory | (default) | Simple brute-force search, good for testing |
//! | Qdrant | `qdrant` | High-performance vector database |
//! | Pinecone | `pinecone` | Managed vector database service |
//! | Weaviate | `weaviate` | Open-source vector search engine |
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use phago_vectors::{VectorStore, InMemoryStore, VectorRecord};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let store = InMemoryStore::new(384); // 384-dimensional vectors
//!
//!     // Store a vector
//!     let record = VectorRecord::new("doc1", vec![0.1; 384])
//!         .with_metadata("title", "Introduction to Cells");
//!     store.upsert(vec![record]).await?;
//!
//!     // Search for similar vectors
//!     let results = store.search(&[0.1; 384], 5).await?;
//!     for result in results {
//!         println!("{}: {:.3}", result.id, result.score);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Feature Flags
//!
//! ```toml
//! # Use Qdrant backend
//! phago-vectors = { version = "0.6", features = ["qdrant"] }
//!
//! # Use all backends
//! phago-vectors = { version = "0.6", features = ["all"] }
//! ```

pub mod memory;

#[cfg(feature = "qdrant")]
pub mod qdrant;

#[cfg(feature = "pinecone")]
pub mod pinecone;

#[cfg(feature = "weaviate")]
pub mod weaviate;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur when working with vector stores.
#[derive(Error, Debug)]
pub enum VectorError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Collection/index error: {0}")]
    Collection(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

/// Result type for vector operations.
pub type VectorResult<T> = Result<T, VectorError>;

/// A vector record to store in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRecord {
    /// Unique identifier for this record.
    pub id: String,
    /// The embedding vector.
    pub vector: Vec<f32>,
    /// Optional metadata associated with the record.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl VectorRecord {
    /// Create a new vector record.
    pub fn new(id: impl Into<String>, vector: Vec<f32>) -> Self {
        Self {
            id: id.into(),
            vector,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the record.
    pub fn with_metadata(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get the vector dimension.
    pub fn dimension(&self) -> usize {
        self.vector.len()
    }
}

/// A search result from the vector store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The record ID.
    pub id: String,
    /// Similarity score (higher is more similar).
    pub score: f32,
    /// The vector (if requested).
    pub vector: Option<Vec<f32>>,
    /// Metadata associated with the record.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Configuration for creating a vector store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    /// Vector dimension.
    pub dimension: usize,
    /// Collection/index name.
    pub collection: String,
    /// Distance metric.
    pub metric: DistanceMetric,
    /// Backend-specific configuration.
    pub backend: BackendConfig,
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        Self {
            dimension: 384,
            collection: "phago".to_string(),
            metric: DistanceMetric::Cosine,
            backend: BackendConfig::InMemory,
        }
    }
}

/// Distance metric for similarity search.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DistanceMetric {
    /// Cosine similarity (normalized dot product).
    Cosine,
    /// Euclidean distance (L2).
    Euclidean,
    /// Dot product (inner product).
    DotProduct,
}

/// Backend-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BackendConfig {
    /// In-memory vector store (for testing).
    InMemory,

    /// Qdrant vector database.
    #[cfg(feature = "qdrant")]
    Qdrant {
        /// Qdrant server URL.
        url: String,
        /// API key (optional).
        api_key: Option<String>,
    },

    /// Pinecone managed service.
    #[cfg(feature = "pinecone")]
    Pinecone {
        /// Pinecone API key.
        api_key: String,
        /// Environment (e.g., "us-east-1-aws").
        environment: String,
        /// Index name.
        index: String,
    },

    /// Weaviate vector search.
    #[cfg(feature = "weaviate")]
    Weaviate {
        /// Weaviate server URL.
        url: String,
        /// API key (optional).
        api_key: Option<String>,
        /// Class name.
        class_name: String,
    },
}

/// Abstract interface for vector storage and search.
///
/// Implementations of this trait provide vector storage and similarity search
/// capabilities, allowing Phago to use different vector database backends.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Get the name of this backend.
    fn name(&self) -> &str;

    /// Get the vector dimension.
    fn dimension(&self) -> usize;

    /// Get the distance metric.
    fn metric(&self) -> DistanceMetric;

    /// Insert or update records in the store.
    ///
    /// If a record with the same ID exists, it will be updated.
    async fn upsert(&self, records: Vec<VectorRecord>) -> VectorResult<()>;

    /// Search for similar vectors.
    ///
    /// Returns the top `k` most similar records.
    async fn search(&self, vector: &[f32], k: usize) -> VectorResult<Vec<SearchResult>>;

    /// Search with metadata filter.
    ///
    /// Only returns records matching the filter criteria.
    async fn search_with_filter(
        &self,
        vector: &[f32],
        k: usize,
        filter: &HashMap<String, serde_json::Value>,
    ) -> VectorResult<Vec<SearchResult>>;

    /// Get a record by ID.
    async fn get(&self, id: &str) -> VectorResult<Option<VectorRecord>>;

    /// Get multiple records by ID.
    async fn get_batch(&self, ids: &[&str]) -> VectorResult<Vec<VectorRecord>>;

    /// Delete a record by ID.
    async fn delete(&self, id: &str) -> VectorResult<()>;

    /// Delete multiple records by ID.
    async fn delete_batch(&self, ids: &[&str]) -> VectorResult<()>;

    /// Get the total number of records in the store.
    async fn count(&self) -> VectorResult<usize>;

    /// Clear all records from the store.
    async fn clear(&self) -> VectorResult<()>;
}

/// Create a vector store from configuration.
pub async fn create_store(config: VectorStoreConfig) -> VectorResult<Box<dyn VectorStore>> {
    match config.backend {
        BackendConfig::InMemory => Ok(Box::new(memory::InMemoryStore::with_config(
            config.dimension,
            config.metric,
        ))),

        #[cfg(feature = "qdrant")]
        BackendConfig::Qdrant { url, api_key } => {
            let store = qdrant::QdrantStore::connect(
                &url,
                api_key.as_deref(),
                &config.collection,
                config.dimension,
                config.metric,
            )
            .await?;
            Ok(Box::new(store))
        }

        #[cfg(feature = "pinecone")]
        BackendConfig::Pinecone {
            api_key,
            environment,
            index,
        } => {
            let store =
                pinecone::PineconeStore::connect(&api_key, &environment, &index, config.dimension)
                    .await?;
            Ok(Box::new(store))
        }

        #[cfg(feature = "weaviate")]
        BackendConfig::Weaviate {
            url,
            api_key,
            class_name,
        } => {
            let store = weaviate::WeaviateStore::connect(
                &url,
                api_key.as_deref(),
                &class_name,
                config.dimension,
            )
            .await?;
            Ok(Box::new(store))
        }
    }
}

// Re-export commonly used types
pub use memory::InMemoryStore;

#[cfg(feature = "qdrant")]
pub use qdrant::QdrantStore;

#[cfg(feature = "pinecone")]
pub use pinecone::PineconeStore;

#[cfg(feature = "weaviate")]
pub use weaviate::WeaviateStore;

/// Utility functions for vector operations.
pub mod util {
    /// Compute cosine similarity between two vectors.
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len(), "Vectors must have same dimension");

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    /// Compute Euclidean distance between two vectors.
    pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len(), "Vectors must have same dimension");

        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Compute dot product between two vectors.
    pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len(), "Vectors must have same dimension");

        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Normalize a vector to unit length.
    pub fn normalize(v: &mut [f32]) {
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in v.iter_mut() {
                *x /= norm;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((util::cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!((util::cosine_similarity(&a, &c) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((util::euclidean_distance(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0];
        util::normalize(&mut v);
        assert!((v[0] - 0.6).abs() < 1e-6);
        assert!((v[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_vector_record() {
        let record = VectorRecord::new("test", vec![0.1, 0.2, 0.3])
            .with_metadata("title", "Test Document")
            .with_metadata("score", 0.95);

        assert_eq!(record.id, "test");
        assert_eq!(record.dimension(), 3);
        assert_eq!(record.metadata.len(), 2);
    }
}
