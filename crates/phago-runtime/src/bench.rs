//! Benchmark harness for running and comparing colony simulations.
//!
//! Provides a standard framework for running timed simulations,
//! collecting snapshots at regular intervals, and comparing metrics
//! across multiple runs with different configurations.

use crate::colony::{Colony, ColonySnapshot};
use crate::metrics::{compute_from_snapshots, ColonyMetrics};
use phago_core::types::Tick;
use serde::Serialize;
use std::time::Instant;

/// A single benchmark run capturing timeline data.
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkRun {
    pub name: String,
    pub ticks: u64,
    pub snapshots: Vec<ColonySnapshot>,
    pub metrics_timeline: Vec<(Tick, ColonyMetrics)>,
    pub wall_time_ms: u64,
}

/// A suite of benchmark runs for comparison.
pub struct BenchmarkSuite {
    pub runs: Vec<BenchmarkRun>,
}

/// Configuration for a benchmark run.
pub struct BenchmarkConfig {
    pub name: String,
    pub ticks: u64,
    /// Take a snapshot every N ticks.
    pub snapshot_interval: u64,
    /// Compute metrics every N ticks.
    pub metrics_interval: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            ticks: 200,
            snapshot_interval: 10,
            metrics_interval: 50,
        }
    }
}

impl BenchmarkConfig {
    pub fn new(name: &str, ticks: u64) -> Self {
        Self {
            name: name.to_string(),
            ticks,
            ..Default::default()
        }
    }

    pub fn with_snapshot_interval(mut self, interval: u64) -> Self {
        self.snapshot_interval = interval;
        self
    }

    pub fn with_metrics_interval(mut self, interval: u64) -> Self {
        self.metrics_interval = interval;
        self
    }
}

/// Run a benchmark on a colony with the given configuration.
///
/// The colony should already have documents ingested and agents spawned.
/// This function runs the simulation and collects data.
pub fn run_benchmark(colony: &mut Colony, config: &BenchmarkConfig) -> BenchmarkRun {
    let mut snapshots = Vec::new();
    let mut metrics_timeline = Vec::new();

    // Initial snapshot
    snapshots.push(colony.snapshot());

    let start = Instant::now();

    for tick_num in 1..=config.ticks {
        colony.tick();

        if tick_num % config.snapshot_interval == 0 {
            snapshots.push(colony.snapshot());
        }

        if tick_num % config.metrics_interval == 0 {
            let metrics = compute_from_snapshots(colony, &snapshots);
            metrics_timeline.push((tick_num, metrics));
        }
    }

    let wall_time = start.elapsed();

    // Final metrics
    let final_metrics = compute_from_snapshots(colony, &snapshots);
    metrics_timeline.push((config.ticks, final_metrics));

    BenchmarkRun {
        name: config.name.clone(),
        ticks: config.ticks,
        snapshots,
        metrics_timeline,
        wall_time_ms: wall_time.as_millis() as u64,
    }
}

impl BenchmarkSuite {
    pub fn new() -> Self {
        Self { runs: Vec::new() }
    }

    pub fn add_run(&mut self, run: BenchmarkRun) {
        self.runs.push(run);
    }

    /// Compare final metrics across all runs.
    pub fn compare(&self) -> ComparisonTable {
        let rows: Vec<ComparisonRow> = self
            .runs
            .iter()
            .map(|run| {
                let final_metrics = run.metrics_timeline.last().map(|(_, m)| m.clone());

                ComparisonRow {
                    name: run.name.clone(),
                    ticks: run.ticks,
                    wall_time_ms: run.wall_time_ms,
                    graph_nodes: final_metrics
                        .as_ref()
                        .map(|m| m.graph_richness.node_count)
                        .unwrap_or(0),
                    graph_edges: final_metrics
                        .as_ref()
                        .map(|m| m.graph_richness.edge_count)
                        .unwrap_or(0),
                    density: final_metrics
                        .as_ref()
                        .map(|m| m.graph_richness.density)
                        .unwrap_or(0.0),
                    clustering: final_metrics
                        .as_ref()
                        .map(|m| m.graph_richness.clustering_coefficient)
                        .unwrap_or(0.0),
                    avg_degree: final_metrics
                        .as_ref()
                        .map(|m| m.graph_richness.avg_degree)
                        .unwrap_or(0.0),
                    shared_term_ratio: final_metrics
                        .as_ref()
                        .map(|m| m.transfer.shared_term_ratio)
                        .unwrap_or(0.0),
                    gini: final_metrics
                        .as_ref()
                        .map(|m| m.vocabulary_spread.gini_coefficient)
                        .unwrap_or(0.0),
                }
            })
            .collect();

        ComparisonTable { rows }
    }

