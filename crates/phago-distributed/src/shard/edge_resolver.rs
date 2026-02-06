//! Cross-shard edge resolution and management.
//!
//! This module provides the `CrossShardEdgeManager` for handling edges that
//! span multiple shards. When a local node has an edge to a node on another
//! shard, this manager:
//!
//! 1. Tracks the pending cross-shard edges
//! 2. Coordinates resolution of ghost nodes
//! 3. Handles edge decay synchronization across shards
//!
//! # Architecture
//!
//! The manager maintains separate maps for outgoing and incoming edges:
//! - Outgoing edges: local node -> edges to remote shards
//! - Incoming edges: local node (target) -> edges from remote shards
//!
//! This separation enables efficient lookups for both traversal directions
//! and supports proper garbage collection when shards go offline.
//!
//! # Example
//!
//! ```ignore
//! use phago_distributed::shard::CrossShardEdgeManager;
//! use phago_distributed::types::CrossShardEdge;
//!
//! let mut manager = CrossShardEdgeManager::new();
//!
//! // Register an outgoing edge to another shard
//! let edge = CrossShardEdge {
//!     from_node: local_node_id,
//!     to_node: remote_node_id,
//!     to_shard: ShardId::new(1),
//!     weight: 0.5,
//! };
//! manager.add_outgoing_edge(edge);
//!
//! // Later, resolve all pending edges
//! for pending in manager.pending_edges() {
//!     // Fetch ghost node data from remote shard
//! }
//! manager.clear_pending();
//! ```

use crate::types::{CrossShardEdge, ShardId};
use phago_core::types::NodeId;
use std::collections::HashMap;

/// Manages edges that cross shard boundaries.
///
/// When a local node has an edge to a node on another shard, this manager:
/// 1. Tracks the pending cross-shard edges
/// 2. Coordinates resolution of ghost nodes
/// 3. Handles edge decay synchronization across shards
///
/// # Thread Safety
///
/// This type is not thread-safe. Wrap in a mutex if concurrent access is needed.
pub struct CrossShardEdgeManager {
    /// Map of local node ID -> list of cross-shard edges (outgoing).
    outgoing_edges: HashMap<NodeId, Vec<CrossShardEdge>>,
    /// Incoming edges from other shards (where we own the target).
    incoming_edges: HashMap<NodeId, Vec<CrossShardEdge>>,
    /// Edges pending resolution (need ghost node data fetched).
    pending_resolution: Vec<CrossShardEdge>,
}

