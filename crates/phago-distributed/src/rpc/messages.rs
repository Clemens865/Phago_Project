//! Additional message types for RPC communication.
//!
//! This module defines message structures for various distributed
//! operations that don't fit directly into the service traits.

use crate::types::{CrossShardEdge, ScoredNode, ShardId};
use phago_core::types::{AgentId, NodeId, Position, SignalType, Tick};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message indicating a distributed tick should start.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartTickMessage {
    /// The tick number to start.
    pub tick: Tick,
    /// The shard that initiated the tick (usually coordinator).
    pub initiator: ShardId,
    /// Timestamp when the tick was initiated.
    pub timestamp_ms: u64,
}

/// Message for cross-shard edge notification.
///
/// Sent during the Exchange phase to notify other shards
/// about edges that cross shard boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossShardEdgeNotification {
    /// The edges being reported.
    pub edges: Vec<CrossShardEdge>,
    /// The shard sending this notification.
    pub source_shard: ShardId,
    /// The tick during which these edges were created.
    pub tick: Tick,
}

/// Query scatter request (from coordinator to shards).
///
/// Part of the scatter-gather query pattern. The coordinator
/// sends this to all shards to execute local queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryScatterRequest {
    /// Unique identifier for this query.
    pub query_id: u64,
    /// Search terms for the query.
    pub terms: Vec<String>,
    /// Maximum results to return from each shard.
    pub max_local_results: usize,
    /// Optional embedding vector for semantic search.
    pub embedding: Option<Vec<f32>>,
    /// Whether to search ghost nodes.
    pub include_ghosts: bool,
}

/// Query gather response (from shards to coordinator).
///
/// Shards send this back to the coordinator with their local results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryGatherResponse {
    /// The query ID this response is for.
    pub query_id: u64,
    /// The shard that produced these results.
    pub shard_id: ShardId,
    /// Matching nodes with scores.
    pub results: Vec<ScoredNode>,
    /// Term frequencies for TF-IDF computation.
    pub term_frequencies: HashMap<String, u64>,
    /// Total document count on this shard (for IDF calculation).
    pub total_documents: u64,
    /// Time taken to execute the query in milliseconds.
    pub execution_time_ms: u64,
}

/// A signal that crosses shard boundaries.
///
/// Sent during the Exchange phase when an agent's signal
/// affects regions on other shards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossShardSignal {
    /// Type of the signal.
    pub signal_type: SignalType,
    /// Intensity of the signal.
    pub intensity: f64,
    /// Position where the signal was emitted.
    pub position: Position,
    /// Agent that emitted the signal.
    pub emitter: AgentId,
    /// Tick when the signal was emitted.
    pub tick: Tick,
    /// Source shard where the signal originated.
    pub source_shard: ShardId,
}

/// Heartbeat message from shard to coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    /// The shard sending the heartbeat.
    pub shard_id: ShardId,
    /// Current tick on the shard.
    pub current_tick: Tick,
    /// Number of agents active on the shard.
    pub agent_count: u64,
    /// Number of documents stored.
    pub document_count: u64,
    /// Number of nodes in the graph.
    pub node_count: u64,
    /// Memory usage in bytes.
    pub memory_bytes: u64,
    /// Timestamp of the heartbeat.
    pub timestamp_ms: u64,
}

/// Response to a heartbeat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    /// Whether the coordinator acknowledged the heartbeat.
    pub acknowledged: bool,
    /// Expected tick (for drift detection).
    pub expected_tick: Tick,
    /// Any pending commands for the shard.
    pub commands: Vec<ShardCommand>,
}

/// Commands that the coordinator can send to shards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShardCommand {
    /// Pause processing (e.g., for maintenance).
    Pause,
    /// Resume processing.
    Resume,
    /// Trigger a consistency check.
    ConsistencyCheck,
    /// Compact the local graph.
    Compact,
    /// Sync ghost nodes.
    SyncGhosts,
    /// Shutdown gracefully.
    Shutdown,
}

/// Request to transfer a node to another shard.
///
/// Used during rebalancing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTransferRequest {
    /// Node to transfer.
    pub node_id: NodeId,
    /// Current shard (source).
    pub from_shard: ShardId,
    /// Target shard (destination).
    pub to_shard: ShardId,
    /// Include connected edges.
    pub include_edges: bool,
}

/// Response to a node transfer request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTransferResponse {
    /// Whether the transfer was successful.
    pub success: bool,
    /// Node ID that was transferred.
    pub node_id: NodeId,
    /// Number of edges transferred with the node.
    pub edges_transferred: u64,
    /// Any error message if transfer failed.
    pub error: Option<String>,
}

/// Batch of updates to apply atomically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchUpdate {
    /// Updates to apply.
    pub updates: Vec<UpdateOperation>,
    /// Tick during which these updates should be applied.
    pub tick: Tick,
    /// Whether all updates must succeed (atomic).
    pub atomic: bool,
}

/// A single update operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateOperation {
    /// Create or update a node.
    UpsertNode {
        id: NodeId,
        label: String,
        position: Position,
        embedding: Option<Vec<f32>>,
    },
    /// Create or update an edge.
    UpsertEdge {
        from: NodeId,
        to: NodeId,
        weight: f64,
    },
    /// Delete a node.
    DeleteNode { id: NodeId },
    /// Delete an edge.
    DeleteEdge { from: NodeId, to: NodeId },
    /// Increment edge weight (Hebbian learning).
    IncrementEdge {
        from: NodeId,
        to: NodeId,
        delta: f64,
    },
}

/// Result of applying a batch update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchUpdateResult {
    /// Number of operations that succeeded.
    pub succeeded: u64,
    /// Number of operations that failed.
    pub failed: u64,
    /// Errors for failed operations (index -> error).
    pub errors: HashMap<usize, String>,
}
