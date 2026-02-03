//! Phago RAG Prelude — convenient imports for common usage.
//!
//! ```rust
//! use phago_rag::prelude::*;
//! ```

// Re-export query types
pub use crate::query::{Query, QueryResult, QueryEngine};
pub use crate::hybrid::{hybrid_query, HybridConfig, HybridResult};
pub use crate::scoring::{QueryScores, AggregateScores, precision_at_k, mrr, ndcg_at_k, score_query, aggregate};
pub use crate::baseline::{tfidf_query, static_graph_query, random_query};

// Re-export MCP types
pub use crate::mcp::{
    phago_remember, phago_recall, phago_explore,
    RememberRequest, RememberResponse,
    RecallRequest, RecallResponse, RecallResult,
    ExploreRequest, ExploreResponse,
    CentralityEntry, BridgeEntry,
};

// Re-export from runtime
pub use phago_runtime::prelude::*;
