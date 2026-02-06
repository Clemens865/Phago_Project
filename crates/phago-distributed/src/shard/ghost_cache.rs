//! Ghost node cache for cross-shard references.
//!
//! This module provides a cache for ghost nodes - lightweight references
//! to nodes that live on remote shards. The cache uses LRU eviction to
//! bound memory usage while keeping frequently accessed ghost nodes warm.

use crate::types::{GhostNode, ShardId};
use phago_core::types::NodeId;
use std::collections::HashMap;

/// Cache of ghost nodes from other shards.
///
/// Ghost nodes are references to nodes that live on remote shards.
/// This cache maintains a bounded set of ghost nodes using LRU eviction
/// to keep frequently accessed nodes available while limiting memory usage.
///
/// # Example
///
/// ```ignore
/// use phago_distributed::shard::GhostNodeCache;
///
/// let mut cache = GhostNodeCache::new(100);
/// cache.insert(ghost_node);
///
/// if let Some(ghost) = cache.get(&node_id) {
///     println!("Found ghost node: {}", ghost.label);
/// }
/// ```
pub struct GhostNodeCache {
    /// Ghost nodes indexed by ID.
    cache: HashMap<NodeId, GhostNode>,
    /// Maximum cache size.
    max_size: usize,
    /// Access order for LRU eviction (most recently used at the end).
    access_order: Vec<NodeId>,
}

impl GhostNodeCache {
    /// Create a new ghost node cache with the specified maximum size.
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum number of ghost nodes to cache
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(max_size),
            max_size,
            access_order: Vec::with_capacity(max_size),
        }
    }

    /// Get a ghost node from cache.
    ///
    /// Updates the access order for LRU tracking.
    ///
    /// # Arguments
    ///
    /// * `id` - The node ID to look up
    ///
    /// # Returns
    ///
    /// A reference to the ghost node if found.
    pub fn get(&mut self, id: &NodeId) -> Option<&GhostNode> {
        if self.cache.contains_key(id) {
            // Update access order for LRU
            self.access_order.retain(|x| x != id);
            self.access_order.push(*id);
            self.cache.get(id)
        } else {
            None
        }
    }

    /// Get a ghost node without updating LRU order (for read-only access).
    pub fn peek(&self, id: &NodeId) -> Option<&GhostNode> {
        self.cache.get(id)
    }

    /// Insert a ghost node into the cache.
    ///
    /// If the cache is at capacity, the least recently used node
    /// will be evicted to make room.
    ///
    /// # Arguments
    ///
    /// * `ghost` - The ghost node to insert
    pub fn insert(&mut self, ghost: GhostNode) {
        let id = ghost.node_id;

        // If already in cache, just update it
        if self.cache.contains_key(&id) {
            self.cache.insert(id, ghost);
            // Update access order
            self.access_order.retain(|x| *x != id);
            self.access_order.push(id);
            return;
        }

        // Evict if at capacity
        while self.cache.len() >= self.max_size && !self.access_order.is_empty() {
            let oldest = self.access_order.remove(0);
            self.cache.remove(&oldest);
        }

        self.cache.insert(id, ghost);
        self.access_order.push(id);
    }

    /// Update a ghost node with full data fetched from the remote shard.
    ///
    /// # Arguments
    ///
    /// * `id` - The node ID to update
    /// * `data` - The full node data
    pub fn update_full_data(&mut self, id: &NodeId, data: phago_core::types::NodeData) {
        if let Some(ghost) = self.cache.get_mut(id) {
            ghost.full_data = Some(data);
        }
    }

    /// Check if a node is cached.
    ///
    /// # Arguments
    ///
    /// * `id` - The node ID to check
    pub fn contains(&self, id: &NodeId) -> bool {
        self.cache.contains_key(id)
    }

    /// Get all ghost nodes from a specific shard.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - The shard to filter by
    ///
    /// # Returns
    ///
    /// A vector of references to ghost nodes from the specified shard.
    pub fn nodes_from_shard(&self, shard_id: ShardId) -> Vec<&GhostNode> {
        self.cache
            .values()
            .filter(|g| g.shard_id == shard_id)
            .collect()
    }

    /// Get all ghost nodes in the cache.
    pub fn all_nodes(&self) -> Vec<&GhostNode> {
        self.cache.values().collect()
    }

    /// Remove a ghost node from the cache.
    ///
    /// # Arguments
    ///
    /// * `id` - The node ID to remove
    ///
    /// # Returns
    ///
    /// The removed ghost node if it was present.
    pub fn remove(&mut self, id: &NodeId) -> Option<GhostNode> {
        self.access_order.retain(|x| x != id);
        self.cache.remove(id)
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }

    /// Number of cached nodes.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get the maximum cache size.
    pub fn capacity(&self) -> usize {
        self.max_size
    }

    /// Invalidate all ghost nodes from a specific shard.
    ///
    /// This is useful when a shard becomes unavailable or
    /// undergoes significant changes.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - The shard whose nodes should be invalidated
    ///
    /// # Returns
    ///
    /// The number of nodes that were invalidated.
    pub fn invalidate_shard(&mut self, shard_id: ShardId) -> usize {
        let to_remove: Vec<NodeId> = self
            .cache
            .iter()
            .filter(|(_, g)| g.shard_id == shard_id)
            .map(|(id, _)| *id)
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            self.cache.remove(&id);
            self.access_order.retain(|x| *x != id);
        }

        count
    }

    /// Get statistics about the cache.
    pub fn stats(&self) -> GhostCacheStats {
        let mut nodes_by_shard: HashMap<ShardId, usize> = HashMap::new();
        let mut with_full_data = 0;

        for ghost in self.cache.values() {
            *nodes_by_shard.entry(ghost.shard_id).or_insert(0) += 1;
            if ghost.full_data.is_some() {
                with_full_data += 1;
            }
        }

        GhostCacheStats {
            total_nodes: self.cache.len(),
            max_capacity: self.max_size,
            nodes_by_shard,
            nodes_with_full_data: with_full_data,
        }
    }
}

