//! tarpc server implementations.
//!
//! This module provides server implementations for the distributed colony's
//! RPC services. Each server wraps the corresponding local component
//! (ShardedColony or Coordinator) and exposes it via tarpc.

use crate::coordinator::Coordinator;
use crate::rpc::messages::CrossShardSignal;
use crate::rpc::protocol::{CoordinatorService, RpcError, RpcResult, ShardService, TickStatus};
use crate::shard::ShardedColony;
use crate::types::*;
use futures::StreamExt;
use phago_core::substrate::Substrate;
use phago_core::topology::TopologyGraph;
use phago_core::types::{Document, DocumentId, NodeData, NodeId};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tarpc::context::Context;
use tarpc::server::{self, Channel};
use tokio_serde::formats::Bincode;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument};

/// Server implementation for a shard.
///
/// ShardServer wraps a `ShardedColony` and implements the `ShardService` trait,
/// allowing remote clients to interact with the shard via tarpc RPC.
///
/// # Thread Safety
///
/// The server uses `Arc<RwLock<ShardedColony>>` to allow safe concurrent access
/// from multiple RPC handler tasks.
///
/// # Example
///
/// ```rust,ignore
/// use phago_distributed::rpc::server::ShardServer;
/// use phago_distributed::shard::ShardedColony;
///
/// let shard = Arc::new(RwLock::new(ShardedColony::new(...)));
/// let server = ShardServer::new(shard);
/// server.serve("127.0.0.1:8080".parse().unwrap()).await?;
/// ```
#[derive(Clone)]
pub struct ShardServer {
    shard: Arc<RwLock<ShardedColony>>,
}

impl ShardServer {
    /// Create a new shard server.
    ///
    /// # Arguments
    ///
    /// * `shard` - The sharded colony to serve
    pub fn new(shard: Arc<RwLock<ShardedColony>>) -> Self {
        Self { shard }
    }

    /// Start serving on the given address.
    ///
    /// This method listens for incoming connections and spawns a task
    /// for each client to handle their requests concurrently.
    ///
    /// # Arguments
    ///
    /// * `addr` - The socket address to bind to
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to bind to the address.
    pub async fn start(self, addr: SocketAddr) -> Result<(), std::io::Error> {
        use crate::rpc::protocol::ShardService;
        let listener = tarpc::serde_transport::tcp::listen(&addr, Bincode::default).await?;
        info!("Shard server listening on {}", addr);

        listener
            .filter_map(|r| futures::future::ready(r.ok()))
            .map(server::BaseChannel::with_defaults)
            .for_each_concurrent(10, |channel| {
                let server = self.clone();
                async move {
                    channel.execute(server.serve()).for_each(|_| async {}).await
                }
            })
            .await;

        Ok(())
    }
}

impl ShardService for ShardServer {
    #[instrument(skip(self, _ctx), fields(doc_title = %doc.title))]
    async fn ingest_document(self, _ctx: Context, doc: Document) -> RpcResult<DocumentId> {
        debug!("Ingesting document: {}", doc.title);
        let mut shard = self.shard.write().await;
        let id = shard.ingest_document_direct(&doc.title, &doc.content, doc.position);
        debug!("Document ingested with ID: {:?}", id);
        Ok(id)
    }

    #[instrument(skip(self, _ctx), fields(phase = %phase, tick = tick))]
    async fn tick_phase(
        self,
        _ctx: Context,
        phase: TickPhase,
        tick: u64,
    ) -> RpcResult<PhaseResult> {
        debug!("Executing tick phase {:?} for tick {}", phase, tick);
        let mut shard = self.shard.write().await;
        let result = shard.tick_phase(phase);
        debug!(
            "Phase complete: {} nodes, {} edges, {} cross-shard edges",
            result.node_count,
            result.edge_count,
            result.cross_shard_edges.len()
        );
        Ok(result)
    }

    #[instrument(skip(self, _ctx, req), fields(terms = ?req.query_terms, max_results = req.max_results))]
    async fn local_query(
        self,
        _ctx: Context,
        req: LocalQueryRequest,
    ) -> RpcResult<LocalQueryResult> {
        debug!("Executing local query with {} terms", req.query_terms.len());
        let shard = self.shard.read().await;
        let result = shard.execute_local_query(&req);
        debug!("Query returned {} results", result.results.len());
        Ok(result)
    }

