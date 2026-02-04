//! OpenAI backend for GPT models.
//!
//! Requires the `api` feature and an OpenAI API key.

use crate::backend::{LlmBackend, LlmConfig, LlmError, LlmResult};
use crate::prompt::{parse_concepts_json, ConceptPrompt, PromptTemplate, RelationshipPrompt};
use crate::types::{Concept, Relationship};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI API request.
#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

/// OpenAI API response.
#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAiError {
    error: OpenAiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAiErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

/// OpenAI backend for GPT models.
///
/// # Example
///
/// ```rust,ignore
/// use phago_llm::{OpenAiBackend, LlmBackend};
///
/// let backend = OpenAiBackend::new("sk-...");
/// let concepts = backend.extract_concepts("Cell biology text").await?;
/// ```
pub struct OpenAiBackend {
    api_key: String,
    config: LlmConfig,
    client: reqwest::Client,
    endpoint: String,
}

impl OpenAiBackend {
    /// Create a new OpenAI backend.
    pub fn new(api_key: &str) -> Self {
        Self::with_config(api_key, LlmConfig::openai())
    }

    /// Create with custom config.
    pub fn with_config(api_key: &str, config: LlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs as u64))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key: api_key.to_string(),
            config,
            client,
            endpoint: OPENAI_API_URL.to_string(),
        }
    }

    /// Create from environment variable.
    pub fn from_env() -> LlmResult<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            LlmError::AuthenticationFailed
        })?;
        Ok(Self::new(&api_key))
    }

    /// Set the model.
    pub fn with_model(mut self, model: &str) -> Self {
        self.config.model = model.to_string();
        self
    }

    /// Use a custom endpoint (for Azure OpenAI or compatible APIs).
    pub fn with_endpoint(mut self, endpoint: &str) -> Self {
        self.endpoint = endpoint.to_string();
        self
    }

    /// Use GPT-4o (latest).
    pub fn gpt4o(mut self) -> Self {
        self.config.model = "gpt-4o".to_string();
        self
    }

    /// Use GPT-4o-mini (faster, cheaper).
    pub fn gpt4o_mini(mut self) -> Self {
        self.config.model = "gpt-4o-mini".to_string();
        self
    }

    /// Use GPT-4 Turbo.
    pub fn gpt4_turbo(mut self) -> Self {
        self.config.model = "gpt-4-turbo".to_string();
        self
    }

    /// Make a request to OpenAI API.
    async fn request(&self, prompt: &str, system: Option<&str>) -> LlmResult<String> {
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(OpenAiMessage {
                role: "system".to_string(),
                content: sys.to_string(),
            });
        }

        messages.push(OpenAiMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        let request = OpenAiRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
        };

        let response = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    LlmError::ConnectionFailed("Cannot connect to OpenAI API".to_string())
                } else if e.is_timeout() {
                    LlmError::Timeout(self.config.timeout_secs)
                } else {
                    LlmError::ApiError(e.to_string())
                }
            })?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();

            if status.as_u16() == 401 {
                return Err(LlmError::AuthenticationFailed);
            }

            if status.as_u16() == 429 {
                return Err(LlmError::RateLimited(60));
            }

            if status.as_u16() == 400 {
                if let Ok(error) = serde_json::from_str::<OpenAiError>(&body) {
                    if error.error.message.contains("maximum context length") {
                        return Err(LlmError::ContextTooLong(0, 0));
                    }
                }
            }

            if status.as_u16() == 404 {
                return Err(LlmError::ModelNotFound(self.config.model.clone()));
            }

            return Err(LlmError::ApiError(format!(
                "OpenAI API error {}: {}",
                status, body
            )));
        }

        let resp: OpenAiResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        resp.choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| LlmError::InvalidResponse("No choices in response".to_string()))
    }
}

#[async_trait]
impl LlmBackend for OpenAiBackend {
    fn name(&self) -> &str {
        "openai"
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
        let response = self
            .request(&prompt.generate(), system.as_deref())
            .await?;

        parse_concepts_json(&response).map_err(|e| {
            LlmError::ParseError(format!("Failed to parse concepts: {}. Response: {}", e, response))
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
        let response = self
            .request(&prompt.generate(), system.as_deref())
            .await?;

        crate::ollama::parse_relationships_json(&response).map_err(|e| {
            LlmError::ParseError(format!(
                "Failed to parse relationships: {}. Response: {}",
                e, response
            ))
        })
    }

    async fn expand_query(&self, query: &str) -> LlmResult<Vec<String>> {
        let prompt = crate::prompt::QueryExpansionPrompt::new(query);
        let system = prompt.system_prompt();
        let response = self
            .request(&prompt.generate(), system.as_deref())
            .await?;

        // Parse JSON array of strings
        let json_str = extract_json_array(&response);
        let expanded: Vec<String> = serde_json::from_str(json_str)
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        let mut result = vec![query.to_string()];
        result.extend(expanded);
        Ok(result)
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
    fn test_openai_config() {
        let backend = OpenAiBackend::new("test-key").gpt4o();
        assert_eq!(backend.config.model, "gpt-4o");
    }

    #[test]
    fn test_model_variants() {
        let mini = OpenAiBackend::new("key").gpt4o_mini();
        assert!(mini.config.model.contains("mini"));

        let turbo = OpenAiBackend::new("key").gpt4_turbo();
        assert!(turbo.config.model.contains("turbo"));
    }

    #[test]
    fn test_custom_endpoint() {
        let backend = OpenAiBackend::new("key")
            .with_endpoint("https://my-azure.openai.azure.com/openai/deployments/gpt-4/chat/completions");

        assert!(backend.endpoint.contains("azure"));
    }
}
