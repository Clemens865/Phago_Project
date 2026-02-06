//! Consistent hashing for document-to-shard routing.
//!
//! This module implements a consistent hash ring that distributes documents
//! across shards with minimal redistribution when the cluster topology changes.
//! Virtual nodes are used to ensure even distribution of data across shards.

use crate::types::ShardId;
use phago_core::types::DocumentId;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

/// Number of virtual nodes per physical shard for better distribution.
const VIRTUAL_NODES_PER_SHARD: u32 = 150;

/// A consistent hash ring for routing documents to shards.
///
/// The ring uses virtual nodes to achieve better load distribution across
/// shards. Each physical shard is represented by multiple virtual nodes
/// on the ring, which helps ensure that documents are distributed evenly.
///
/// # Thread Safety
///
/// The ring itself is `Clone` and can be wrapped in `Arc<RwLock<_>>` for
/// thread-safe access with dynamic updates, or `Arc<_>` for read-only
/// concurrent access.
///
/// # Example
///
/// ```
/// use phago_distributed::hashing::ConsistentHashRing;
/// use phago_core::types::DocumentId;
///
/// let ring = ConsistentHashRing::new(3);
/// let doc_id = DocumentId::from_seed(42);
/// let shard = ring.get_shard(&doc_id);
/// println!("Document maps to shard: {}", shard);
/// ```
#[derive(Debug, Clone)]
pub struct ConsistentHashRing {
    /// Ring mapping hash positions to shard IDs.
    ring: BTreeMap<u64, ShardId>,
    /// Number of shards in the ring.
    shard_count: u32,
    /// Virtual nodes per shard.
    virtual_nodes: u32,
}

impl ConsistentHashRing {
    /// Create a new hash ring with the specified number of shards.
    ///
    /// Each shard will be represented by `VIRTUAL_NODES_PER_SHARD` virtual
    /// nodes on the ring for better distribution.
    ///
    /// # Arguments
    ///
    /// * `num_shards` - The number of physical shards to distribute across
    ///
    /// # Panics
    ///
    /// Panics if `num_shards` is 0.
    pub fn new(num_shards: u32) -> Self {
        assert!(num_shards > 0, "Number of shards must be greater than 0");

        let mut ring = BTreeMap::new();

        for shard_id in 0..num_shards {
            for vnode in 0..VIRTUAL_NODES_PER_SHARD {
                let hash = Self::hash_shard_vnode(shard_id, vnode);
                ring.insert(hash, ShardId::new(shard_id));
            }
        }

        Self {
            ring,
            shard_count: num_shards,
            virtual_nodes: VIRTUAL_NODES_PER_SHARD,
        }
    }

    /// Create a new hash ring with custom virtual nodes per shard.
    ///
    /// More virtual nodes generally result in better distribution but
    /// increase memory usage and lookup time slightly.
    ///
    /// # Arguments
    ///
    /// * `num_shards` - The number of physical shards
    /// * `virtual_nodes` - Number of virtual nodes per shard
    pub fn with_virtual_nodes(num_shards: u32, virtual_nodes: u32) -> Self {
        assert!(num_shards > 0, "Number of shards must be greater than 0");
        assert!(virtual_nodes > 0, "Virtual nodes must be greater than 0");

        let mut ring = BTreeMap::new();

        for shard_id in 0..num_shards {
            for vnode in 0..virtual_nodes {
                let hash = Self::hash_shard_vnode(shard_id, vnode);
                ring.insert(hash, ShardId::new(shard_id));
            }
        }

        Self {
            ring,
            shard_count: num_shards,
            virtual_nodes,
        }
    }

    /// Get the shard ID for a document.
    ///
    /// This operation is O(log n) where n is the total number of virtual nodes.
    ///
    /// # Arguments
    ///
    /// * `doc_id` - The document ID to route
    ///
    /// # Returns
    ///
    /// The shard ID that should store this document.
    pub fn get_shard(&self, doc_id: &DocumentId) -> ShardId {
        let hash = Self::hash_document(doc_id);

        // Find the first shard with a hash >= document hash (clockwise)
        if let Some((&_pos, &shard_id)) = self.ring.range(hash..).next() {
            shard_id
        } else {
            // Wrap around to the first shard
            *self.ring.values().next().unwrap_or(&ShardId::new(0))
        }
    }

