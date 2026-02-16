//! Query the knowledge graph.

use anyhow::{bail, Result};
use colored::Colorize;
use phago::prelude::*;

use crate::config::current_session_path;

pub fn run(query: &str, max_results: usize, alpha: f64) -> Result<()> {
    let session_path = current_session_path()?;

    if !session_path.exists() {
        bail!("No session found. Run {} first.", "phago ingest".cyan());
    }

    // Load session
    let state = load_session(&session_path)?;
    let mut colony = Colony::new();
    restore_into_colony(&mut colony, &state);

    // Run hybrid query
    let config = HybridConfig {
        alpha,
        max_results,
        candidate_multiplier: 3,
    };

    let results = hybrid_query(&colony, query, &config);

    if results.is_empty() {
        println!("{} No results found for: {}", "•".yellow(), query.cyan());
        return Ok(());
    }

    println!(
        "{} Results for {} (alpha={}):",
        "→".blue(),
        query.cyan().bold(),
        alpha
    );
    println!();

    for (i, result) in results.iter().enumerate() {
        let rank = format!("{}.", i + 1);
        let score = format!("{:.3}", result.final_score);

        println!(
            "  {} {} {}",
            rank.blue(),
            result.label.white().bold(),
            format!("({})", score).dimmed()
        );

        // Show score breakdown
        println!(
            "      TF-IDF: {:.3}  Graph: {:.3}",
            result.tfidf_score, result.graph_score
        );
    }

    println!();
    println!(
        "{} {} results",
        "✓".green(),
        results.len().to_string().cyan()
    );

    Ok(())
}
