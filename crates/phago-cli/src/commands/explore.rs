//! Explore graph structure.

use anyhow::{bail, Result};
use colored::Colorize;
use phago::prelude::*;

use crate::config::current_session_path;

fn load_colony() -> Result<Colony> {
    let session_path = current_session_path()?;

    if !session_path.exists() {
        bail!("No session found. Run {} first.", "phago ingest".cyan());
    }

    let state = load_session(&session_path)?;
    let mut colony = Colony::new();
    restore_into_colony(&mut colony, &state);
    Ok(colony)
}

pub fn centrality(top: usize) -> Result<()> {
    let colony = load_colony()?;
    let graph = colony.substrate().graph();

    println!(
        "{} Top {} central concepts:",
        "→".blue(),
        top.to_string().cyan()
    );
    println!();

    let centrality = graph.betweenness_centrality(100);

    for (i, (node_id, score)) in centrality.iter().take(top).enumerate() {
        if let Some(node) = graph.get_node(node_id) {
            let rank = format!("{}.", i + 1);
            println!(
                "  {} {} {}",
                rank.blue(),
                node.label.white().bold(),
                format!("({:.4})", score).dimmed()
            );
        }
    }

    Ok(())
}

pub fn bridges(top: usize) -> Result<()> {
    let colony = load_colony()?;
    let graph = colony.substrate().graph();

    println!(
        "{} Top {} bridge concepts (connect clusters):",
        "→".blue(),
        top.to_string().cyan()
    );
    println!();

    let bridges = graph.bridge_nodes(top);

    for (i, (node_id, fragility)) in bridges.iter().enumerate() {
        if let Some(node) = graph.get_node(node_id) {
            let rank = format!("{}.", i + 1);
            println!(
                "  {} {} {}",
                rank.blue(),
                node.label.white().bold(),
                format!("(fragility: {:.4})", fragility).dimmed()
            );
        }
    }

    Ok(())
}

pub fn path(from: &str, to: &str) -> Result<()> {
    let colony = load_colony()?;
    let graph = colony.substrate().graph();

    // Find nodes by label
    let from_nodes = graph.find_nodes_by_label(from);
    let to_nodes = graph.find_nodes_by_label(to);

    if from_nodes.is_empty() {
        bail!("No node found with label: {}", from);
    }
    if to_nodes.is_empty() {
        bail!("No node found with label: {}", to);
    }

    let from_id = &from_nodes[0];
    let to_id = &to_nodes[0];

    println!(
        "{} Finding path: {} → {}",
        "→".blue(),
        from.cyan(),
        to.cyan()
    );
    println!();

    match graph.shortest_path(from_id, to_id) {
        Some((path, weight)) => {
            println!(
                "  Path length: {} hops",
                (path.len() - 1).to_string().green()
            );
            println!("  Total weight: {:.4}", weight);
            println!();
            println!("  Path:");
            for (i, node_id) in path.iter().enumerate() {
                if let Some(node) = graph.get_node(node_id) {
                    let prefix = if i == 0 {
                        "  ●".green().to_string()
                    } else if i == path.len() - 1 {
                        "  ●".blue().to_string()
                    } else {
                        "  │".dimmed().to_string()
                    };
                    println!("{} {}", prefix, node.label);
                }
            }
        }
        None => {
            println!("  {} No path found between {} and {}", "✗".red(), from, to);
        }
    }

    Ok(())
}

pub fn components() -> Result<()> {
    let colony = load_colony()?;
    let graph = colony.substrate().graph();

    let count = graph.connected_components();

    println!(
        "{} Connected components: {}",
        "→".blue(),
        count.to_string().cyan().bold()
    );

    if count == 1 {
        println!("  The graph is fully connected.");
    } else {
        println!("  The graph has {} disconnected regions.", count);
    }

    Ok(())
}
