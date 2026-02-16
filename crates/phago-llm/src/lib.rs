//! # Phago LLM
//!
//! LLM integration for Phago semantic intelligence.
//!
//! This crate provides optional LLM backends for enhanced concept extraction,
//! relationship labeling, and query expansion.
//!
//! ## Features
//!
//! - `api`: Cloud API backends (Claude, OpenAI)
//! - `local`: Local backends (Ollama)
//! - `full`: All backends
//!
//! ## Usage
//!
//! ```rust,ignore
//! use phago_llm::{LlmBackend, OllamaBackend};
//!
//! let backend = OllamaBackend::new("http://localhost:11434");
//! let concepts = backend.extract_concepts("Cell membrane transport").await?;
//! ```

mod backend;
mod prompt;
mod types;

pub use backend::{LlmBackend, LlmError, LlmResult};
pub use prompt::{ConceptPrompt, PromptTemplate, RelationshipPrompt};
pub use types::{Concept, ConceptType, RelationType, Relationship};

#[cfg(feature = "local")]
mod ollama;
#[cfg(feature = "local")]
pub use ollama::OllamaBackend;

#[cfg(feature = "api")]
mod claude;
#[cfg(feature = "api")]
pub use claude::ClaudeBackend;

#[cfg(feature = "api")]
mod openai;
#[cfg(feature = "api")]
pub use openai::OpenAiBackend;

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::{Concept, ConceptType, RelationType, Relationship};
    pub use crate::{ConceptPrompt, PromptTemplate};
    pub use crate::{LlmBackend, LlmError, LlmResult};

    #[cfg(feature = "local")]
    pub use crate::OllamaBackend;

    #[cfg(feature = "api")]
    pub use crate::{ClaudeBackend, OpenAiBackend};
}
