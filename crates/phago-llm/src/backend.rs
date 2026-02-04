//! Core LLM backend trait.

use crate::types::{Concept, ExtractionResponse, Relationship};
use async_trait::async_trait;
use thiserror::Error;

/// LLM-related errors.
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Rate limited: retry after {0} seconds")]
    RateLimited(u32),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Parsing failed: {0}")]
    ParseError(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Context too long: {0} tokens (max: {1})")]
    ContextTooLong(usize, usize),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Timeout after {0} seconds")]
    Timeout(u32),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for LLM operations.
pub type LlmResult<T> = Result<T, LlmError>;

/// Configuration for LLM requests.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Model name/identifier.
    pub model: String,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Temperature (0.0 = deterministic, 1.0 = creative).
    pub temperature: f32,
    /// Request timeout in seconds.
    pub timeout_secs: u32,
    /// Whether to include reasoning/explanation.
    pub include_reasoning: bool,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: "default".to_string(),
            max_tokens: 1024,
            temperature: 0.0,
            timeout_secs: 30,
            include_reasoning: false,
        }
    }
}

impl LlmConfig {
    /// Create config for Claude.
    pub fn claude() -> Self {
        Self {
            model: "claude-3-haiku-20240307".to_string(),
            max_tokens: 1024,
            temperature: 0.0,
            timeout_secs: 30,
            include_reasoning: false,
        }
    }

    /// Create config for OpenAI.
    pub fn openai() -> Self {
        Self {
            model: "gpt-4o-mini".to_string(),
            max_tokens: 1024,
            temperature: 0.0,
            timeout_secs: 30,
            include_reasoning: false,
        }
    }

    /// Create config for Ollama.
    pub fn ollama() -> Self {
        Self {
            model: "llama3.2".to_string(),
            max_tokens: 1024,
            temperature: 0.0,
            timeout_secs: 60, // Local models can be slower
            include_reasoning: false,
        }
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout_secs: u32) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }
}

/// Core trait for LLM backends.
///
/// Implementors provide concept extraction and relationship identification
/// using various LLM providers.
#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Get the backend name.
    fn name(&self) -> &str;

    /// Get the current configuration.
    fn config(&self) -> &LlmConfig;

    /// Generate a completion for a prompt.
    async fn complete(&self, prompt: &str) -> LlmResult<String>;

    /// Extract concepts from text.
    async fn extract_concepts(&self, text: &str) -> LlmResult<Vec<Concept>>;

    /// Identify relationships between concepts.
    async fn identify_relationships(
        &self,
        text: &str,
        concepts: &[Concept],
    ) -> LlmResult<Vec<Relationship>>;

    /// Full extraction: concepts and relationships.
    async fn extract(&self, text: &str) -> LlmResult<ExtractionResponse> {
        let concepts = self.extract_concepts(text).await?;
        let relationships = self.identify_relationships(text, &concepts).await?;
        Ok(ExtractionResponse {
            concepts,
            relationships,
            raw_response: None,
            tokens_used: None,
        })
    }

    /// Expand a query for better recall.
    async fn expand_query(&self, query: &str) -> LlmResult<Vec<String>> {
        // Default implementation: return the query as-is
        Ok(vec![query.to_string()])
    }

    /// Summarize a cluster of concepts.
    async fn summarize_cluster(&self, concepts: &[&str]) -> LlmResult<String> {
        // Default implementation: join concepts
        Ok(concepts.join(", "))
    }

    /// Check if the backend is available.
    async fn health_check(&self) -> LlmResult<bool> {
        // Default: try a simple completion
        match self.complete("ping").await {
            Ok(_) => Ok(true),
            Err(e) => {
                // Connection errors mean unavailable, other errors might be OK
                match e {
                    LlmError::ConnectionFailed(_) => Ok(false),
                    LlmError::AuthenticationFailed => Ok(false),
                    _ => Ok(true),
                }
            }
        }
    }
}

/// A mock backend for testing.
pub struct MockBackend {
    config: LlmConfig,
    responses: std::collections::HashMap<String, String>,
}

impl MockBackend {
    /// Create a new mock backend.
    pub fn new() -> Self {
        Self {
            config: LlmConfig::default(),
            responses: std::collections::HashMap::new(),
        }
    }

    /// Add a canned response for a prompt pattern.
    pub fn with_response(mut self, pattern: &str, response: &str) -> Self {
        self.responses.insert(pattern.to_string(), response.to_string());
        self
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmBackend for MockBackend {
    fn name(&self) -> &str {
        "mock"
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }

    async fn complete(&self, prompt: &str) -> LlmResult<String> {
        // Check for matching pattern
        for (pattern, response) in &self.responses {
            if prompt.contains(pattern) {
                return Ok(response.clone());
            }
        }
        Ok("Mock response".to_string())
    }

    async fn extract_concepts(&self, text: &str) -> LlmResult<Vec<Concept>> {
        // Simple keyword extraction for testing
        let words: Vec<&str> = text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() >= 4)
            .collect();

        let concepts: Vec<Concept> = words
            .into_iter()
            .take(5)
            .map(|w| Concept::new(w.to_lowercase()))
            .collect();

        Ok(concepts)
    }

    async fn identify_relationships(
        &self,
        _text: &str,
        concepts: &[Concept],
    ) -> LlmResult<Vec<Relationship>> {
        // Create simple relationships between consecutive concepts
        let relationships: Vec<Relationship> = concepts
            .windows(2)
            .map(|pair| {
                Relationship::new(&pair[0].label, &pair[1].label, "related_to")
            })
            .collect();

        Ok(relationships)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_backend() {
        let backend = MockBackend::new()
            .with_response("test", "Test response");

        let response = backend.complete("This is a test").await.unwrap();
        assert_eq!(response, "Test response");
    }

    #[tokio::test]
    async fn test_mock_extract_concepts() {
        let backend = MockBackend::new();
        let concepts = backend
            .extract_concepts("The mitochondria produces ATP in the cell")
            .await
            .unwrap();

        assert!(!concepts.is_empty());
        assert!(concepts.iter().any(|c| c.label == "mitochondria"));
    }

    #[tokio::test]
    async fn test_mock_relationships() {
        let backend = MockBackend::new();
        let concepts = vec![
            Concept::new("mitochondria"),
            Concept::new("ATP"),
            Concept::new("cell"),
        ];

        let relationships = backend
            .identify_relationships("", &concepts)
            .await
            .unwrap();

        assert_eq!(relationships.len(), 2);
        assert_eq!(relationships[0].source, "mitochondria");
        assert_eq!(relationships[0].target, "ATP");
    }

    #[test]
    fn test_config_builders() {
        let claude = LlmConfig::claude();
        assert!(claude.model.contains("claude"));

        let openai = LlmConfig::openai();
        assert!(openai.model.contains("gpt"));

        let ollama = LlmConfig::ollama();
        assert!(ollama.model.contains("llama"));
    }
}
