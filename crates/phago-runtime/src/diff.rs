//! Graph diffing — compare two colony snapshots.
//!
//! Produces a structural changelog between two `GraphState` snapshots,
//! useful for understanding how the knowledge graph evolves over time.
//!
//! # Example
//!
//! ```rust,ignore
//! use phago_runtime::session::{save_session, load_session};
//! use phago_runtime::diff::diff_sessions;
//!
//! let before = load_session(&Path::new("session_v1.json"))?;
//! let after = load_session(&Path::new("session_v2.json"))?;
//! let diff = diff_sessions(&before, &after);
//! println!("{}", diff.summary());
//! ```

use crate::session::{GraphState, SerializedEdge, SerializedNode};
use std::collections::{HashMap, HashSet};

/// The result of diffing two graph snapshots.
#[derive(Debug, Clone)]
pub struct GraphDiff {
    /// Nodes present in `after` but not `before`.
    pub nodes_added: Vec<String>,
    /// Nodes present in `before` but not `after`.
    pub nodes_removed: Vec<String>,
    /// Edges present in `after` but not `before`.
    pub edges_added: Vec<(String, String)>,
    /// Edges present in `before` but not `after`.
    pub edges_removed: Vec<(String, String)>,
    /// Edges whose weight increased between snapshots.
    pub edges_strengthened: Vec<EdgeWeightChange>,
    /// Edges whose weight decreased between snapshots.
    pub edges_weakened: Vec<EdgeWeightChange>,
    /// Tick of `before` snapshot.
    pub before_tick: u64,
    /// Tick of `after` snapshot.
    pub after_tick: u64,
}

/// A change in edge weight between two snapshots.
#[derive(Debug, Clone)]
pub struct EdgeWeightChange {
    pub from: String,
    pub to: String,
    pub before_weight: f64,
    pub after_weight: f64,
}

impl EdgeWeightChange {
    /// Absolute change in weight.
    pub fn delta(&self) -> f64 {
        self.after_weight - self.before_weight
    }
}

/// An edge key for matching between snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EdgeKey(String, String);

impl EdgeKey {
    /// Create a canonical key (sorted labels for undirected matching).
    fn new(from: &str, to: &str) -> Self {
        if from <= to {
            Self(from.to_string(), to.to_string())
        } else {
            Self(to.to_string(), from.to_string())
        }
    }
}

