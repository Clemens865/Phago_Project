//! Prompt templates for LLM concept extraction.

use crate::types::Concept;

/// A prompt template for LLM requests.
pub trait PromptTemplate {
    /// Generate the prompt text.
    fn generate(&self) -> String;

    /// Get the system prompt (if any).
    fn system_prompt(&self) -> Option<String> {
        None
    }
}

/// Prompt for concept extraction.
#[derive(Debug, Clone)]
pub struct ConceptPrompt {
    /// The text to extract concepts from.
    pub text: String,
    /// Maximum number of concepts to extract.
    pub max_concepts: usize,
    /// Whether to include descriptions.
    pub include_descriptions: bool,
    /// Whether to identify concept types.
    pub include_types: bool,
    /// Domain hint for better extraction.
    pub domain: Option<String>,
}

impl ConceptPrompt {
    /// Create a new concept extraction prompt.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            max_concepts: 10,
            include_descriptions: false,
            include_types: true,
            domain: None,
        }
    }

    /// Set max concepts.
    pub fn with_max_concepts(mut self, max: usize) -> Self {
        self.max_concepts = max;
        self
    }

    /// Include descriptions in output.
    pub fn with_descriptions(mut self) -> Self {
        self.include_descriptions = true;
        self
    }

    /// Set domain hint.
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }
}

impl PromptTemplate for ConceptPrompt {
    fn system_prompt(&self) -> Option<String> {
        let domain_hint = self.domain.as_ref()
            .map(|d| format!(" in the domain of {}", d))
            .unwrap_or_default();

        Some(format!(
            "You are an expert at extracting key concepts{domain_hint}. \
             Extract the most important concepts from the given text. \
             Respond ONLY with a JSON array of concepts, no explanation."
        ))
    }

    fn generate(&self) -> String {
        let type_instruction = if self.include_types {
            r#", "type": "<entity|concept|process|property>""#
        } else {
            ""
        };

        let desc_instruction = if self.include_descriptions {
            r#", "description": "<brief description>""#
        } else {
            ""
        };

        format!(
            r#"Extract up to {} key concepts from this text:

---
{}
---

Respond with a JSON array like:
[{{"label": "<concept>"{}{}, "confidence": <0.0-1.0>}}]

JSON:"#,
            self.max_concepts,
            self.text,
            type_instruction,
            desc_instruction
        )
    }
}

/// Prompt for relationship identification.
#[derive(Debug, Clone)]
pub struct RelationshipPrompt {
    /// The original text.
    pub text: String,
    /// Concepts to find relationships between.
    pub concepts: Vec<String>,
    /// Maximum relationships to identify.
    pub max_relationships: usize,
}

impl RelationshipPrompt {
    /// Create a new relationship prompt.
    pub fn new(text: impl Into<String>, concepts: Vec<String>) -> Self {
        Self {
            text: text.into(),
            concepts,
            max_relationships: 20,
        }
    }

    /// Set max relationships.
    pub fn with_max_relationships(mut self, max: usize) -> Self {
        self.max_relationships = max;
        self
    }
}

impl PromptTemplate for RelationshipPrompt {
    fn system_prompt(&self) -> Option<String> {
        Some(
            "You are an expert at identifying relationships between concepts. \
             Identify meaningful relationships from the given text and concepts. \
             Respond ONLY with a JSON array, no explanation.".to_string()
        )
    }

    fn generate(&self) -> String {
        let concepts_list = self.concepts.join(", ");

        format!(
            r#"Given this text and concepts, identify relationships between them.

Text:
---
{}
---

Concepts: {}

Find up to {} relationships. Respond with a JSON array like:
[{{"source": "<concept>", "target": "<concept>", "relation": "<is_a|part_of|causes|enables|requires|produces|regulates|interacts_with|located_in|related_to>", "label": "<human readable relationship>"}}]

JSON:"#,
            self.text,
            concepts_list,
            self.max_relationships
        )
    }
}