    /// Get the shard ID for an arbitrary key.
    ///
    /// This is useful for routing non-document data to shards.
    ///
    /// # Arguments
    ///
    /// * `key` - Any hashable key
    pub fn get_shard_for_key<K: Hash>(&self, key: &K) -> ShardId {
        let hash = Self::hash_key(key);

        if let Some((&_pos, &shard_id)) = self.ring.range(hash..).next() {
            shard_id
        } else {
            *self.ring.values().next().unwrap_or(&ShardId::new(0))
        }
    }

    /// Add a new shard to the ring.
    ///
    /// This will redistribute approximately `1 / (n+1)` of the keys from
    /// existing shards to the new shard, where n is the current number of shards.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - The shard ID to add
    pub fn add_shard(&mut self, shard_id: ShardId) {
        for vnode in 0..self.virtual_nodes {
            let hash = Self::hash_shard_vnode(shard_id.0, vnode);
            self.ring.insert(hash, shard_id);
        }
        self.shard_count += 1;
    }

    /// Remove a shard from the ring.
    ///
    /// Documents previously assigned to this shard will be redistributed
    /// to the next shard in the ring (clockwise).
    ///
    /// # Arguments
    ///
    /// * `shard_id` - The shard ID to remove
    pub fn remove_shard(&mut self, shard_id: ShardId) {
        self.ring.retain(|_, &mut sid| sid != shard_id);
        self.shard_count = self.shard_count.saturating_sub(1);
    }

    /// Get the number of shards.
    pub fn shard_count(&self) -> u32 {
        self.shard_count
    }

    /// Get all shard IDs in the ring.
    ///
    /// Returns a sorted, deduplicated list of all shard IDs.
    pub fn all_shards(&self) -> Vec<ShardId> {
        let mut shards: Vec<ShardId> = self.ring.values().copied().collect();
        shards.sort_by_key(|s| s.0);
        shards.dedup();
        shards
    }

    /// Get the number of virtual nodes per shard.
    pub fn virtual_nodes_per_shard(&self) -> u32 {
        self.virtual_nodes
    }

    /// Get the total number of virtual nodes in the ring.
    pub fn total_virtual_nodes(&self) -> usize {
        self.ring.len()
    }

    /// Get replica shards for a document.
    ///
    /// Returns the primary shard plus `replica_count` additional shards
    /// that should store replicas of the document.
    ///
    /// # Arguments
    ///
    /// * `doc_id` - The document ID
    /// * `replica_count` - Number of additional replicas (excluding primary)
    ///
    /// # Returns
    ///
    /// A vector of shard IDs, with the primary shard first.
    pub fn get_replica_shards(&self, doc_id: &DocumentId, replica_count: usize) -> Vec<ShardId> {
        let hash = Self::hash_document(doc_id);
        let mut shards = Vec::with_capacity(replica_count + 1);
        let mut seen_shards = std::collections::HashSet::new();

        // Start from the document's hash position and walk clockwise
        for (&_pos, &shard_id) in self.ring.range(hash..).chain(self.ring.iter()) {
            if seen_shards.insert(shard_id) {
                shards.push(shard_id);
                if shards.len() > replica_count {
                    break;
                }
            }
        }

        shards
    }

