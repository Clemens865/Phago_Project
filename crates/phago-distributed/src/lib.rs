//! # Phago Distributed
//!
//! Distributed colony implementation enabling horizontal scaling across
//! multiple processes with consistent hashing, cross-shard queries,
//! and fault tolerance.

pub mod bench;
pub mod coordinator;
pub mod hashing;
pub mod query;
pub mod rpc;
pub mod runner;
pub mod shard;
pub mod types;

pub use bench::{
    compare_single_vs_distributed, create_bench_cluster, generate_documents, print_scaling_results,
    run_benchmark, run_quick_benchmark, scaling_benchmark, BenchConfig, BenchResults,
};
pub use coordinator::{ClusterStats, Coordinator, RegisteredShard, ShardRegistry, TickBarrier};
pub use hashing::ConsistentHashRing;
pub use query::{merge_results, tokenize, DistributedHybridConfig, DistributedQueryEngine};
pub use runner::{DistributedRunner, DistributedTickResult, RunnerConfig};
pub use shard::{GhostCacheStats, GhostNodeCache, ShardedColony};
pub use types::*;
