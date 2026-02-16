//! Phago Agents Prelude — convenient imports for common usage.
//!
//! ```rust
//! use phago_agents::prelude::*;
//! ```

// Re-export agent types
pub use crate::code_digester::{
    elements_to_document, extract_code_elements, CodeElement, CodeElementKind,
};
pub use crate::digester::Digester;
pub use crate::fitness::{AgentFitness, FitnessTracker};
pub use crate::genome::AgentGenome;
pub use crate::sentinel::Sentinel;
pub use crate::serialize::{AgentType, SerializableAgent, SerializedAgent};
pub use crate::spawn::{FitnessSpawnPolicy, SpawnPolicy};
pub use crate::synthesizer::Synthesizer;

// Semantic digester (requires "semantic" feature)
#[cfg(feature = "semantic")]
pub use crate::semantic_digester::{SemanticConcept, SemanticConfig, SemanticDigester};

// Re-export from core
pub use phago_core::prelude::*;
