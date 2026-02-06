//! Sharded colony implementation.
//!
//! This module provides the `ShardedColony` type which wraps a local Colony
//! and adds distributed coordination capabilities:
//!
//! - Document routing via consistent hash ring
//! - Ghost node cache for cross-shard references
//! - Tick phase coordination with the coordinator
//!
//! # Architecture
//!
//! Each shard in the cluster runs a `ShardedColony` that manages:
//! - A local `Colony` instance with its agents and substrate
//! - A `GhostNodeCache` for caching references to nodes on other shards
//! - A reference to the `ConsistentHashRing` for document routing
//!
//! # Example
//!
//! ```ignore
//! use phago_distributed::shard::ShardedColony;
//! use phago_distributed::hashing::ConsistentHashRing;
//! use phago_runtime::colony::ColonyConfig;
//!
//! let hash_ring = Arc::new(RwLock::new(ConsistentHashRing::new(3)));
//! let mut shard = ShardedColony::new(
//!     ShardId::new(0),
//!     ColonyConfig::default(),
//!     hash_ring,
//! );
//!
//! // Check if this shard owns a document
//! if shard.owns_document(&doc_id).await {
//!     shard.ingest_document("title", "content", position).await?;
//! }
//!
//! // Execute tick phases
//! let result = shard.tick_phase(TickPhase::Sense);
//! ```

mod edge_resolver;
mod ghost_cache;

pub use edge_resolver::{CrossShardEdgeManager, CrossShardEdgeStats};
pub use ghost_cache::{GhostCacheStats, GhostNodeCache};

use crate::hashing::ConsistentHashRing;
use crate::types::*;
use phago_core::substrate::Substrate;
use phago_core::topology::TopologyGraph;
use phago_core::types::{DocumentId, NodeData, NodeId, Position, Tick};
use phago_runtime::colony::{Colony, ColonyConfig, ColonyStats};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A sharded colony that participates in distributed coordination.
///
/// ShardedColony wraps a local Colony instance and adds the machinery
/// needed for distributed operation:
///
/// - Consistent hashing for document routing
/// - Ghost node cache for cross-shard edge references
/// - Cross-shard edge management for edges spanning shards
/// - Peer shard address tracking
/// - Cross-shard edge collection during tick phases
pub struct ShardedColony {
    /// This shard's ID.
    shard_id: ShardId,
    /// The local colony instance.
    local: Colony,
    /// Cache of ghost nodes from other shards.
    ghost_cache: GhostNodeCache,
    /// Manager for cross-shard edges.
    edge_manager: CrossShardEdgeManager,
    /// Hash ring for document routing.
    hash_ring: Arc<RwLock<ConsistentHashRing>>,
    /// Addresses of peer shards.
    peers: HashMap<ShardId, String>,
    /// Pending cross-shard edges to resolve.
    pending_cross_edges: Vec<CrossShardEdge>,
}

impl ShardedColony {
    /// Create a new sharded colony.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - Unique identifier for this shard
    /// * `config` - Colony configuration parameters
    /// * `hash_ring` - Shared consistent hash ring for routing
    ///
    /// # Example
    ///
    /// ```ignore
    /// let shard = ShardedColony::new(
    ///     ShardId::new(0),
    ///     ColonyConfig::default(),
    ///     Arc::new(RwLock::new(ConsistentHashRing::new(3))),
    /// );
    /// ```
    pub fn new(
        shard_id: ShardId,
        config: ColonyConfig,
        hash_ring: Arc<RwLock<ConsistentHashRing>>,
    ) -> Self {
        Self {
            shard_id,
            local: Colony::from_config(config),
            ghost_cache: GhostNodeCache::new(1000), // Cache up to 1000 ghost nodes
            edge_manager: CrossShardEdgeManager::new(),
            hash_ring,
            peers: HashMap::new(),
            pending_cross_edges: Vec::new(),
        }
    }