/// Compare two graph snapshots and produce a structural diff.
///
/// Nodes are matched by label+type. Edges are matched by (from_label, to_label).
pub fn diff_sessions(before: &GraphState, after: &GraphState) -> GraphDiff {
    // Match nodes by label
    let before_labels: HashSet<&str> = before.nodes.iter().map(|n| n.label.as_str()).collect();
    let after_labels: HashSet<&str> = after.nodes.iter().map(|n| n.label.as_str()).collect();

    let nodes_added: Vec<String> = after_labels
        .difference(&before_labels)
        .map(|s| s.to_string())
        .collect();

    let nodes_removed: Vec<String> = before_labels
        .difference(&after_labels)
        .map(|s| s.to_string())
        .collect();

    // Build edge maps
    let before_edges: HashMap<EdgeKey, f64> = before
        .edges
        .iter()
        .map(|e| (EdgeKey::new(&e.from_label, &e.to_label), e.weight))
        .collect();

    let after_edges: HashMap<EdgeKey, f64> = after
        .edges
        .iter()
        .map(|e| (EdgeKey::new(&e.from_label, &e.to_label), e.weight))
        .collect();

    let before_edge_keys: HashSet<&EdgeKey> = before_edges.keys().collect();
    let after_edge_keys: HashSet<&EdgeKey> = after_edges.keys().collect();

    let edges_added: Vec<(String, String)> = after_edge_keys
        .difference(&before_edge_keys)
        .map(|k| (k.0.clone(), k.1.clone()))
        .collect();

    let edges_removed: Vec<(String, String)> = before_edge_keys
        .difference(&after_edge_keys)
        .map(|k| (k.0.clone(), k.1.clone()))
        .collect();

    // Find weight changes for edges that exist in both
    let mut edges_strengthened = vec![];
    let mut edges_weakened = vec![];

    for key in before_edge_keys.intersection(&after_edge_keys) {
        let bw = before_edges[key];
        let aw = after_edges[key];
        let delta = aw - bw;

        if delta.abs() > 1e-6 {
            let change = EdgeWeightChange {
                from: key.0.clone(),
                to: key.1.clone(),
                before_weight: bw,
                after_weight: aw,
            };
            if delta > 0.0 {
                edges_strengthened.push(change);
            } else {
                edges_weakened.push(change);
            }
        }
    }

    // Sort by absolute delta
    edges_strengthened.sort_by(|a, b| {
        b.delta()
            .abs()
            .partial_cmp(&a.delta().abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    edges_weakened.sort_by(|a, b| {
        b.delta()
            .abs()
            .partial_cmp(&a.delta().abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    GraphDiff {
        nodes_added,
        nodes_removed,
        edges_added,
        edges_removed,
        edges_strengthened,
        edges_weakened,
        before_tick: before.metadata.tick,
        after_tick: after.metadata.tick,
    }
}

impl GraphDiff {
    /// Human-readable summary of the changes.
    pub fn summary(&self) -> String {
        format!(
            "Since tick {} → {}: +{} nodes, -{} nodes, +{} edges, -{} edges, \
             {} edges strengthened, {} edges weakened",
            self.before_tick,
            self.after_tick,
            self.nodes_added.len(),
            self.nodes_removed.len(),
            self.edges_added.len(),
            self.edges_removed.len(),
            self.edges_strengthened.len(),
            self.edges_weakened.len(),
        )
    }

    /// Total number of changes.
    pub fn total_changes(&self) -> usize {
        self.nodes_added.len()
            + self.nodes_removed.len()
            + self.edges_added.len()
            + self.edges_removed.len()
            + self.edges_strengthened.len()
            + self.edges_weakened.len()
    }

    /// Whether the two snapshots are identical.
    pub fn is_empty(&self) -> bool {
        self.total_changes() == 0
    }

    /// Apply this diff to the `before` state to reconstruct the `after` state.
    ///
    /// Returns a new GraphState that should be equivalent to the `after` snapshot.
    pub fn apply(&self, before: &GraphState) -> GraphState {
        let mut nodes: Vec<SerializedNode> = before
            .nodes
            .iter()
            .filter(|n| !self.nodes_removed.contains(&n.label))
            .cloned()
            .collect();

        // Add new nodes (minimal data since we don't have full NodeData)
        for label in &self.nodes_added {
            nodes.push(SerializedNode {
                label: label.clone(),
                node_type: "Concept".to_string(),
                access_count: 0,
                position_x: 0.0,
                position_y: 0.0,
                created_tick: self.after_tick,
                embedding: None,
            });
        }

        // Build edge set
        let removed_edges: HashSet<EdgeKey> = self
            .edges_removed
            .iter()
            .map(|(f, t)| EdgeKey::new(f, t))
            .collect();

        let mut edges: Vec<SerializedEdge> = before
            .edges
            .iter()
            .filter(|e| {
                let key = EdgeKey::new(&e.from_label, &e.to_label);
                !removed_edges.contains(&key)
            })
            .cloned()
            .collect();

        // Apply weight changes
        let strengthened: HashMap<EdgeKey, f64> = self
            .edges_strengthened
            .iter()
            .map(|c| (EdgeKey::new(&c.from, &c.to), c.after_weight))
            .collect();

        let weakened: HashMap<EdgeKey, f64> = self
            .edges_weakened
            .iter()
            .map(|c| (EdgeKey::new(&c.from, &c.to), c.after_weight))
            .collect();

        for edge in &mut edges {
            let key = EdgeKey::new(&edge.from_label, &edge.to_label);
            if let Some(&new_weight) = strengthened.get(&key) {
                edge.weight = new_weight;
            }
            if let Some(&new_weight) = weakened.get(&key) {
                edge.weight = new_weight;
            }
        }

        // Add new edges
        for (from, to) in &self.edges_added {
            edges.push(SerializedEdge {
                from_label: from.clone(),
                to_label: to.clone(),
                weight: 0.5,
                co_activations: 1,
                created_tick: self.after_tick,
                last_activated_tick: self.after_tick,
            });
        }

        GraphState {
            nodes,
            edges,
            agents: vec![],
            metadata: crate::session::SessionMetadata {
                session_id: format!("diff-applied-{}", self.after_tick),
                tick: self.after_tick,
                node_count: 0, // Will be updated
                edge_count: 0,
                agent_count: 0,
                files_indexed: vec![],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionMetadata;

    fn make_node(label: &str) -> SerializedNode {
        SerializedNode {
            label: label.to_string(),
            node_type: "Concept".to_string(),
            access_count: 1,
            position_x: 0.0,
            position_y: 0.0,
            created_tick: 0,
            embedding: None,
        }
    }

    fn make_edge(from: &str, to: &str, weight: f64) -> SerializedEdge {
        SerializedEdge {
            from_label: from.to_string(),
            to_label: to.to_string(),
            weight,
            co_activations: 1,
            created_tick: 0,
            last_activated_tick: 0,
        }
    }

    fn make_metadata(tick: u64, nodes: usize, edges: usize) -> SessionMetadata {
        SessionMetadata {
            session_id: "test".to_string(),
            tick,
            node_count: nodes,
            edge_count: edges,
            agent_count: 0,
            files_indexed: vec![],
        }
    }

    #[test]
    fn identical_snapshots_produce_empty_diff() {
        let state = GraphState {
            nodes: vec![make_node("cell"), make_node("membrane")],
            edges: vec![make_edge("cell", "membrane", 0.8)],
            agents: vec![],
            metadata: make_metadata(10, 2, 1),
        };
        let diff = diff_sessions(&state, &state);
        assert!(diff.is_empty());
    }

    #[test]
    fn detects_added_nodes_and_edges() {
        let before = GraphState {
            nodes: vec![make_node("cell")],
            edges: vec![],
            agents: vec![],
            metadata: make_metadata(0, 1, 0),
        };

        let after = GraphState {
            nodes: vec![make_node("cell"), make_node("membrane")],
            edges: vec![make_edge("cell", "membrane", 0.5)],
            agents: vec![],
            metadata: make_metadata(10, 2, 1),
        };

        let diff = diff_sessions(&before, &after);
        assert_eq!(diff.nodes_added.len(), 1);
        assert_eq!(diff.nodes_added[0], "membrane");
        assert_eq!(diff.edges_added.len(), 1);
        assert!(diff.nodes_removed.is_empty());
        assert!(diff.edges_removed.is_empty());
    }

    #[test]
    fn detects_removed_nodes() {
        let before = GraphState {
            nodes: vec![make_node("cell"), make_node("old")],
            edges: vec![],
            agents: vec![],
            metadata: make_metadata(0, 2, 0),
        };

        let after = GraphState {
            nodes: vec![make_node("cell")],
            edges: vec![],
            agents: vec![],
            metadata: make_metadata(10, 1, 0),
        };

        let diff = diff_sessions(&before, &after);
        assert_eq!(diff.nodes_removed.len(), 1);
        assert_eq!(diff.nodes_removed[0], "old");
    }

    #[test]
    fn detects_weight_changes() {
        let before = GraphState {
            nodes: vec![make_node("a"), make_node("b")],
            edges: vec![make_edge("a", "b", 0.3)],
            agents: vec![],
            metadata: make_metadata(0, 2, 1),
        };

        let after = GraphState {
            nodes: vec![make_node("a"), make_node("b")],
            edges: vec![make_edge("a", "b", 0.9)],
            agents: vec![],
            metadata: make_metadata(10, 2, 1),
        };

        let diff = diff_sessions(&before, &after);
        assert_eq!(diff.edges_strengthened.len(), 1);
        assert!(diff.edges_weakened.is_empty());
        let change = &diff.edges_strengthened[0];
        assert!((change.before_weight - 0.3).abs() < 1e-6);
        assert!((change.after_weight - 0.9).abs() < 1e-6);
    }

    #[test]
    fn summary_format() {
        let before = GraphState {
            nodes: vec![make_node("a")],
            edges: vec![],
            agents: vec![],
            metadata: make_metadata(0, 1, 0),
        };
        let after = GraphState {
            nodes: vec![make_node("a"), make_node("b"), make_node("c")],
            edges: vec![make_edge("a", "b", 0.5)],
            agents: vec![],
            metadata: make_metadata(50, 3, 1),
        };

        let diff = diff_sessions(&before, &after);
        let summary = diff.summary();
        assert!(summary.contains("+2 nodes"));
        assert!(summary.contains("+1 edges"));
        assert!(summary.contains("0 → 50"));
    }

    #[test]
    fn apply_reconstructs_after() {
        let before = GraphState {
            nodes: vec![make_node("a")],
            edges: vec![],
            agents: vec![],
            metadata: make_metadata(0, 1, 0),
        };
        let after = GraphState {
            nodes: vec![make_node("a"), make_node("b")],
            edges: vec![make_edge("a", "b", 0.5)],
            agents: vec![],
            metadata: make_metadata(10, 2, 1),
        };

        let diff = diff_sessions(&before, &after);
        let reconstructed = diff.apply(&before);

        assert_eq!(reconstructed.nodes.len(), 2);
        assert_eq!(reconstructed.edges.len(), 1);
    }
}
