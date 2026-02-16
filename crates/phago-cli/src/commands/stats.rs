//! Show colony statistics.

use anyhow::{bail, Result};
use colored::Colorize;
use phago::prelude::*;

use crate::config::current_session_path;

pub fn run() -> Result<()> {
    let session_path = current_session_path()?;

    if !session_path.exists() {
        bail!("No session found. Run {} first.", "phago ingest".cyan());
    }

    // Load session
    let state = load_session(&session_path)?;
    let mut colony = Colony::new();
    restore_into_colony(&mut colony, &state);

    let stats = colony.stats();
    let graph = colony.substrate().graph();

    // Calculate additional metrics
    let components = graph.connected_components();
    let all_nodes = graph.all_nodes();

    // Count node types
    let mut concept_count = 0;
    let mut document_count = 0;
    let mut total_access = 0u64;

    for node_id in &all_nodes {
        if let Some(node) = graph.get_node(node_id) {
            total_access += node.access_count;
            match node.node_type {
                NodeType::Concept => concept_count += 1,
                NodeType::Document => document_count += 1,
                _ => {}
            }
        }
    }

    let avg_access = if !all_nodes.is_empty() {
        total_access as f64 / all_nodes.len() as f64
    } else {
        0.0
    };

    // Calculate edge stats
    let mut total_weight = 0.0;
    let mut edge_count = 0;
    let mut strong_edges = 0;

    for node_id in &all_nodes {
        for (_, edge_data) in graph.neighbors(node_id) {
            total_weight += edge_data.weight;
            edge_count += 1;
            if edge_data.co_activations >= 2 {
                strong_edges += 1;
            }
        }
    }
    edge_count /= 2; // Each edge counted twice

    let avg_weight = if edge_count > 0 {
        total_weight / (edge_count * 2) as f64
    } else {
        0.0
    };

    // Print stats
    println!("{}", "Phago Colony Statistics".white().bold());
    println!("{}", "═".repeat(40).dimmed());
    println!();

    println!("{}", "Graph Structure".blue().bold());
    println!(
        "  Total nodes:       {}",
        stats.graph_nodes.to_string().cyan()
    );
    println!(
        "  Total edges:       {}",
        stats.graph_edges.to_string().cyan()
    );
    println!("  Components:        {}", components.to_string().cyan());
    println!();

    println!("{}", "Node Types".blue().bold());
    println!("  Concepts:          {}", concept_count.to_string().cyan());
    println!("  Documents:         {}", document_count.to_string().cyan());
    println!("  Avg access count:  {:.2}", avg_access);
    println!();

    println!("{}", "Edge Quality".blue().bold());
    println!(
        "  Strong edges:      {} ({:.1}%)",
        strong_edges.to_string().green(),
        if edge_count > 0 {
            (strong_edges as f64 / edge_count as f64) * 100.0
        } else {
            0.0
        }
    );
    println!("  Avg edge weight:   {:.4}", avg_weight);
    println!();

    // Density calculation
    if stats.graph_nodes > 1 {
        let max_edges = (stats.graph_nodes * (stats.graph_nodes - 1)) / 2;
        let density = stats.graph_edges as f64 / max_edges as f64;
        println!("{}", "Density".blue().bold());
        println!("  Graph density:     {:.6}", density);
    }

    println!();
    println!("{}", "═".repeat(40).dimmed());

    Ok(())
}
