//! Phago MCP Server — Model Context Protocol interface for the
//! biological knowledge graph.
//!
//! Provides three tools:
//! - `phago_remember`: Ingest documents into the colony
//! - `phago_recall`: Hybrid query with TF-IDF + graph scoring
//! - `phago_explore`: Structural graph queries (paths, centrality, bridges, stats)
//!
//! Uses a dedicated worker thread for Colony operations (Colony is not
//! Send+Sync due to trait object agents).

pub mod tools;
pub mod worker;
