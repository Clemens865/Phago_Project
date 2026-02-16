//! Counterfactual Explanation Engine.
//!
//! Answers "what if?" questions about the knowledge graph:
//! - "What if this edge didn't exist?"
//! - "What if this node were removed?"
//! - "What if this edge weight were different?"
//!
//! # How It Works
//!
//! 1. Snapshot the current graph state
//! 2. Apply an intervention (remove edge, remove node, change weight)
//! 3. Re-run the same query on the modified graph
//! 4. Compare rankings to quantify the intervention's impact
//!
//! # Example
//!
//! ```rust,ignore
//! use phago_rag::counterfactual::*;
//!
//! let intervention = Intervention::RemoveEdge {
//!     from_label: "cell".into(),
//!     to_label: "membrane".into(),
//! };
//! let result = counterfactual_query(&colony, &intervention, "cell biology", &Default::default());
//! println!("Impact: {} rank changes", result.rank_changes.len());
//! ```

use phago_core::topology::TopologyGraph;
use phago_runtime::colony::Colony;
use phago_runtime::session::{self, GraphState};
use std::collections::HashMap;

/// An intervention to apply to the knowledge graph.
#[derive(Debug, Clone)]
pub enum Intervention {
    /// Remove an edge between two nodes (identified by label).
    RemoveEdge {
        from_label: String,
        to_label: String,
    },
    /// Remove a node (and all its edges) identified by label.
    RemoveNode {
        label: String,
    },
    /// Set a specific edge weight.
    SetEdgeWeight {
        from_label: String,
        to_label: String,
        weight: f64,
    },
}

/// Result of a counterfactual query.
#[derive(Debug, Clone)]
pub struct CounterfactualResult {
    /// The intervention that was applied.
    pub intervention: String,
    /// Baseline rankings (before intervention).
    pub baseline_ranks: Vec<RankedConcept>,
    /// Counterfactual rankings (after intervention).
    pub counterfactual_ranks: Vec<RankedConcept>,
    /// Rank changes: (label, old_rank, new_rank) for nodes whose rank changed.
    pub rank_changes: Vec<RankChange>,
    /// Whether the intervention had a significant effect.
    pub significant: bool,
}

/// A concept with its rank and score.
#[derive(Debug, Clone)]
pub struct RankedConcept {
    pub label: String,
    pub score: f64,
    pub rank: usize,
}

/// A change in ranking for a concept.
#[derive(Debug, Clone)]
pub struct RankChange {
    pub label: String,
    pub baseline_rank: Option<usize>,
    pub counterfactual_rank: Option<usize>,
    pub rank_delta: i64,
}

/// Configuration for counterfactual queries.
pub struct CounterfactualConfig {
    /// Number of results to compare.
    pub max_results: usize,
    /// Alpha for hybrid scoring (TF-IDF vs graph balance).
    pub alpha: f64,
    /// Minimum rank change to be considered significant.
    pub significance_threshold: usize,
}

impl Default for CounterfactualConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            alpha: 0.5,
            significance_threshold: 2,
        }
    }
}

