//! Comprehensive benchmark tests for Phago capabilities.
//!
//! Run with: cargo test --release --features "sqlite,async" -p phago-runtime --test benchmarks -- --nocapture
//!
//! These benchmarks measure:
//! 1. Simulation throughput (ticks per second)
//! 2. SQLite persistence (save/load performance)
//! 3. Async runtime (throughput comparison)
//! 4. Graph scaling (node/edge counts)
//! 5. Semantic wiring overhead

use phago_agents::digester::Digester;
use phago_core::types::Position;
use phago_runtime::bench::{run_benchmark, BenchmarkConfig, BenchmarkSuite};
use phago_runtime::corpus::Corpus;
use phago_runtime::prelude::*;
use std::time::Instant;

/// Helper to create a colony with documents and agents.
fn setup_colony(doc_count: usize, agent_count: usize) -> Colony {
    let mut colony = Colony::new();

    // Add documents in a grid pattern
    for i in 0..doc_count {
        let x = (i % 10) as f64 * 5.0;
        let y = (i / 10) as f64 * 5.0;
        colony.ingest_document(
            &format!("Doc {}", i),
            &format!(
                "The cell membrane contains proteins that facilitate transport. \
                 Molecules pass through channels and receptors. Document {} discusses \
                 cellular biology, protein synthesis, and metabolic pathways.",
                i
            ),
            Position::new(x, y),
        );
    }

    // Add agents distributed across the space
    for i in 0..agent_count {
        let x = (i % 10) as f64 * 5.0;
        let y = (i / 10) as f64 * 5.0;
        colony.spawn(Box::new(
            Digester::new(Position::new(x, y)).with_max_idle(100),
        ));
    }

    colony
}

// ============================================================================
// BENCHMARK 1: Simulation Throughput
// ============================================================================

#[test]
fn bench_simulation_throughput() {
    println!("\n=== BENCHMARK: Simulation Throughput ===\n");

    let configs = [
        ("Small (5 docs, 2 agents)", 5, 2, 100),
        ("Medium (20 docs, 5 agents)", 20, 5, 100),
        ("Large (50 docs, 10 agents)", 50, 10, 100),
    ];

    println!(
        "{:<30} {:>10} {:>12} {:>15}",
        "Configuration", "Ticks", "Time (ms)", "Ticks/sec"
    );
    println!("{:-<70}", "");

    for (name, docs, agents, ticks) in configs {
        let mut colony = setup_colony(docs, agents);

        let start = Instant::now();
        colony.run(ticks);
        let elapsed = start.elapsed();

        let ticks_per_sec = ticks as f64 / elapsed.as_secs_f64();

        println!(
            "{:<30} {:>10} {:>12.2} {:>15.1}",
            name,
            ticks,
            elapsed.as_millis(),
            ticks_per_sec
        );
    }

    println!();
}

// ============================================================================
// BENCHMARK 2: Graph Scaling
// ============================================================================

#[test]
fn bench_graph_scaling() {
    println!("\n=== BENCHMARK: Graph Scaling ===\n");

    let configs = [
        ("10 docs", 10, 3, 50),
        ("25 docs", 25, 5, 50),
        ("50 docs", 50, 8, 50),
        ("100 docs", 100, 10, 50),
    ];

    println!(
        "{:<15} {:>8} {:>8} {:>10} {:>12} {:>10}",
        "Config", "Nodes", "Edges", "Density", "Time (ms)", "Nodes/ms"
    );
    println!("{:-<70}", "");

    for (name, docs, agents, ticks) in configs {
        let mut colony = setup_colony(docs, agents);

        let start = Instant::now();
        colony.run(ticks);
        let elapsed = start.elapsed();

        let stats = colony.stats();
        let density = if stats.graph_nodes > 1 {
            (2.0 * stats.graph_edges as f64)
                / (stats.graph_nodes as f64 * (stats.graph_nodes as f64 - 1.0))
        } else {
            0.0
        };

        let nodes_per_ms = stats.graph_nodes as f64 / elapsed.as_millis().max(1) as f64;

        println!(
            "{:<15} {:>8} {:>8} {:>10.4} {:>12} {:>10.2}",
            name,
            stats.graph_nodes,
            stats.graph_edges,
            density,
            elapsed.as_millis(),
            nodes_per_ms
        );
    }

    println!();
}

// ============================================================================
// BENCHMARK 3: SQLite Persistence (requires sqlite feature)
// ============================================================================

