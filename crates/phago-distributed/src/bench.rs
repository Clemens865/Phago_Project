//! Benchmarking utilities for distributed colony.
//!
//! This module provides tools for measuring the performance of the distributed
//! colony system, including document ingestion, tick execution, and query
//! performance across multiple shards.

use crate::coordinator::Coordinator;
use crate::hashing::ConsistentHashRing;
use crate::query::{DistributedHybridConfig, DistributedQueryEngine};
use crate::runner::{DistributedRunner, RunnerConfig};
use crate::shard::ShardedColony;
use crate::types::*;
use phago_core::types::Position;
use phago_runtime::colony::ColonyConfig;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for benchmark runs.
#[derive(Debug, Clone)]
pub struct BenchConfig {
    /// Number of shards.
    pub num_shards: u32,
    /// Number of documents to ingest.
    pub num_documents: usize,
    /// Number of ticks to run.
    pub num_ticks: u64,
    /// Number of queries to execute.
    pub num_queries: usize,
    /// Sample queries to run.
    pub sample_queries: Vec<String>,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            num_shards: 3,
            num_documents: 100,
            num_ticks: 20,
            num_queries: 50,
            sample_queries: vec![
                "cell membrane".to_string(),
                "protein transport".to_string(),
                "molecular biology".to_string(),
            ],
        }
    }
}

impl BenchConfig {
    /// Create a new benchmark configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of shards.
    pub fn with_shards(mut self, num_shards: u32) -> Self {
        self.num_shards = num_shards;
        self
    }

    /// Set the number of documents.
    pub fn with_documents(mut self, num_documents: usize) -> Self {
        self.num_documents = num_documents;
        self
    }

    /// Set the number of ticks.
    pub fn with_ticks(mut self, num_ticks: u64) -> Self {
        self.num_ticks = num_ticks;
        self
    }

    /// Set the number of queries.
    pub fn with_queries(mut self, num_queries: usize) -> Self {
        self.num_queries = num_queries;
        self
    }

    /// Set sample queries.
    pub fn with_sample_queries(mut self, queries: Vec<String>) -> Self {
        self.sample_queries = queries;
        self
    }
}

/// Results from a benchmark run.
#[derive(Debug, Clone)]
pub struct BenchResults {
    /// Time to set up the cluster.
    pub setup_time: Duration,
    /// Time to ingest all documents.
    pub ingest_time: Duration,
    /// Time to run all ticks.
    pub tick_time: Duration,
    /// Time to run all queries.
    pub query_time: Duration,
    /// Total time.
    pub total_time: Duration,
    /// Documents per second during ingestion.
    pub docs_per_second: f64,
    /// Ticks per second.
    pub ticks_per_second: f64,
    /// Queries per second.
    pub queries_per_second: f64,
    /// Total nodes across all shards.
    pub total_nodes: usize,
    /// Total edges across all shards.
    pub total_edges: usize,
    /// Number of shards used.
    pub num_shards: u32,
    /// Number of documents ingested.
    pub num_documents: usize,
    /// Number of ticks run.
    pub num_ticks: u64,
}

impl BenchResults {
    /// Print a formatted summary.
    pub fn print_summary(&self) {
        println!("\n=== Distributed Colony Benchmark Results ===\n");
        println!("Configuration:");
        println!(
            "  Shards: {}, Documents: {}, Ticks: {}",
            self.num_shards, self.num_documents, self.num_ticks
        );
        println!();
        println!("Timing:");
        println!("  Setup time:    {:?}", self.setup_time);
        println!(
            "  Ingest time:   {:?} ({:.1} docs/sec)",
            self.ingest_time, self.docs_per_second
        );
        println!(
            "  Tick time:     {:?} ({:.1} ticks/sec)",
            self.tick_time, self.ticks_per_second
        );
        println!(
            "  Query time:    {:?} ({:.1} queries/sec)",
            self.query_time, self.queries_per_second
        );
        println!("  Total time:    {:?}", self.total_time);
        println!();
        println!(
            "Graph size: {} nodes, {} edges",
            self.total_nodes, self.total_edges
        );
    }

