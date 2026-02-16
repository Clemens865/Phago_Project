//! In-memory vector store implementation.
//!
//! This module provides a simple in-memory vector store that uses brute-force
//! search. It's useful for testing and small-scale applications.

use crate::{DistanceMetric, SearchResult, VectorError, VectorRecord, VectorResult, VectorStore};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory vector store using brute-force search.
///
/// This is the simplest implementation, suitable for:
/// - Testing and development
/// - Small datasets (< 10,000 vectors)
/// - Prototyping before moving to a production database
///
/// # Example
///
/// ```rust
/// use phago_vectors::{InMemoryStore, VectorStore, VectorRecord};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let store = InMemoryStore::new(3);
///
///     // Insert records
///     store.upsert(vec![
///         VectorRecord::new("a", vec![1.0, 0.0, 0.0]),
///         VectorRecord::new("b", vec![0.0, 1.0, 0.0]),
///         VectorRecord::new("c", vec![0.7, 0.7, 0.0]),
///     ]).await?;
///
///     // Search
///     let results = store.search(&[1.0, 0.0, 0.0], 2).await?;
///     assert_eq!(results[0].id, "a");
///
///     Ok(())
/// }
/// ```
pub struct InMemoryStore {
    records: RwLock<HashMap<String, VectorRecord>>,
    dimension: usize,
    metric: DistanceMetric,
}

impl InMemoryStore {
    /// Create a new in-memory store with the specified dimension.
    ///
    /// Uses cosine similarity by default.
    pub fn new(dimension: usize) -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            dimension,
            metric: DistanceMetric::Cosine,
        }
    }

    /// Create a new in-memory store with a specific distance metric.
    pub fn with_config(dimension: usize, metric: DistanceMetric) -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            dimension,
            metric,
        }
    }

    /// Compute similarity/distance between two vectors.
    fn compute_score(&self, a: &[f32], b: &[f32]) -> f32 {
        match self.metric {
            DistanceMetric::Cosine => crate::util::cosine_similarity(a, b),
            DistanceMetric::Euclidean => {
                // Convert distance to similarity (higher is better)
                let dist = crate::util::euclidean_distance(a, b);
                1.0 / (1.0 + dist)
            }
            DistanceMetric::DotProduct => crate::util::dot_product(a, b),
        }
    }
}