    #[instrument(skip(self, _ctx), fields(term_count = terms.len()))]
    async fn get_term_frequencies(
        self,
        _ctx: Context,
        terms: Vec<String>,
    ) -> RpcResult<HashMap<String, u64>> {
        debug!("Getting term frequencies for {} terms", terms.len());
        let shard = self.shard.read().await;
        let freqs = shard.get_term_frequencies(&terms);
        debug!("Returned {} term frequencies", freqs.len());
        Ok(freqs)
    }

    #[instrument(skip(self, _ctx), fields(node_id = ?id))]
    async fn get_node(self, _ctx: Context, id: NodeId) -> RpcResult<Option<NodeData>> {
        debug!("Getting node {:?}", id);
        let shard = self.shard.read().await;
        let node = shard.get_node(&id);
        debug!("Node found: {}", node.is_some());
        Ok(node)
    }

    #[instrument(skip(self, _ctx))]
    async fn health_check(self, _ctx: Context) -> RpcResult<ShardHealth> {
        debug!("Health check requested");
        let shard = self.shard.read().await;
        let health = shard.health();
        debug!(
            "Health: healthy={}, load={:.2}",
            health.healthy, health.load
        );
        Ok(health)
    }

    #[instrument(skip(self, _ctx), fields(node_count = node_ids.len()))]
    async fn resolve_ghost_nodes(
        self,
        _ctx: Context,
        node_ids: Vec<NodeId>,
    ) -> RpcResult<Vec<GhostNode>> {
        debug!("Resolving {} ghost nodes", node_ids.len());
        let shard = self.shard.read().await;
        let shard_id = shard.shard_id();

        let mut ghosts = Vec::with_capacity(node_ids.len());
        for id in node_ids {
            if let Some(node) = shard.get_node(&id) {
                let mut ghost = GhostNode::new(id, shard_id, node.label.clone());
                ghost.resolve(node);
                ghosts.push(ghost);
            }
        }

        debug!("Resolved {} ghost nodes", ghosts.len());
        Ok(ghosts)
    }

    #[instrument(skip(self, _ctx), fields(node_id = ?node_id))]
    async fn get_neighbors(self, _ctx: Context, node_id: NodeId) -> RpcResult<Vec<NodeId>> {
        debug!("Getting neighbors for node {:?}", node_id);
        let shard = self.shard.read().await;
        let graph = shard.local().substrate().graph();
        let neighbors: Vec<NodeId> = graph
            .neighbors(&node_id)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        debug!("Found {} neighbors", neighbors.len());
        Ok(neighbors)
    }

    #[instrument(skip(self, _ctx, signals), fields(signal_count = signals.len()))]
    async fn receive_signals(self, _ctx: Context, signals: Vec<CrossShardSignal>) -> RpcResult<()> {
        debug!("Receiving {} cross-shard signals", signals.len());

        let mut shard = self.shard.write().await;
        for sig in &signals {
            let local_signal = phago_core::types::Signal {
                signal_type: sig.signal_type.clone(),
                intensity: sig.intensity,
                position: sig.position.clone(),
                emitter: sig.emitter,
                tick: sig.tick,
            };
            shard.local_mut().substrate_mut().emit_signal(local_signal);
        }

        debug!("Applied {} signals to substrate", signals.len());
        Ok(())
    }
}

/// Server implementation for the coordinator.
///
/// CoordinatorServer wraps a `Coordinator` and implements the `CoordinatorService`
/// trait, allowing shards and clients to interact with the coordinator via tarpc RPC.
///
/// # Thread Safety
///
/// The coordinator is wrapped in `Arc` as it uses interior mutability
/// (`RwLock`, `AtomicU64`) for thread-safe access.
///
/// # Example
///
/// ```rust,ignore
/// use phago_distributed::rpc::server::CoordinatorServer;
/// use phago_distributed::coordinator::Coordinator;
///
/// let coordinator = Arc::new(Coordinator::new(3));
/// let server = CoordinatorServer::new(coordinator);
/// server.serve("127.0.0.1:8080".parse().unwrap()).await?;
/// ```
#[derive(Clone)]
pub struct CoordinatorServer {
    coordinator: Arc<Coordinator>,
}

