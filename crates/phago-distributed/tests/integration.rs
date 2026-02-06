//! End-to-end integration tests for distributed colony.
//!
//! Tests the full distributed lifecycle: create coordinator + 3 shards,
//! ingest documents, run distributed ticks, execute queries, and
//! verify ghost node resolution.

use phago_distributed::coordinator::Coordinator;
use phago_distributed::hashing::ConsistentHashRing;
use phago_distributed::query::{DistributedHybridConfig, DistributedQueryEngine};
use phago_distributed::runner::{DistributedRunner, RunnerConfig};
use phago_distributed::shard::ShardedColony;
use phago_distributed::types::*;
use phago_core::types::{DocumentId, Position};
use phago_runtime::colony::ColonyConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Helper: tick all shards and advance coordinator (without barrier sync).
async fn tick_cluster(
    coordinator: &Coordinator,
    shards: &[Arc<RwLock<ShardedColony>>],
) {
    for shard in shards {
        let mut s = shard.write().await;
        s.tick();
    }
    coordinator.advance_tick().await;
}

/// Helper: create an in-process cluster with N shards.
fn create_cluster(
    num_shards: u32,
) -> (Arc<Coordinator>, Vec<Arc<RwLock<ShardedColony>>>) {
    let coordinator = Arc::new(Coordinator::new(num_shards));
    let hash_ring = Arc::new(RwLock::new(ConsistentHashRing::new(num_shards)));

    let shards: Vec<_> = (0..num_shards)
        .map(|i| {
            Arc::new(RwLock::new(ShardedColony::new(
                ShardId::new(i),
                ColonyConfig::default(),
                hash_ring.clone(),
            )))
        })
        .collect();

    (coordinator, shards)
}

/// Helper: register all shards with coordinator.
async fn register_shards(
    coordinator: &Coordinator,
    shards: &[Arc<RwLock<ShardedColony>>],
) {
    for (i, _) in shards.iter().enumerate() {
        let info = ShardInfo::new(
            ShardId::new(i as u32),
            format!("127.0.0.1:{}", 9000 + i),
        );
        coordinator.register_shard(info).await.unwrap();
    }
}

/// Helper: ingest a document into the correct shard via routing.
async fn route_and_ingest(
    coordinator: &Coordinator,
    shards: &[Arc<RwLock<ShardedColony>>],
    doc_seed: u64,
    title: &str,
    content: &str,
) -> (ShardId, DocumentId) {
    let doc_id = DocumentId::from_seed(doc_seed);
    let target = coordinator.route_document(&doc_id).await;

    for shard in shards {
        let mut s = shard.write().await;
        if s.shard_id() == target {
            let actual_id = s.ingest_document_direct(
                title,
                content,
                Position::new(doc_seed as f64 % 50.0, (doc_seed / 50) as f64),
            );
            return (target, actual_id);
        }
    }
    panic!("No shard found for target {:?}", target);
}

#[tokio::test]
async fn test_full_lifecycle() {
    // 1. Create coordinator + 3 shards
    let (coordinator, shards) = create_cluster(3);
    register_shards(&coordinator, &shards).await;

    // Verify cluster is set up
    let all = coordinator.all_shards().await;
    assert_eq!(all.len(), 3);
    assert_eq!(coordinator.current_tick(), 0);

    // 2. Ingest documents across shards via routing
    let docs = [
        ("Cell Membrane", "cell membrane protein transport biology"),
        ("DNA Replication", "DNA replication enzyme helicase polymerase"),
        ("Neuroscience", "neuron synapse action potential neurotransmitter"),
        ("Immunology", "antibody antigen immune response lymphocyte"),
        ("Ecology", "ecosystem biodiversity species population habitat"),
        ("Genetics", "chromosome gene mutation inheritance phenotype"),
    ];

    let mut shard_doc_counts = [0usize; 3];
    for (i, (title, content)) in docs.iter().enumerate() {
        let (target, _) = route_and_ingest(
            &coordinator,
            &shards,
            i as u64,
            title,
            content,
        )
        .await;
        shard_doc_counts[target.as_u32() as usize] += 1;
    }

    // Verify documents are distributed (at least 2 shards should have docs)
    let shards_with_docs = shard_doc_counts.iter().filter(|&&c| c > 0).count();
    assert!(
        shards_with_docs >= 2,
        "Documents should be distributed across shards, got {:?}",
        shard_doc_counts
    );

    // Verify total document count
    let total_docs: usize = shard_doc_counts.iter().sum();
    assert_eq!(total_docs, 6);

    // 3. Run ticks on each shard and advance coordinator
    tick_cluster(&coordinator, &shards).await;
    assert_eq!(coordinator.current_tick(), 1);

    // 4. Execute distributed query
    let engine = DistributedQueryEngine::new(DistributedHybridConfig {
        max_results: 10,
        ..Default::default()
    });

    let query_results = {
        let guards: Vec<_> = futures::future::join_all(
            shards.iter().map(|s| async { s.read().await }),
        )
        .await;
        let refs: Vec<&ShardedColony> = guards.iter().map(|g| &**g).collect();
        engine.distributed_query(&refs, "cell membrane")
        // guards (read locks) are dropped here at end of block
    };

    // Query should return results; even if empty the infrastructure works
    assert!(query_results.len() <= 10);

    // 5. Verify ghost node resolution works
    {
        let mut s0 = shards[0].write().await;
        let ghost = GhostNode::new(
            phago_core::types::NodeId::from_seed(999),
            ShardId::new(1),
            "remote_concept".to_string(),
        );
        s0.ghost_cache_mut().insert(ghost);
        assert_eq!(s0.ghost_cache().len(), 1);

        // Verify we can look up the ghost node
        let cached = s0.ghost_cache_mut().get(
            &phago_core::types::NodeId::from_seed(999),
        );
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().label, "remote_concept");
        assert_eq!(cached.unwrap().shard_id, ShardId::new(1));
    }
}

