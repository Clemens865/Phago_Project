//! Pinecone vector database adapter.
//!
//! This module provides integration with [Pinecone](https://www.pinecone.io/),
//! a managed vector database service.
//!
//! # Feature Flag
//!
//! This module requires the `pinecone` feature:
//! ```toml
//! phago-vectors = { version = "0.6", features = ["pinecone"] }
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use phago_vectors::{PineconeStore, VectorStore, VectorRecord};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let store = PineconeStore::connect(
//!         "your-api-key",
//!         "us-east-1-aws",
//!         "phago-index",
//!         384,
//!     ).await?;
//!
//!     // Store vectors
//!     store.upsert(vec![
//!         VectorRecord::new("doc1", vec![0.1; 384])
//!             .with_metadata("title", "Introduction"),
//!     ]).await?;
//!
//!     // Search
//!     let results = store.search(&[0.1; 384], 5).await?;
//!     Ok(())
//! }
//! ```

use crate::{
    DistanceMetric, SearchResult, VectorError, VectorRecord, VectorResult, VectorStore,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pinecone vector database adapter.
pub struct PineconeStore {
    client: Client,
    api_key: String,
    host: String,
    dimension: usize,
}

#[derive(Serialize)]
struct UpsertRequest {
    vectors: Vec<PineconeVector>,
    namespace: String,
}

#[derive(Serialize)]
struct PineconeVector {
    id: String,
    values: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize)]
struct QueryRequest {
    vector: Vec<f32>,
    #[serde(rename = "topK")]
    top_k: usize,
    #[serde(rename = "includeMetadata")]
    include_metadata: bool,
    #[serde(rename = "includeValues")]
    include_values: bool,
    namespace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Deserialize)]
struct QueryResponse {
    matches: Vec<PineconeMatch>,
}

#[derive(Deserialize)]
struct PineconeMatch {
    id: String,
    score: f32,
    #[serde(default)]
    values: Vec<f32>,
    #[serde(default)]
    metadata: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
struct FetchRequest {
    ids: Vec<String>,
    namespace: String,
}

#[derive(Deserialize)]
struct FetchResponse {
    vectors: HashMap<String, FetchedVector>,
}

#[derive(Deserialize)]
struct FetchedVector {
    id: String,
    values: Vec<f32>,
    #[serde(default)]
    metadata: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
struct DeleteRequest {
    ids: Vec<String>,
    namespace: String,
}

#[derive(Deserialize)]
struct StatsResponse {
    namespaces: HashMap<String, NamespaceStats>,
    #[serde(rename = "totalVectorCount")]
    total_vector_count: usize,
}

#[derive(Deserialize)]
struct NamespaceStats {
    #[serde(rename = "vectorCount")]
    vector_count: usize,
}

impl PineconeStore {
    /// Connect to a Pinecone index.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Your Pinecone API key
    /// * `environment` - The Pinecone environment (e.g., "us-east-1-aws")
    /// * `index` - The index name
    /// * `dimension` - Vector dimension (must match index configuration)
    pub async fn connect(
        api_key: &str,
        environment: &str,
        index: &str,
        dimension: usize,
    ) -> VectorResult<Self> {
        // Pinecone host format: {index}-{project}.svc.{environment}.pinecone.io
        // For simplicity, we construct a basic host. In production, you'd want to
        // query the Pinecone API to get the actual host.
        let host = format!("https://{}.svc.{}.pinecone.io", index, environment);

        let client = Client::builder()
            .build()
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        // Verify connection by getting stats
        let store = Self {
            client,
            api_key: api_key.to_string(),
            host,
            dimension,
        };

        // Test the connection
        store.stats().await?;

        Ok(store)
    }

