//! Core types for distributed colony coordination.
//!
//! This module defines the core data structures used across the distributed
//! system including shard identifiers, tick phases, cross-shard edges,
//! query requests/results, and ghost nodes for remote references.

use phago_core::types::{DocumentId, NodeData, NodeId, Tick};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Unique identifier for a shard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ShardId(pub u32);

impl ShardId {
    /// Create a new shard identifier.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the underlying shard number.
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for ShardId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "shard-{}", self.0)
    }
}

/// Address of a node in the distributed cluster.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeAddress {
    /// Host address (IP or hostname).
    pub host: String,
    /// Port number.
    pub port: u16,
}

impl NodeAddress {
    /// Create a new node address.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// Format as a socket address string.
    pub fn to_socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl std::fmt::Display for NodeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

/// Configuration for the distributed colony.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    /// Number of shards to distribute data across.
    pub num_shards: u32,
    /// Replication factor for fault tolerance.
    pub replication_factor: u32,
    /// Timeout for RPC calls in milliseconds.
    pub rpc_timeout_ms: u64,
    /// Number of virtual nodes per shard for consistent hashing.
    pub virtual_nodes_per_shard: u32,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            num_shards: 3,
            replication_factor: 2,
            rpc_timeout_ms: 5000,
            virtual_nodes_per_shard: 150,
        }
    }
}

/// Status of a shard in the cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShardStatus {
    /// Shard is online and accepting requests.
    Online,
    /// Shard is offline or unreachable.
    Offline,
    /// Shard is recovering/rebalancing data.
    Recovering,
    /// Shard is draining (preparing to go offline).
    Draining,
}

/// Information about a shard registered with the coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    /// The shard's unique identifier.
    pub id: ShardId,
    /// The network address of this shard (e.g., "127.0.0.1:8081").
    pub address: String,
    /// Number of nodes on this shard.
    pub node_count: usize,
    /// Number of edges on this shard (including ghost edges).
    pub edge_count: usize,
    /// Number of documents assigned to this shard.
    pub document_count: usize,
    /// Unix timestamp of the last heartbeat from this shard.
    pub last_heartbeat: u64,
}

impl ShardInfo {
    /// Create a new shard info with the given ID and address.
    pub fn new(id: ShardId, address: String) -> Self {
        Self {
            id,
            address,
            node_count: 0,
            edge_count: 0,
            document_count: 0,
            last_heartbeat: 0,
        }
    }
}

/// Errors that can occur in distributed operations.
#[derive(Error, Debug, Clone)]
pub enum DistributedError {
    #[error("Shard {0:?} not found")]
    ShardNotFound(ShardId),

    #[error("Coordinator unavailable")]
    CoordinatorUnavailable,

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Timeout waiting for phase {0:?}")]
    PhaseTimeout(TickPhase),

    #[error("Document routing failed for {0:?}")]
    RoutingFailed(DocumentId),

    #[error("Cross-shard edge resolution failed")]
    EdgeResolutionFailed,

    #[error("Ghost node not found: {0:?}")]
    GhostNodeNotFound(NodeId),

    #[error("Barrier synchronization failed")]
    BarrierFailed,
}

/// Result type for distributed operations.
pub type DistributedResult<T> = Result<T, DistributedError>;

/// Phases of a distributed tick.
///
/// Each tick is divided into phases that must be synchronized across all shards.
/// The coordinator ensures all shards complete each phase before proceeding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TickPhase {
    /// Agents sense the substrate (read-only phase).
    Sense,
    /// Process agent actions (write phase).
    Act,
    /// Decay signals, traces, and edges (maintenance phase).
    Decay,
    /// Advance tick counter (finalization phase).
    Advance,
}

impl std::fmt::Display for TickPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TickPhase::Sense => write!(f, "Sense"),
            TickPhase::Act => write!(f, "Act"),
            TickPhase::Decay => write!(f, "Decay"),
            TickPhase::Advance => write!(f, "Advance"),
        }
    }
}

/// Result of completing a tick phase on a shard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    /// The shard that completed this phase.
    pub shard_id: ShardId,
    /// The phase that was completed.
    pub phase: TickPhase,
    /// The tick number this phase belongs to.
    pub tick: Tick,
    /// Cross-shard edges created this phase (need ghost resolution).
    pub cross_shard_edges: Vec<CrossShardEdge>,
    /// Local node count after this phase.
    pub node_count: usize,
    /// Local edge count after this phase.
    pub edge_count: usize,
}

