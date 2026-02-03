//! Initialize a new Phago project.

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use crate::config::Config;

pub fn run(path: Option<String>) -> Result<()> {
    let base_path = path
        .map(|p| Path::new(&p).to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    println!("{} Initializing Phago project...", "→".blue());

    // Create .phago directory
    let phago_dir = base_path.join(".phago");
    std::fs::create_dir_all(&phago_dir)
        .with_context(|| format!("Failed to create {}", phago_dir.display()))?;
    println!("  {} Created {}", "✓".green(), phago_dir.display());

    // Create sessions directory
    let sessions_dir = phago_dir.join("sessions");
    std::fs::create_dir_all(&sessions_dir)
        .with_context(|| format!("Failed to create {}", sessions_dir.display()))?;
    println!("  {} Created {}", "✓".green(), sessions_dir.display());

    // Create default config
    let config_path = base_path.join("phago.toml");
    if !config_path.exists() {
        let config = Config::default();
        config.save(&config_path)?;
        println!("  {} Created {}", "✓".green(), config_path.display());
    } else {
        println!("  {} {} already exists", "•".yellow(), config_path.display());
    }

    // Create .gitignore for .phago
    let gitignore_path = phago_dir.join(".gitignore");
    if !gitignore_path.exists() {
        std::fs::write(&gitignore_path, "current.json\nsessions/\n")?;
        println!("  {} Created {}", "✓".green(), gitignore_path.display());
    }

    println!();
    println!("{} Phago project initialized!", "✓".green().bold());
    println!();
    println!("Next steps:");
    println!("  {} phago ingest <documents>", "1.".blue());
    println!("  {} phago query \"your search\"", "2.".blue());
    println!("  {} phago stats", "3.".blue());

    Ok(())
}