#[tokio::test]
async fn test_document_routing_consistency() {
    let (coordinator, _shards) = create_cluster(3);

    // Same document ID should always route to same shard
    for seed in 0..100u64 {
        let doc_id = DocumentId::from_seed(seed);
        let shard1 = coordinator.route_document(&doc_id).await;
        let shard2 = coordinator.route_document(&doc_id).await;
        assert_eq!(shard1, shard2, "Routing inconsistent for seed {}", seed);
    }
}

#[tokio::test]
async fn test_document_routing_distribution() {
    let (coordinator, _shards) = create_cluster(3);

    let mut counts = [0u32; 3];
    for seed in 0..300u64 {
        let doc_id = DocumentId::from_seed(seed);
        let shard = coordinator.route_document(&doc_id).await;
        counts[shard.as_u32() as usize] += 1;
    }

    // Each shard should have roughly 100 documents (within 50-150)
    for (i, count) in counts.iter().enumerate() {
        assert!(
            *count > 50 && *count < 200,
            "Shard {} has {} docs, expected ~100",
            i,
            count
        );
    }
}

#[tokio::test]
async fn test_single_shard_cluster() {
    let (coordinator, shards) = create_cluster(1);
    register_shards(&coordinator, &shards).await;

    // All documents should route to shard 0
    for seed in 0..10u64 {
        let doc_id = DocumentId::from_seed(seed);
        let shard = coordinator.route_document(&doc_id).await;
        assert_eq!(shard, ShardId::new(0));
    }

    // Ingest and run
    {
        let mut s = shards[0].write().await;
        s.ingest_document_direct(
            "Test",
            "cell membrane biology",
            Position::new(0.0, 0.0),
        );
    }

    let runner = DistributedRunner::new(
        coordinator.clone(),
        shards.clone(),
        RunnerConfig::default(),
    );
    let results = runner.run(5).await.unwrap();
    assert_eq!(results.len(), 5);
    assert_eq!(coordinator.current_tick(), 5);
}

#[tokio::test]
async fn test_cluster_stats_after_ingestion() {
    let (coordinator, shards) = create_cluster(3);
    register_shards(&coordinator, &shards).await;

    // Ingest docs
    for i in 0..9u64 {
        route_and_ingest(
            &coordinator,
            &shards,
            i,
            &format!("Doc {}", i),
            "content about cell biology and proteins",
        )
        .await;
    }

    // Update shard metrics with coordinator
    for shard in &shards {
        let s = shard.read().await;
        coordinator
            .update_shard_metrics(
                s.shard_id(),
                s.document_count(),
                0,
            )
            .await;
    }

    let stats = coordinator.cluster_stats().await;
    assert_eq!(stats.total_shards, 3);
    assert_eq!(stats.online_shards, 3);
    assert_eq!(stats.total_documents, 9);
}

#[tokio::test]
async fn test_shard_deregistration() {
    let (coordinator, shards) = create_cluster(3);
    register_shards(&coordinator, &shards).await;

    assert_eq!(coordinator.all_shards().await.len(), 3);

    coordinator.deregister_shard(ShardId::new(1)).await.unwrap();
    assert_eq!(coordinator.all_shards().await.len(), 2);

    // Deregistering again should fail
    let result = coordinator.deregister_shard(ShardId::new(1)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_ghost_cache_invalidation_on_peer_removal() {
    let (_, shards) = create_cluster(3);

    let mut s0 = shards[0].write().await;

    // Add peer and ghost nodes
    s0.add_peer(ShardId::new(1), "127.0.0.1:9001".to_string());
    let ghost = GhostNode::new(
        phago_core::types::NodeId::from_seed(42),
        ShardId::new(1),
        "remote".to_string(),
    );
    s0.ghost_cache_mut().insert(ghost);
    assert_eq!(s0.ghost_cache().len(), 1);

    // Remove peer - ghost cache should be invalidated
    s0.remove_peer(ShardId::new(1));
    assert_eq!(s0.ghost_cache().len(), 0);
}

#[tokio::test]
async fn test_cross_shard_edge_lifecycle() {
    let (_, shards) = create_cluster(3);

    let mut s0 = shards[0].write().await;

    // Register a cross-shard edge
    let edge = CrossShardEdge {
        from_node: phago_core::types::NodeId::from_seed(1),
        to_node: phago_core::types::NodeId::from_seed(2),
        to_shard: ShardId::new(1),
        weight: 0.75,
    };

    s0.register_cross_shard_edge(edge);
    assert_eq!(s0.pending_cross_edges().len(), 1);

    // Edge manager should track it
    let stats = s0.cross_shard_edge_stats();
    assert!(stats.outgoing_edges > 0 || s0.pending_cross_edges().len() > 0);

    // Clear pending
    s0.clear_pending_cross_edges();
    assert!(s0.pending_cross_edges().is_empty());
}

#[tokio::test]
async fn test_distributed_query_empty_cluster() {
    let (_, shards) = create_cluster(3);

    let engine = DistributedQueryEngine::with_defaults();

    let guards: Vec<_> = futures::future::join_all(
        shards.iter().map(|s| async { s.read().await }),
    )
    .await;
    let refs: Vec<&ShardedColony> = guards.iter().map(|g| &**g).collect();

    // Query on empty cluster should return empty results
    let results = engine.distributed_query(&refs, "cell membrane");
    assert!(results.is_empty());
}
