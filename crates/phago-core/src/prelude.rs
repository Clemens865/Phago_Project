//! Phago Core Prelude — convenient imports for common usage.
//!
//! ```rust
//! use phago_core::prelude::*;
//! ```

// Re-export commonly used types
pub use crate::types::{
    AgentAction, AgentId, BoundaryContext, CellHealth, Classification, DeathCause, DeathSignal,
    DigestionResult, Document, DocumentId, EdgeData, FragmentPresentation, Gradient, NodeData,
    NodeId, NodeType, Orientation, Position, Signal, SignalType, SymbiontInfo, SymbiosisEval,
    SymbiosisFailure, Tick, Trace, TraceType,
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
    compute_semantic_weight, cosine_similarity, dot_product, l2_distance, l2_normalize,
    l2_normalized, normalized_similarity, SemanticWiringConfig,
};

// Re-export Louvain community detection
pub use crate::louvain::{compute_modularity, louvain_communities, LouvainResult};