    /// Hash a document ID to a ring position.
    fn hash_document(doc_id: &DocumentId) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        doc_id.0.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash an arbitrary key to a ring position.
    fn hash_key<K: Hash>(key: &K) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash a shard + virtual node combination.
    fn hash_shard_vnode(shard_id: u32, vnode: u32) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        shard_id.hash(&mut hasher);
        vnode.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for ConsistentHashRing {
    fn default() -> Self {
        Self::new(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_ring() {
        let ring = ConsistentHashRing::new(3);
        assert_eq!(ring.shard_count(), 3);
        assert_eq!(ring.all_shards().len(), 3);
        assert_eq!(
            ring.total_virtual_nodes(),
            3 * VIRTUAL_NODES_PER_SHARD as usize
        );
    }

    #[test]
    fn test_distribution() {
        let ring = ConsistentHashRing::new(3);

        // Create 100 documents and check distribution
        let mut counts = [0u32; 3];
        for i in 0..100 {
            let doc_id = DocumentId::from_seed(i);
            let shard = ring.get_shard(&doc_id);
            counts[shard.0 as usize] += 1;
        }

        // Each shard should get roughly 33 documents
        for count in counts {
            assert!(
                count >= 20 && count <= 50,
                "Distribution skewed: {:?}",
                counts
            );
        }
    }

    #[test]
    fn test_consistency() {
        let ring = ConsistentHashRing::new(3);
        let doc_id = DocumentId::from_seed(42);

        // Same document should always map to same shard
        let shard1 = ring.get_shard(&doc_id);
        let shard2 = ring.get_shard(&doc_id);
        assert_eq!(shard1, shard2);
    }

    #[test]
    fn test_add_shard_minimal_redistribution() {
        let mut ring = ConsistentHashRing::new(3);

        // Record initial assignments
        let initial: Vec<ShardId> = (0..100)
            .map(|i| ring.get_shard(&DocumentId::from_seed(i)))
            .collect();

        // Add a fourth shard
        ring.add_shard(ShardId::new(3));

        // Check how many documents moved
        let mut moved = 0;
        for i in 0..100 {
            let doc_id = DocumentId::from_seed(i);
            if ring.get_shard(&doc_id) != initial[i as usize] {
                moved += 1;
            }
        }

        // With consistent hashing, only ~25% should move to the new shard
        assert!(moved <= 35, "Too many documents moved: {}", moved);
    }

    #[test]
    fn test_remove_shard() {
        let mut ring = ConsistentHashRing::new(3);
        assert_eq!(ring.shard_count(), 3);

        ring.remove_shard(ShardId::new(1));
        assert_eq!(ring.shard_count(), 2);

        // Documents should still be assignable
        let doc_id = DocumentId::from_seed(42);
        let shard = ring.get_shard(&doc_id);
        assert!(shard.0 != 1, "Document assigned to removed shard");
    }

    #[test]
    fn test_replica_shards() {
        let ring = ConsistentHashRing::new(5);
        let doc_id = DocumentId::from_seed(42);

        let replicas = ring.get_replica_shards(&doc_id, 2);
        assert_eq!(replicas.len(), 3); // primary + 2 replicas

        // All replicas should be unique
        let unique: std::collections::HashSet<_> = replicas.iter().collect();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn test_custom_virtual_nodes() {
        let ring = ConsistentHashRing::with_virtual_nodes(3, 50);
        assert_eq!(ring.virtual_nodes_per_shard(), 50);
        assert_eq!(ring.total_virtual_nodes(), 150);
    }

    #[test]
    fn test_get_shard_for_key() {
        let ring = ConsistentHashRing::new(3);

        // String keys should work
        let shard1 = ring.get_shard_for_key(&"user:123");
        let shard2 = ring.get_shard_for_key(&"user:123");
        assert_eq!(shard1, shard2);

        // Different keys may go to different shards
        let shard3 = ring.get_shard_for_key(&"user:456");
        // This might or might not be equal, just verify it works
        let _ = shard3;
    }

    #[test]
    #[should_panic(expected = "Number of shards must be greater than 0")]
    fn test_zero_shards_panics() {
        let _ = ConsistentHashRing::new(0);
    }

    #[test]
    fn test_default() {
        let ring = ConsistentHashRing::default();
        assert_eq!(ring.shard_count(), 1);
    }

    #[test]
    fn test_shard_id_display() {
        let shard = ShardId::new(5);
        assert_eq!(format!("{}", shard), "shard-5");
    }
}
