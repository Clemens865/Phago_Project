//! Ollama backend for local LLM inference.
//!
//! Requires the `local` feature and a running Ollama instance.

use crate::backend::{LlmBackend, LlmConfig, LlmError, LlmResult};
use crate::prompt::{parse_concepts_json, ConceptPrompt, PromptTemplate, RelationshipPrompt};
use crate::types::{Concept, RelationType, Relationship};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Ollama API request.
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

/// Ollama API response.
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    eval_count: Option<u32>,
}

/// Ollama backend for local LLM inference.
///
/// # Example
///
/// ```rust,ignore
/// use phago_llm::{OllamaBackend, LlmBackend};
///
/// let backend = OllamaBackend::new("http://localhost:11434");
/// let concepts = backend.extract_concepts("Cell biology text").await?;
/// ```
pub struct OllamaBackend {
    endpoint: String,
    config: LlmConfig,
    client: reqwest::Client,
}

impl OllamaBackend {
    /// Create a new Ollama backend with default endpoint.
    pub fn new(endpoint: &str) -> Self {
        Self::with_config(endpoint, LlmConfig::ollama())
    }

    /// Create with custom config.
    pub fn with_config(endpoint: &str, config: LlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs as u64))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            config,
            client,
        }
    }

    /// Create with default localhost endpoint.
    pub fn localhost() -> Self {
        Self::new("http://localhost:11434")
    }

    /// Set the model.
    pub fn with_model(mut self, model: &str) -> Self {
        self.config.model = model.to_string();
        self
    }

    /// Make a request to Ollama.
    async fn request(&self, prompt: &str, system: Option<&str>) -> LlmResult<String> {
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            system: system.map(|s| s.to_string()),
            stream: false,
            options: OllamaOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens,
            },
        };

        let url = format!("{}/api/generate", self.endpoint);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    LlmError::ConnectionFailed(format!(
                        "Cannot connect to Ollama at {}. Is Ollama running?",
                        self.endpoint
                    ))
                } else if e.is_timeout() {
                    LlmError::Timeout(self.config.timeout_secs)
                } else {
                    LlmError::ApiError(e.to_string())
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if status.as_u16() == 404 {
                return Err(LlmError::ModelNotFound(format!(
                    "Model '{}' not found. Run: ollama pull {}",
                    self.config.model, self.config.model
                )));
            }

            return Err(LlmError::ApiError(format!(
                "Ollama error {}: {}",
                status, body
            )));
        }

        let resp: OllamaResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(resp.response)
    }
}

#[async_trait]
impl LlmBackend for OllamaBackend {
    fn name(&self) -> &str {
        "ollama"
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }

    async fn complete(&self, prompt: &str) -> LlmResult<String> {
        self.request(prompt, None).await
    }

    async fn extract_concepts(&self, text: &str) -> LlmResult<Vec<Concept>> {
        let prompt = ConceptPrompt::new(text)
            .with_max_concepts(15)
            .with_descriptions();

        let system = prompt.system_prompt();
        let response = self.request(&prompt.generate(), system.as_deref()).await?;

        parse_concepts_json(&response).map_err(|e| {
            LlmError::ParseError(format!(
                "Failed to parse concepts: {}. Response: {}",
                e, response
            ))
        })
    }

    async fn identify_relationships(
        &self,
        text: &str,
        concepts: &[Concept],
    ) -> LlmResult<Vec<Relationship>> {
        let concept_labels: Vec<String> = concepts.iter().map(|c| c.label.clone()).collect();
        let prompt = RelationshipPrompt::new(text, concept_labels);

        let system = prompt.system_prompt();
        let response = self.request(&prompt.generate(), system.as_deref()).await?;

        parse_relationships_json(&response).map_err(|e| {
            LlmError::ParseError(format!(
                "Failed to parse relationships: {}. Response: {}",
                e, response
            ))
        })
    }

    async fn health_check(&self) -> LlmResult<bool> {
        let url = format!("{}/api/tags", self.endpoint);

        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

/// Parse relationships from JSON response.
pub(crate) fn parse_relationships_json(json: &str) -> Result<Vec<Relationship>, serde_json::Error> {
    // Extract JSON array
    let json_str = extract_json_array(json);

    #[derive(Deserialize)]
    struct RawRelationship {
        source: String,
        target: String,
        #[serde(default)]
        relation: Option<String>,
        #[serde(default)]
        label: Option<String>,
    }

    let raw: Vec<RawRelationship> = serde_json::from_str(json_str)?;

    Ok(raw
        .into_iter()
        .map(|r| {
            let relation_type = r
                .relation
                .as_deref()
                .map(parse_relation_type)
                .unwrap_or_default();
            let label = r
                .label
                .unwrap_or_else(|| format!("{} -> {}", r.source, r.target));

            Relationship::new(r.source, r.target, label).with_type(relation_type)
        })
        .collect())
}

fn parse_relation_type(s: &str) -> RelationType {
    match s.to_lowercase().as_str() {
        "is_a" | "isa" => RelationType::IsA,
        "part_of" | "partof" => RelationType::PartOf,
        "causes" => RelationType::Causes,
        "enables" => RelationType::Enables,
        "requires" => RelationType::Requires,
        "produces" => RelationType::Produces,
        "regulates" => RelationType::Regulates,
        "interacts_with" | "interactswith" => RelationType::InteractsWith,
        "located_in" | "locatedin" => RelationType::LocatedIn,
        _ => RelationType::RelatedTo,
    }
}

fn extract_json_array(text: &str) -> &str {
    let text = text.trim();
    let text = text.strip_prefix("```json").unwrap_or(text);
    let text = text.strip_prefix("```").unwrap_or(text);
    let text = text.strip_suffix("```").unwrap_or(text);
    let text = text.trim();

    if let (Some(start), Some(end)) = (text.find('['), text.rfind(']')) {
        &text[start..=end]
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_config() {
        let backend = OllamaBackend::localhost().with_model("mistral");
        assert_eq!(backend.config.model, "mistral");
        assert_eq!(backend.endpoint, "http://localhost:11434");
    }

    #[test]
    fn test_parse_relationships() {
        let json = r#"[
            {"source": "mitochondria", "target": "ATP", "relation": "produces", "label": "produces energy"},
            {"source": "cell", "target": "membrane", "relation": "part_of"}
        ]"#;

        let rels = parse_relationships_json(json).unwrap();
        assert_eq!(rels.len(), 2);
        assert_eq!(rels[0].source, "mitochondria");
        assert_eq!(rels[0].relation_type, RelationType::Produces);
    }

    #[test]
    fn test_parse_relation_types() {
        assert_eq!(parse_relation_type("is_a"), RelationType::IsA);
        assert_eq!(parse_relation_type("CAUSES"), RelationType::Causes);
        assert_eq!(parse_relation_type("unknown"), RelationType::RelatedTo);
    }
}
