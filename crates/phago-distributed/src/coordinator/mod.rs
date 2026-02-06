//! Coordinator for distributed colony orchestration.
//!
//! The coordinator is the central component of the distributed colony system,
//! responsible for:
//! - Managing the cluster topology via the shard registry
//! - Routing documents to shards using consistent hashing
//! - Synchronizing ticks across shards using barriers
//! - Aggregating global statistics like document frequencies

mod shard_registry;
mod tick_barrier;

pub use shard_registry::{RegisteredShard, ShardRegistry};
pub use tick_barrier::TickBarrier;

use crate::hashing::ConsistentHashRing;
use crate::types::*;
use phago_core::types::{DocumentId, Tick};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// The distributed coordinator.
///
/// The coordinator manages the distributed colony by:
/// - Maintaining a registry of all active shards
/// - Routing documents to shards using consistent hashing
/// - Synchronizing tick phases across all shards
/// - Aggregating global statistics for TF-IDF computation
///
/// # Thread Safety
///
/// The coordinator is designed for concurrent access. It uses interior
/// mutability with `RwLock` for the registry and hash ring, and atomics
/// for the tick counter.
///
/// # Example
///
/// ```rust,ignore
/// use phago_distributed::Coordinator;
/// use phago_distributed::types::{ShardInfo, NodeAddress};
///
/// let coordinator = Coordinator::new(3);
///
/// // Register a shard
/// let info = ShardInfo::new(NodeAddress::new("127.0.0.1", 8080));
/// let shard_id = coordinator.register_shard(info).await?;
///
/// // Route a document
/// let doc_id = DocumentId::new();
/// let target_shard = coordinator.route_document(&doc_id).await;
/// ```
pub struct Coordinator {
    /// Registry of all shards.
    shards: Arc<RwLock<ShardRegistry>>,
    /// Current global tick.
    current_tick: Arc<AtomicU64>,
    /// Tick barrier for synchronization.
    barrier: Arc<TickBarrier>,
    /// Consistent hash ring for document routing.
    hash_ring: Arc<RwLock<ConsistentHashRing>>,
    /// Configuration for the distributed system.
    config: DistributedConfig,
}

impl Coordinator {
    /// Create a new coordinator with the specified number of shards.
    ///
    /// The coordinator will initialize the hash ring with the given
    /// number of shards, but actual shards must be registered before
    /// they can receive documents.
    pub fn new(num_shards: u32) -> Self {
        Self {
            shards: Arc::new(RwLock::new(ShardRegistry::new())),
            current_tick: Arc::new(AtomicU64::new(0)),
            barrier: Arc::new(TickBarrier::new(num_shards as usize)),
            hash_ring: Arc::new(RwLock::new(ConsistentHashRing::new(num_shards))),
            config: DistributedConfig {
                num_shards,
                ..Default::default()
            },
        }
    }

    /// Create a coordinator with custom configuration.
    pub fn with_config(config: DistributedConfig) -> Self {
        let num_shards = config.num_shards;
        Self {
            shards: Arc::new(RwLock::new(ShardRegistry::new())),
            current_tick: Arc::new(AtomicU64::new(0)),
            barrier: Arc::new(TickBarrier::new(num_shards as usize)),
            hash_ring: Arc::new(RwLock::new(ConsistentHashRing::with_virtual_nodes(
                num_shards,
                config.virtual_nodes_per_shard,
            ))),
            config,
        }
    }

    /// Register a shard with the coordinator.
    ///
    /// The shard will be assigned a unique ID, added to the registry,
    /// and included in the hash ring for document routing.
    ///
    /// # Arguments
    ///
    /// * `info` - Information about the shard to register
    ///
    /// # Returns
    ///
    /// The assigned shard ID.
    pub async fn register_shard(&self, info: ShardInfo) -> DistributedResult<ShardId> {
        let mut registry = self.shards.write().await;
        let shard_id = registry.register(info);

        // Add to hash ring
        let mut ring = self.hash_ring.write().await;
        ring.add_shard(shard_id);

        // Update barrier for new shard count
        self.barrier.set_shard_count(registry.count()).await;

        Ok(shard_id)
    }

    /// Deregister a shard from the coordinator.
    ///
    /// The shard will be removed from the registry and hash ring.
    /// Documents previously assigned to this shard will be redistributed.
    pub async fn deregister_shard(&self, shard_id: ShardId) -> DistributedResult<()> {
        let mut registry = self.shards.write().await;

        if registry.remove(&shard_id).is_none() {
            return Err(DistributedError::ShardNotFound(shard_id));
        }

        // Remove from hash ring
        let mut ring = self.hash_ring.write().await;
        ring.remove_shard(shard_id);

        // Update barrier
        self.barrier.set_shard_count(registry.count()).await;

        Ok(())
    }