    /// Create a new sharded colony with a custom ghost cache size.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - Unique identifier for this shard
    /// * `config` - Colony configuration parameters
    /// * `hash_ring` - Shared consistent hash ring for routing
    /// * `ghost_cache_size` - Maximum number of ghost nodes to cache
    pub fn with_ghost_cache_size(
        shard_id: ShardId,
        config: ColonyConfig,
        hash_ring: Arc<RwLock<ConsistentHashRing>>,
        ghost_cache_size: usize,
    ) -> Self {
        Self {
            shard_id,
            local: Colony::from_config(config),
            ghost_cache: GhostNodeCache::new(ghost_cache_size),
            edge_manager: CrossShardEdgeManager::new(),
            hash_ring,
            peers: HashMap::new(),
            pending_cross_edges: Vec::new(),
        }
    }

    /// Get this shard's ID.
    pub fn shard_id(&self) -> ShardId {
        self.shard_id
    }

    /// Check if a document belongs to this shard.
    ///
    /// Uses the consistent hash ring to determine ownership.
    ///
    /// # Arguments
    ///
    /// * `doc_id` - The document ID to check
    pub async fn owns_document(&self, doc_id: &DocumentId) -> bool {
        let ring = self.hash_ring.read().await;
        ring.get_shard(doc_id) == self.shard_id
    }

    /// Check ownership synchronously (for non-async contexts).
    ///
    /// This is a blocking operation and should only be used when
    /// async is not available.
    pub fn owns_document_sync(&self, doc_id: &DocumentId, ring: &ConsistentHashRing) -> bool {
        ring.get_shard(doc_id) == self.shard_id
    }

    /// Ingest a document (only if this shard owns it).
    ///
    /// Returns an error if the document should be routed to a different shard.
    ///
    /// # Arguments
    ///
    /// * `title` - Document title
    /// * `content` - Document content
    /// * `position` - Spatial position in the substrate
    ///
    /// # Returns
    ///
    /// The document ID if successful, or a routing error if this shard
    /// doesn't own the document.
    pub async fn ingest_document(
        &mut self,
        title: &str,
        content: &str,
        position: Position,
    ) -> DistributedResult<DocumentId> {
        // Create document first to get ID
        let doc_id = DocumentId::new();

        // Check if we own this document
        if !self.owns_document(&doc_id).await {
            return Err(DistributedError::RoutingFailed(doc_id));
        }

        // Ingest into local colony
        let actual_id = self.local.ingest_document(title, content, position);
        Ok(actual_id)
    }

    /// Ingest a document with a pre-determined ID.
    ///
    /// This is useful when routing a document from the coordinator,
    /// where the ID has already been assigned.
    ///
    /// # Arguments
    ///
    /// * `title` - Document title
    /// * `content` - Document content
    /// * `position` - Spatial position in the substrate
    pub fn ingest_document_direct(
        &mut self,
        title: &str,
        content: &str,
        position: Position,
    ) -> DocumentId {
        self.local.ingest_document(title, content, position)
    }

    /// Execute a tick phase.
    ///
    /// Each tick is divided into phases that must be synchronized across
    /// all shards. The coordinator ensures all shards complete each phase
    /// before moving to the next.
    ///
    /// # Arguments
    ///
    /// * `phase` - The phase to execute
    ///
    /// # Returns
    ///
    /// A `PhaseResult` containing statistics and any cross-shard edges
    /// that need resolution.
    pub fn tick_phase(&mut self, phase: TickPhase) -> PhaseResult {
        match phase {
            TickPhase::Sense => {
                // In distributed mode, sense phase prepares agent decisions
                // but doesn't execute them yet. This allows the coordinator
                // to synchronize before the Act phase.
                PhaseResult {
                    shard_id: self.shard_id,
                    phase,
                    tick: self.local.substrate().current_tick(),
                    cross_shard_edges: Vec::new(),
                    node_count: self.local.stats().graph_nodes,
                    edge_count: self.local.stats().graph_edges,
                }
            }
            TickPhase::Act | TickPhase::Decay => {
                // Run a full local tick (Colony.tick() handles both agent
                // actions and decay in one pass)
                let _events = self.local.tick();

                // Collect any cross-shard edges from this tick
                let cross_edges = std::mem::take(&mut self.pending_cross_edges);

                PhaseResult {
                    shard_id: self.shard_id,
                    phase,
                    tick: self.local.substrate().current_tick(),
                    cross_shard_edges: cross_edges,
                    node_count: self.local.stats().graph_nodes,
                    edge_count: self.local.stats().graph_edges,
                }
            }
            TickPhase::Advance => {
                // Advance tick is handled by coordinator - we just report status
                PhaseResult {
                    shard_id: self.shard_id,
                    phase,
                    tick: self.local.substrate().current_tick(),
                    cross_shard_edges: Vec::new(),
                    node_count: self.local.stats().graph_nodes,
                    edge_count: self.local.stats().graph_edges,
                }
            }
        }
    }