/// Statistics about the ghost node cache.
#[derive(Debug, Clone)]
pub struct GhostCacheStats {
    /// Total number of cached ghost nodes.
    pub total_nodes: usize,
    /// Maximum capacity of the cache.
    pub max_capacity: usize,
    /// Number of nodes cached from each shard.
    pub nodes_by_shard: HashMap<ShardId, usize>,
    /// Number of nodes that have full data fetched.
    pub nodes_with_full_data: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ghost(id: u64, shard: u32) -> GhostNode {
        GhostNode::new(
            NodeId::from_seed(id),
            ShardId::new(shard),
            format!("node_{}", id),
        )
    }

    #[test]
    fn test_insert_and_get() {
        let mut cache = GhostNodeCache::new(10);
        let ghost = make_ghost(1, 0);
        let id = ghost.node_id;

        cache.insert(ghost);

        assert!(cache.contains(&id));
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get(&id).unwrap();
        assert_eq!(retrieved.label, "node_1");
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = GhostNodeCache::new(3);

        // Insert 3 nodes
        cache.insert(make_ghost(1, 0));
        cache.insert(make_ghost(2, 0));
        cache.insert(make_ghost(3, 0));

        // Access node 1 to make it more recently used
        let _ = cache.get(&NodeId::from_seed(1));

        // Insert a 4th node - should evict node 2 (least recently used)
        cache.insert(make_ghost(4, 0));

        assert!(cache.contains(&NodeId::from_seed(1)));
        assert!(!cache.contains(&NodeId::from_seed(2))); // Evicted
        assert!(cache.contains(&NodeId::from_seed(3)));
        assert!(cache.contains(&NodeId::from_seed(4)));
    }

    #[test]
    fn test_nodes_from_shard() {
        let mut cache = GhostNodeCache::new(10);

        cache.insert(make_ghost(1, 0));
        cache.insert(make_ghost(2, 1));
        cache.insert(make_ghost(3, 0));
        cache.insert(make_ghost(4, 2));

        let shard0_nodes = cache.nodes_from_shard(ShardId::new(0));
        assert_eq!(shard0_nodes.len(), 2);

        let shard1_nodes = cache.nodes_from_shard(ShardId::new(1));
        assert_eq!(shard1_nodes.len(), 1);
    }

    #[test]
    fn test_invalidate_shard() {
        let mut cache = GhostNodeCache::new(10);

        cache.insert(make_ghost(1, 0));
        cache.insert(make_ghost(2, 1));
        cache.insert(make_ghost(3, 0));

        let count = cache.invalidate_shard(ShardId::new(0));
        assert_eq!(count, 2);
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(&NodeId::from_seed(2)));
    }

    #[test]
    fn test_clear() {
        let mut cache = GhostNodeCache::new(10);

        cache.insert(make_ghost(1, 0));
        cache.insert(make_ghost(2, 0));

        cache.clear();

        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_stats() {
        let mut cache = GhostNodeCache::new(10);

        cache.insert(make_ghost(1, 0));
        cache.insert(make_ghost(2, 1));
        cache.insert(make_ghost(3, 0));

        let stats = cache.stats();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.max_capacity, 10);
        assert_eq!(*stats.nodes_by_shard.get(&ShardId::new(0)).unwrap(), 2);
        assert_eq!(*stats.nodes_by_shard.get(&ShardId::new(1)).unwrap(), 1);
    }
}