#[cfg(feature = "sqlite")]
#[test]
fn bench_sqlite_persistence() {
    println!("\n=== BENCHMARK: SQLite Persistence ===\n");

    let tmp_dir = std::env::temp_dir();

    let configs = [
        ("Small graph", 10, 2, 30),
        ("Medium graph", 30, 5, 50),
        ("Large graph", 100, 10, 50),
    ];

    println!(
        "{:<15} {:>8} {:>8} {:>12} {:>12} {:>12}",
        "Config", "Nodes", "Edges", "Save (ms)", "Load (ms)", "Total (ms)"
    );
    println!("{:-<75}", "");

    for (name, docs, agents, ticks) in configs {
        let db_path = tmp_dir.join(format!("bench_persist_{}.db", docs));
        let _ = std::fs::remove_file(&db_path);

        // Create and populate colony
        let mut colony = ColonyBuilder::new()
            .with_persistence(&db_path)
            .build()
            .expect("Failed to create colony");

        // Add data
        for i in 0..docs {
            let x = (i % 10) as f64 * 5.0;
            let y = (i / 10) as f64 * 5.0;
            colony.ingest_document(
                &format!("Doc {}", i),
                "Cell membrane proteins facilitate molecular transport through channels.",
                Position::new(x, y),
            );
        }

        for i in 0..agents {
            colony.spawn(Box::new(
                Digester::new(Position::new((i % 10) as f64 * 5.0, (i / 10) as f64 * 5.0))
                    .with_max_idle(100),
            ));
        }

        colony.run(ticks);

        let stats = colony.stats();
        let node_count = stats.graph_nodes;
        let edge_count = stats.graph_edges;

        // Benchmark save
        let save_start = Instant::now();
        colony.save().expect("Failed to save");
        let save_time = save_start.elapsed();

        drop(colony);

        // Benchmark load
        let load_start = Instant::now();
        let _loaded = ColonyBuilder::new()
            .with_persistence(&db_path)
            .build()
            .expect("Failed to load colony");
        let load_time = load_start.elapsed();

        println!(
            "{:<15} {:>8} {:>8} {:>12.2} {:>12.2} {:>12.2}",
            name,
            node_count,
            edge_count,
            save_time.as_millis(),
            load_time.as_millis(),
            save_time.as_millis() + load_time.as_millis()
        );

        let _ = std::fs::remove_file(&db_path);
    }

    println!();
}

// ============================================================================
// BENCHMARK 4: Async Runtime (requires async feature)
// ============================================================================

#[cfg(feature = "async")]
#[tokio::test]
async fn bench_async_runtime() {
    use phago_runtime::async_runtime::run_in_local;

    println!("\n=== BENCHMARK: Async Runtime ===\n");

    let configs = [
        ("Small (10 docs)", 10, 2, 50),
        ("Medium (30 docs)", 30, 5, 50),
        ("Large (50 docs)", 50, 8, 50),
    ];

    println!(
        "{:<20} {:>12} {:>12} {:>12}",
        "Config", "Sync (ms)", "Async (ms)", "Ratio"
    );
    println!("{:-<60}", "");

    for (name, docs, agents, ticks) in configs {
        // Sync benchmark
        let mut sync_colony = setup_colony(docs, agents);
        let sync_start = Instant::now();
        sync_colony.run(ticks);
        let sync_time = sync_start.elapsed();

        // Async benchmark
        let async_colony = setup_colony(docs, agents);
        let async_start = Instant::now();
        run_in_local(async_colony, |ac| async move { ac.run_async(ticks).await }).await;
        let async_time = async_start.elapsed();

        let ratio = async_time.as_secs_f64() / sync_time.as_secs_f64();

        println!(
            "{:<20} {:>12.2} {:>12.2} {:>12.2}x",
            name,
            sync_time.as_millis(),
            async_time.as_millis(),
            ratio
        );
    }

    println!(
        "\n(Ratio > 1 means sync is faster; async adds yield overhead but enables concurrency)\n"
    );
}

// ============================================================================
// BENCHMARK 5: Agent Serialization
// ============================================================================

#[test]
fn bench_agent_serialization() {
    use phago_agents::sentinel::Sentinel;
    use phago_agents::serialize::SerializableAgent;
    use phago_agents::synthesizer::Synthesizer;

    println!("\n=== BENCHMARK: Agent Serialization ===\n");

    let agent_counts = [10, 50, 100, 200];

    println!(
        "{:<15} {:>12} {:>12} {:>12}",
        "Agent Count", "Export (µs)", "Import (µs)", "Total (µs)"
    );
    println!("{:-<55}", "");

    for count in agent_counts {
        // Create typed agent vectors
        let mut digesters: Vec<Digester> = Vec::new();
        let mut synthesizers: Vec<Synthesizer> = Vec::new();
        let mut sentinels: Vec<Sentinel> = Vec::new();

        for i in 0..count {
            let pos = Position::new((i % 10) as f64, (i / 10) as f64);
            match i % 3 {
                0 => digesters.push(Digester::new(pos)),
                1 => synthesizers.push(Synthesizer::new(pos)),
                _ => sentinels.push(Sentinel::new(pos)),
            }
        }

        // Benchmark export
        let export_start = Instant::now();
        let mut states = Vec::new();
        for d in &digesters {
            states.push(d.export_state());
        }
        for s in &synthesizers {
            states.push(s.export_state());
        }
        for s in &sentinels {
            states.push(s.export_state());
        }
        let export_time = export_start.elapsed();

        // Benchmark import (reconstruction)
        let import_start = Instant::now();
        let mut _count = 0usize;
        for state in &states {
            if let Some(_d) = Digester::from_state(state) {
                _count += 1;
            } else if let Some(_s) = Synthesizer::from_state(state) {
                _count += 1;
            } else if let Some(_s) = Sentinel::from_state(state) {
                _count += 1;
            }
        }
        let import_time = import_start.elapsed();

        println!(
            "{:<15} {:>12} {:>12} {:>12}",
            count,
            export_time.as_micros(),
            import_time.as_micros(),
            export_time.as_micros() + import_time.as_micros()
        );
    }

    println!();
}

