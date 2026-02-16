//! Ingest documents into the colony.

use anyhow::{bail, Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use phago::prelude::*;
use std::path::Path;

use crate::config::{current_session_path, data_dir, Config};

pub fn run(path: &str, ticks: u64, extensions: &str, verbose: bool) -> Result<()> {
    let path = Path::new(path);
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }

    // Load config
    let config = Config::load()?;

    // Ensure .phago directory exists
    let data = data_dir()?;
    if !data.exists() {
        std::fs::create_dir_all(&data)?;
    }

    // Load or create colony
    let session_path = current_session_path()?;
    let mut colony = if session_path.exists() {
        println!("{} Loading existing session...", "→".blue());
        let state = load_session(&session_path)?;
        let mut c = Colony::new();
        restore_into_colony(&mut c, &state);
        c
    } else {
        Colony::new()
    };

    // Collect files to ingest
    let ext_list: Vec<&str> = extensions.split(',').collect();
    let files = collect_files(path, &ext_list)?;

    if files.is_empty() {
        bail!("No files found with extensions: {}", extensions);
    }

    println!(
        "{} Ingesting {} files...",
        "→".blue(),
        files.len().to_string().cyan()
    );

    // Ingest each file
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    for (i, file) in files.iter().enumerate() {
        let content = std::fs::read_to_string(file)
            .with_context(|| format!("Failed to read: {}", file.display()))?;

        let title = file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("doc-{}", i));

        // Position documents in a grid
        let x = (i % 10) as f64;
        let y = (i / 10) as f64;

        colony.ingest_document(&title, &content, Position::new(x, y));

        if verbose {
            pb.set_message(format!("{}", file.file_name().unwrap().to_string_lossy()));
        }
        pb.inc(1);
    }
    pb.finish_with_message("done");

    // Spawn digesters
    let num_digesters = (files.len() / 3).max(1).min(config.colony.max_agents);
    println!(
        "{} Spawning {} digesters...",
        "→".blue(),
        num_digesters.to_string().cyan()
    );

    for i in 0..num_digesters {
        let x = (i % 10) as f64;
        let y = (i / 10) as f64;
        colony.spawn(Box::new(
            Digester::new(Position::new(x, y)).with_max_idle(config.digester.max_idle),
        ));
    }

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
        colony.tick();
        pb.inc(1);
    }
    pb.finish_with_message("done");

    // Save session
    save_session(&colony, &session_path, &[])?;

    // Print stats
    let stats = colony.stats();
    println!();
    println!("{} Ingestion complete!", "✓".green().bold());
    println!("  Nodes: {}", stats.graph_nodes.to_string().cyan());
    println!("  Edges: {}", stats.graph_edges.to_string().cyan());
    println!("  Documents: {}", files.len().to_string().cyan());

    Ok(())
}

fn collect_files(path: &Path, extensions: &[&str]) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        for entry in walkdir(path)? {
            let ext = entry.extension().and_then(|e| e.to_str()).unwrap_or("");
            if extensions.contains(&ext) {
                files.push(entry);
            }
        }
    }

    Ok(files)
}

fn walkdir(path: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walkdir(&path)?);
        } else {
            files.push(path);
        }
    }

    Ok(files)
}
