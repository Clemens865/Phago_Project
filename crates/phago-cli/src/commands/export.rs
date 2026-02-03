//! Export the knowledge graph.

use anyhow::{bail, Result};
use colored::Colorize;
use phago::prelude::*;
use serde::Serialize;
use std::path::Path;

use crate::config::current_session_path;

#[derive(Serialize)]
struct ExportedGraph {
    nodes: Vec<ExportedNode>,
    edges: Vec<ExportedEdge>,
    metadata: ExportMetadata,
}

#[derive(Serialize)]
struct ExportedNode {
    id: String,
    label: String,
    node_type: String,
    access_count: u64,
}

#[derive(Serialize)]
struct ExportedEdge {
    source: String,
    target: String,
    weight: f64,
    co_activation_count: u32,
}

#[derive(Serialize)]
struct ExportMetadata {
    node_count: usize,
    edge_count: usize,
    exported_at: String,
}

pub fn run(output: &str, format: &str) -> Result<()> {
    let session_path = current_session_path()?;

    if !session_path.exists() {
        bail!(
            "No session found. Run {} first.",
            "phago ingest".cyan()
        );
    }

    // Load session
    println!("{} Loading session...", "→".blue());
    let state = load_session(&session_path)?;
    let mut colony = Colony::new();
    restore_into_colony(&mut colony, &state);

    let graph = colony.substrate().graph();
    let all_nodes = graph.all_nodes();

    // Build export data
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for node_id in &all_nodes {
        if let Some(node) = graph.get_node(node_id) {
            nodes.push(ExportedNode {
                id: format!("{:?}", node_id),
                label: node.label.clone(),
                node_type: format!("{:?}", node.node_type),
                access_count: node.access_count,
            });

            // Get edges for this node
            for (neighbor_id, edge_data) in graph.neighbors(node_id) {
                // Only add edge once (from lower to higher ID)
                if format!("{:?}", node_id) < format!("{:?}", neighbor_id) {
                    edges.push(ExportedEdge {
                        source: format!("{:?}", node_id),
                        target: format!("{:?}", neighbor_id),
                        weight: edge_data.weight,
                        co_activation_count: edge_data.co_activations as u32,
                    });
                }
            }
        }
    }

    let export = ExportedGraph {
        metadata: ExportMetadata {
            node_count: nodes.len(),
            edge_count: edges.len(),
            exported_at: chrono_now(),
        },
        nodes,
        edges,
    };

    // Write to file
    let output_path = Path::new(output);
    match format.to_lowercase().as_str() {
        "json" => {
            let content = serde_json::to_string_pretty(&export)?;
            std::fs::write(output_path, content)?;
        }
        _ => {
            bail!("Unsupported format: {}. Use 'json'.", format);
        }
    }

    println!();
    println!("{} Exported to {}", "✓".green().bold(), output.cyan());
    println!("  Nodes: {}", export.metadata.node_count.to_string().cyan());
    println!("  Edges: {}", export.metadata.edge_count.to_string().cyan());

    Ok(())
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    format!("{}", duration.as_secs())
}