impl CrossShardEdgeManager {
    /// Create a new cross-shard edge manager.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let manager = CrossShardEdgeManager::new();
    /// assert_eq!(manager.edge_count(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            outgoing_edges: HashMap::new(),
            incoming_edges: HashMap::new(),
            pending_resolution: Vec::new(),
        }
    }

    /// Create a new manager with pre-allocated capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Initial capacity for edge maps
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            outgoing_edges: HashMap::with_capacity(capacity),
            incoming_edges: HashMap::with_capacity(capacity),
            pending_resolution: Vec::with_capacity(capacity),
        }
    }

    /// Register an outgoing cross-shard edge.
    ///
    /// The edge is added to both the outgoing edges map and the pending
    /// resolution queue. Call `clear_pending()` after resolving ghost nodes.
    ///
    /// # Arguments
    ///
    /// * `edge` - The cross-shard edge to register
    ///
    /// # Example
    ///
    /// ```ignore
    /// let edge = CrossShardEdge {
    ///     from_node: local_id,
    ///     to_node: remote_id,
    ///     to_shard: ShardId::new(1),
    ///     weight: 0.5,
    /// };
    /// manager.add_outgoing_edge(edge);
    /// ```
    pub fn add_outgoing_edge(&mut self, edge: CrossShardEdge) {
        self.outgoing_edges
            .entry(edge.from_node)
            .or_insert_with(Vec::new)
            .push(edge.clone());
        self.pending_resolution.push(edge);
    }

    /// Register multiple outgoing edges at once.
    ///
    /// More efficient than calling `add_outgoing_edge` in a loop.
    ///
    /// # Arguments
    ///
    /// * `edges` - Iterator of edges to register
    pub fn add_outgoing_edges(&mut self, edges: impl IntoIterator<Item = CrossShardEdge>) {
        for edge in edges {
            self.add_outgoing_edge(edge);
        }
    }

    /// Register an incoming cross-shard edge.
    ///
    /// Incoming edges are from nodes on other shards that point to a node
    /// we own locally.
    ///
    /// # Arguments
    ///
    /// * `edge` - The cross-shard edge to register
    pub fn add_incoming_edge(&mut self, edge: CrossShardEdge) {
        self.incoming_edges
            .entry(edge.to_node)
            .or_insert_with(Vec::new)
            .push(edge);
    }

    /// Get all pending edges that need ghost node resolution.
    ///
    /// These are edges that have been registered but whose target nodes
    /// have not yet been fetched from their remote shards.
    ///
    /// # Returns
    ///
    /// A slice of pending cross-shard edges.
    pub fn pending_edges(&self) -> &[CrossShardEdge] {
        &self.pending_resolution
    }

    /// Get the number of pending edges.
    pub fn pending_count(&self) -> usize {
        self.pending_resolution.len()
    }

    /// Check if there are pending edges.
    pub fn has_pending(&self) -> bool {
        !self.pending_resolution.is_empty()
    }

    /// Clear pending edges after resolution.
    ///
    /// Call this after successfully fetching ghost node data for all
    /// pending edges.
    pub fn clear_pending(&mut self) {
        self.pending_resolution.clear();
    }

    /// Take ownership of pending edges and clear the queue.
    ///
    /// This is useful when you need to process the edges and don't
    /// want to clone them.
    ///
    /// # Returns
    ///
    /// The vector of pending edges.
    pub fn take_pending(&mut self) -> Vec<CrossShardEdge> {
        std::mem::take(&mut self.pending_resolution)
    }

    /// Get outgoing edges for a node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The local node ID to look up
    ///
    /// # Returns
    ///
    /// The list of cross-shard edges from this node, if any.
    pub fn get_outgoing(&self, node_id: &NodeId) -> Option<&Vec<CrossShardEdge>> {
        self.outgoing_edges.get(node_id)
    }

    /// Get incoming edges for a node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The local node ID to look up
    ///
    /// # Returns
    ///
    /// The list of cross-shard edges to this node, if any.
    pub fn get_incoming(&self, node_id: &NodeId) -> Option<&Vec<CrossShardEdge>> {
        self.incoming_edges.get(node_id)
    }

    /// Check if a node has outgoing cross-shard edges.
    pub fn has_outgoing(&self, node_id: &NodeId) -> bool {
        self.outgoing_edges
            .get(node_id)
            .map_or(false, |v| !v.is_empty())
    }

    /// Check if a node has incoming cross-shard edges.
    pub fn has_incoming(&self, node_id: &NodeId) -> bool {
        self.incoming_edges
            .get(node_id)
            .map_or(false, |v| !v.is_empty())
    }

    /// Remove edges to/from a specific shard.
    ///
    /// This is useful when a shard goes offline and all edges to/from
    /// it should be invalidated.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - The shard whose edges should be removed
    ///
    /// # Returns
    ///
    /// The number of edges that were removed.
    pub fn remove_shard_edges(&mut self, shard_id: ShardId) -> usize {
        let mut removed = 0;

        for edges in self.outgoing_edges.values_mut() {
            let before = edges.len();
            edges.retain(|e| e.to_shard != shard_id);
            removed += before - edges.len();
        }

        for edges in self.incoming_edges.values_mut() {
            let before = edges.len();
            edges.retain(|e| e.to_shard != shard_id);
            removed += before - edges.len();
        }

        self.pending_resolution.retain(|e| e.to_shard != shard_id);

        removed
    }

    /// Remove all edges for a specific local node.
    ///
    /// Call this when a local node is deleted.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The node whose edges should be removed
    ///
    /// # Returns
    ///
    /// A tuple of (outgoing_removed, incoming_removed).
    pub fn remove_node_edges(&mut self, node_id: &NodeId) -> (usize, usize) {
        let outgoing = self.outgoing_edges.remove(node_id).map_or(0, |v| v.len());
        let incoming = self.incoming_edges.remove(node_id).map_or(0, |v| v.len());

        self.pending_resolution.retain(|e| e.from_node != *node_id);

        (outgoing, incoming)
    }

    /// Decay cross-shard edge weights.
    ///
    /// Applies exponential decay to all edge weights and removes edges
    /// that fall below the threshold.
    ///
    /// # Arguments
    ///
    /// * `rate` - Decay rate (0.0 to 1.0), e.g., 0.1 means 10% decay
    /// * `threshold` - Minimum weight threshold; edges below this are pruned
    ///
    /// # Returns
    ///
    /// Vector of edges that were pruned due to low weight.
    pub fn decay_edges(&mut self, rate: f64, threshold: f64) -> Vec<CrossShardEdge> {
        let mut pruned = Vec::new();

        for edges in self.outgoing_edges.values_mut() {
            let mut i = 0;
            while i < edges.len() {
                let new_weight = edges[i].weight * (1.0 - rate);
                if new_weight < threshold {
                    pruned.push(edges.swap_remove(i));
                } else {
                    edges[i].weight = new_weight;
                    i += 1;
                }
            }
        }

        // Also decay incoming edges
        for edges in self.incoming_edges.values_mut() {
            edges.retain_mut(|e| {
                e.weight *= 1.0 - rate;
                e.weight >= threshold
            });
        }

        pruned
    }

    /// Strengthen an edge weight.
    ///
    /// # Arguments
    ///
    /// * `from_node` - Source node ID
    /// * `to_node` - Target node ID
    /// * `amount` - Amount to add to the weight
    ///
    /// # Returns
    ///
    /// The new weight if the edge was found, None otherwise.
    pub fn strengthen_edge(
        &mut self,
        from_node: &NodeId,
        to_node: &NodeId,
        amount: f64,
    ) -> Option<f64> {
        if let Some(edges) = self.outgoing_edges.get_mut(from_node) {
            for edge in edges.iter_mut() {
                if edge.to_node == *to_node {
                    edge.weight = (edge.weight + amount).min(1.0);
                    return Some(edge.weight);
                }
            }
        }
        None
    }

    /// Get all unique remote shards that have edges.
    ///
    /// # Returns
    ///
    /// A sorted, deduplicated vector of shard IDs.
    pub fn connected_shards(&self) -> Vec<ShardId> {
        let mut shards: Vec<ShardId> = self
            .outgoing_edges
            .values()
            .flat_map(|edges| edges.iter().map(|e| e.to_shard))
            .collect();
        shards.sort();
        shards.dedup();
        shards
    }

    /// Get edges grouped by target shard.
    ///
    /// Useful for batching requests to remote shards.
    ///
    /// # Returns
    ///
    /// A map of shard ID to edges targeting that shard.
    pub fn edges_by_shard(&self) -> HashMap<ShardId, Vec<&CrossShardEdge>> {
        let mut by_shard: HashMap<ShardId, Vec<&CrossShardEdge>> = HashMap::new();
        for edges in self.outgoing_edges.values() {
            for edge in edges {
                by_shard.entry(edge.to_shard).or_default().push(edge);
            }
        }
        by_shard
    }

    /// Get pending edges grouped by target shard.
    ///
    /// Useful for batching ghost node resolution requests.
    pub fn pending_by_shard(&self) -> HashMap<ShardId, Vec<&CrossShardEdge>> {
        let mut by_shard: HashMap<ShardId, Vec<&CrossShardEdge>> = HashMap::new();
        for edge in &self.pending_resolution {
            by_shard.entry(edge.to_shard).or_default().push(edge);
        }
        by_shard
    }

    /// Total number of cross-shard edges (outgoing + incoming).
    pub fn edge_count(&self) -> usize {
        self.outgoing_edges.values().map(|v| v.len()).sum::<usize>()
            + self.incoming_edges.values().map(|v| v.len()).sum::<usize>()
    }

    /// Number of outgoing cross-shard edges.
    pub fn outgoing_count(&self) -> usize {
        self.outgoing_edges.values().map(|v| v.len()).sum()
    }

    /// Number of incoming cross-shard edges.
    pub fn incoming_count(&self) -> usize {
        self.incoming_edges.values().map(|v| v.len()).sum()
    }

    /// Number of unique local nodes with outgoing edges.
    pub fn nodes_with_outgoing(&self) -> usize {
        self.outgoing_edges
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .count()
    }

    /// Number of unique local nodes with incoming edges.
    pub fn nodes_with_incoming(&self) -> usize {
        self.incoming_edges
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .count()
    }

    /// Clear all edges.
    pub fn clear(&mut self) {
        self.outgoing_edges.clear();
        self.incoming_edges.clear();
        self.pending_resolution.clear();
    }

    /// Check if the manager has any edges.
    pub fn is_empty(&self) -> bool {
        self.outgoing_edges.values().all(|v| v.is_empty())
            && self.incoming_edges.values().all(|v| v.is_empty())
    }

    /// Get statistics about cross-shard edges.
    pub fn stats(&self) -> CrossShardEdgeStats {
        let mut edges_by_shard: HashMap<ShardId, usize> = HashMap::new();
        let mut total_weight = 0.0;
        let mut edge_count = 0;

        for edges in self.outgoing_edges.values() {
            for edge in edges {
                *edges_by_shard.entry(edge.to_shard).or_insert(0) += 1;
                total_weight += edge.weight;
                edge_count += 1;
            }
        }

        CrossShardEdgeStats {
            outgoing_edges: self.outgoing_count(),
            incoming_edges: self.incoming_count(),
            pending_resolution: self.pending_resolution.len(),
            connected_shards: self.connected_shards().len(),
            edges_by_shard,
            average_weight: if edge_count > 0 {
                total_weight / edge_count as f64
            } else {
                0.0
            },
        }
    }
}

