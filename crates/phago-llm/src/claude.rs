//! Claude backend for Anthropic API.
//!
//! Requires the `api` feature and an Anthropic API key.

use crate::backend::{LlmBackend, LlmConfig, LlmError, LlmResult};
use crate::prompt::{parse_concepts_json, ConceptPrompt, PromptTemplate, RelationshipPrompt};
use crate::types::{Concept, Relationship};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Claude API request.
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<ClaudeMessage>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

/// Claude API response.
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ClaudeError {
    error: ClaudeErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ClaudeErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

/// Claude backend for Anthropic API.
///
/// # Example
///
/// ```rust,ignore
/// use phago_llm::{ClaudeBackend, LlmBackend};
///
/// let backend = ClaudeBackend::new("sk-ant-...");
/// let concepts = backend.extract_concepts("Cell biology text").await?;
/// ```
pub struct ClaudeBackend {
    api_key: String,
    config: LlmConfig,
    client: reqwest::Client,
}

impl ClaudeBackend {
    /// Create a new Claude backend.
    pub fn new(api_key: &str) -> Self {
        Self::with_config(api_key, LlmConfig::claude())
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
        }
    }

    /// Create from environment variable.
    pub fn from_env() -> LlmResult<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
            LlmError::AuthenticationFailed
        })?;
        Ok(Self::new(&api_key))
    }

    /// Set the model.
    pub fn with_model(mut self, model: &str) -> Self {
        self.config.model = model.to_string();
        self
    }

    /// Use Claude 3 Opus.
    pub fn opus(mut self) -> Self {
        self.config.model = "claude-3-opus-20240229".to_string();
        self
    }

    /// Use Claude 3 Sonnet.
    pub fn sonnet(mut self) -> Self {
        self.config.model = "claude-3-5-sonnet-20241022".to_string();
        self
    }

    /// Use Claude 3 Haiku (default, fastest).
    pub fn haiku(mut self) -> Self {
        self.config.model = "claude-3-haiku-20240307".to_string();
        self
    }

    /// Make a request to Claude API.
    async fn request(&self, prompt: &str, system: Option<&str>) -> LlmResult<String> {
        let request = ClaudeRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            system: system.map(|s| s.to_string()),
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: self.config.temperature,
        };

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    LlmError::ConnectionFailed("Cannot connect to Anthropic API".to_string())
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
                // Try to parse rate limit response
                if let Ok(error) = serde_json::from_str::<ClaudeError>(&body) {
                    return Err(LlmError::RateLimited(60)); // Default 60s
                }
                return Err(LlmError::RateLimited(60));
            }

            if status.as_u16() == 400 {
                if let Ok(error) = serde_json::from_str::<ClaudeError>(&body) {
                    if error.error.error_type == "invalid_request_error"
                        && error.error.message.contains("token")
                    {
                        return Err(LlmError::ContextTooLong(0, 0));
                    }
                }
            }

            return Err(LlmError::ApiError(format!(
                "Claude API error {}: {}",
                status, body
            )));
        }

        let resp: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        resp.content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| LlmError::InvalidResponse("No content in response".to_string()))
    }
}

#[async_trait]
impl LlmBackend for ClaudeBackend {
    fn name(&self) -> &str {
        "claude"
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

        // Include original query
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
    fn test_claude_config() {
        let backend = ClaudeBackend::new("test-key").sonnet();
        assert!(backend.config.model.contains("sonnet"));
    }

    #[test]
    fn test_model_variants() {
        let opus = ClaudeBackend::new("key").opus();
        assert!(opus.config.model.contains("opus"));

        let haiku = ClaudeBackend::new("key").haiku();
        assert!(haiku.config.model.contains("haiku"));
    }
}
