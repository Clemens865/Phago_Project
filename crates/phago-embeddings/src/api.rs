//! API-based embeddings (OpenAI, Voyage, etc.).
//!
//! Requires the `api` feature.

use crate::{Embedder, EmbeddingError, EmbeddingResult};
use serde::{Deserialize, Serialize};

/// API provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiProvider {
    /// OpenAI embeddings (text-embedding-3-small, text-embedding-3-large).
    OpenAI,
    /// Voyage AI embeddings (voyage-large-2, voyage-code-2).
    Voyage,
    /// Cohere embeddings (embed-english-v3.0).
    Cohere,
    /// Custom API endpoint.
    Custom,
}

/// Configuration for API-based embeddings.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// API provider.
    pub provider: ApiProvider,
    /// API key (required for most providers).
    pub api_key: String,
    /// Model name.
    pub model: String,
    /// API endpoint (optional, uses default for provider).
    pub endpoint: Option<String>,
    /// Embedding dimension (for providers that support it).
    pub dimensions: Option<usize>,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
}

impl ApiConfig {
    /// Create config for OpenAI embeddings.
    pub fn openai(api_key: &str) -> Self {
        Self {
            provider: ApiProvider::OpenAI,
            api_key: api_key.to_string(),
            model: "text-embedding-3-small".to_string(),
            endpoint: None,
            dimensions: Some(1536),
            timeout_secs: 30,
        }
    }

    /// Create config for OpenAI with large model.
    pub fn openai_large(api_key: &str) -> Self {
        Self {
            provider: ApiProvider::OpenAI,
            api_key: api_key.to_string(),
            model: "text-embedding-3-large".to_string(),
            endpoint: None,
            dimensions: Some(3072),
            timeout_secs: 30,
        }
    }

    /// Create config for Voyage AI.
    pub fn voyage(api_key: &str) -> Self {
        Self {
            provider: ApiProvider::Voyage,
            api_key: api_key.to_string(),
            model: "voyage-large-2".to_string(),
            endpoint: None,
            dimensions: Some(1024),
            timeout_secs: 30,
        }
    }

    /// Create config for Cohere.
    pub fn cohere(api_key: &str) -> Self {
        Self {
            provider: ApiProvider::Cohere,
            api_key: api_key.to_string(),
            model: "embed-english-v3.0".to_string(),
            endpoint: None,
            dimensions: Some(1024),
            timeout_secs: 30,
        }
    }

    /// Create config for a custom API endpoint.
    pub fn custom(endpoint: &str, api_key: &str, model: &str, dimensions: usize) -> Self {
        Self {
            provider: ApiProvider::Custom,
            api_key: api_key.to_string(),
            model: model.to_string(),
            endpoint: Some(endpoint.to_string()),
            dimensions: Some(dimensions),
            timeout_secs: 30,
        }
    }