    /// Export all runs to CSV format.
    pub fn to_csv(&self) -> String {
        let table = self.compare();
        let mut csv = String::new();
        csv.push_str("name,ticks,wall_time_ms,nodes,edges,density,clustering,avg_degree,shared_term_ratio,gini\n");
        for row in &table.rows {
            csv.push_str(&format!(
                "{},{},{},{},{},{:.4},{:.4},{:.2},{:.4},{:.4}\n",
                row.name,
                row.ticks,
                row.wall_time_ms,
                row.graph_nodes,
                row.graph_edges,
                row.density,
                row.clustering,
                row.avg_degree,
                row.shared_term_ratio,
                row.gini,
            ));
        }
        csv
    }
}

impl Default for BenchmarkSuite {
    fn default() -> Self {
        Self::new()
    }
}

/// Comparison table across benchmark runs.
#[derive(Debug, Clone, Serialize)]
pub struct ComparisonTable {
    pub rows: Vec<ComparisonRow>,
}

/// A single row in the comparison table.
#[derive(Debug, Clone, Serialize)]
pub struct ComparisonRow {
    pub name: String,
    pub ticks: u64,
    pub wall_time_ms: u64,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub density: f64,
    pub clustering: f64,
    pub avg_degree: f64,
    pub shared_term_ratio: f64,
    pub gini: f64,
}

impl ComparisonTable {
    /// Print a formatted comparison table to the terminal.
    pub fn print(&self) {
        println!(
            "┌{:─<20}┬{:─<7}┬{:─<10}┬{:─<7}┬{:─<7}┬{:─<8}┬{:─<10}┬{:─<9}┐",
            "", "", "", "", "", "", "", ""
        );
        println!(
            "│{:<20}│{:>7}│{:>10}│{:>7}│{:>7}│{:>8}│{:>10}│{:>9}│",
            " Run", " Nodes", " Edges", " Dense", " Clust", " AvgDeg", " Shared%", " Gini"
        );
        println!(
            "├{:─<20}┼{:─<7}┼{:─<10}┼{:─<7}┼{:─<7}┼{:─<8}┼{:─<10}┼{:─<9}┤",
            "", "", "", "", "", "", "", ""
        );
        for row in &self.rows {
            println!(
                "│{:<20}│{:>7}│{:>10}│{:>7.3}│{:>7.3}│{:>8.2}│{:>9.1}%│{:>9.3}│",
                row.name,
                row.graph_nodes,
                row.graph_edges,
                row.density,
                row.clustering,
                row.avg_degree,
                row.shared_term_ratio * 100.0,
                row.gini,
            );
        }
        println!(
            "└{:─<20}┴{:─<7}┴{:─<10}┴{:─<7}┴{:─<7}┴{:─<8}┴{:─<10}┴{:─<9}┘",
            "", "", "", "", "", "", "", ""
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::Corpus;
    use phago_agents::digester::Digester;
    use phago_core::types::Position;

    #[test]
    fn benchmark_run_produces_data() {
        let mut colony = Colony::new();
        // Use inline corpus (fixed 20 docs) for deterministic test timing
        let corpus = Corpus::inline_corpus();
        corpus.ingest_into(&mut colony);
        colony.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(80),
        ));

        let config = BenchmarkConfig::new("test", 20)
            .with_snapshot_interval(5)
            .with_metrics_interval(10);
        let run = run_benchmark(&mut colony, &config);

        assert_eq!(run.ticks, 20);
        assert!(!run.snapshots.is_empty());
        assert!(!run.metrics_timeline.is_empty());
        assert!(run.wall_time_ms < 10_000); // should be fast
    }

    #[test]
    fn suite_comparison_works() {
        let mut suite = BenchmarkSuite::new();

        let mut colony1 = Colony::new();
        colony1.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(80),
        ));
        let run1 = run_benchmark(&mut colony1, &BenchmarkConfig::new("empty", 10));
        suite.add_run(run1);

        let csv = suite.to_csv();
        assert!(csv.contains("empty"));
    }
}
