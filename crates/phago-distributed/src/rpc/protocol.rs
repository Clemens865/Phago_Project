//! tarpc service trait definitions.
//!
//! This module defines the RPC service interfaces for distributed
//! colony coordination using tarpc's procedural macro system.

use crate::types::{
    GhostNode, LocalQueryRequest, LocalQueryResult, PhaseResult, ShardHealth, ShardId, ShardInfo,
    TickPhase,
};
use phago_core::types::{Document, DocumentId, NodeData, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result type for RPC operations that needs to be serializable.
pub type RpcResult<T> = Result<T, RpcError>;

/// Serializable error type for RPC calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcError {
    /// Shard not found.
    ShardNotFound(u32),
    /// Coordinator unavailable.
    CoordinatorUnavailable,
    /// RPC call failed.
    RpcFailed(String),
    /// Phase timeout.
    PhaseTimeout(String),
    /// Document routing failed.
    RoutingFailed,
    /// Edge resolution failed.
    EdgeResolutionFailed,
    /// Ghost node not found.
    GhostNodeNotFound,
    /// Barrier synchronization failed.
    BarrierFailed,
    /// Internal error.
    Internal(String),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::ShardNotFound(id) => write!(f, "Shard {} not found", id),
            RpcError::CoordinatorUnavailable => write!(f, "Coordinator unavailable"),
            RpcError::RpcFailed(msg) => write!(f, "RPC failed: {}", msg),
            RpcError::PhaseTimeout(phase) => write!(f, "Phase timeout: {}", phase),
            RpcError::RoutingFailed => write!(f, "Document routing failed"),
            RpcError::EdgeResolutionFailed => write!(f, "Edge resolution failed"),
            RpcError::GhostNodeNotFound => write!(f, "Ghost node not found"),
            RpcError::BarrierFailed => write!(f, "Barrier synchronization failed"),
            RpcError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for RpcError {}

/// Service provided by each shard in the distributed colony.
///
/// This service handles document ingestion, tick phase execution,
/// local queries, and cross-shard coordination.
#[tarpc::service]
pub trait ShardService {
    /// Ingest a document into this shard.
    ///
    /// The document will be processed by local agents during the next tick.
    /// Returns the document ID assigned to the ingested document.
    async fn ingest_document(doc: Document) -> RpcResult<DocumentId>;

    /// Execute a tick phase on this shard.
    ///
    /// Phases are executed in order: Sense -> Act -> Decay -> Advance.
    /// Each phase must complete on all shards before the next phase begins (barrier sync).
    async fn tick_phase(phase: TickPhase, tick: u64) -> RpcResult<PhaseResult>;

    /// Execute a local query (part of distributed query).
    ///
    /// Returns matching nodes from this shard's portion of the graph.
    /// Results are combined by the coordinator using scatter-gather.
    async fn local_query(req: LocalQueryRequest) -> RpcResult<LocalQueryResult>;

    /// Get term frequencies for global DF computation.
    ///
    /// Returns a map of term -> document frequency for TF-IDF calculation.
    /// Used during global DF aggregation for TF-IDF scoring.
    async fn get_term_frequencies(terms: Vec<String>) -> RpcResult<HashMap<String, u64>>;

    /// Fetch a node's full data (for ghost node resolution).
    ///
    /// Used when a shard needs detailed information about a node
    /// that exists on another shard.
    async fn get_node(id: NodeId) -> RpcResult<Option<NodeData>>;

    /// Health check.
    ///
    /// Returns the current health status of this shard including
    /// resource usage and processing statistics.
    async fn health_check() -> RpcResult<ShardHealth>;

    /// Resolve cross-shard edges by fetching ghost node data.
    ///
    /// Batch operation to fetch data for multiple nodes at once,
    /// creating ghost node representations for cross-shard edges.
    async fn resolve_ghost_nodes(node_ids: Vec<NodeId>) -> RpcResult<Vec<GhostNode>>;

    /// Get the list of nodes connected to a given node.
    ///
    /// Returns node IDs of all neighbors, including cross-shard references.
    async fn get_neighbors(node_id: NodeId) -> RpcResult<Vec<NodeId>>;

    /// Receive cross-shard signals during the Exchange phase.
    ///
    /// Signals from other shards are delivered here for local processing.
    async fn receive_signals(signals: Vec<crate::rpc::messages::CrossShardSignal>)
        -> RpcResult<()>;
}

/// Service provided by the coordinator.
///
/// The coordinator manages shard registration, tick synchronization,
/// query distribution, and global state aggregation.
#[tarpc::service]
pub trait CoordinatorService {
    /// Register a new shard with the coordinator.
    ///
    /// Returns the assigned shard ID. The coordinator will include
    /// this shard in subsequent tick coordination.
    async fn register(info: ShardInfo) -> RpcResult<ShardId>;

    /// Unregister a shard from the coordinator.
    ///
    /// Should be called during graceful shutdown. The coordinator
    /// will stop routing requests to this shard.
    async fn unregister(shard_id: ShardId) -> RpcResult<()>;

    /// Report that a shard has completed a tick phase.
    ///
    /// Used for barrier synchronization. The coordinator tracks
    /// completion across all shards before advancing to the next phase.
    async fn phase_complete(shard_id: ShardId, phase: TickPhase, tick: u64) -> RpcResult<()>;

    /// Get the shard responsible for a document.
    ///
    /// Uses consistent hashing to determine which shard owns a document.
    async fn route_document(doc_id: DocumentId) -> ShardId;

    /// Get the shard responsible for a node.
    ///
    /// Uses consistent hashing based on node ID.
    async fn route_node(node_id: NodeId) -> ShardId;

    /// Get global document frequencies for TF-IDF.
    ///
    /// Returns aggregated term frequencies across all shards.
    async fn get_global_df(terms: Vec<String>) -> RpcResult<HashMap<String, u64>>;

    /// Signal ready for next phase (barrier).
    ///
    /// Returns true when all shards are ready and the phase can proceed.
    async fn barrier_ready(shard_id: ShardId, phase: TickPhase, tick: u64) -> RpcResult<bool>;

    /// Get current tick number.
    ///
    /// Returns the global tick counter maintained by the coordinator.
    async fn current_tick() -> u64;

    /// Get all registered shards.
    ///
    /// Returns information about all shards in the cluster.
    async fn list_shards() -> Vec<ShardInfo>;

    /// Request to start a new tick.
    ///
    /// Only succeeds if no tick is currently in progress.
    async fn start_tick() -> RpcResult<u64>;

    /// Get the status of the current tick.
    ///
    /// Returns the current phase and which shards have completed it.
    async fn tick_status() -> RpcResult<TickStatus>;
}

/// Status of the current tick across all shards.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TickStatus {
    /// Current tick number.
    pub tick: u64,
    /// Current phase being executed.
    pub phase: TickPhase,
    /// Shards that have completed the current phase.
    pub completed_shards: Vec<ShardId>,
    /// Shards that are still processing.
    pub pending_shards: Vec<ShardId>,
    /// Whether the tick is complete.
    pub tick_complete: bool,
}