impl CoordinatorServer {
    /// Create a new coordinator server.
    ///
    /// # Arguments
    ///
    /// * `coordinator` - The coordinator to serve
    pub fn new(coordinator: Arc<Coordinator>) -> Self {
        Self { coordinator }
    }

    /// Start serving on the given address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The socket address to bind to
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to bind to the address.
    pub async fn start(self, addr: SocketAddr) -> Result<(), std::io::Error> {
        use crate::rpc::protocol::CoordinatorService;
        let listener = tarpc::serde_transport::tcp::listen(&addr, Bincode::default).await?;
        info!("Coordinator server listening on {}", addr);

        listener
            .filter_map(|r| futures::future::ready(r.ok()))
            .map(server::BaseChannel::with_defaults)
            .for_each_concurrent(10, |channel| {
                let server = self.clone();
                async move {
                    channel.execute(server.serve()).for_each(|_| async {}).await
                }
            })
            .await;

        Ok(())
    }
}

impl CoordinatorService for CoordinatorServer {
    #[instrument(skip(self, _ctx), fields(shard_id = ?info.id, address = %info.address))]
    async fn register(self, _ctx: Context, info: ShardInfo) -> RpcResult<ShardId> {
        info!("Registering shard at {}", info.address);
        match self.coordinator.register_shard(info).await {
            Ok(id) => {
                info!("Shard registered with ID {:?}", id);
                Ok(id)
            }
            Err(e) => {
                error!("Failed to register shard: {}", e);
                Err(RpcError::Internal(e.to_string()))
            }
        }
    }

    #[instrument(skip(self, _ctx), fields(shard_id = ?shard_id))]
    async fn unregister(self, _ctx: Context, shard_id: ShardId) -> RpcResult<()> {
        info!("Unregistering shard {:?}", shard_id);
        match self.coordinator.deregister_shard(shard_id).await {
            Ok(()) => {
                info!("Shard {:?} unregistered", shard_id);
                Ok(())
            }
            Err(DistributedError::ShardNotFound(_)) => {
                Err(RpcError::ShardNotFound(shard_id.as_u32()))
            }
            Err(e) => {
                error!("Failed to unregister shard: {}", e);
                Err(RpcError::Internal(e.to_string()))
            }
        }
    }

    #[instrument(skip(self, _ctx), fields(shard_id = ?shard_id, phase = %phase, tick = tick))]
    async fn phase_complete(
        self,
        _ctx: Context,
        shard_id: ShardId,
        phase: TickPhase,
        tick: u64,
    ) -> RpcResult<()> {
        debug!(
            "Shard {:?} completed phase {:?} for tick {}",
            shard_id, phase, tick
        );
        match self.coordinator.phase_complete(shard_id, phase, tick).await {
            Ok(()) => Ok(()),
            Err(DistributedError::BarrierFailed) => Err(RpcError::BarrierFailed),
            Err(DistributedError::PhaseTimeout(p)) => Err(RpcError::PhaseTimeout(p.to_string())),
            Err(e) => Err(RpcError::Internal(e.to_string())),
        }
    }

    #[instrument(skip(self, _ctx), fields(doc_id = ?doc_id))]
    async fn route_document(self, _ctx: Context, doc_id: DocumentId) -> ShardId {
        let shard = self.coordinator.route_document(&doc_id).await;
        debug!("Document {:?} routed to shard {:?}", doc_id, shard);
        shard
    }

    #[instrument(skip(self, _ctx), fields(node_id = ?node_id))]
    async fn route_node(self, _ctx: Context, node_id: NodeId) -> ShardId {
        // Use the hash ring to route based on node ID
        // Since node IDs are UUIDs, we can hash them the same way as documents
        let doc_id = DocumentId(node_id.0);
        let shard = self.coordinator.route_document(&doc_id).await;
        debug!("Node {:?} routed to shard {:?}", node_id, shard);
        shard
    }