#[async_trait]
impl VectorStore for InMemoryStore {
    fn name(&self) -> &str {
        "in-memory"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn metric(&self) -> DistanceMetric {
        self.metric
    }

    async fn upsert(&self, records: Vec<VectorRecord>) -> VectorResult<()> {
        let mut store = self
            .records
            .write()
            .map_err(|e| VectorError::Connection(format!("Failed to acquire write lock: {}", e)))?;

        for record in records {
            if record.vector.len() != self.dimension {
                return Err(VectorError::DimensionMismatch {
                    expected: self.dimension,
                    actual: record.vector.len(),
                });
            }
            store.insert(record.id.clone(), record);
        }

        Ok(())
    }

    async fn search(&self, vector: &[f32], k: usize) -> VectorResult<Vec<SearchResult>> {
        if vector.len() != self.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        let store = self
            .records
            .read()
            .map_err(|e| VectorError::Connection(format!("Failed to acquire read lock: {}", e)))?;

        // Compute scores for all records
        let mut scored: Vec<_> = store
            .values()
            .map(|record| {
                let score = self.compute_score(vector, &record.vector);
                (record, score)
            })
            .collect();

        // Sort by score (descending)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        let results = scored
            .into_iter()
            .take(k)
            .map(|(record, score)| SearchResult {
                id: record.id.clone(),
                score,
                vector: Some(record.vector.clone()),
                metadata: record.metadata.clone(),
            })
            .collect();

        Ok(results)
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

        let store = self
            .records
            .read()
            .map_err(|e| VectorError::Connection(format!("Failed to acquire read lock: {}", e)))?;

        // Filter and compute scores
        let mut scored: Vec<_> = store
            .values()
            .filter(|record| {
                // Check if all filter conditions match
                filter
                    .iter()
                    .all(|(key, value)| record.metadata.get(key).map_or(false, |v| v == value))
            })
            .map(|record| {
                let score = self.compute_score(vector, &record.vector);
                (record, score)
            })
            .collect();

        // Sort by score (descending)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        let results = scored
            .into_iter()
            .take(k)
            .map(|(record, score)| SearchResult {
                id: record.id.clone(),
                score,
                vector: Some(record.vector.clone()),
                metadata: record.metadata.clone(),
            })
            .collect();

        Ok(results)
    }

    async fn get(&self, id: &str) -> VectorResult<Option<VectorRecord>> {
        let store = self
            .records
            .read()
            .map_err(|e| VectorError::Connection(format!("Failed to acquire read lock: {}", e)))?;

        Ok(store.get(id).cloned())
    }

    async fn get_batch(&self, ids: &[&str]) -> VectorResult<Vec<VectorRecord>> {
        let store = self
            .records
            .read()
            .map_err(|e| VectorError::Connection(format!("Failed to acquire read lock: {}", e)))?;

        let records: Vec<_> = ids
            .iter()
            .filter_map(|id| store.get(*id).cloned())
            .collect();

        Ok(records)
    }

    async fn delete(&self, id: &str) -> VectorResult<()> {
        let mut store = self
            .records
            .write()
            .map_err(|e| VectorError::Connection(format!("Failed to acquire write lock: {}", e)))?;

        store.remove(id);
        Ok(())
    }

    async fn delete_batch(&self, ids: &[&str]) -> VectorResult<()> {
        let mut store = self
            .records
            .write()
            .map_err(|e| VectorError::Connection(format!("Failed to acquire write lock: {}", e)))?;

        for id in ids {
            store.remove(*id);
        }

        Ok(())
    }

    async fn count(&self) -> VectorResult<usize> {
        let store = self
            .records
            .read()
            .map_err(|e| VectorError::Connection(format!("Failed to acquire read lock: {}", e)))?;

        Ok(store.len())
    }

    async fn clear(&self) -> VectorResult<()> {
        let mut store = self
            .records
            .write()
            .map_err(|e| VectorError::Connection(format!("Failed to acquire write lock: {}", e)))?;

        store.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_upsert_and_search() {
        let store = InMemoryStore::new(3);

        // Insert records
        store
            .upsert(vec![
                VectorRecord::new("a", vec![1.0, 0.0, 0.0]),
                VectorRecord::new("b", vec![0.0, 1.0, 0.0]),
                VectorRecord::new("c", vec![0.7, 0.7, 0.0]),
            ])
            .await
            .unwrap();

        // Search for vector close to 'a'
        let results = store.search(&[1.0, 0.0, 0.0], 2).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
        assert!((results[0].score - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_get_and_delete() {
        let store = InMemoryStore::new(2);

        store
            .upsert(vec![VectorRecord::new("x", vec![1.0, 0.0])])
            .await
            .unwrap();

        // Get
        let record = store.get("x").await.unwrap();
        assert!(record.is_some());
        assert_eq!(record.unwrap().id, "x");

        // Delete
        store.delete("x").await.unwrap();

        // Verify deleted
        let record = store.get("x").await.unwrap();
        assert!(record.is_none());
    }

    #[tokio::test]
    async fn test_search_with_filter() {
        let store = InMemoryStore::new(2);

        store
            .upsert(vec![
                VectorRecord::new("a", vec![1.0, 0.0]).with_metadata("type", "doc"),
                VectorRecord::new("b", vec![1.0, 0.0]).with_metadata("type", "query"),
                VectorRecord::new("c", vec![1.0, 0.0]).with_metadata("type", "doc"),
            ])
            .await
            .unwrap();

        let mut filter = HashMap::new();
        filter.insert("type".to_string(), serde_json::json!("doc"));

        let results = store
            .search_with_filter(&[1.0, 0.0], 10, &filter)
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.id != "b"));
    }

    #[tokio::test]
    async fn test_count_and_clear() {
        let store = InMemoryStore::new(2);

        store
            .upsert(vec![
                VectorRecord::new("a", vec![1.0, 0.0]),
                VectorRecord::new("b", vec![0.0, 1.0]),
            ])
            .await
            .unwrap();

        assert_eq!(store.count().await.unwrap(), 2);

        store.clear().await.unwrap();
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_dimension_mismatch() {
        let store = InMemoryStore::new(3);

        let result = store
            .upsert(vec![
                VectorRecord::new("a", vec![1.0, 0.0]), // Wrong dimension
            ])
            .await;

        assert!(matches!(result, Err(VectorError::DimensionMismatch { .. })));
    }

    #[tokio::test]
    async fn test_euclidean_metric() {
        let store = InMemoryStore::with_config(2, DistanceMetric::Euclidean);

        store
            .upsert(vec![
                VectorRecord::new("close", vec![0.1, 0.0]),
                VectorRecord::new("far", vec![10.0, 0.0]),
            ])
            .await
            .unwrap();

        let results = store.search(&[0.0, 0.0], 2).await.unwrap();
        assert_eq!(results[0].id, "close"); // Closer vector should rank first
    }
}
