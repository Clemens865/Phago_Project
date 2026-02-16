//! Session management commands.

use anyhow::{bail, Context, Result};
use colored::Colorize;
use phago::prelude::*;

use crate::config::{current_session_path, sessions_dir};

pub fn save(name: &str) -> Result<()> {
    let current_path = current_session_path()?;

    if !current_path.exists() {
        bail!("No active session. Run {} first.", "phago ingest".cyan());
    }

    // Ensure sessions directory exists
    let sessions = sessions_dir()?;
    std::fs::create_dir_all(&sessions)?;

    // Copy current session to named session
    let session_path = sessions.join(format!("{}.json", name));
    std::fs::copy(&current_path, &session_path)
        .with_context(|| format!("Failed to save session: {}", name))?;

    println!("{} Session saved: {}", "✓".green().bold(), name.cyan());

    Ok(())
}

pub fn load(name: &str) -> Result<()> {
    let sessions = sessions_dir()?;
    let session_path = sessions.join(format!("{}.json", name));

    if !session_path.exists() {
        bail!("Session not found: {}", name);
    }

    // Load the named session to verify it's valid
    let state = load_session(&session_path)?;

    // Copy to current session
    let current_path = current_session_path()?;
    std::fs::copy(&session_path, &current_path)
        .with_context(|| format!("Failed to load session: {}", name))?;

    // Restore to get stats
    let mut colony = Colony::new();
    restore_into_colony(&mut colony, &state);
    let stats = colony.stats();

    println!("{} Session loaded: {}", "✓".green().bold(), name.cyan());
    println!("  Nodes: {}", stats.graph_nodes.to_string().cyan());
    println!("  Edges: {}", stats.graph_edges.to_string().cyan());

    Ok(())
}

pub fn list() -> Result<()> {
    let sessions = sessions_dir()?;

    if !sessions.exists() {
        println!("{} No saved sessions.", "•".yellow());
        return Ok(());
    }

    let mut found = false;
    println!("{} Saved sessions:", "→".blue());
    println!();

    for entry in std::fs::read_dir(&sessions)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            // Try to load and get stats
            if let Ok(state) = load_session(&path) {
                let mut colony = Colony::new();
                restore_into_colony(&mut colony, &state);
                let stats = colony.stats();

                println!(
                    "  {} {} ({} nodes, {} edges)",
                    "•".blue(),
                    name.white().bold(),
                    stats.graph_nodes,
                    stats.graph_edges
                );
                found = true;
            }
        }
    }

    if !found {
        println!("  {} No saved sessions.", "•".yellow());
    }

    Ok(())
}
