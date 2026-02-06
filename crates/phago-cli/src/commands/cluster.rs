//! Distributed cluster CLI commands.
//!
//! These commands manage a distributed Phago colony cluster including
//! starting coordinators, starting shards, querying cluster status,
//! and running benchmarks.

use anyhow::Result;
use colored::Colorize;

/// Start a coordinator node.
pub fn start_coordinator(port: u16, num_shards: u32) -> Result<()> {
    println!(
        "{} Starting coordinator on port {} for {} shards...",
        "cluster".green().bold(),
        port,
        num_shards
    );

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        use phago_distributed::coordinator::Coordinator;
        use phago_distributed::rpc::server::CoordinatorServer;
        use std::sync::Arc;

        let coordinator = Arc::new(Coordinator::new(num_shards));
        let server = CoordinatorServer::new(coordinator);

        let addr: std::net::SocketAddr = format!("0.0.0.0:{}", port).parse()?;
        println!(
            "{} Coordinator listening on {}",
            "ready".green().bold(),
            addr
        );

        server.start(addr).await?;
        Ok(())
    })
}

/// Start a shard node and register with the coordinator.
pub fn start_shard(
    shard_port: u16,
    coordinator_addr: &str,
    shard_id: u32,
) -> Result<()> {
    println!(
        "{} Starting shard {} on port {}, coordinator at {}...",
        "cluster".green().bold(),
        shard_id,
        shard_port,
        coordinator_addr
    );

    let coordinator_addr = coordinator_addr.to_string();
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        use phago_distributed::hashing::ConsistentHashRing;
        use phago_distributed::rpc::client::connect_to_coordinator;
        use phago_distributed::rpc::server::ShardServer;
        use phago_distributed::shard::ShardedColony;
        use phago_distributed::types::{ShardId, ShardInfo};
        use phago_runtime::colony::ColonyConfig;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        // Connect to coordinator and register
        let coord_addr: std::net::SocketAddr = coordinator_addr.parse()?;
        let coord_client = connect_to_coordinator(coord_addr).await?;

        let info = ShardInfo::new(
            ShardId::new(shard_id),
            format!("127.0.0.1:{}", shard_port),
        );

        let registered_id = coord_client
            .register(tarpc::context::current(), info)
            .await??;
        println!(
            "{} Registered as shard {:?}",
            "ready".green().bold(),
            registered_id
        );

        // Create the shard
        let hash_ring = Arc::new(RwLock::new(ConsistentHashRing::new(3)));
        let shard = Arc::new(RwLock::new(ShardedColony::new(
            registered_id,
            ColonyConfig::default(),
            hash_ring,
        )));

        // Start serving
        let server = ShardServer::new(shard);
        let addr: std::net::SocketAddr = format!("0.0.0.0:{}", shard_port).parse()?;
        println!(
            "{} Shard {} listening on {}",
            "ready".green().bold(),
            shard_id,
            addr
        );

        server.start(addr).await?;
        Ok(())
    })
}

/// Query cluster status from the coordinator.
pub fn status(coordinator_addr: &str) -> Result<()> {
    let coordinator_addr = coordinator_addr.to_string();
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        use phago_distributed::rpc::client::connect_to_coordinator;

        let addr: std::net::SocketAddr = coordinator_addr.parse()?;
        let client = connect_to_coordinator(addr).await?;

        let ctx = tarpc::context::current();
        let shards = client.list_shards(ctx).await?;

        let ctx = tarpc::context::current();
        let tick = client.current_tick(ctx).await?;

        println!("{}", "Cluster Status".green().bold());
        println!("  Coordinator: {}", coordinator_addr);
        println!("  Current tick: {}", tick);
        println!("  Shards: {}", shards.len());

        for shard in &shards {
            println!(
                "    {} -- {} (nodes: {}, edges: {}, docs: {})",
                format!("{}", shard.id).cyan(),
                shard.address,
                shard.node_count,
                shard.edge_count,
                shard.document_count,
            );
        }

        if shards.is_empty() {
            println!("    (no shards registered)");
        }

        Ok(())
    })
}

/// Run distributed benchmarks.
pub fn bench(mode: &str) -> Result<()> {
    println!(
        "{} Running distributed benchmark ({})...",
        "bench".green().bold(),
        mode
    );

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        match mode {
            "quick" => {
                let results = phago_distributed::run_quick_benchmark().await;
                results.print_summary();
            }
            "full" => {
                let config = phago_distributed::BenchConfig::new()
                    .with_shards(3)
                    .with_documents(100)
                    .with_ticks(20);
                let results = phago_distributed::run_benchmark(config).await;
                results.print_summary();
            }
            "scaling" => {
                let results =
                    phago_distributed::scaling_benchmark(100, 10).await;
                phago_distributed::print_scaling_results(&results);
            }
            other => {
                anyhow::bail!(
                    "Unknown benchmark mode: {}. Use 'quick', 'full', or 'scaling'.",
                    other
                );
            }
        }
        Ok(())
    })
}