/// Prompt for query expansion.
#[derive(Debug, Clone)]
pub struct QueryExpansionPrompt {
    /// The original query.
    pub query: String,
    /// Number of expanded queries to generate.
    pub num_expansions: usize,
}

impl QueryExpansionPrompt {
    /// Create a new query expansion prompt.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            num_expansions: 3,
        }
    }

    /// Set number of expansions.
    pub fn with_num_expansions(mut self, num: usize) -> Self {
        self.num_expansions = num;
        self
    }
}

impl PromptTemplate for QueryExpansionPrompt {
    fn system_prompt(&self) -> Option<String> {
        Some(
            "You are a search query expansion expert. Generate alternative queries \
             that capture the same intent but use different terminology.".to_string()
        )
    }

    fn generate(&self) -> String {
        format!(
            r#"Expand this search query into {} alternative queries that capture the same meaning:

Query: {}

Respond with a JSON array of strings:
["<query1>", "<query2>", ...]

JSON:"#,
            self.num_expansions,
            self.query
        )
    }
}

/// Parse concepts from JSON response.
pub fn parse_concepts_json(json: &str) -> Result<Vec<Concept>, serde_json::Error> {
    // Try to find JSON array in response
    let json_str = extract_json_array(json);

    #[derive(serde::Deserialize)]
    struct RawConcept {
        label: String,
        #[serde(rename = "type", default)]
        concept_type: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default = "default_confidence")]
        confidence: f32,
    }

    fn default_confidence() -> f32 {
        1.0
    }

    let raw: Vec<RawConcept> = serde_json::from_str(json_str)?;

    Ok(raw
        .into_iter()
        .map(|r| {
            let mut concept = Concept::new(r.label).with_confidence(r.confidence);
            if let Some(desc) = r.description {
                concept = concept.with_description(desc);
            }
            if let Some(t) = r.concept_type {
                concept = concept.with_type(match t.to_lowercase().as_str() {
                    "entity" => crate::types::ConceptType::Entity,
                    "process" => crate::types::ConceptType::Process,
                    "property" => crate::types::ConceptType::Property,
                    "relationship" => crate::types::ConceptType::Relationship,
                    _ => crate::types::ConceptType::Concept,
                });
            }
            concept
        })
        .collect())
}

/// Extract JSON array from text (handles markdown code blocks).
fn extract_json_array(text: &str) -> &str {
    // Remove markdown code blocks
    let text = text.trim();
    let text = text.strip_prefix("```json").unwrap_or(text);
    let text = text.strip_prefix("```").unwrap_or(text);
    let text = text.strip_suffix("```").unwrap_or(text);
    let text = text.trim();

    // Find array bounds
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
    fn test_concept_prompt() {
        let prompt = ConceptPrompt::new("The cell membrane controls transport.")
            .with_max_concepts(5)
            .with_domain("biology");

        let generated = prompt.generate();
        assert!(generated.contains("cell membrane"));
        assert!(generated.contains("5"));

        let system = prompt.system_prompt().unwrap();
        assert!(system.contains("biology"));
    }

    #[test]
    fn test_relationship_prompt() {
        let prompt = RelationshipPrompt::new(
            "Mitochondria produce ATP.",
            vec!["mitochondria".into(), "ATP".into()],
        );

        let generated = prompt.generate();
        assert!(generated.contains("mitochondria"));
        assert!(generated.contains("ATP"));
    }

    #[test]
    fn test_parse_concepts_json() {
        let json = r#"[
            {"label": "cell", "type": "concept", "confidence": 0.95},
            {"label": "membrane", "type": "concept", "confidence": 0.9}
        ]"#;

        let concepts = parse_concepts_json(json).unwrap();
        assert_eq!(concepts.len(), 2);
        assert_eq!(concepts[0].label, "cell");
        assert!((concepts[0].confidence - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_parse_concepts_with_code_block() {
        let json = r#"```json
        [{"label": "test", "confidence": 1.0}]
        ```"#;

        let concepts = parse_concepts_json(json).unwrap();
        assert_eq!(concepts.len(), 1);
        assert_eq!(concepts[0].label, "test");
    }
}