impl Default for CrossShardEdgeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about cross-shard edges.
#[derive(Debug, Clone)]
pub struct CrossShardEdgeStats {
    /// Number of outgoing cross-shard edges.
    pub outgoing_edges: usize,
    /// Number of incoming cross-shard edges.
    pub incoming_edges: usize,
    /// Number of edges pending ghost node resolution.
    pub pending_resolution: usize,
    /// Number of unique remote shards connected.
    pub connected_shards: usize,
    /// Edges grouped by target shard.
    pub edges_by_shard: HashMap<ShardId, usize>,
    /// Average edge weight.
    pub average_weight: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_edge(from: u64, to: u64, shard: u32) -> CrossShardEdge {
        CrossShardEdge {
            from_node: NodeId::from_seed(from),
            to_node: NodeId::from_seed(to),
            to_shard: ShardId::new(shard),
            weight: 0.5,
        }
    }

    fn make_edge_with_weight(from: u64, to: u64, shard: u32, weight: f64) -> CrossShardEdge {
        CrossShardEdge {
            from_node: NodeId::from_seed(from),
            to_node: NodeId::from_seed(to),
            to_shard: ShardId::new(shard),
            weight,
        }
    }

    #[test]
    fn test_new() {
        let manager = CrossShardEdgeManager::new();
        assert_eq!(manager.edge_count(), 0);
        assert!(manager.is_empty());
        assert!(!manager.has_pending());
    }

