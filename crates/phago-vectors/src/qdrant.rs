//! Qdrant vector database adapter.
//!
//! This module provides integration with [Qdrant](https://qdrant.tech/),
//! a high-performance vector database.
//!
//! # Feature Flag
//!
//! This module requires the `qdrant` feature:
//! ```toml
//! phago-vectors = { version = "0.6", features = ["qdrant"] }
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use phago_vectors::{QdrantStore, VectorStore, VectorRecord, DistanceMetric};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let store = QdrantStore::connect(
//!         "http://localhost:6334",
//!         None, // No API key
//!         "phago_embeddings",
//!         384,
//!         DistanceMetric::Cosine,
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
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointStruct, SearchPointsBuilder,
    UpsertPointsBuilder, VectorParamsBuilder, DeletePointsBuilder,
    PointsIdsList, PointId, GetPointsBuilder, ScrollPointsBuilder,
};
use std::collections::HashMap;

/// Qdrant vector database adapter.
pub struct QdrantStore {
    client: Qdrant,
    collection: String,
    dimension: usize,
    metric: DistanceMetric,
}

impl QdrantStore {
    /// Connect to a Qdrant server and ensure the collection exists.
    pub async fn connect(
        url: &str,
        api_key: Option<&str>,
        collection: &str,
        dimension: usize,
        metric: DistanceMetric,
    ) -> VectorResult<Self> {
        let client = if let Some(key) = api_key {
            Qdrant::from_url(url)
                .api_key(key)
                .build()
                .map_err(|e| VectorError::Connection(e.to_string()))?
        } else {
            Qdrant::from_url(url)
                .build()
                .map_err(|e| VectorError::Connection(e.to_string()))?
        };

        let store = Self {
            client,
            collection: collection.to_string(),
            dimension,
            metric,
        };

        // Ensure collection exists
        store.ensure_collection().await?;

        Ok(store)
    }

    /// Ensure the collection exists, creating it if necessary.
    async fn ensure_collection(&self) -> VectorResult<()> {
        let collections = self.client
            .list_collections()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection);

        if !exists {
            let distance = match self.metric {
                DistanceMetric::Cosine => Distance::Cosine,
                DistanceMetric::Euclidean => Distance::Euclid,
                DistanceMetric::DotProduct => Distance::Dot,
            };

            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.collection)
                        .vectors_config(VectorParamsBuilder::new(
                            self.dimension as u64,
                            distance,
                        )),
                )
                .await
                .map_err(|e| VectorError::Collection(e.to_string()))?;
        }

        Ok(())
    }

    /// Convert metadata to Qdrant payload.
    fn to_payload(metadata: &HashMap<String, serde_json::Value>) -> HashMap<String, qdrant_client::qdrant::Value> {
        metadata
            .iter()
            .filter_map(|(k, v)| {
                let value = match v {
                    serde_json::Value::String(s) => {
                        qdrant_client::qdrant::Value::from(s.clone())
                    }
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            qdrant_client::qdrant::Value::from(i)
                        } else if let Some(f) = n.as_f64() {
                            qdrant_client::qdrant::Value::from(f)
                        } else {
                            return None;
                        }
                    }
                    serde_json::Value::Bool(b) => {
                        qdrant_client::qdrant::Value::from(*b)
                    }
                    _ => return None,
                };
                Some((k.clone(), value))
            })
            .collect()
    }

    /// Convert Qdrant payload to metadata.
    fn from_payload(payload: &HashMap<String, qdrant_client::qdrant::Value>) -> HashMap<String, serde_json::Value> {
        payload
            .iter()
            .filter_map(|(k, v)| {
                use qdrant_client::qdrant::value::Kind;
                let value = match &v.kind {
                    Some(Kind::StringValue(s)) => serde_json::Value::String(s.clone()),
                    Some(Kind::IntegerValue(i)) => serde_json::json!(*i),
                    Some(Kind::DoubleValue(f)) => serde_json::json!(*f),
                    Some(Kind::BoolValue(b)) => serde_json::Value::Bool(*b),
                    _ => return None,
                };
                Some((k.clone(), value))
            })
            .collect()
    }
}

#[async_trait]
impl VectorStore for QdrantStore {
    fn name(&self) -> &str {
        "qdrant"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn metric(&self) -> DistanceMetric {
        self.metric
    }

    async fn upsert(&self, records: Vec<VectorRecord>) -> VectorResult<()> {
        let points: Vec<PointStruct> = records
            .into_iter()
            .map(|record| {
                PointStruct::new(
                    record.id,
                    record.vector,
                    Self::to_payload(&record.metadata),
                )
            })
            .collect();

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection, points).wait(true))
            .await
            .map_err(|e| VectorError::Api(e.to_string()))?;

