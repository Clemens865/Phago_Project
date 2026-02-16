//! # Phago RAG
//!
//! Biological Retrieval-Augmented Generation.
//!
//! Instead of vector similarity search, queries traverse the Hebbian knowledge
//! graph following strongest connections. The graph learns from usage —
//! frequently traversed paths strengthen, unused ones decay.
//!
//! ## How it differs from standard RAG
//!
//! | Standard RAG | Phago RAG |
//! |-------------|-----------|
//! | Chunk → embed → vector search | Digest → wire → graph traversal |
//! | Static index | Self-reinforcing graph |
//! | No learning from queries | Traversed paths strengthen |
//! | No anomaly detection | Sentinels flag what doesn't fit |
//! | Flat retrieval | Structured, weighted paths |

pub mod baseline;
pub mod code_query;
pub mod counterfactual;
pub mod hybrid;
pub mod mcp;
pub mod prelude;
pub mod query;
pub mod scoring;

pub use hybrid::{hybrid_query, HybridConfig, HybridResult};
pub use mcp::{phago_explore, phago_recall, phago_remember};
pub use query::{Query, QueryEngine, QueryResult};