    /// Get local term frequencies for TF-IDF computation.
    ///
    /// This is called by the coordinator to aggregate document frequencies
    /// across all shards for proper TF-IDF scoring.
    ///
    /// # Arguments
    ///
    /// * `terms` - The terms to count
    ///
    /// # Returns
    ///
    /// A map of term to document frequency on this shard.
    pub fn get_term_frequencies(&self, terms: &[String]) -> HashMap<String, u64> {
        let mut freqs = HashMap::new();
        let graph = self.local.substrate().graph();

        for term in terms {
            let count = graph.find_nodes_by_label(term).len();
            if count > 0 {
                freqs.insert(term.clone(), count as u64);
            }
        }

        freqs
    }

    /// Get a node's data from the local graph.
    ///
    /// # Arguments
    ///
    /// * `id` - The node ID to look up
    ///
    /// # Returns
    ///
    /// The node data if found on this shard.
    pub fn get_node(&self, id: &NodeId) -> Option<NodeData> {
        self.local.substrate().graph().get_node(id).cloned()
    }

    /// Add a peer shard address.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - The peer's shard ID
    /// * `address` - The peer's network address (e.g., "127.0.0.1:8081")
    pub fn add_peer(&mut self, shard_id: ShardId, address: String) {
        self.peers.insert(shard_id, address);
    }

    /// Remove a peer shard.
    ///
    /// Also invalidates any ghost nodes from that shard.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - The peer's shard ID
    pub fn remove_peer(&mut self, shard_id: ShardId) {
        self.peers.remove(&shard_id);
        self.ghost_cache.invalidate_shard(shard_id);
    }

    /// Get peer addresses.
    pub fn peers(&self) -> &HashMap<ShardId, String> {
        &self.peers
    }

    /// Get a peer's address.
    pub fn peer_address(&self, shard_id: &ShardId) -> Option<&String> {
        self.peers.get(shard_id)
    }

    /// Get the local colony reference.
    pub fn local(&self) -> &Colony {
        &self.local
    }

    /// Get mutable local colony reference.
    pub fn local_mut(&mut self) -> &mut Colony {
        &mut self.local
    }

    /// Get ghost node cache.
    pub fn ghost_cache(&self) -> &GhostNodeCache {
        &self.ghost_cache
    }

    /// Get mutable ghost node cache.
    pub fn ghost_cache_mut(&mut self) -> &mut GhostNodeCache {
        &mut self.ghost_cache
    }

    /// Get the cross-shard edge manager.
    pub fn edge_manager(&self) -> &CrossShardEdgeManager {
        &self.edge_manager
    }

    /// Get mutable cross-shard edge manager.
    pub fn edge_manager_mut(&mut self) -> &mut CrossShardEdgeManager {
        &mut self.edge_manager
    }

    /// Register an outgoing cross-shard edge with the edge manager.
    ///
    /// This registers the edge for tracking and ghost node resolution.
    /// The edge is also added to pending_cross_edges for phase synchronization.
    ///
    /// # Arguments
    ///
    /// * `edge` - The cross-shard edge to register
    pub fn register_cross_shard_edge(&mut self, edge: CrossShardEdge) {
        self.edge_manager.add_outgoing_edge(edge.clone());
        self.pending_cross_edges.push(edge);
    }

