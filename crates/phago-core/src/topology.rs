//! Topology — the knowledge graph structure.
//!
//! The topology graph is the substrate's structural backbone.
//! It encodes relationships between concepts, documents, and insights.
//! Following Hebbian learning: the structure IS the memory.

use crate::types::*;

/// A handle to the topology graph, used by the Wire primitive.
///
/// This is a trait rather than a concrete type so that different
/// substrate implementations can use different graph backends.
pub trait TopologyGraph {
    /// Add a node and return its ID.
    fn add_node(&mut self, data: NodeData) -> NodeId;

    /// Get node data by ID.
    fn get_node(&self, id: &NodeId) -> Option<&NodeData>;

    /// Get mutable node data by ID.
    fn get_node_mut(&mut self, id: &NodeId) -> Option<&mut NodeData>;

    /// Add or update an edge. If the edge exists, the data is replaced.
    fn set_edge(&mut self, from: NodeId, to: NodeId, data: EdgeData);

    /// Get edge data.
    fn get_edge(&self, from: &NodeId, to: &NodeId) -> Option<&EdgeData>;

    /// Get mutable edge data.
    fn get_edge_mut(&mut self, from: &NodeId, to: &NodeId) -> Option<&mut EdgeData>;

    /// Get all neighbors of a node.
    fn neighbors(&self, node: &NodeId) -> Vec<(NodeId, &EdgeData)>;

    /// Remove an edge. Returns the removed edge data if it existed.
    fn remove_edge(&mut self, from: &NodeId, to: &NodeId) -> Option<EdgeData>;

    /// Get all node IDs.
    fn all_nodes(&self) -> Vec<NodeId>;

    /// Get all edges.
    fn all_edges(&self) -> Vec<(NodeId, NodeId, &EdgeData)>;

    /// Number of nodes.
    fn node_count(&self) -> usize;

    /// Number of edges.
    fn edge_count(&self) -> usize;

    /// Decay all edge weights by a rate (0.0-1.0).
    /// Returns edges that fell below the threshold after decay.
    fn decay_edges(&mut self, rate: f64, prune_threshold: f64) -> Vec<PrunedConnection>;

    /// Find nodes matching a label (substring match).
    fn find_nodes_by_label(&self, query: &str) -> Vec<NodeId>;
}