    #[test]
    fn test_with_capacity() {
        let manager = CrossShardEdgeManager::with_capacity(100);
        assert_eq!(manager.edge_count(), 0);
    }

    #[test]
    fn test_add_and_get_outgoing_edges() {
        let mut manager = CrossShardEdgeManager::new();
        let edge = make_edge(1, 2, 1);

        manager.add_outgoing_edge(edge.clone());

        assert_eq!(manager.edge_count(), 1);
        assert_eq!(manager.outgoing_count(), 1);
        assert!(manager.has_outgoing(&NodeId::from_seed(1)));
        assert!(!manager.has_outgoing(&NodeId::from_seed(2)));

        let outgoing = manager.get_outgoing(&NodeId::from_seed(1)).unwrap();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].to_shard, ShardId::new(1));
    }

    #[test]
    fn test_add_incoming_edge() {
        let mut manager = CrossShardEdgeManager::new();
        let edge = make_edge(1, 2, 1);

        manager.add_incoming_edge(edge);

        assert_eq!(manager.incoming_count(), 1);
        assert!(manager.has_incoming(&NodeId::from_seed(2)));
    }

    #[test]
    fn test_pending_edges() {
        let mut manager = CrossShardEdgeManager::new();

        assert!(!manager.has_pending());
        assert_eq!(manager.pending_count(), 0);

        manager.add_outgoing_edge(make_edge(1, 2, 1));

        assert!(manager.has_pending());
        assert_eq!(manager.pending_count(), 1);
        assert_eq!(manager.pending_edges().len(), 1);

        manager.clear_pending();

        assert!(!manager.has_pending());
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_take_pending() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge(1, 2, 1));
        manager.add_outgoing_edge(make_edge(3, 4, 2));

        let pending = manager.take_pending();

        assert_eq!(pending.len(), 2);
        assert!(!manager.has_pending());
    }

    #[test]
    fn test_remove_shard_edges() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge(1, 2, 1));
        manager.add_outgoing_edge(make_edge(3, 4, 2));
        manager.add_outgoing_edge(make_edge(5, 6, 1));

        let removed = manager.remove_shard_edges(ShardId::new(1));

        assert_eq!(removed, 2);
        assert_eq!(manager.outgoing_count(), 1);
        assert!(manager
            .get_outgoing(&NodeId::from_seed(1))
            .unwrap()
            .is_empty());
        assert!(!manager
            .get_outgoing(&NodeId::from_seed(3))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_remove_node_edges() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge(1, 2, 1));
        manager.add_outgoing_edge(make_edge(1, 3, 2));
        manager.add_incoming_edge(make_edge(5, 1, 0));

        let (outgoing, incoming) = manager.remove_node_edges(&NodeId::from_seed(1));

        assert_eq!(outgoing, 2);
        assert_eq!(incoming, 1);
        assert!(manager.get_outgoing(&NodeId::from_seed(1)).is_none());
    }

    #[test]
    fn test_decay_edges() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge_with_weight(1, 2, 1, 0.5));
        manager.add_outgoing_edge(make_edge_with_weight(3, 4, 2, 0.1));

        // Clear pending to focus on decay test
        manager.clear_pending();

        // 50% decay with 0.1 threshold should prune the 0.1 edge
        let pruned = manager.decay_edges(0.5, 0.1);

        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0].from_node, NodeId::from_seed(3));

        // The remaining edge should have decayed from 0.5 to 0.25
        let remaining = manager.get_outgoing(&NodeId::from_seed(1)).unwrap();
        assert!((remaining[0].weight - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_strengthen_edge() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge_with_weight(1, 2, 1, 0.3));

        let new_weight = manager.strengthen_edge(&NodeId::from_seed(1), &NodeId::from_seed(2), 0.2);

        assert_eq!(new_weight, Some(0.5));

        // Test clamping to 1.0
        let clamped = manager.strengthen_edge(&NodeId::from_seed(1), &NodeId::from_seed(2), 0.8);

        assert_eq!(clamped, Some(1.0));
    }

    #[test]
    fn test_connected_shards() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge(1, 2, 1));
        manager.add_outgoing_edge(make_edge(3, 4, 2));
        manager.add_outgoing_edge(make_edge(5, 6, 1));

        let shards = manager.connected_shards();

        assert_eq!(shards.len(), 2);
        assert!(shards.contains(&ShardId::new(1)));
        assert!(shards.contains(&ShardId::new(2)));
    }

    #[test]
    fn test_edges_by_shard() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge(1, 2, 1));
        manager.add_outgoing_edge(make_edge(3, 4, 2));
        manager.add_outgoing_edge(make_edge(5, 6, 1));

        let by_shard = manager.edges_by_shard();

        assert_eq!(by_shard.get(&ShardId::new(1)).unwrap().len(), 2);
        assert_eq!(by_shard.get(&ShardId::new(2)).unwrap().len(), 1);
    }

    #[test]
    fn test_pending_by_shard() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge(1, 2, 1));
        manager.add_outgoing_edge(make_edge(3, 4, 2));
        manager.add_outgoing_edge(make_edge(5, 6, 1));

        let by_shard = manager.pending_by_shard();

        assert_eq!(by_shard.get(&ShardId::new(1)).unwrap().len(), 2);
        assert_eq!(by_shard.get(&ShardId::new(2)).unwrap().len(), 1);
    }

    #[test]
    fn test_edge_counts() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge(1, 2, 1));
        manager.add_outgoing_edge(make_edge(1, 3, 2));
        manager.add_incoming_edge(make_edge(4, 5, 0));

        assert_eq!(manager.outgoing_count(), 2);
        assert_eq!(manager.incoming_count(), 1);
        assert_eq!(manager.edge_count(), 3);
        assert_eq!(manager.nodes_with_outgoing(), 1);
        assert_eq!(manager.nodes_with_incoming(), 1);
    }

    #[test]
    fn test_clear() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge(1, 2, 1));
        manager.add_incoming_edge(make_edge(3, 4, 0));

        manager.clear();

        assert!(manager.is_empty());
        assert_eq!(manager.edge_count(), 0);
        assert!(!manager.has_pending());
    }

    #[test]
    fn test_stats() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge_with_weight(1, 2, 1, 0.4));
        manager.add_outgoing_edge(make_edge_with_weight(3, 4, 2, 0.6));
        manager.add_incoming_edge(make_edge(5, 6, 0));

        let stats = manager.stats();

        assert_eq!(stats.outgoing_edges, 2);
        assert_eq!(stats.incoming_edges, 1);
        assert_eq!(stats.pending_resolution, 2);
        assert_eq!(stats.connected_shards, 2);
        assert!((stats.average_weight - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_add_outgoing_edges_batch() {
        let mut manager = CrossShardEdgeManager::new();
        let edges = vec![make_edge(1, 2, 1), make_edge(3, 4, 2), make_edge(5, 6, 3)];

        manager.add_outgoing_edges(edges);

        assert_eq!(manager.outgoing_count(), 3);
        assert_eq!(manager.pending_count(), 3);
    }

    #[test]
    fn test_default() {
        let manager = CrossShardEdgeManager::default();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_multiple_edges_same_source() {
        let mut manager = CrossShardEdgeManager::new();
        manager.add_outgoing_edge(make_edge(1, 2, 1));
        manager.add_outgoing_edge(make_edge(1, 3, 2));
        manager.add_outgoing_edge(make_edge(1, 4, 3));

        let outgoing = manager.get_outgoing(&NodeId::from_seed(1)).unwrap();
        assert_eq!(outgoing.len(), 3);
        assert_eq!(manager.nodes_with_outgoing(), 1);
    }
}
