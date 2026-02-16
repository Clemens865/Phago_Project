//! Phago RAG Prelude — convenient imports for common usage.
//!
//! ```rust
//! use phago_rag::prelude::*;
//! ```

// Re-export query types
pub use crate::baseline::{random_query, static_graph_query, tfidf_query};
pub use crate::hybrid::{hybrid_query, HybridConfig, HybridResult};
pub use crate::query::{Query, QueryEngine, QueryResult};
pub use crate::scoring::{
    aggregate, mrr, ndcg_at_k, precision_at_k, score_query, AggregateScores, QueryScores,
};

// Re-export MCP types
pub use crate::mcp::{
    phago_explore, phago_recall, phago_remember, BridgeEntry, CentralityEntry, ExploreRequest,
    ExploreResponse, RecallRequest, RecallResponse, RecallResult, RememberRequest,
    RememberResponse,
};

// Re-export from runtime
pub use phago_runtime::prelude::*;