    /// Handle edges when a shard goes offline.
    ///
    /// Removes all edges to/from the specified shard and invalidates
    /// corresponding ghost nodes.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - The shard that went offline
    ///
    /// # Returns
    ///
    /// A tuple of (edges_removed, ghosts_invalidated).
    pub fn handle_shard_offline(&mut self, shard_id: ShardId) -> (usize, usize) {
        let edges_removed = self.edge_manager.remove_shard_edges(shard_id);
        let ghosts_invalidated = self.ghost_cache.invalidate_shard(shard_id);
        (edges_removed, ghosts_invalidated)
    }

    /// Decay all cross-shard edges and prune weak ones.
    ///
    /// # Arguments
    ///
    /// * `rate` - Decay rate (0.0 to 1.0)
    /// * `threshold` - Minimum weight threshold
    ///
    /// # Returns
    ///
    /// Vector of pruned edges.
    pub fn decay_cross_shard_edges(&mut self, rate: f64, threshold: f64) -> Vec<CrossShardEdge> {
        self.edge_manager.decay_edges(rate, threshold)
    }

    /// Get statistics about cross-shard edges.
    pub fn cross_shard_edge_stats(&self) -> CrossShardEdgeStats {
        self.edge_manager.stats()
    }

    /// Get all shards this shard has edges to.
    pub fn connected_shards(&self) -> Vec<ShardId> {
        self.edge_manager.connected_shards()
    }

    /// Resolve pending edges by fetching ghost nodes.
    ///
    /// Takes the pending edges from the manager and returns them for
    /// processing. After ghost nodes are fetched, the caller should
    /// insert them into the ghost cache.
    ///
    /// # Returns
    ///
    /// Pending edges grouped by target shard for efficient batching.
    pub fn take_pending_for_resolution(&mut self) -> HashMap<ShardId, Vec<CrossShardEdge>> {
        let pending = self.edge_manager.take_pending();
        let mut by_shard: HashMap<ShardId, Vec<CrossShardEdge>> = HashMap::new();
        for edge in pending {
            by_shard.entry(edge.to_shard).or_default().push(edge);
        }
        by_shard
    }

    /// Add a cross-shard edge to be synchronized.
    ///
    /// Cross-shard edges are collected during the Act phase and sent
    /// to the coordinator for resolution during the Exchange phase.
    ///
    /// # Arguments
    ///
    /// * `edge` - The cross-shard edge to add
    pub fn add_pending_cross_edge(&mut self, edge: CrossShardEdge) {
        self.pending_cross_edges.push(edge);
    }

    /// Get pending cross-shard edges.
    pub fn pending_cross_edges(&self) -> &[CrossShardEdge] {
        &self.pending_cross_edges
    }

    /// Clear pending cross-shard edges (after they've been processed).
    pub fn clear_pending_cross_edges(&mut self) {
        self.pending_cross_edges.clear();
    }

    /// Get the hash ring reference.
    pub fn hash_ring(&self) -> &Arc<RwLock<ConsistentHashRing>> {
        &self.hash_ring
    }

    /// Get shard health information.
    pub fn health(&self) -> ShardHealth {
        let stats = self.local.stats();
        ShardHealth {
            shard_id: self.shard_id,
            healthy: true,
            load: stats.agents_alive as f64 / 100.0, // Rough load estimate
            memory_usage_mb: 0,                      // Would need actual measurement
            pending_operations: self.pending_cross_edges.len(),
        }
    }

    /// Get detailed colony statistics.
    pub fn stats(&self) -> ColonyStats {
        self.local.stats()
    }

    /// Get the current tick number.
    pub fn current_tick(&self) -> Tick {
        self.local.substrate().current_tick()
    }

    /// Get the total number of nodes in this shard's graph.
    pub fn node_count(&self) -> usize {
        self.local.substrate().node_count()
    }