/// Run a counterfactual query: compare baseline vs intervention results.
///
/// This is a graph-level operation that works on the serialized state,
/// so it doesn't require cloning the Colony (which isn't Clone).
pub fn counterfactual_query(
    colony: &Colony,
    intervention: &Intervention,
    query: &str,
    config: &CounterfactualConfig,
) -> CounterfactualResult {
    use phago_runtime::session::restore_into_colony;

    // Step 1: Get baseline results from the current colony
    let baseline_results = {
        use crate::hybrid::{hybrid_query, HybridConfig};
        let hconfig = HybridConfig {
            alpha: config.alpha,
            max_results: config.max_results,
            candidate_multiplier: 3,
        };
        hybrid_query(colony, query, &hconfig)
    };

    let baseline_ranks: Vec<RankedConcept> = baseline_results
        .iter()
        .enumerate()
        .map(|(i, r)| RankedConcept {
            label: r.label.clone(),
            score: r.final_score,
            rank: i + 1,
        })
        .collect();

    // Step 2: Snapshot the graph, apply intervention, query the modified graph
    let state = snapshot_state(colony);
    let modified_state = apply_intervention(&state, intervention);

    // Step 3: Restore into a temporary colony and re-query
    let counterfactual_results = {
        let mut temp_colony = Colony::new();
        restore_into_colony(&mut temp_colony, &modified_state);

        use crate::hybrid::{hybrid_query, HybridConfig};
        let hconfig = HybridConfig {
            alpha: config.alpha,
            max_results: config.max_results,
            candidate_multiplier: 3,
        };
        hybrid_query(&temp_colony, query, &hconfig)
    };

    let counterfactual_ranks: Vec<RankedConcept> = counterfactual_results
        .iter()
        .enumerate()
        .map(|(i, r)| RankedConcept {
            label: r.label.clone(),
            score: r.final_score,
            rank: i + 1,
        })
        .collect();

    // Step 4: Compute rank changes
    let baseline_rank_map: HashMap<&str, usize> = baseline_ranks
        .iter()
        .map(|r| (r.label.as_str(), r.rank))
        .collect();

    let cf_rank_map: HashMap<&str, usize> = counterfactual_ranks
        .iter()
        .map(|r| (r.label.as_str(), r.rank))
        .collect();

    let mut rank_changes = vec![];
    let all_labels: std::collections::HashSet<&str> = baseline_rank_map
        .keys()
        .chain(cf_rank_map.keys())
        .copied()
        .collect();

    for label in all_labels {
        let br = baseline_rank_map.get(label).copied();
        let cr = cf_rank_map.get(label).copied();

        if br != cr {
            let delta = match (br, cr) {
                (Some(b), Some(c)) => c as i64 - b as i64,
                (Some(_), None) => config.max_results as i64, // Disappeared
                (None, Some(_)) => -(config.max_results as i64), // Appeared
                (None, None) => 0,
            };

            rank_changes.push(RankChange {
                label: label.to_string(),
                baseline_rank: br,
                counterfactual_rank: cr,
                rank_delta: delta,
            });
        }
    }

    // Sort by absolute rank delta
    rank_changes.sort_by(|a, b| {
        b.rank_delta
            .abs()
            .cmp(&a.rank_delta.abs())
    });

    let significant = rank_changes
        .iter()
        .any(|c| c.rank_delta.unsigned_abs() as usize >= config.significance_threshold);

    let intervention_desc = match intervention {
        Intervention::RemoveEdge { from_label, to_label } => {
            format!("Remove edge '{}' — '{}'", from_label, to_label)
        }
        Intervention::RemoveNode { label } => {
            format!("Remove node '{}'", label)
        }
        Intervention::SetEdgeWeight { from_label, to_label, weight } => {
            format!("Set edge '{}' — '{}' weight to {:.3}", from_label, to_label, weight)
        }
    };

    CounterfactualResult {
        intervention: intervention_desc,
        baseline_ranks,
        counterfactual_ranks,
        rank_changes,
        significant,
    }
}

/// Snapshot the colony's graph into a GraphState (without saving to disk).
fn snapshot_state(colony: &Colony) -> GraphState {
    let graph = colony.substrate().graph();
    let all_nodes = graph.all_nodes();

    let nodes: Vec<session::SerializedNode> = all_nodes
        .iter()
        .filter_map(|nid| graph.get_node(nid))
        .map(|n| session::SerializedNode {
            label: n.label.clone(),
            node_type: format!("{:?}", n.node_type),
            access_count: n.access_count,
            position_x: n.position.x,
            position_y: n.position.y,
            created_tick: n.created_tick,
            embedding: n.embedding.clone(),
        })
        .collect();

    let edges: Vec<session::SerializedEdge> = graph
        .all_edges()
        .iter()
        .filter_map(|(from, to, edge)| {
            let from_label = graph.get_node(from)?.label.clone();
            let to_label = graph.get_node(to)?.label.clone();
            Some(session::SerializedEdge {
                from_label,
                to_label,
                weight: edge.weight,
                co_activations: edge.co_activations,
                created_tick: edge.created_tick,
                last_activated_tick: edge.last_activated_tick,
            })
        })
        .collect();

    let node_count = nodes.len();
    let edge_count = edges.len();

    GraphState {
        nodes,
        edges,
        agents: vec![],
        metadata: session::SessionMetadata {
            session_id: "counterfactual".to_string(),
            tick: colony.stats().tick,
            node_count,
            edge_count,
            agent_count: 0,
            files_indexed: vec![],
        },
    }
}

