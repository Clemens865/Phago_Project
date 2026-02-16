//! Core types for LLM integration.

use serde::{Deserialize, Serialize};

/// Type of concept extracted by LLM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConceptType {
    /// A named entity (person, place, organization).
    Entity,
    /// A scientific or technical concept.
    Concept,
    /// A process or action.
    Process,
    /// A property or attribute.
    Property,
    /// A relationship or connection.
    Relationship,
    /// Unknown or unclassified.
    Other,
}

impl Default for ConceptType {
    fn default() -> Self {
        Self::Concept
    }
}

/// A concept extracted from text by an LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    /// The concept label/name.
    pub label: String,
    /// Type of concept.
    pub concept_type: ConceptType,
    /// Confidence score (0.0-1.0).
    pub confidence: f32,
    /// Optional description or definition.
    pub description: Option<String>,
    /// Related concepts mentioned in the same context.
    pub related: Vec<String>,
}

impl Concept {
    /// Create a new concept with just a label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            concept_type: ConceptType::default(),
            confidence: 1.0,
            description: None,
            related: Vec::new(),
        }
    }

    /// Set the concept type.
    pub fn with_type(mut self, concept_type: ConceptType) -> Self {
        self.concept_type = concept_type;
        self
    }

    /// Set the confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add related concepts.
    pub fn with_related(mut self, related: Vec<String>) -> Self {
        self.related = related;
        self
    }
}

/// Type of relationship between concepts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    /// A is a type of B (hypernym).
    IsA,
    /// A is part of B (meronym).
    PartOf,
    /// A causes B.
    Causes,
    /// A enables B.
    Enables,
    /// A requires B.
    Requires,
    /// A is related to B (general).
    RelatedTo,
    /// A produces B.
    Produces,
    /// A regulates B.
    Regulates,
    /// A interacts with B.
    InteractsWith,
    /// A is located in B.
    LocatedIn,
    /// Custom relationship type.
    Custom(String),
}

impl Default for RelationType {
    fn default() -> Self {
        Self::RelatedTo
    }
}

/// A relationship between two concepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Source concept.
    pub source: String,
    /// Target concept.
    pub target: String,
    /// Type of relationship.
    pub relation_type: RelationType,
    /// Relationship label (human-readable).
    pub label: String,
    /// Confidence score (0.0-1.0).
    pub confidence: f32,
    /// Whether the relationship is bidirectional.
    pub bidirectional: bool,
}

impl Relationship {
    /// Create a new relationship.
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            relation_type: RelationType::default(),
            label: label.into(),
            confidence: 1.0,
            bidirectional: false,
        }
    }

    /// Set the relationship type.
    pub fn with_type(mut self, relation_type: RelationType) -> Self {
        self.relation_type = relation_type;
        self
    }

    /// Set the confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Mark as bidirectional.
    pub fn bidirectional(mut self) -> Self {
        self.bidirectional = true;
        self
    }
}

/// Response from concept extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResponse {
    /// Extracted concepts.
    pub concepts: Vec<Concept>,
    /// Identified relationships.
    pub relationships: Vec<Relationship>,
    /// Raw LLM response (for debugging).
    pub raw_response: Option<String>,
    /// Tokens used in the request.
    pub tokens_used: Option<u32>,
}

impl ExtractionResponse {
    /// Create an empty response.
    pub fn empty() -> Self {
        Self {
            concepts: Vec::new(),
            relationships: Vec::new(),
            raw_response: None,
            tokens_used: None,
        }
    }

    /// Create from concepts only.
    pub fn from_concepts(concepts: Vec<Concept>) -> Self {
        Self {
            concepts,
            relationships: Vec::new(),
            raw_response: None,
            tokens_used: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concept_builder() {
        let concept = Concept::new("mitochondria")
            .with_type(ConceptType::Concept)
            .with_confidence(0.95)
            .with_description("Powerhouse of the cell")
            .with_related(vec!["ATP".into(), "cell".into()]);

        assert_eq!(concept.label, "mitochondria");
        assert_eq!(concept.concept_type, ConceptType::Concept);
        assert!((concept.confidence - 0.95).abs() < 0.001);
        assert_eq!(concept.description, Some("Powerhouse of the cell".into()));
        assert_eq!(concept.related.len(), 2);
    }

    #[test]
    fn test_relationship_builder() {
        let rel = Relationship::new("mitochondria", "ATP", "produces")
            .with_type(RelationType::Produces)
            .with_confidence(0.9);

        assert_eq!(rel.source, "mitochondria");
        assert_eq!(rel.target, "ATP");
        assert_eq!(rel.relation_type, RelationType::Produces);
    }
}
