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

    /// Activity-aware decay: edges that haven't been co-activated recently decay faster.
    /// Edges younger than `maturation_ticks` get base rate only.
    fn decay_edges_activity(
        &mut self,
        base_rate: f64,
        prune_threshold: f64,
        current_tick: u64,
        staleness_factor: f64,
        maturation_ticks: u64,
    ) -> Vec<PrunedConnection>;

    /// Competitive pruning: for each node, keep only top `max_degree` edges by weight.
    /// An edge survives if EITHER endpoint keeps it in their top-K.
    fn prune_to_max_degree(&mut self, max_degree: usize) -> Vec<PrunedConnection>;

    /// Find nodes matching a label (substring match).
    fn find_nodes_by_label(&self, query: &str) -> Vec<NodeId>;

    /// Find nodes with an exact label match (case-insensitive).
    /// Default implementation falls back to find_nodes_by_label with filtering.
    fn find_nodes_by_exact_label(&self, label: &str) -> Vec<NodeId> {
        // Default: use find_nodes_by_label and filter for exact matches
        let label_lower = label.to_lowercase();
        self.find_nodes_by_label(label)
            .into_iter()
            .filter(|id| {
                self.get_node(id)
                    .map_or(false, |n| n.label.to_lowercase() == label_lower)
            })
            .collect()
    }

    // --- Structural query types ---

    /// Find the shortest weighted path between two nodes.
    /// Returns (path_of_node_ids, total_weight). Uses inverse weight as cost
    /// so stronger edges are preferred.
    fn shortest_path(&self, from: &NodeId, to: &NodeId) -> Option<(Vec<NodeId>, f64)>;

    /// Compute betweenness centrality for all nodes (approximate, sampled).
    /// Returns (node_id, centrality_score) sorted descending by centrality.
    /// Centrality measures how often a node lies on shortest paths between
    /// other node pairs — high centrality = hub/bridge concept.
    fn betweenness_centrality(&self, sample_size: usize) -> Vec<(NodeId, f64)>;

    /// Find bridge nodes whose removal would most increase graph fragmentation.
    /// Returns (node_id, fragility_score) sorted descending.
    /// Fragility = (components_after_removal - components_before) / node_degree.
    fn bridge_nodes(&self, top_k: usize) -> Vec<(NodeId, f64)>;

    /// Count connected components in the graph.
    fn connected_components(&self) -> usize;
}
