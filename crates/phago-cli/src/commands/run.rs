//! Run the colony simulation.

use anyhow::{bail, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use phago::prelude::*;

use crate::config::current_session_path;

pub fn run(ticks: u64, verbose: bool) -> Result<()> {
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

    let initial_stats = colony.stats();
    println!(
        "  Loaded: {} nodes, {} edges",
        initial_stats.graph_nodes.to_string().cyan(),
        initial_stats.graph_edges.to_string().cyan()
    );

    // Run simulation
    println!(
        "{} Running {} ticks...",
        "→".blue(),
        ticks.to_string().cyan()
    );

    let pb = ProgressBar::new(ticks);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ticks")
            .unwrap()
            .progress_chars("#>-"),
    );

    for _ in 0..ticks {
        let events = colony.tick();
        if verbose && !events.is_empty() {
            pb.println(format!("  {} events", events.len()));
        }
        pb.inc(1);
    }
    pb.finish_with_message("done");

    // Save session
    save_session(&colony, &session_path, &[])?;

    // Print stats
    let final_stats = colony.stats();
    println!();
    println!("{} Simulation complete!", "✓".green().bold());
    println!(
        "  Nodes: {} → {}",
        initial_stats.graph_nodes.to_string().yellow(),
        final_stats.graph_nodes.to_string().green()
    );
    println!(
        "  Edges: {} → {}",
        initial_stats.graph_edges.to_string().yellow(),
        final_stats.graph_edges.to_string().green()
    );

    Ok(())
}