    #[instrument(skip(self, _ctx), fields(term_count = terms.len()))]
    async fn get_global_df(
        self,
        _ctx: Context,
        terms: Vec<String>,
    ) -> RpcResult<HashMap<String, u64>> {
        debug!("Getting global DF for {} terms", terms.len());

        // The coordinator does not hold shard client connections, so it cannot
        // fan out to shards directly. Global DF computation is handled by the
        // DistributedQueryEngine which performs its own scatter-gather via the
        // ShardClientPool. This endpoint exists for future use cases where a
        // client might want pre-computed global DF from the coordinator.
        let global_df = HashMap::new();
        debug!("Returning {} global DF entries (scatter-gather handled by query engine)", global_df.len());
        Ok(global_df)
    }

    #[instrument(skip(self, _ctx), fields(shard_id = ?shard_id, phase = %phase, tick = tick))]
    async fn barrier_ready(
        self,
        _ctx: Context,
        shard_id: ShardId,
        phase: TickPhase,
        tick: u64,
    ) -> RpcResult<bool> {
        debug!(
            "Shard {:?} checking barrier for phase {:?}, tick {}",
            shard_id, phase, tick
        );

        // Signal completion and wait for all shards
        match self.coordinator.phase_complete(shard_id, phase, tick).await {
            Ok(()) => {
                // Phase completed successfully, barrier released
                debug!("Barrier released for phase {:?}", phase);
                Ok(true)
            }
            Err(DistributedError::BarrierFailed) => {
                debug!("Barrier not ready yet");
                Ok(false)
            }
            Err(e) => Err(RpcError::Internal(e.to_string())),
        }
    }

    #[instrument(skip(self, _ctx))]
    async fn current_tick(self, _ctx: Context) -> u64 {
        let tick = self.coordinator.current_tick();
        debug!("Current tick: {}", tick);
        tick
    }

    #[instrument(skip(self, _ctx))]
    async fn list_shards(self, _ctx: Context) -> Vec<ShardInfo> {
        let shards = self.coordinator.all_shards().await;
        debug!("Listed {} shards", shards.len());
        shards
    }

    #[instrument(skip(self, _ctx))]
    async fn start_tick(self, _ctx: Context) -> RpcResult<u64> {
        info!("Starting new tick");
        let new_tick = self.coordinator.advance_tick().await;
        info!("Started tick {}", new_tick);
        Ok(new_tick)
    }

