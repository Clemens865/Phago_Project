//! MCP Adapter — Model Context Protocol interface for Phago.
//!
//! Provides three core tools for external LLMs/agents to interact
//! with the biological knowledge graph:
//!
//! - `phago_remember`: Ingest text into the colony (document → digestion → graph)
//! - `phago_recall`: Query the knowledge graph with hybrid scoring
//! - `phago_explore`: Structural queries (paths, bridges, centrality, components)
//!
//! All operations use serializable request/response types compatible
//! with JSON-RPC or any other transport layer.

use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use phago_runtime::colony::Colony;
use serde::{Deserialize, Serialize};

// === phago_remember ===

#[derive(Debug, Deserialize)]
pub struct RememberRequest {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub ticks: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct RememberResponse {
    pub document_id: String,
    pub nodes_created: usize,
    pub edges_created: usize,
    pub tick: u64,
}

/// Ingest a document into the colony and run digestion.
pub fn phago_remember(colony: &mut Colony, req: &RememberRequest) -> RememberResponse {
    use phago_agents::digester::Digester;

    let before_nodes = colony.stats().graph_nodes;
    let before_edges = colony.stats().graph_edges;

    let doc_id = colony.ingest_document(&req.title, &req.content, Position::new(0.0, 0.0));

    // Spawn a digester to process the document
    colony.spawn(Box::new(
        Digester::new(Position::new(0.0, 0.0)).with_max_idle(30),
    ));

    // Run enough ticks for digestion
    let ticks = req.ticks.unwrap_or(15);
    colony.run(ticks);

    let after_nodes = colony.stats().graph_nodes;
    let after_edges = colony.stats().graph_edges;

    RememberResponse {
        document_id: format!("{}", doc_id.0),
        nodes_created: after_nodes.saturating_sub(before_nodes),
        edges_created: after_edges.saturating_sub(before_edges),
        tick: colony.stats().tick,
    }
}

// === phago_recall ===

#[derive(Debug, Deserialize)]
pub struct RecallRequest {
    pub query: String,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_alpha")]
    pub alpha: f64,
}

fn default_max_results() -> usize { 10 }
fn default_alpha() -> f64 { 0.5 }

#[derive(Debug, Serialize)]
pub struct RecallResult {
    pub label: String,
    pub score: f64,
    pub tfidf_score: f64,
    pub graph_score: f64,
}

#[derive(Debug, Serialize)]
pub struct RecallResponse {
    pub results: Vec<RecallResult>,
    pub total_nodes: usize,
    pub total_edges: usize,
}

/// Query the knowledge graph using hybrid scoring.
pub fn phago_recall(colony: &Colony, req: &RecallRequest) -> RecallResponse {
    use crate::hybrid::{hybrid_query, HybridConfig};

    let config = HybridConfig {
        alpha: req.alpha,
        max_results: req.max_results,
        candidate_multiplier: 3,
    };

    let results = hybrid_query(colony, &req.query, &config);

    RecallResponse {
        results: results
            .into_iter()
            .map(|r| RecallResult {
                label: r.label,
                score: r.final_score,
                tfidf_score: r.tfidf_score,
                graph_score: r.graph_score,
            })
            .collect(),
        total_nodes: colony.stats().graph_nodes,
        total_edges: colony.stats().graph_edges,
    }
}

// === phago_explore ===

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ExploreRequest {
    #[serde(rename = "path")]
    ShortestPath { from: String, to: String },
    #[serde(rename = "centrality")]
    Centrality {
        #[serde(default = "default_top_k")]
        top_k: usize,
    },
    #[serde(rename = "bridges")]
    Bridges {
        #[serde(default = "default_top_k")]
        top_k: usize,
    },
    #[serde(rename = "stats")]
    Stats,
}

fn default_top_k() -> usize { 10 }

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ExploreResponse {
    #[serde(rename = "path")]
    Path {
        found: bool,
        path: Vec<String>,
        cost: f64,
    },
    #[serde(rename = "centrality")]
    Centrality {
        nodes: Vec<CentralityEntry>,
    },
    #[serde(rename = "bridges")]
    Bridges {
        nodes: Vec<BridgeEntry>,
    },
    #[serde(rename = "stats")]
    Stats {
        total_nodes: usize,
        total_edges: usize,
        connected_components: usize,
        tick: u64,
        agents_alive: usize,
    },
}

#[derive(Debug, Serialize)]
pub struct CentralityEntry {
    pub label: String,
    pub centrality: f64,
}

#[derive(Debug, Serialize)]
pub struct BridgeEntry {
    pub label: String,
    pub fragility: f64,
}

/// Explore the graph structure.
pub fn phago_explore(colony: &Colony, req: &ExploreRequest) -> ExploreResponse {
    let graph = colony.substrate().graph();

    match req {
        ExploreRequest::ShortestPath { from, to } => {
            let from_nodes = graph.find_nodes_by_label(from);
            let to_nodes = graph.find_nodes_by_label(to);

            if let (Some(&from_id), Some(&to_id)) = (from_nodes.first(), to_nodes.first()) {
                if let Some((path, cost)) = graph.shortest_path(&from_id, &to_id) {
                    let labels: Vec<String> = path
                        .iter()
                        .filter_map(|nid| graph.get_node(nid).map(|n| n.label.clone()))
                        .collect();
                    ExploreResponse::Path {
                        found: true,
                        path: labels,
                        cost,
                    }
                } else {
                    ExploreResponse::Path {
                        found: false,
                        path: Vec::new(),
                        cost: 0.0,
                    }
                }
            } else {
                ExploreResponse::Path {
                    found: false,
                    path: Vec::new(),
                    cost: 0.0,
                }
            }
        }
        ExploreRequest::Centrality { top_k } => {
            let centrality = graph.betweenness_centrality(100);
            let entries: Vec<CentralityEntry> = centrality
                .into_iter()
                .take(*top_k)
                .filter_map(|(nid, c)| {
                    graph.get_node(&nid).map(|n| CentralityEntry {
                        label: n.label.clone(),
                        centrality: c,
                    })
                })
                .collect();
            ExploreResponse::Centrality { nodes: entries }
        }
        ExploreRequest::Bridges { top_k } => {
            let bridges = graph.bridge_nodes(*top_k);
            let entries: Vec<BridgeEntry> = bridges
                .into_iter()
                .filter_map(|(nid, f)| {
                    graph.get_node(&nid).map(|n| BridgeEntry {
                        label: n.label.clone(),
                        fragility: f,
                    })
                })
                .collect();
            ExploreResponse::Bridges { nodes: entries }
        }
        ExploreRequest::Stats => {
            let stats = colony.stats();
            ExploreResponse::Stats {
                total_nodes: stats.graph_nodes,
                total_edges: stats.graph_edges,
                connected_components: graph.connected_components(),
                tick: stats.tick,
                agents_alive: stats.agents_alive,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remember_creates_nodes_and_edges() {
        let mut colony = Colony::new();
        let req = RememberRequest {
            title: "Biology 101".into(),
            content: "The cell membrane controls transport of molecules and proteins".into(),
            ticks: Some(15),
        };
        let resp = phago_remember(&mut colony, &req);
        assert!(resp.nodes_created > 0, "should create nodes");
    }

    #[test]
    fn recall_returns_results() {
        let mut colony = Colony::new();
        let _ = phago_remember(&mut colony, &RememberRequest {
            title: "Bio".into(),
            content: "cell membrane protein transport channel receptor".into(),
            ticks: Some(15),
        });
        let _ = phago_remember(&mut colony, &RememberRequest {
            title: "Bio2".into(),
            content: "cell membrane protein signaling pathway cascade".into(),
            ticks: Some(15),
        });

        let resp = phago_recall(&colony, &RecallRequest {
            query: "cell membrane".into(),
            max_results: 5,
            alpha: 0.5,
        });
        assert!(!resp.results.is_empty(), "should return results");
    }

    #[test]
    fn explore_stats_works() {
        let mut colony = Colony::new();
        let _ = phago_remember(&mut colony, &RememberRequest {
            title: "Bio".into(),
            content: "cell membrane protein".into(),
            ticks: Some(15),
        });

        let resp = phago_explore(&colony, &ExploreRequest::Stats);
        match resp {
            ExploreResponse::Stats { total_nodes, .. } => {
                assert!(total_nodes > 0, "should have nodes");
            }
            _ => panic!("expected Stats response"),
        }
    }
}