    /// Get the total number of documents in this shard.
    pub fn document_count(&self) -> usize {
        self.local.substrate().all_documents().len()
    }

    /// Run a single tick on the local colony.
    pub fn tick(&mut self) {
        self.local.tick();
    }

    /// Run multiple ticks on the local colony.
    pub fn run(&mut self, ticks: u64) {
        self.local.run(ticks);
    }

    /// Get shard info for registration with coordinator.
    pub fn shard_info(&self, address: String) -> ShardInfo {
        let stats = self.local.stats();
        ShardInfo {
            id: self.shard_id,
            address,
            node_count: stats.graph_nodes,
            edge_count: stats.graph_edges,
            document_count: stats.documents_total,
            last_heartbeat: 0, // Set by coordinator
        }
    }

    /// Execute a local query and return scored results.
    ///
    /// # Arguments
    ///
    /// * `request` - The query request from the coordinator
    ///
    /// # Returns
    ///
    /// Local query results including scored nodes and term frequencies.
    pub fn execute_local_query(&self, request: &LocalQueryRequest) -> LocalQueryResult {
        let graph = self.local.substrate().graph();
        let mut results = Vec::new();

        // Simple term matching (a real implementation would use TF-IDF)
        for term in &request.query_terms {
            let matching_nodes = graph.find_nodes_by_label(term);
            for node_id in matching_nodes {
                if let Some(node) = graph.get_node(&node_id) {
                    // Score based on access count (simple relevance proxy)
                    let score = node.access_count as f64 * 0.1;
                    results.push(ScoredNode {
                        node_id,
                        label: node.label.clone(),
                        score,
                        shard_id: self.shard_id,
                    });
                }
            }
        }

        // Sort by score descending and limit results
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(request.max_results);

        // Get term frequencies for global DF computation
        let term_frequencies = self.get_term_frequencies(&request.query_terms);

        LocalQueryResult {
            shard_id: self.shard_id,
            results,
            term_frequencies,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_shard() -> (ShardedColony, Arc<RwLock<ConsistentHashRing>>) {
        let hash_ring = Arc::new(RwLock::new(ConsistentHashRing::new(3)));
        let shard = ShardedColony::new(ShardId::new(0), ColonyConfig::default(), hash_ring.clone());
        (shard, hash_ring)
    }

    #[test]
    fn test_new_sharded_colony() {
        let (shard, _) = create_test_shard();
        assert_eq!(shard.shard_id(), ShardId::new(0));
        assert!(shard.pending_cross_edges().is_empty());
        assert_eq!(shard.ghost_cache().len(), 0);
    }

    #[test]
    fn test_add_peer() {
        let (mut shard, _) = create_test_shard();
        shard.add_peer(ShardId::new(1), "127.0.0.1:8081".to_string());
        shard.add_peer(ShardId::new(2), "127.0.0.1:8082".to_string());

        assert_eq!(shard.peers().len(), 2);
        assert_eq!(
            shard.peer_address(&ShardId::new(1)),
            Some(&"127.0.0.1:8081".to_string())
        );
    }

    #[test]
    fn test_remove_peer_invalidates_ghosts() {
        let (mut shard, _) = create_test_shard();
        shard.add_peer(ShardId::new(1), "127.0.0.1:8081".to_string());

        // Add some ghost nodes from shard 1
        let ghost = GhostNode::new(NodeId::from_seed(1), ShardId::new(1), "test".to_string());
        shard.ghost_cache_mut().insert(ghost);
        assert_eq!(shard.ghost_cache().len(), 1);

        // Remove peer - should invalidate ghosts
        shard.remove_peer(ShardId::new(1));
        assert_eq!(shard.ghost_cache().len(), 0);
    }

    #[test]
    fn test_tick_phase_sense() {
        let (mut shard, _) = create_test_shard();
        let result = shard.tick_phase(TickPhase::Sense);

        assert_eq!(result.shard_id, ShardId::new(0));
        assert_eq!(result.phase, TickPhase::Sense);
        assert!(result.cross_shard_edges.is_empty());
    }

    #[test]
    fn test_tick_phase_act() {
        let (mut shard, _) = create_test_shard();
        let result = shard.tick_phase(TickPhase::Act);

        assert_eq!(result.shard_id, ShardId::new(0));
        assert_eq!(result.phase, TickPhase::Act);
    }

    #[test]
    fn test_health() {
        let (shard, _) = create_test_shard();
        let health = shard.health();

        assert_eq!(health.shard_id, ShardId::new(0));
        assert!(health.healthy);
        assert_eq!(health.pending_operations, 0);
    }

    #[test]
    fn test_add_pending_cross_edge() {
        let (mut shard, _) = create_test_shard();

        let edge = CrossShardEdge {
            from_node: NodeId::from_seed(1),
            to_node: NodeId::from_seed(2),
            to_shard: ShardId::new(1),
            weight: 0.5,
        };

        shard.add_pending_cross_edge(edge);
        assert_eq!(shard.pending_cross_edges().len(), 1);

        shard.clear_pending_cross_edges();
        assert!(shard.pending_cross_edges().is_empty());
    }

    #[test]
    fn test_ingest_document_direct() {
        let (mut shard, _) = create_test_shard();

        let doc_id = shard.ingest_document_direct("Test", "Content", Position::new(0.0, 0.0));

        let stats = shard.stats();
        assert_eq!(stats.documents_total, 1);
        assert!(!doc_id.0.is_nil());
    }

    #[test]
    fn test_get_term_frequencies() {
        let (shard, _) = create_test_shard();

        // Empty graph should return empty frequencies
        let freqs = shard.get_term_frequencies(&["test".to_string()]);
        assert!(freqs.is_empty());
    }

    #[test]
    fn test_execute_local_query() {
        let (shard, _) = create_test_shard();

        let request = LocalQueryRequest {
            query_terms: vec!["test".to_string()],
            max_results: 10,
            global_df: HashMap::new(),
        };

        let result = shard.execute_local_query(&request);
        assert_eq!(result.shard_id, ShardId::new(0));
        assert!(result.results.is_empty()); // No nodes yet
    }

    #[test]
    fn test_shard_info() {
        let (shard, _) = create_test_shard();
        let info = shard.shard_info("127.0.0.1:8080".to_string());

        assert_eq!(info.id, ShardId::new(0));
        assert_eq!(info.address, "127.0.0.1:8080");
        assert_eq!(info.node_count, 0);
        assert_eq!(info.edge_count, 0);
    }

    #[test]
    fn test_node_count_and_document_count() {
        let (mut shard, _) = create_test_shard();

        assert_eq!(shard.node_count(), 0);
        assert_eq!(shard.document_count(), 0);

        shard.ingest_document_direct("Test", "Content", Position::new(0.0, 0.0));
        assert_eq!(shard.document_count(), 1);
    }

    #[test]
    fn test_tick_and_run() {
        let (mut shard, _) = create_test_shard();

        shard.tick();
        assert_eq!(shard.current_tick(), 1);

        shard.run(5);
        assert_eq!(shard.current_tick(), 6);
    }

    #[tokio::test]
    async fn test_owns_document() {
        let (shard, _hash_ring) = create_test_shard();

        // Test a few document IDs - at least some should be owned by shard 0
        let mut owned_count = 0;
        for i in 0..100 {
            let doc_id = DocumentId::from_seed(i);
            if shard.owns_document(&doc_id).await {
                owned_count += 1;
            }
        }

        // With 3 shards, shard 0 should own roughly 1/3 of documents
        assert!(owned_count > 20 && owned_count < 50);
    }

    #[test]
    fn test_with_ghost_cache_size() {
        let hash_ring = Arc::new(RwLock::new(ConsistentHashRing::new(3)));
        let shard = ShardedColony::with_ghost_cache_size(
            ShardId::new(0),
            ColonyConfig::default(),
            hash_ring,
            500,
        );

        assert_eq!(shard.ghost_cache().capacity(), 500);
    }
}
