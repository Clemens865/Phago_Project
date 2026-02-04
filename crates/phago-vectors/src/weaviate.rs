//! Weaviate vector database adapter.
//!
//! This module provides integration with [Weaviate](https://weaviate.io/),
//! an open-source vector search engine.
//!
//! # Feature Flag
//!
//! This module requires the `weaviate` feature:
//! ```toml
//! phago-vectors = { version = "0.6", features = ["weaviate"] }
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use phago_vectors::{WeaviateStore, VectorStore, VectorRecord};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let store = WeaviateStore::connect(
//!         "http://localhost:8080",
//!         None, // No API key for local
//!         "Document",
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

/// Weaviate vector database adapter.
pub struct WeaviateStore {
    client: Client,
    url: String,
    api_key: Option<String>,
    class_name: String,
    dimension: usize,
}

#[derive(Serialize)]
struct WeaviateObject {
    class: String,
    id: String,
    properties: HashMap<String, serde_json::Value>,
    vector: Vec<f32>,
}

#[derive(Serialize)]
struct BatchRequest {
    objects: Vec<WeaviateObject>,
}

#[derive(Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize)]
struct GraphQLData {
    #[serde(rename = "Get")]
    get: Option<HashMap<String, Vec<WeaviateResult>>>,
}

#[derive(Deserialize)]
struct GraphQLError {
    message: String,
}

#[derive(Deserialize)]
struct WeaviateResult {
    _additional: Option<AdditionalData>,
    #[serde(flatten)]
    properties: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct AdditionalData {
    id: Option<String>,
    distance: Option<f32>,
    vector: Option<Vec<f32>>,
}

#[derive(Deserialize)]
struct ClassSchema {
    class: String,
    #[serde(rename = "vectorIndexConfig")]
    vector_index_config: Option<VectorIndexConfig>,
}

#[derive(Deserialize)]
struct VectorIndexConfig {
    distance: Option<String>,
}

#[derive(Deserialize)]
struct SchemaResponse {
    classes: Vec<ClassSchema>,
}

#[derive(Deserialize)]
struct AggregateResponse {
    data: Option<AggregateData>,
}

#[derive(Deserialize)]
struct AggregateData {
    #[serde(rename = "Aggregate")]
    aggregate: Option<HashMap<String, Vec<AggregateResult>>>,
}

#[derive(Deserialize)]
struct AggregateResult {
    meta: Option<MetaCount>,
}

#[derive(Deserialize)]
struct MetaCount {
    count: Option<usize>,
}

impl WeaviateStore {
    /// Connect to a Weaviate server.
    ///
    /// # Arguments
    ///
    /// * `url` - The Weaviate server URL (e.g., "http://localhost:8080")
    /// * `api_key` - Optional API key for authentication
    /// * `class_name` - The class name to use for storing objects
    /// * `dimension` - Vector dimension
    pub async fn connect(
        url: &str,
        api_key: Option<&str>,
        class_name: &str,
        dimension: usize,
    ) -> VectorResult<Self> {
        let client = Client::builder()
            .build()
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        let store = Self {
            client,
            url: url.trim_end_matches('/').to_string(),
            api_key: api_key.map(String::from),
            class_name: class_name.to_string(),
            dimension,
        };

        // Ensure the class exists
        store.ensure_class().await?;

        Ok(store)
    }

    /// Build a request with optional API key header.
    fn build_request(&self, method: reqwest::Method, endpoint: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.request(method, format!("{}{}", self.url, endpoint));
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        req
    }

    /// Ensure the class exists in the schema.
    async fn ensure_class(&self) -> VectorResult<()> {
        // Check if class exists
        let response = self.build_request(reqwest::Method::GET, "/v1/schema")
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(VectorError::Api(format!("Schema query failed: {}", error)));
        }

        let schema: SchemaResponse = response
            .json()
            .await
            .map_err(|e| VectorError::Serialization(e.to_string()))?;

        let class_exists = schema.classes.iter().any(|c| c.class == self.class_name);

        if !class_exists {
            // Create the class
            let class_def = serde_json::json!({
                "class": self.class_name,
                "vectorizer": "none",  // We provide vectors ourselves
                "properties": [
                    {
                        "name": "phago_id",
                        "dataType": ["text"],
                        "description": "Original record ID"
                    },
                    {
                        "name": "metadata_json",
                        "dataType": ["text"],
                        "description": "JSON-encoded metadata"
                    }
                ]
            });

            let response = self.build_request(reqwest::Method::POST, "/v1/schema")
                .json(&class_def)
                .send()
                .await
                .map_err(|e| VectorError::Connection(e.to_string()))?;

            if !response.status().is_success() {
                let error = response.text().await.unwrap_or_default();
                return Err(VectorError::Collection(format!("Class creation failed: {}", error)));
            }
        }

        Ok(())
    }

    /// Execute a GraphQL query.
    async fn graphql(&self, query: &str) -> VectorResult<GraphQLResponse> {
        let response = self.build_request(reqwest::Method::POST, "/v1/graphql")
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(VectorError::Api(format!("GraphQL query failed: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| VectorError::Serialization(e.to_string()))
    }
}

#[async_trait]
impl VectorStore for WeaviateStore {
    fn name(&self) -> &str {
        "weaviate"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn metric(&self) -> DistanceMetric {
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

        let objects: Vec<WeaviateObject> = records
            .into_iter()
            .map(|r| {
                let mut properties = HashMap::new();
                properties.insert(
                    "phago_id".to_string(),
                    serde_json::Value::String(r.id.clone()),
                );
                properties.insert(
                    "metadata_json".to_string(),
                    serde_json::Value::String(serde_json::to_string(&r.metadata).unwrap_or_default()),
                );

                WeaviateObject {
                    class: self.class_name.clone(),
                    id: uuid::Uuid::new_v5(
                        &uuid::Uuid::NAMESPACE_DNS,
                        r.id.as_bytes(),
                    ).to_string(),
                    properties,
                    vector: r.vector,
                }
            })
            .collect();

        let request = BatchRequest { objects };

        let response = self.build_request(reqwest::Method::POST, "/v1/batch/objects")
            .json(&request)
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(VectorError::Api(format!("Batch upsert failed: {}", error)));
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

        let vector_str = vector
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        // Build where clause for filter
        let where_clause = if filter.is_empty() {
            String::new()
        } else {
            // Build simple equality filters
            let conditions: Vec<String> = filter
                .iter()
                .filter_map(|(k, v)| {
                    match v {
                        serde_json::Value::String(s) => Some(format!(
                            "{{ path: [\"metadata_json\"], operator: Contains, valueText: \"\\\"{}\\\":\\\"{}\\\"\" }}",
                            k, s
                        )),
                        _ => None,
                    }
                })
                .collect();

            if conditions.is_empty() {
                String::new()
            } else {
                format!("where: {{ operator: And, operands: [{}] }}", conditions.join(", "))
            }
        };

        let query = format!(
            r#"{{
                Get {{
                    {class_name}(
                        nearVector: {{ vector: [{vector}] }}
                        limit: {k}
                        {where_clause}
                    ) {{
                        phago_id
                        metadata_json
                        _additional {{
                            id
                            distance
                            vector
                        }}
                    }}
                }}
            }}"#,
            class_name = self.class_name,
            vector = vector_str,
            k = k,
            where_clause = where_clause,
        );

        let response = self.graphql(&query).await?;

        if let Some(errors) = response.errors {
            if !errors.is_empty() {
                return Err(VectorError::Api(errors[0].message.clone()));
            }
        }

        let results = response
            .data
            .and_then(|d| d.get)
            .and_then(|g| g.get(&self.class_name).cloned())
            .unwrap_or_default();

        Ok(results
            .into_iter()
            .map(|r| {
                let additional = r._additional.unwrap_or_default();
                let phago_id = r.properties
                    .get("phago_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let metadata: HashMap<String, serde_json::Value> = r.properties
                    .get("metadata_json")
                    .and_then(|v| v.as_str())
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();

                // Weaviate returns distance (lower is better), convert to similarity
                let score = additional.distance.map(|d| 1.0 - d).unwrap_or(0.0);

                SearchResult {
                    id: phago_id,
                    score,
                    vector: additional.vector,
                    metadata,
                }
            })
            .collect())
    }

    async fn get(&self, id: &str) -> VectorResult<Option<VectorRecord>> {
        let records = self.get_batch(&[id]).await?;
        Ok(records.into_iter().next())
    }

    async fn get_batch(&self, ids: &[&str]) -> VectorResult<Vec<VectorRecord>> {
        // Build OR filter for IDs
        let conditions: Vec<String> = ids
            .iter()
            .map(|id| format!(
                "{{ path: [\"phago_id\"], operator: Equal, valueText: \"{}\" }}",
                id
            ))
            .collect();

        let query = format!(
            r#"{{
                Get {{
                    {class_name}(
                        where: {{ operator: Or, operands: [{conditions}] }}
                    ) {{
                        phago_id
                        metadata_json
                        _additional {{
                            vector
                        }}
                    }}
                }}
            }}"#,
            class_name = self.class_name,
            conditions = conditions.join(", "),
        );

        let response = self.graphql(&query).await?;

        let results = response
            .data
            .and_then(|d| d.get)
            .and_then(|g| g.get(&self.class_name).cloned())
            .unwrap_or_default();

        Ok(results
            .into_iter()
            .map(|r| {
                let additional = r._additional.unwrap_or_default();
                let phago_id = r.properties
                    .get("phago_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let metadata: HashMap<String, serde_json::Value> = r.properties
                    .get("metadata_json")
                    .and_then(|v| v.as_str())
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();

                VectorRecord {
                    id: phago_id,
                    vector: additional.vector.unwrap_or_default(),
                    metadata,
                }
            })
            .collect())
    }

    async fn delete(&self, id: &str) -> VectorResult<()> {
        // Get the Weaviate UUID for this phago_id
        let uuid = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, id.as_bytes());

        let response = self.build_request(
            reqwest::Method::DELETE,
            &format!("/v1/objects/{}/{}", self.class_name, uuid),
        )
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        // 204 No Content or 404 Not Found are both acceptable
        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND {
            let error = response.text().await.unwrap_or_default();
            return Err(VectorError::Api(format!("Delete failed: {}", error)));
        }

        Ok(())
    }

    async fn delete_batch(&self, ids: &[&str]) -> VectorResult<()> {
        for id in ids {
            self.delete(id).await?;
        }
        Ok(())
    }

    async fn count(&self) -> VectorResult<usize> {
        let query = format!(
            r#"{{
                Aggregate {{
                    {class_name} {{
                        meta {{
                            count
                        }}
                    }}
                }}
            }}"#,
            class_name = self.class_name,
        );

        let response: AggregateResponse = self.build_request(reqwest::Method::POST, "/v1/graphql")
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?
            .json()
            .await
            .map_err(|e| VectorError::Serialization(e.to_string()))?;

        let count = response
            .data
            .and_then(|d| d.aggregate)
            .and_then(|a| a.get(&self.class_name).cloned())
            .and_then(|v| v.into_iter().next())
            .and_then(|r| r.meta)
            .and_then(|m| m.count)
            .unwrap_or(0);

        Ok(count)
    }

    async fn clear(&self) -> VectorResult<()> {
        // Delete the class and recreate it
        let response = self.build_request(
            reqwest::Method::DELETE,
            &format!("/v1/schema/{}", self.class_name),
        )
            .send()
            .await
            .map_err(|e| VectorError::Connection(e.to_string()))?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND {
            let error = response.text().await.unwrap_or_default();
            return Err(VectorError::Api(format!("Clear failed: {}", error)));
        }

        // Recreate the class
        self.ensure_class().await
    }
}

impl Default for AdditionalData {
    fn default() -> Self {
        Self {
            id: None,
            distance: None,
            vector: None,
        }
    }
}