// ============================================================================
// BENCHMARK 6: Semantic Wiring Overhead
// ============================================================================

#[test]
fn bench_semantic_wiring() {
    use phago_core::semantic::SemanticWiringConfig;

    println!("\n=== BENCHMARK: Semantic Wiring Overhead ===\n");

    let ticks = 50;
    let docs = 30;
    let agents = 5;

    // Test different semantic configurations
    let configs = [
        ("No semantic (baseline)", SemanticWiringConfig::default()),
        ("Relaxed semantic", SemanticWiringConfig::relaxed()),
        ("Strict semantic", SemanticWiringConfig::strict()),
    ];

    println!(
        "{:<25} {:>12} {:>10} {:>10} {:>12}",
        "Config", "Time (ms)", "Nodes", "Edges", "Edges/Node"
    );
    println!("{:-<75}", "");

    for (name, semantic_config) in configs {
        let mut colony = setup_colony(docs, agents);
        colony.set_semantic_wiring(semantic_config);

        let start = Instant::now();
        colony.run(ticks);
        let elapsed = start.elapsed();

        let stats = colony.stats();
        let edges_per_node = if stats.graph_nodes > 0 {
            stats.graph_edges as f64 / stats.graph_nodes as f64
        } else {
            0.0
        };

        println!(
            "{:<25} {:>12.2} {:>10} {:>10} {:>12.2}",
            name,
            elapsed.as_millis(),
            stats.graph_nodes,
            stats.graph_edges,
            edges_per_node
        );
    }

    println!();
}

// ============================================================================
// BENCHMARK 7: Full Suite Comparison
// ============================================================================

#[test]
fn bench_full_suite() {
    println!("\n=== BENCHMARK: Full Suite Comparison ===\n");

    let mut suite = BenchmarkSuite::new();

    // Run with inline corpus for consistent comparison
    let corpus = Corpus::inline_corpus();

    // Configuration 1: Minimal agents
    let mut colony1 = Colony::new();
    corpus.ingest_into(&mut colony1);
    colony1.spawn(Box::new(
        Digester::new(Position::new(0.0, 0.0)).with_max_idle(100),
    ));
    let run1 = run_benchmark(&mut colony1, &BenchmarkConfig::new("1 agent", 100));
    suite.add_run(run1);

    // Configuration 2: Multiple agents
    let mut colony2 = Colony::new();
    corpus.ingest_into(&mut colony2);
    for i in 0..5 {
        colony2.spawn(Box::new(
            Digester::new(Position::new((i * 10) as f64, 0.0)).with_max_idle(100),
        ));
    }
    let run2 = run_benchmark(&mut colony2, &BenchmarkConfig::new("5 agents", 100));
    suite.add_run(run2);

    // Configuration 3: Dense agents
    let mut colony3 = Colony::new();
    corpus.ingest_into(&mut colony3);
    for i in 0..10 {
        colony3.spawn(Box::new(
            Digester::new(Position::new((i % 5 * 5) as f64, (i / 5 * 5) as f64)).with_max_idle(100),
        ));
    }
    let run3 = run_benchmark(&mut colony3, &BenchmarkConfig::new("10 agents dense", 100));
    suite.add_run(run3);

    // Print comparison table
    let table = suite.compare();
    table.print();

    // Also print wall times
    println!("\nWall Times:");
    for run in &suite.runs {
        println!("  {}: {} ms", run.name, run.wall_time_ms);
    }

    println!();
}

// ============================================================================
// SUMMARY
// ============================================================================

#[test]
fn bench_summary() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                    PHAGO BENCHMARK SUMMARY                       ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║ Run individual benchmarks with:                                  ║");
    println!("║   cargo test --release --features \"sqlite,async\" \\              ║");
    println!("║     -p phago-runtime --test benchmarks -- --nocapture            ║");
    println!("║                                                                  ║");
    println!("║ Benchmarks included:                                             ║");
    println!("║   1. Simulation Throughput  - Ticks per second at scale          ║");
    println!("║   2. Graph Scaling         - Node/edge growth patterns           ║");
    println!("║   3. SQLite Persistence    - Save/load performance               ║");
    println!("║   4. Async Runtime         - Sync vs async comparison            ║");
    println!("║   5. Agent Serialization   - Export/import overhead              ║");
    println!("║   6. Semantic Wiring       - Similarity computation cost         ║");
    println!("║   7. Full Suite            - Complete comparison table           ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();
}