/// A cross-shard edge reference.
///
/// When an edge is created that spans two shards, the local shard stores
/// the edge with a ghost node reference. This struct captures the information
/// needed to resolve that ghost node on the remote shard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossShardEdge {
    /// The local node (source of the edge).
    pub from_node: NodeId,
    /// The remote node (target of the edge).
    pub to_node: NodeId,
    /// The shard that owns the target node.
    pub to_shard: ShardId,
    /// The edge weight.
    pub weight: f64,
}

/// Request for a local query on a shard.
///
/// The coordinator sends this to each shard during distributed query execution.
/// The shard uses the global document frequencies to compute proper TF-IDF scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalQueryRequest {
    /// The query terms to search for.
    pub query_terms: Vec<String>,
    /// Maximum number of results to return from this shard.
    pub max_results: usize,
    /// Global document frequencies (from coordinator).
    /// Used for proper TF-IDF scoring across shards.
    pub global_df: HashMap<String, u64>,
}

/// Result of a local query on a shard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalQueryResult {
    /// The shard that produced these results.
    pub shard_id: ShardId,
    /// The scored nodes matching the query.
    pub results: Vec<ScoredNode>,
    /// Local term frequencies for global DF computation.
    /// Sent back to coordinator for aggregation.
    pub term_frequencies: HashMap<String, u64>,
}

/// A node with its relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredNode {
    /// The node identifier.
    pub node_id: NodeId,
    /// The node's label/content.
    pub label: String,
    /// The relevance score (higher is better).
    pub score: f64,
    /// The shard this node belongs to.
    pub shard_id: ShardId,
}

impl PartialEq for ScoredNode {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id && self.shard_id == other.shard_id
    }
}

impl Eq for ScoredNode {}

impl PartialOrd for ScoredNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering for max-heap behavior (higher scores first)
        other
            .score
            .partial_cmp(&self.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Health status of a shard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardHealth {
    /// The shard being reported on.
    pub shard_id: ShardId,
    /// Whether the shard is healthy and responsive.
    pub healthy: bool,
    /// Current load factor (0.0 = idle, 1.0 = fully loaded).
    pub load: f64,
    /// Memory usage in megabytes.
    pub memory_usage_mb: u64,
    /// Number of pending operations in the queue.
    pub pending_operations: usize,
}

impl ShardHealth {
    /// Create a healthy shard status with default values.
    pub fn healthy(shard_id: ShardId) -> Self {
        Self {
            shard_id,
            healthy: true,
            load: 0.0,
            memory_usage_mb: 0,
            pending_operations: 0,
        }
    }

    /// Create an unhealthy shard status.
    pub fn unhealthy(shard_id: ShardId) -> Self {
        Self {
            shard_id,
            healthy: false,
            load: 0.0,
            memory_usage_mb: 0,
            pending_operations: 0,
        }
    }
}

/// A ghost node - minimal reference to a node on another shard.
///
/// Ghost nodes are placeholders for nodes that exist on remote shards.
/// They enable local graph traversal to continue even when edges cross
/// shard boundaries. The full data can be fetched lazily when needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostNode {
    /// The actual node ID (same as on the owning shard).
    pub node_id: NodeId,
    /// The shard that owns this node.
    pub shard_id: ShardId,
    /// The node's label (cached for display/search).
    pub label: String,
    /// Full data fetched lazily when needed for operations.
    pub full_data: Option<NodeData>,
}

impl GhostNode {
    /// Create a new ghost node reference.
    pub fn new(node_id: NodeId, shard_id: ShardId, label: String) -> Self {
        Self {
            node_id,
            shard_id,
            label,
            full_data: None,
        }
    }

    /// Check if the full data has been fetched.
    pub fn is_resolved(&self) -> bool {
        self.full_data.is_some()
    }