    /// Route a document to the appropriate shard.
    ///
    /// Uses consistent hashing to determine which shard should store
    /// the document. The same document will always route to the same
    /// shard (unless the cluster topology changes).
    pub async fn route_document(&self, doc_id: &DocumentId) -> ShardId {
        let ring = self.hash_ring.read().await;
        ring.get_shard(doc_id)
    }

    /// Get replica shards for a document.
    ///
    /// Returns the primary shard plus additional replica shards based
    /// on the configured replication factor.
    pub async fn get_replica_shards(&self, doc_id: &DocumentId) -> Vec<ShardId> {
        let ring = self.hash_ring.read().await;
        ring.get_replica_shards(doc_id, self.config.replication_factor as usize)
    }

    /// Signal that a shard has completed a phase.
    ///
    /// This is called by each shard when it finishes a phase of the tick.
    /// The coordinator tracks progress and releases the barrier when all
    /// shards have completed.
    pub async fn phase_complete(
        &self,
        shard_id: ShardId,
        phase: TickPhase,
        tick: Tick,
    ) -> DistributedResult<()> {
        self.barrier.complete(shard_id, phase, tick).await
    }

    /// Wait for all shards to complete a phase.
    ///
    /// Blocks until all registered shards have signaled completion
    /// of the specified phase.
    pub async fn wait_for_phase(&self, phase: TickPhase, tick: Tick) -> DistributedResult<()> {
        self.barrier.wait_all(phase, tick).await
    }

    /// Advance to the next tick.
    ///
    /// This should be called after all phases of the current tick
    /// are complete. Returns the new tick number.
    pub async fn advance_tick(&self) -> Tick {
        let new_tick = self.current_tick.fetch_add(1, Ordering::SeqCst) + 1;
        self.barrier.reset_for_tick(new_tick).await;
        new_tick
    }

    /// Get the current tick number.
    pub fn current_tick(&self) -> Tick {
        self.current_tick.load(Ordering::SeqCst)
    }

    /// Aggregate global document frequencies from all shards.
    ///
    /// This is used for computing global TF-IDF scores. Each shard
    /// provides its local document frequencies, and the coordinator
    /// sums them to produce global counts.
    ///
    /// # Arguments
    ///
    /// * `local_dfs` - Vector of term->count maps from each shard
    ///
    /// # Returns
    ///
    /// A map of term->global_count across all shards.
    pub fn aggregate_global_df(
        &self,
        local_dfs: Vec<HashMap<String, u64>>,
    ) -> HashMap<String, u64> {
        let mut global_df = HashMap::new();
        for local in local_dfs {
            for (term, count) in local {
                *global_df.entry(term).or_insert(0) += count;
            }
        }
        global_df
    }

    /// Get all registered shards.
    pub async fn all_shards(&self) -> Vec<ShardInfo> {
        let registry = self.shards.read().await;
        registry.all()
    }

    /// Get all online shards.
    pub async fn online_shards(&self) -> Vec<ShardInfo> {
        let registry = self.shards.read().await;
        registry.online_shards()
    }

    /// Get a specific shard's information.
    pub async fn get_shard(&self, shard_id: ShardId) -> Option<ShardInfo> {
        let registry = self.shards.read().await;
        registry.get(&shard_id).cloned()
    }

    /// Update heartbeat for a shard.
    ///
    /// Called periodically by shards to indicate they are still alive.
    pub async fn shard_heartbeat(&self, shard_id: ShardId) {
        let mut registry = self.shards.write().await;
        registry.heartbeat(&shard_id);
    }

    /// Check for dead shards and mark them offline.
    ///
    /// Returns the IDs of shards that were marked offline.
    pub async fn check_shard_health(&self) -> Vec<ShardId> {
        let mut registry = self.shards.write().await;
        registry.check_dead_shards()
    }

    /// Update shard metrics.
    pub async fn update_shard_metrics(
        &self,
        shard_id: ShardId,
        document_count: usize,
        memory_bytes: u64,
    ) {
        let mut registry = self.shards.write().await;
        registry.update_metrics(&shard_id, document_count, memory_bytes);
    }

    /// Get the total number of documents across all shards.
    pub async fn total_documents(&self) -> u64 {
        let registry = self.shards.read().await;
        registry.total_documents()
    }