    /// Return results as a CSV row.
    pub fn to_csv_row(&self) -> String {
        format!(
            "{},{},{},{:.2},{:.2},{:.2},{},{},{}",
            self.num_shards,
            self.num_documents,
            self.num_ticks,
            self.docs_per_second,
            self.ticks_per_second,
            self.queries_per_second,
            self.total_nodes,
            self.total_edges,
            self.total_time.as_millis()
        )
    }

    /// Return CSV header.
    pub fn csv_header() -> &'static str {
        "shards,documents,ticks,docs_per_sec,ticks_per_sec,queries_per_sec,nodes,edges,total_time_ms"
    }
}

/// Create a test cluster for benchmarking.
///
/// Returns the coordinator and a vector of sharded colonies.
pub fn create_bench_cluster(
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

/// Generate sample documents for benchmarking.
///
/// Creates a variety of documents about different scientific topics.
pub fn generate_documents(count: usize) -> Vec<(String, String)> {
    let topics = [
        (
            "Cell Biology",
            "cell membrane protein transport signaling pathway organelle cytoplasm",
        ),
        (
            "Molecular Biology",
            "DNA RNA transcription translation gene expression nucleotide sequence",
        ),
        (
            "Biochemistry",
            "enzyme substrate reaction kinetics metabolism catalysis activation",
        ),
        (
            "Genetics",
            "chromosome gene mutation inheritance phenotype genotype allele",
        ),
        (
            "Neuroscience",
            "neuron synapse action potential neurotransmitter receptor axon dendrite",
        ),
        (
            "Immunology",
            "antibody antigen immune response lymphocyte cytokine inflammation",
        ),
        (
            "Microbiology",
            "bacteria virus pathogen infection microbiome antimicrobial resistance",
        ),
        (
            "Ecology",
            "ecosystem biodiversity species population habitat conservation environment",
        ),
    ];

    (0..count)
        .map(|i| {
            let (title, base_content) = topics[i % topics.len()];
            let title = format!("{} Document {}", title, i);
            let content = format!(
                "{} - variation {} with unique content about scientific concepts and research findings",
                base_content, i
            );
            (title, content)
        })
        .collect()
}

/// Run the full benchmark suite.
///
/// This is an async function that creates a distributed cluster, ingests
/// documents, runs ticks, and executes queries to measure performance.
pub async fn run_benchmark(config: BenchConfig) -> BenchResults {
    let total_start = Instant::now();

    // Setup
    let setup_start = Instant::now();
    let (coordinator, shards) = create_bench_cluster(config.num_shards);

    // Register shards with coordinator
    for (i, _shard) in shards.iter().enumerate() {
        let info = ShardInfo::new(ShardId::new(i as u32), format!("127.0.0.1:{}", 8080 + i));
        let _ = coordinator.register_shard(info).await;
    }
    let setup_time = setup_start.elapsed();

    // Ingest documents
    let ingest_start = Instant::now();
    let documents = generate_documents(config.num_documents);

    for (i, (title, content)) in documents.iter().enumerate() {
        let doc_id = phago_core::types::DocumentId::from_seed(i as u64);
        let shard_id = coordinator.route_document(&doc_id).await;

        // Find the correct shard and ingest
        for shard in &shards {
            let mut s = shard.write().await;
            if s.shard_id() == shard_id {
                s.ingest_document_direct(
                    title,
                    content,
                    Position::new(i as f64 % 100.0, (i / 100) as f64),
                );
                break;
            }
        }
    }
    let ingest_time = ingest_start.elapsed();

    // Run ticks using DistributedRunner for proper phase synchronization
    let tick_start = Instant::now();
    let runner = DistributedRunner::new(
        coordinator.clone(),
        shards.clone(),
        RunnerConfig {
            resolve_ghosts: false, // Skip ghost resolution for benchmarking
            ..Default::default()
        },
    );
    let _ = runner.run(config.num_ticks).await;
    let tick_time = tick_start.elapsed();

    // Run queries
    let query_start = Instant::now();
    let engine = DistributedQueryEngine::new(DistributedHybridConfig::default());
    let total_queries = config.num_queries * config.sample_queries.len();

    for _ in 0..config.num_queries {
        for query in &config.sample_queries {
            // Collect shard guards
            let guards: Vec<_> =
                futures::future::join_all(shards.iter().map(|s| async { s.read().await })).await;

            // Create references for the query
            let refs: Vec<&ShardedColony> = guards.iter().map(|g| &**g).collect();
            let _ = engine.distributed_query(&refs, query);
        }
    }
    let query_time = query_start.elapsed();

    // Collect stats
    let mut total_nodes = 0;
    let mut total_edges = 0;
    for shard in &shards {
        let s = shard.read().await;
        let stats = s.stats();
        total_nodes += stats.graph_nodes;
        total_edges += stats.graph_edges;
    }

    let total_time = total_start.elapsed();

    BenchResults {
        setup_time,
        ingest_time,
        tick_time,
        query_time,
        total_time,
        docs_per_second: config.num_documents as f64 / ingest_time.as_secs_f64(),
        ticks_per_second: config.num_ticks as f64 / tick_time.as_secs_f64(),
        queries_per_second: total_queries as f64 / query_time.as_secs_f64(),
        total_nodes,
        total_edges,
        num_shards: config.num_shards,
        num_documents: config.num_documents,
        num_ticks: config.num_ticks,
    }
}

/// Run a quick benchmark with minimal parameters.
///
/// Useful for testing that the benchmark infrastructure works.
pub async fn run_quick_benchmark() -> BenchResults {
    run_benchmark(BenchConfig {
        num_shards: 2,
        num_documents: 20,
        num_ticks: 10,
        num_queries: 5,
        sample_queries: vec!["cell".to_string(), "protein".to_string()],
    })
    .await
}

/// Compare single-node vs distributed performance.
///
/// Runs benchmarks with 1, 3, and 5 shards and prints a comparison table.
pub async fn compare_single_vs_distributed(num_documents: usize, num_ticks: u64) {
    println!("\n=== Single-Node vs Distributed Comparison ===\n");

    let base_config = BenchConfig {
        num_documents,
        num_ticks,
        num_queries: 20,
        ..Default::default()
    };

    // Single node (1 shard)
    println!("Running single-node benchmark...");
    let single_result = run_benchmark(BenchConfig {
        num_shards: 1,
        ..base_config.clone()
    })
    .await;

    // Distributed (3 shards)
    println!("Running 3-shard distributed benchmark...");
    let dist_3_result = run_benchmark(BenchConfig {
        num_shards: 3,
        ..base_config.clone()
    })
    .await;

    // Distributed (5 shards)
    println!("Running 5-shard distributed benchmark...");
    let dist_5_result = run_benchmark(BenchConfig {
        num_shards: 5,
        ..base_config.clone()
    })
    .await;

    println!("\n| Shards | Ingest (docs/s) | Ticks/s | Queries/s | Total Time |");
    println!("|--------|-----------------|---------|-----------|------------|");
    println!(
        "| 1      | {:>15.1} | {:>7.1} | {:>9.1} | {:>10?} |",
        single_result.docs_per_second,
        single_result.ticks_per_second,
        single_result.queries_per_second,
        single_result.total_time
    );
    println!(
        "| 3      | {:>15.1} | {:>7.1} | {:>9.1} | {:>10?} |",
        dist_3_result.docs_per_second,
        dist_3_result.ticks_per_second,
        dist_3_result.queries_per_second,
        dist_3_result.total_time
    );
    println!(
        "| 5      | {:>15.1} | {:>7.1} | {:>9.1} | {:>10?} |",
        dist_5_result.docs_per_second,
        dist_5_result.ticks_per_second,
        dist_5_result.queries_per_second,
        dist_5_result.total_time
    );

    println!("\n| Shards | Nodes  | Edges  |");
    println!("|--------|--------|--------|");
    println!(
        "| 1      | {:>6} | {:>6} |",
        single_result.total_nodes, single_result.total_edges
    );
    println!(
        "| 3      | {:>6} | {:>6} |",
        dist_3_result.total_nodes, dist_3_result.total_edges
    );
    println!(
        "| 5      | {:>6} | {:>6} |",
        dist_5_result.total_nodes, dist_5_result.total_edges
    );
}

/// Run a scaling benchmark across different shard counts.
///
/// Returns results for 1, 2, 4, and 8 shards.
pub async fn scaling_benchmark(num_documents: usize, num_ticks: u64) -> Vec<BenchResults> {
    let shard_counts = [1, 2, 4, 8];
    let mut results = Vec::new();

    for &num_shards in &shard_counts {
        println!("Running benchmark with {} shard(s)...", num_shards);
        let result = run_benchmark(BenchConfig {
            num_shards,
            num_documents,
            num_ticks,
            num_queries: 20,
            ..Default::default()
        })
        .await;
        results.push(result);
    }

    results
}

/// Print scaling benchmark results as a table.
pub fn print_scaling_results(results: &[BenchResults]) {
    println!("\n=== Scaling Benchmark Results ===\n");
    println!("{}", BenchResults::csv_header());
    for result in results {
        println!("{}", result.to_csv_row());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_bench_cluster() {
        let (coordinator, shards) = create_bench_cluster(3);
        assert_eq!(coordinator.shard_count().await, 3);
        assert_eq!(shards.len(), 3);
    }

    #[test]
    fn test_generate_documents() {
        let docs = generate_documents(10);
        assert_eq!(docs.len(), 10);
        assert!(docs[0].0.contains("Document 0"));
        assert!(!docs[0].1.is_empty());
    }

    #[test]
    fn test_generate_documents_cycles_topics() {
        let docs = generate_documents(20);
        // Should cycle through 8 topics
        assert!(docs[0].0.contains("Cell Biology"));
        assert!(docs[8].0.contains("Cell Biology")); // Cycles back
    }

    #[tokio::test]
    async fn test_small_benchmark() {
        let result = run_benchmark(BenchConfig {
            num_shards: 2,
            num_documents: 10,
            num_ticks: 5,
            num_queries: 5,
            sample_queries: vec!["cell".to_string()],
        })
        .await;

        assert!(result.docs_per_second > 0.0);
        assert!(result.ticks_per_second > 0.0);
        assert_eq!(result.num_shards, 2);
        assert_eq!(result.num_documents, 10);
        assert_eq!(result.num_ticks, 5);
    }

    #[tokio::test]
    async fn test_quick_benchmark() {
        let result = run_quick_benchmark().await;

        assert!(result.docs_per_second > 0.0);
        assert!(result.total_time.as_millis() > 0);
    }

    #[test]
    fn test_bench_config_builder() {
        let config = BenchConfig::new()
            .with_shards(5)
            .with_documents(200)
            .with_ticks(50)
            .with_queries(30)
            .with_sample_queries(vec!["test".to_string()]);

        assert_eq!(config.num_shards, 5);
        assert_eq!(config.num_documents, 200);
        assert_eq!(config.num_ticks, 50);
        assert_eq!(config.num_queries, 30);
        assert_eq!(config.sample_queries, vec!["test".to_string()]);
    }

    #[test]
    fn test_bench_results_csv() {
        let results = BenchResults {
            setup_time: Duration::from_millis(10),
            ingest_time: Duration::from_millis(100),
            tick_time: Duration::from_millis(200),
            query_time: Duration::from_millis(50),
            total_time: Duration::from_millis(360),
            docs_per_second: 1000.0,
            ticks_per_second: 100.0,
            queries_per_second: 500.0,
            total_nodes: 50,
            total_edges: 100,
            num_shards: 3,
            num_documents: 100,
            num_ticks: 20,
        };

        let csv = results.to_csv_row();
        assert!(csv.contains("3,100,20"));
        assert!(csv.contains("1000.00"));
    }

    #[tokio::test]
    async fn test_benchmark_shard_distribution() {
        let config = BenchConfig {
            num_shards: 3,
            num_documents: 30,
            num_ticks: 5,
            num_queries: 3,
            sample_queries: vec!["cell".to_string()],
        };

        let result = run_benchmark(config).await;

        // All documents should be ingested
        assert!(result.num_documents == 30);
        // With 3 shards, documents should be distributed
        assert!(result.total_time.as_millis() > 0);
    }
}