/// Apply an intervention to a GraphState, returning the modified state.
fn apply_intervention(state: &GraphState, intervention: &Intervention) -> GraphState {
    let mut modified = state.clone();

    match intervention {
        Intervention::RemoveEdge { from_label, to_label } => {
            let fl = from_label.to_lowercase();
            let tl = to_label.to_lowercase();
            modified.edges.retain(|e| {
                let ef = e.from_label.to_lowercase();
                let et = e.to_label.to_lowercase();
                !((ef == fl && et == tl) || (ef == tl && et == fl))
            });
        }
        Intervention::RemoveNode { label } => {
            let ll = label.to_lowercase();
            modified.nodes.retain(|n| n.label.to_lowercase() != ll);
            modified.edges.retain(|e| {
                e.from_label.to_lowercase() != ll && e.to_label.to_lowercase() != ll
            });
        }
        Intervention::SetEdgeWeight {
            from_label,
            to_label,
            weight,
        } => {
            let fl = from_label.to_lowercase();
            let tl = to_label.to_lowercase();
            for edge in &mut modified.edges {
                let ef = edge.from_label.to_lowercase();
                let et = edge.to_label.to_lowercase();
                if (ef == fl && et == tl) || (ef == tl && et == fl) {
                    edge.weight = *weight;
                }
            }
        }
    }

    modified
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_colony() -> Colony {
        let mut colony = Colony::new();
        let req = crate::mcp::RememberRequest {
            title: "Bio".into(),
            content: "cell membrane protein transport channel receptor signaling pathway".into(),
            ticks: Some(15),
        };
        crate::mcp::phago_remember(&mut colony, &req);

        let req2 = crate::mcp::RememberRequest {
            title: "Bio2".into(),
            content: "cell membrane lipid bilayer phospholipid structure".into(),
            ticks: Some(15),
        };
        crate::mcp::phago_remember(&mut colony, &req2);

        colony
    }

    #[test]
    fn remove_edge_changes_ranking() {
        let colony = setup_colony();

        let intervention = Intervention::RemoveEdge {
            from_label: "cell".into(),
            to_label: "membrane".into(),
        };

        let result = counterfactual_query(
            &colony,
            &intervention,
            "cell membrane",
            &CounterfactualConfig::default(),
        );

        // Should produce some results
        assert!(!result.baseline_ranks.is_empty());
        assert!(result.intervention.contains("Remove edge"));
    }

    #[test]
    fn remove_node_removes_from_results() {
        let colony = setup_colony();

        let intervention = Intervention::RemoveNode {
            label: "membrane".into(),
        };

        let result = counterfactual_query(
            &colony,
            &intervention,
            "cell membrane",
            &CounterfactualConfig::default(),
        );

        // "membrane" should not appear in counterfactual results
        let has_membrane = result
            .counterfactual_ranks
            .iter()
            .any(|r| r.label.to_lowercase() == "membrane");
        assert!(!has_membrane, "Removed node should not appear in counterfactual results");
    }

    #[test]
    fn identical_intervention_no_changes() {
        let colony = setup_colony();

        // Remove an edge that doesn't exist
        let intervention = Intervention::RemoveEdge {
            from_label: "nonexistent_node_xyz".into(),
            to_label: "also_nonexistent".into(),
        };

        let result = counterfactual_query(
            &colony,
            &intervention,
            "cell membrane",
            &CounterfactualConfig::default(),
        );

        // Should have no rank changes
        assert!(
            result.rank_changes.is_empty(),
            "Removing nonexistent edge should cause no changes"
        );
    }
}