    /// Resolve this ghost node with full data from the remote shard.
    pub fn resolve(&mut self, data: NodeData) {
        self.full_data = Some(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phago_core::types::Position;

    #[test]
    fn test_shard_id() {
        let shard = ShardId::new(42);
        assert_eq!(shard.0, 42);
        assert_eq!(shard.as_u32(), 42);
        assert_eq!(format!("{}", shard), "shard-42");
    }

    #[test]
    fn test_node_address() {
        let addr = NodeAddress::new("127.0.0.1", 8080);
        assert_eq!(addr.host, "127.0.0.1");
        assert_eq!(addr.port, 8080);
        assert_eq!(addr.to_socket_addr(), "127.0.0.1:8080");
        assert_eq!(format!("{}", addr), "127.0.0.1:8080");
    }

    #[test]
    fn test_distributed_config_default() {
        let config = DistributedConfig::default();
        assert_eq!(config.num_shards, 3);
        assert_eq!(config.replication_factor, 2);
        assert_eq!(config.rpc_timeout_ms, 5000);
        assert_eq!(config.virtual_nodes_per_shard, 150);
    }

    #[test]
    fn test_tick_phase_display() {
        assert_eq!(format!("{}", TickPhase::Sense), "Sense");
        assert_eq!(format!("{}", TickPhase::Act), "Act");
        assert_eq!(format!("{}", TickPhase::Decay), "Decay");
        assert_eq!(format!("{}", TickPhase::Advance), "Advance");
    }

    #[test]
    fn test_scored_node_ordering() {
        let node1 = ScoredNode {
            node_id: NodeId::from_seed(1),
            label: "high".to_string(),
            score: 0.9,
            shard_id: ShardId::new(0),
        };
        let node2 = ScoredNode {
            node_id: NodeId::from_seed(2),
            label: "low".to_string(),
            score: 0.1,
            shard_id: ShardId::new(0),
        };

        // Higher score should come first (reverse ordering)
        assert!(node1 < node2);
    }

    #[test]
    fn test_ghost_node_resolution() {
        let mut ghost = GhostNode::new(NodeId::from_seed(1), ShardId::new(1), "test".to_string());
        assert!(!ghost.is_resolved());

        let data = NodeData {
            id: NodeId::from_seed(1),
            label: "test".to_string(),
            node_type: phago_core::types::NodeType::Concept,
            position: Position::new(0.0, 0.0),
            access_count: 0,
            created_tick: 0,
            embedding: None,
        };
        ghost.resolve(data);
        assert!(ghost.is_resolved());
    }

    #[test]
    fn test_shard_health() {
        let healthy = ShardHealth::healthy(ShardId::new(0));
        assert!(healthy.healthy);
        assert_eq!(healthy.load, 0.0);

        let unhealthy = ShardHealth::unhealthy(ShardId::new(1));
        assert!(!unhealthy.healthy);
    }

    #[test]
    fn test_shard_info_new() {
        let info = ShardInfo::new(ShardId::new(5), "127.0.0.1:8085".to_string());
        assert_eq!(info.id, ShardId::new(5));
        assert_eq!(info.address, "127.0.0.1:8085");
        assert_eq!(info.node_count, 0);
        assert_eq!(info.edge_count, 0);
        assert_eq!(info.document_count, 0);
    }

    #[test]
    fn test_phase_result() {
        let result = PhaseResult {
            shard_id: ShardId::new(0),
            phase: TickPhase::Sense,
            tick: 42,
            cross_shard_edges: vec![],
            node_count: 100,
            edge_count: 250,
        };
        assert_eq!(result.tick, 42);
        assert_eq!(result.node_count, 100);
    }

    #[test]
    fn test_cross_shard_edge() {
        let edge = CrossShardEdge {
            from_node: NodeId::from_seed(1),
            to_node: NodeId::from_seed(2),
            to_shard: ShardId::new(1),
            weight: 0.75,
        };
        assert_eq!(edge.to_shard, ShardId::new(1));
        assert!((edge.weight - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_local_query_request() {
        let mut global_df = HashMap::new();
        global_df.insert("rust".to_string(), 100);
        global_df.insert("programming".to_string(), 200);

        let request = LocalQueryRequest {
            query_terms: vec!["rust".to_string(), "programming".to_string()],
            max_results: 10,
            global_df,
        };
        assert_eq!(request.query_terms.len(), 2);
        assert_eq!(request.max_results, 10);
    }
}