    /// Get index statistics.
    async fn stats(&self) -> VectorResult<StatsResponse> {
        let response = self.client
            .get(format!("{}/describe_index_stats", self.host))
            .header("Api-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(VectorError::Api(format!("Pinecone API error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| VectorError::Serialization(e.to_string()))
    }
}

#[async_trait]
impl VectorStore for PineconeStore {
    fn name(&self) -> &str {
        "pinecone"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn metric(&self) -> DistanceMetric {
        // Pinecone supports cosine, euclidean, and dotproduct
        // Default to cosine as it's most common
        DistanceMetric::Cosine
    }

    async fn upsert(&self, records: Vec<VectorRecord>) -> VectorResult<()> {
        for record in &records {
            if record.vector.len() != self.dimension {
                return Err(VectorError::DimensionMismatch {
                    expected: self.dimension,
                    actual: record.vector.len(),
                });
            }
        }

        let vectors: Vec<PineconeVector> = records
            .into_iter()
            .map(|r| PineconeVector {
                id: r.id,
                values: r.vector,
                metadata: if r.metadata.is_empty() {
                    None
                } else {
                    Some(r.metadata)
                },
            })
            .collect();

        let request = UpsertRequest {
            vectors,
            namespace: "".to_string(),
        };

        let response = self.client
            .post(format!("{}/vectors/upsert", self.host))
            .header("Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(VectorError::Api(format!("Upsert failed: {}", error)));
        }

        Ok(())
    }

    async fn search(&self, vector: &[f32], k: usize) -> VectorResult<Vec<SearchResult>> {
        self.search_with_filter(vector, k, &HashMap::new()).await
    }

    async fn search_with_filter(
        &self,
        vector: &[f32],
        k: usize,
        filter: &HashMap<String, serde_json::Value>,
    ) -> VectorResult<Vec<SearchResult>> {
        if vector.len() != self.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        let request = QueryRequest {
            vector: vector.to_vec(),
            top_k: k,
            include_metadata: true,
            include_values: true,
            namespace: "".to_string(),
            filter: if filter.is_empty() {
                None
            } else {
                Some(filter.clone())
            },
        };

        let response = self.client
            .post(format!("{}/query", self.host))
            .header("Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(VectorError::Api(format!("Query failed: {}", error)));
        }

        let query_response: QueryResponse = response
            .json()
            .await
            .map_err(|e| VectorError::Serialization(e.to_string()))?;

        Ok(query_response
            .matches
            .into_iter()
            .map(|m| SearchResult {
                id: m.id,
                score: m.score,
                vector: if m.values.is_empty() {
                    None
                } else {
                    Some(m.values)
                },
                metadata: m.metadata,
            })
            .collect())
    }

    async fn get(&self, id: &str) -> VectorResult<Option<VectorRecord>> {
        let records = self.get_batch(&[id]).await?;
        Ok(records.into_iter().next())
    }

    async fn get_batch(&self, ids: &[&str]) -> VectorResult<Vec<VectorRecord>> {
        let url = format!(
            "{}/vectors/fetch?ids={}&namespace=",
            self.host,
            ids.join(",")
        );

        let response = self.client
            .get(&url)
            .header("Api-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(VectorError::Api(format!("Fetch failed: {}", error)));
        }

        let fetch_response: FetchResponse = response
            .json()
            .await
            .map_err(|e| VectorError::Serialization(e.to_string()))?;

        Ok(fetch_response
            .vectors
            .into_values()
            .map(|v| VectorRecord {
                id: v.id,
                vector: v.values,
                metadata: v.metadata,
            })
            .collect())
    }

    async fn delete(&self, id: &str) -> VectorResult<()> {
        self.delete_batch(&[id]).await
    }

    async fn delete_batch(&self, ids: &[&str]) -> VectorResult<()> {
        let request = DeleteRequest {
            ids: ids.iter().map(|s| s.to_string()).collect(),
            namespace: "".to_string(),
        };

        let response = self.client
            .post(format!("{}/vectors/delete", self.host))
            .header("Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(VectorError::Api(format!("Delete failed: {}", error)));
        }

        Ok(())
    }

    async fn count(&self) -> VectorResult<usize> {
        let stats = self.stats().await?;
        Ok(stats.total_vector_count)
    }

    async fn clear(&self) -> VectorResult<()> {
        // Pinecone requires deleting by filter or IDs
        // For a full clear, we delete the default namespace
        let response = self.client
            .post(format!("{}/vectors/delete", self.host))
            .header("Api-Key", &self.api_key)
            .json(&serde_json::json!({
                "deleteAll": true,
                "namespace": ""
            }))
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(VectorError::Api(format!("Clear failed: {}", error)));
        }

        Ok(())
    }
}