    /// Get cluster statistics.
    pub async fn cluster_stats(&self) -> ClusterStats {
        let registry = self.shards.read().await;
        ClusterStats {
            total_shards: registry.count() as u32,
            online_shards: registry.online_shards().len() as u32,
            total_documents: registry.total_documents(),
            total_memory_bytes: registry.total_memory(),
            current_tick: self.current_tick(),
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &DistributedConfig {
        &self.config
    }

    /// Get the shard count from the hash ring.
    pub async fn shard_count(&self) -> u32 {
        let ring = self.hash_ring.read().await;
        ring.shard_count()
    }
}

/// Statistics about the distributed cluster.
#[derive(Debug, Clone)]
pub struct ClusterStats {
    /// Total number of shards in the cluster.
    pub total_shards: u32,
    /// Number of currently online shards.
    pub online_shards: u32,
    /// Total documents across all shards.
    pub total_documents: u64,
    /// Total memory usage across all shards.
    pub total_memory_bytes: u64,
    /// Current simulation tick.
    pub current_tick: Tick,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_shard_info() -> ShardInfo {
        ShardInfo::new(ShardId::new(0), "127.0.0.1:8080".to_string())
    }

    #[tokio::test]
    async fn test_coordinator_creation() {
        let coord = Coordinator::new(3);
        assert_eq!(coord.current_tick(), 0);
        assert_eq!(coord.shard_count().await, 3);
    }

    #[tokio::test]
    async fn test_register_shard() {
        let coord = Coordinator::new(3);
        let info = test_shard_info();

        let shard_id = coord.register_shard(info).await.unwrap();
        assert_eq!(shard_id, ShardId::new(0));

        let shards = coord.all_shards().await;
        assert_eq!(shards.len(), 1);
    }

    #[tokio::test]
    async fn test_document_routing() {
        let coord = Coordinator::new(3);

        // Register some shards
        for _ in 0..3 {
            coord.register_shard(test_shard_info()).await.unwrap();
        }

        let doc_id = DocumentId::from_seed(42);

        // Routing should be consistent
        let shard1 = coord.route_document(&doc_id).await;
        let shard2 = coord.route_document(&doc_id).await;
        assert_eq!(shard1, shard2);
    }

    #[tokio::test]
    async fn test_advance_tick() {
        let coord = Coordinator::new(1);

        assert_eq!(coord.current_tick(), 0);

        let tick1 = coord.advance_tick().await;
        assert_eq!(tick1, 1);
        assert_eq!(coord.current_tick(), 1);

        let tick2 = coord.advance_tick().await;
        assert_eq!(tick2, 2);
        assert_eq!(coord.current_tick(), 2);
    }

    #[tokio::test]
    async fn test_aggregate_global_df() {
        let coord = Coordinator::new(2);

        let local1 = HashMap::from([("hello".to_string(), 5), ("world".to_string(), 3)]);
        let local2 = HashMap::from([("hello".to_string(), 2), ("rust".to_string(), 7)]);

        let global = coord.aggregate_global_df(vec![local1, local2]);

        assert_eq!(global.get("hello"), Some(&7));
        assert_eq!(global.get("world"), Some(&3));
        assert_eq!(global.get("rust"), Some(&7));
    }

    #[tokio::test]
    async fn test_deregister_shard() {
        let coord = Coordinator::new(3);

        let id1 = coord.register_shard(test_shard_info()).await.unwrap();
        let id2 = coord.register_shard(test_shard_info()).await.unwrap();

        assert_eq!(coord.all_shards().await.len(), 2);

        coord.deregister_shard(id1).await.unwrap();
        assert_eq!(coord.all_shards().await.len(), 1);

        // Deregistering again should error
        let result = coord.deregister_shard(id1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cluster_stats() {
        let coord = Coordinator::new(3);

        coord.register_shard(test_shard_info()).await.unwrap();
        let id2 = coord.register_shard(test_shard_info()).await.unwrap();

        coord.update_shard_metrics(id2, 100, 1024).await;

        let stats = coord.cluster_stats().await;
        assert_eq!(stats.total_shards, 2);
        assert_eq!(stats.online_shards, 2);
        assert_eq!(stats.total_documents, 100);
        assert_eq!(stats.total_memory_bytes, 1024);
    }

    #[tokio::test]
    async fn test_replica_shards() {
        let config = DistributedConfig {
            num_shards: 5,
            replication_factor: 2,
            ..Default::default()
        };
        let coord = Coordinator::with_config(config);

        let doc_id = DocumentId::from_seed(42);
        let replicas = coord.get_replica_shards(&doc_id).await;

        // Should get primary + 2 replicas = 3 shards
        assert_eq!(replicas.len(), 3);

        // All should be unique
        let unique: std::collections::HashSet<_> = replicas.iter().collect();
        assert_eq!(unique.len(), 3);
    }
}
