//! Phago Core Prelude — convenient imports for common usage.
//!
//! ```rust
//! use phago_core::prelude::*;
//! ```

// Re-export commonly used types
pub use crate::types::{
    AgentId, NodeId, DocumentId,
    Position, Document,
    Signal, SignalType, Gradient,
    Trace, TraceType,
    CellHealth, DeathSignal, DeathCause,
    NodeData, NodeType, EdgeData,
    AgentAction, FragmentPresentation,
    DigestionResult,
    SymbiosisEval, SymbiontInfo, SymbiosisFailure,
    Classification,
    BoundaryContext,
    Orientation,
    Tick,
};

// Re-export the Agent trait
pub use crate::agent::Agent;

// Re-export the Substrate trait
pub use crate::substrate::Substrate;

// Re-export the TopologyGraph trait
pub use crate::topology::TopologyGraph;

// Re-export error types
pub use crate::error::{PhagoError, Result};

// Re-export semantic utilities
pub use crate::semantic::{
    cosine_similarity, normalized_similarity, compute_semantic_weight,
    l2_distance, dot_product, l2_normalize, l2_normalized,
    SemanticWiringConfig,
};
