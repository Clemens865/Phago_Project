//! Phago Agents Prelude — convenient imports for common usage.
//!
//! ```rust
//! use phago_agents::prelude::*;
//! ```

// Re-export agent types
pub use crate::digester::Digester;
pub use crate::sentinel::Sentinel;
pub use crate::synthesizer::Synthesizer;
pub use crate::code_digester::{CodeElement, CodeElementKind, extract_code_elements, elements_to_document};
pub use crate::genome::AgentGenome;
pub use crate::fitness::{AgentFitness, FitnessTracker};
pub use crate::spawn::{SpawnPolicy, FitnessSpawnPolicy};

// Re-export from core
pub use phago_core::prelude::*;