        Ok(())
    }

    async fn search(&self, vector: &[f32], k: usize) -> VectorResult<Vec<SearchResult>> {
        self.search_with_filter(vector, k, &HashMap::new()).await
    }

    async fn search_with_filter(
        &self,
        vector: &[f32],
        k: usize,
        _filter: &HashMap<String, serde_json::Value>,
    ) -> VectorResult<Vec<SearchResult>> {
        if vector.len() != self.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        let response = self.client
            .search_points(
                SearchPointsBuilder::new(&self.collection, vector.to_vec(), k as u64)
                    .with_payload(true)
                    .with_vectors(true),
            )
            .await
            .map_err(|e| VectorError::Api(e.to_string()))?;

        let results = response
            .result
            .into_iter()
            .map(|point| {
                let id = match point.id {
                    Some(PointId { point_id_options: Some(opt) }) => {
                        match opt {
                            qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u) => u,
                            qdrant_client::qdrant::point_id::PointIdOptions::Num(n) => n.to_string(),
                        }
                    }
                    _ => String::new(),
                };

                let vector = point.vectors.and_then(|v| {
                    match v.vectors_options {
                        Some(qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(vec)) => {
                            Some(vec.data.clone())
                        }
                        _ => None,
                    }
                });

                SearchResult {
                    id,
                    score: point.score,
                    vector,
                    metadata: Self::from_payload(&point.payload),
                }
            })
            .collect();

        Ok(results)
    }

    async fn get(&self, id: &str) -> VectorResult<Option<VectorRecord>> {
        let records = self.get_batch(&[id]).await?;
        Ok(records.into_iter().next())
    }

    async fn get_batch(&self, ids: &[&str]) -> VectorResult<Vec<VectorRecord>> {
        let point_ids: Vec<PointId> = ids
            .iter()
            .map(|id| PointId::from(id.to_string()))
            .collect();

        let response = self.client
            .get_points(
                GetPointsBuilder::new(&self.collection, point_ids)
                    .with_payload(true)
                    .with_vectors(true),
            )
            .await
            .map_err(|e| VectorError::Api(e.to_string()))?;

        Ok(response
            .result
            .into_iter()
            .map(|point| {
                let id = match point.id {
                    Some(PointId { point_id_options: Some(opt) }) => {
                        match opt {
                            qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u) => u,
                            qdrant_client::qdrant::point_id::PointIdOptions::Num(n) => n.to_string(),
                        }
                    }
                    _ => String::new(),
                };

                let vector = point.vectors.and_then(|v| {
                    match v.vectors_options {
                        Some(qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(vec)) => {
                            Some(vec.data.clone())
                        }
                        _ => None,
                    }
                }).unwrap_or_default();

                VectorRecord {
                    id,
                    vector,
                    metadata: Self::from_payload(&point.payload),
                }
            })
            .collect())
    }

    async fn delete(&self, id: &str) -> VectorResult<()> {
        self.delete_batch(&[id]).await
    }

    async fn delete_batch(&self, ids: &[&str]) -> VectorResult<()> {
        let point_ids: Vec<PointId> = ids
            .iter()
            .map(|id| PointId::from(id.to_string()))
            .collect();

        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.collection)
                    .points(PointsIdsList { ids: point_ids })
                    .wait(true),
            )
            .await
            .map_err(|e| VectorError::Api(e.to_string()))?;

        Ok(())
    }

    async fn count(&self) -> VectorResult<usize> {
        let info = self.client
            .collection_info(&self.collection)
            .await
            .map_err(|e| VectorError::Api(e.to_string()))?;

        Ok(info.result.map(|r| r.points_count.unwrap_or(0) as usize).unwrap_or(0))
    }

    async fn clear(&self) -> VectorResult<()> {
        // Scroll through all points and delete them
        let mut offset: Option<PointId> = None;
        loop {
            let mut builder = ScrollPointsBuilder::new(&self.collection).limit(1000);
            if let Some(ref o) = offset {
                builder = builder.offset(o.clone());
            }

            let response = self.client
                .scroll(builder)
                .await
                .map_err(|e| VectorError::Api(e.to_string()))?;

            let points = response.result;
            if points.is_empty() {
                break;
            }

            let ids: Vec<&str> = points
                .iter()
                .filter_map(|p| {
                    p.id.as_ref().and_then(|id| {
                        match &id.point_id_options {
                            Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u)) => Some(u.as_str()),
                            _ => None,
                        }
                    })
                })
                .collect();

            if !ids.is_empty() {
                self.delete_batch(&ids).await?;
            }

            offset = response.next_page_offset;
            if offset.is_none() {
                break;
            }
        }

        Ok(())
    }
}
