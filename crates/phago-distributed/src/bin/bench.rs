//! Benchmark runner binary for phago-distributed.
//!
//! Run with: cargo run --bin phago-bench --release

use phago_distributed::bench::{
    compare_single_vs_distributed, run_benchmark, scaling_benchmark, BenchConfig,
};
use std::env;

fn print_usage() {
    println!("Phago Distributed Colony Benchmarks");
    println!();
    println!("Usage: phago-bench [command] [options]");
    println!();
    println!("Commands:");
    println!("  quick       Run a quick benchmark with default settings");
    println!("  full        Run a full benchmark suite");
    println!("  compare     Compare single-node vs distributed performance");
    println!("  scale       Run scaling benchmark across shard counts");
    println!("  custom      Run with custom parameters");
    println!();
    println!("Options for 'custom':");
    println!("  --shards N     Number of shards (default: 3)");
    println!("  --docs N       Number of documents (default: 100)");
    println!("  --ticks N      Number of ticks (default: 20)");
    println!("  --queries N    Number of queries (default: 50)");
    println!();
    println!("Examples:");
    println!("  phago-bench quick");
    println!("  phago-bench compare");
    println!("  phago-bench custom --shards 5 --docs 200 --ticks 50");
}

fn parse_custom_args(args: &[String]) -> BenchConfig {
    let mut config = BenchConfig::default();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--shards" => {
                if i + 1 < args.len() {
                    config.num_shards = args[i + 1].parse().unwrap_or(config.num_shards);
                    i += 1;
                }
            }
            "--docs" => {
                if i + 1 < args.len() {
                    config.num_documents = args[i + 1].parse().unwrap_or(config.num_documents);
                    i += 1;
                }
            }
            "--ticks" => {
                if i + 1 < args.len() {
                    config.num_ticks = args[i + 1].parse().unwrap_or(config.num_ticks);
                    i += 1;
                }
            }
            "--queries" => {
                if i + 1 < args.len() {
                    config.num_queries = args[i + 1].parse().unwrap_or(config.num_queries);
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    config
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "quick" => {
            println!("Running quick benchmark...\n");
            let result = run_benchmark(BenchConfig {
                num_shards: 3,
                num_documents: 50,
                num_ticks: 10,
                num_queries: 10,
                ..Default::default()
            })
            .await;
            result.print_summary();
        }

        "full" => {
            println!("Running full benchmark suite...\n");
            let result = run_benchmark(BenchConfig {
                num_shards: 3,
                num_documents: 100,
                num_ticks: 20,
                num_queries: 20,
                ..Default::default()
            })
            .await;
            result.print_summary();
        }

        "compare" => {
            compare_single_vs_distributed(100, 20).await;
        }

        "scale" => {
            println!("Running scaling benchmark...\n");
            let results = scaling_benchmark(100, 20).await;

            println!("\n=== Scaling Results ===\n");
            println!(
                "| {:>6} | {:>12} | {:>10} | {:>12} | {:>8} | {:>8} |",
                "Shards", "Docs/sec", "Ticks/sec", "Queries/sec", "Nodes", "Edges"
            );
            println!("|--------|--------------|------------|--------------|----------|----------|");
            for r in &results {
                println!(
                    "| {:>6} | {:>12.1} | {:>10.1} | {:>12.1} | {:>8} | {:>8} |",
                    r.num_shards,
                    r.docs_per_second,
                    r.ticks_per_second,
                    r.queries_per_second,
                    r.total_nodes,
                    r.total_edges
                );
            }
        }

        "custom" => {
            let config = parse_custom_args(&args[2..]);
            println!("Running custom benchmark...");
            println!(
                "  Shards: {}, Documents: {}, Ticks: {}, Queries: {}",
                config.num_shards, config.num_documents, config.num_ticks, config.num_queries
            );
            println!();

            let result = run_benchmark(config).await;
            result.print_summary();
        }

        "--help" | "-h" | "help" => {
            print_usage();
        }

        _ => {
            println!("Unknown command: {}", args[1]);
            println!();
            print_usage();
        }
    }
}