    #[instrument(skip(self, _ctx))]
    async fn tick_status(self, _ctx: Context) -> RpcResult<TickStatus> {
        debug!("Getting tick status");
        let tick = self.coordinator.current_tick();
        let all_shards = self.coordinator.all_shards().await;
        let shard_ids: Vec<ShardId> = all_shards.iter().map(|s| s.id).collect();

        // The barrier's per-phase completion tracking is internal to the
        // coordinator and not directly queryable without blocking. We report
        // the current tick, registered shards, and whether any shards exist.
        // A tick is considered complete when tick > 0 and no shards are
        // actively processing (approximated by the barrier having been reset
        // for the current tick via advance_tick).
        let tick_complete = tick > 0 && shard_ids.is_empty();

        let status = TickStatus {
            tick,
            phase: TickPhase::Sense,
            completed_shards: vec![],
            pending_shards: shard_ids,
            tick_complete,
        };

        debug!(
            "Tick status: tick={}, shards={}, complete={}",
            status.tick,
            status.pending_shards.len(),
            status.tick_complete,
        );
        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::ConsistentHashRing;
    use phago_core::types::Position;
    use phago_runtime::colony::ColonyConfig;

    fn create_test_shard() -> Arc<RwLock<ShardedColony>> {
        let hash_ring = Arc::new(RwLock::new(ConsistentHashRing::new(3)));
        Arc::new(RwLock::new(ShardedColony::new(
            ShardId::new(0),
            ColonyConfig::default(),
            hash_ring,
        )))
    }

    fn create_test_coordinator() -> Arc<Coordinator> {
        Arc::new(Coordinator::new(3))
    }

    #[tokio::test]
    async fn test_shard_server_health_check() {
        let shard = create_test_shard();
        let server = ShardServer::new(shard);

        let ctx = tarpc::context::current();
        let health = server.health_check(ctx).await.unwrap();

        assert!(health.healthy);
        assert_eq!(health.shard_id, ShardId::new(0));
    }

    #[tokio::test]
    async fn test_shard_server_ingest_document() {
        let shard = create_test_shard();
        let server = ShardServer::new(shard.clone());

        let doc = Document {
            id: DocumentId::new(),
            title: "Test".to_string(),
            content: "Test content".to_string(),
            position: Position::new(0.0, 0.0),
            digested: false,
        };

        let ctx = tarpc::context::current();
        let doc_id = server.ingest_document(ctx, doc).await.unwrap();

        assert!(!doc_id.0.is_nil());

        // Verify document was ingested
        let shard_guard = shard.read().await;
        assert_eq!(shard_guard.document_count(), 1);
    }

    #[tokio::test]
    async fn test_shard_server_tick_phase() {
        let shard = create_test_shard();
        let server = ShardServer::new(shard);

        let ctx = tarpc::context::current();
        let result = server.tick_phase(ctx, TickPhase::Sense, 0).await.unwrap();

        assert_eq!(result.shard_id, ShardId::new(0));
        assert_eq!(result.phase, TickPhase::Sense);
    }

    #[tokio::test]
    async fn test_shard_server_local_query() {
        let shard = create_test_shard();
        let server = ShardServer::new(shard);

        let req = LocalQueryRequest {
            query_terms: vec!["test".to_string()],
            max_results: 10,
            global_df: HashMap::new(),
        };

        let ctx = tarpc::context::current();
        let result = server.local_query(ctx, req).await.unwrap();

        assert_eq!(result.shard_id, ShardId::new(0));
        assert!(result.results.is_empty()); // No documents yet
    }

    #[tokio::test]
    async fn test_shard_server_get_term_frequencies() {
        let shard = create_test_shard();
        let server = ShardServer::new(shard);

        let ctx = tarpc::context::current();
        let freqs = server
            .get_term_frequencies(ctx, vec!["test".to_string()])
            .await
            .unwrap();

        assert!(freqs.is_empty()); // No nodes yet
    }

    #[tokio::test]
    async fn test_shard_server_get_node_not_found() {
        let shard = create_test_shard();
        let server = ShardServer::new(shard);

        let ctx = tarpc::context::current();
        let node = server.get_node(ctx, NodeId::from_seed(999)).await.unwrap();

        assert!(node.is_none());
    }

    #[tokio::test]
    async fn test_shard_server_resolve_ghost_nodes_empty() {
        let shard = create_test_shard();
        let server = ShardServer::new(shard);

        let ctx = tarpc::context::current();
        let ghosts = server
            .resolve_ghost_nodes(ctx, vec![NodeId::from_seed(1), NodeId::from_seed(2)])
            .await
            .unwrap();

        assert!(ghosts.is_empty()); // No matching nodes
    }

    #[tokio::test]
    async fn test_shard_server_get_neighbors_empty() {
        let shard = create_test_shard();
        let server = ShardServer::new(shard);

        let ctx = tarpc::context::current();
        let neighbors = server
            .get_neighbors(ctx, NodeId::from_seed(1))
            .await
            .unwrap();

        assert!(neighbors.is_empty());
    }

    #[tokio::test]
    async fn test_shard_server_receive_signals() {
        let shard = create_test_shard();
        let server = ShardServer::new(shard);

        let signals = vec![CrossShardSignal {
            signal_type: phago_core::types::SignalType::Input,
            intensity: 0.5,
            position: Position::new(0.0, 0.0),
            emitter: phago_core::types::AgentId::from_seed(1),
            tick: 0,
            source_shard: ShardId::new(1),
        }];

        let ctx = tarpc::context::current();
        let result = server.receive_signals(ctx, signals).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_coordinator_server_register() {
        let coordinator = create_test_coordinator();
        let server = CoordinatorServer::new(coordinator.clone());

        let info = ShardInfo::new(ShardId::new(0), "127.0.0.1:8080".to_string());

        let ctx = tarpc::context::current();
        let shard_id = server.register(ctx, info).await.unwrap();

        assert_eq!(shard_id, ShardId::new(0));

        // Verify registration
        let shards = coordinator.all_shards().await;
        assert_eq!(shards.len(), 1);
    }

    #[tokio::test]
    async fn test_coordinator_server_unregister() {
        let coordinator = create_test_coordinator();
        let server = CoordinatorServer::new(coordinator.clone());

        // Register first
        let info = ShardInfo::new(ShardId::new(0), "127.0.0.1:8080".to_string());
        let ctx = tarpc::context::current();
        let shard_id = server.clone().register(ctx, info).await.unwrap();

        // Now unregister
        let ctx = tarpc::context::current();
        let result = server.unregister(ctx, shard_id).await;

        assert!(result.is_ok());

        // Verify unregistration
        let shards = coordinator.all_shards().await;
        assert!(shards.is_empty());
    }

    #[tokio::test]
    async fn test_coordinator_server_unregister_not_found() {
        let coordinator = create_test_coordinator();
        let server = CoordinatorServer::new(coordinator);

        let ctx = tarpc::context::current();
        let result = server.unregister(ctx, ShardId::new(999)).await;

        assert!(matches!(result, Err(RpcError::ShardNotFound(999))));
    }

    #[tokio::test]
    async fn test_coordinator_server_route_document() {
        let coordinator = create_test_coordinator();
        let server = CoordinatorServer::new(coordinator);

        let doc_id = DocumentId::from_seed(42);

        let ctx = tarpc::context::current();
        let shard1 = server.clone().route_document(ctx, doc_id).await;

        let ctx = tarpc::context::current();
        let shard2 = server.route_document(ctx, doc_id).await;

        // Routing should be consistent
        assert_eq!(shard1, shard2);
    }

    #[tokio::test]
    async fn test_coordinator_server_current_tick() {
        let coordinator = create_test_coordinator();
        let server = CoordinatorServer::new(coordinator);

        let ctx = tarpc::context::current();
        let tick = server.current_tick(ctx).await;

        assert_eq!(tick, 0);
    }

    #[tokio::test]
    async fn test_coordinator_server_start_tick() {
        let coordinator = create_test_coordinator();
        let server = CoordinatorServer::new(coordinator.clone());

        let ctx = tarpc::context::current();
        let tick1 = server.clone().start_tick(ctx).await.unwrap();
        assert_eq!(tick1, 1);

        let ctx = tarpc::context::current();
        let tick2 = server.start_tick(ctx).await.unwrap();
        assert_eq!(tick2, 2);

        assert_eq!(coordinator.current_tick(), 2);
    }

    #[tokio::test]
    async fn test_coordinator_server_list_shards() {
        let coordinator = create_test_coordinator();
        let server = CoordinatorServer::new(coordinator);

        // Register some shards
        let ctx = tarpc::context::current();
        server
            .clone()
            .register(
                ctx,
                ShardInfo::new(ShardId::new(0), "127.0.0.1:8080".to_string()),
            )
            .await
            .unwrap();

        let ctx = tarpc::context::current();
        server
            .clone()
            .register(
                ctx,
                ShardInfo::new(ShardId::new(0), "127.0.0.1:8081".to_string()),
            )
            .await
            .unwrap();

        let ctx = tarpc::context::current();
        let shards = server.list_shards(ctx).await;

        assert_eq!(shards.len(), 2);
    }

    #[tokio::test]
    async fn test_coordinator_server_tick_status() {
        let coordinator = create_test_coordinator();
        let server = CoordinatorServer::new(coordinator);

        let ctx = tarpc::context::current();
        let status = server.tick_status(ctx).await.unwrap();

        assert_eq!(status.tick, 0);
        assert!(!status.tick_complete);
    }

    #[tokio::test]
    async fn test_coordinator_server_get_global_df() {
        let coordinator = create_test_coordinator();
        let server = CoordinatorServer::new(coordinator);

        let ctx = tarpc::context::current();
        let df = server
            .get_global_df(ctx, vec!["test".to_string()])
            .await
            .unwrap();

        // Empty since no shards are connected for fan-out
        assert!(df.is_empty());
    }
}