    /// Set the model.
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Set dimensions (for models that support dimension reduction).
    pub fn with_dimensions(mut self, dimensions: usize) -> Self {
        self.dimensions = Some(dimensions);
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

// ============================================================================
// Request/Response types for different APIs
// ============================================================================

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    data: Vec<OpenAIEmbedding>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbedding {
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct VoyageRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct VoyageResponse {
    data: Vec<VoyageEmbedding>,
}

#[derive(Debug, Deserialize)]
struct VoyageEmbedding {
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct CohereRequest {
    model: String,
    texts: Vec<String>,
    input_type: String,
}

#[derive(Debug, Deserialize)]
struct CohereResponse {
    embeddings: Vec<Vec<f32>>,
}

// ============================================================================
// ApiEmbedder implementation
// ============================================================================

/// API-based embedder for cloud embedding services.
///
/// Supports OpenAI, Voyage, Cohere, and custom endpoints.
///
/// # Example
///
/// ```rust,ignore
/// use phago_embeddings::{ApiEmbedder, ApiConfig};
///
/// let config = ApiConfig::openai("sk-...");
/// let embedder = ApiEmbedder::new(config)?;
/// let vec = embedder.embed("hello world")?;
/// ```
pub struct ApiEmbedder {
    config: ApiConfig,
    client: reqwest::blocking::Client,
}

impl ApiEmbedder {
    /// Create a new API embedder with the given config.
    pub fn new(config: ApiConfig) -> EmbeddingResult<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

        Ok(Self { config, client })
    }

    /// Get the API endpoint for the configured provider.
    fn endpoint(&self) -> &str {
        if let Some(ref endpoint) = self.config.endpoint {
            return endpoint;
        }

        match self.config.provider {
            ApiProvider::OpenAI => "https://api.openai.com/v1/embeddings",
            ApiProvider::Voyage => "https://api.voyageai.com/v1/embeddings",
            ApiProvider::Cohere => "https://api.cohere.ai/v1/embed",
            ApiProvider::Custom => "",
        }
    }

    /// Embed texts using OpenAI API.
    fn embed_openai(&self, texts: &[&str]) -> EmbeddingResult<Vec<Vec<f32>>> {
        let request = OpenAIRequest {
            model: self.config.model.clone(),
            input: texts.iter().map(|s| s.to_string()).collect(),
            dimensions: self.config.dimensions,
        };

        let response = self
            .client
            .post(self.endpoint())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(EmbeddingError::ApiError(format!(
                "OpenAI API error {}: {}",
                status, body
            )));
        }

        let resp: OpenAIResponse = response
            .json()
            .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

        Ok(resp.data.into_iter().map(|e| e.embedding).collect())
    }

    /// Embed texts using Voyage API.
    fn embed_voyage(&self, texts: &[&str]) -> EmbeddingResult<Vec<Vec<f32>>> {
        let request = VoyageRequest {
            model: self.config.model.clone(),
            input: texts.iter().map(|s| s.to_string()).collect(),
        };

        let response = self
            .client
            .post(self.endpoint())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(EmbeddingError::ApiError(format!(
                "Voyage API error {}: {}",
                status, body
            )));
        }

        let resp: VoyageResponse = response
            .json()
            .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

        Ok(resp.data.into_iter().map(|e| e.embedding).collect())
    }

    /// Embed texts using Cohere API.
    fn embed_cohere(&self, texts: &[&str]) -> EmbeddingResult<Vec<Vec<f32>>> {
        let request = CohereRequest {
            model: self.config.model.clone(),
            texts: texts.iter().map(|s| s.to_string()).collect(),
            input_type: "search_document".to_string(),
        };

        let response = self
            .client
            .post(self.endpoint())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(EmbeddingError::ApiError(format!(
                "Cohere API error {}: {}",
                status, body
            )));
        }

        let resp: CohereResponse = response
            .json()
            .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

        Ok(resp.embeddings)
    }
}

impl Embedder for ApiEmbedder {
    fn embed(&self, text: &str) -> EmbeddingResult<Vec<f32>> {
        let results = self.embed_batch(&[text])?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::ApiError("No embedding returned".to_string()))
    }

    fn embed_batch(&self, texts: &[&str]) -> EmbeddingResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        match self.config.provider {
            ApiProvider::OpenAI => self.embed_openai(texts),
            ApiProvider::Voyage => self.embed_voyage(texts),
            ApiProvider::Cohere => self.embed_cohere(texts),
            ApiProvider::Custom => {
                // Custom endpoint uses OpenAI-compatible format
                self.embed_openai(texts)
            }
        }
    }

    fn dimension(&self) -> usize {
        self.config.dimensions.unwrap_or(1536)
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builders() {
        let openai = ApiConfig::openai("test-key");
        assert_eq!(openai.provider, ApiProvider::OpenAI);
        assert_eq!(openai.model, "text-embedding-3-small");
        assert_eq!(openai.dimensions, Some(1536));

        let voyage = ApiConfig::voyage("test-key");
        assert_eq!(voyage.provider, ApiProvider::Voyage);
        assert_eq!(voyage.model, "voyage-large-2");

        let custom = ApiConfig::custom("https://my-api.com", "key", "my-model", 256);
        assert_eq!(custom.provider, ApiProvider::Custom);
        assert_eq!(custom.endpoint, Some("https://my-api.com".to_string()));
    }

    #[test]
    fn test_config_chaining() {
        let config = ApiConfig::openai("key")
            .with_model("text-embedding-3-large")
            .with_dimensions(3072)
            .with_timeout(60);

        assert_eq!(config.model, "text-embedding-3-large");
        assert_eq!(config.dimensions, Some(3072));
        assert_eq!(config.timeout_secs, 60);
    }
}
