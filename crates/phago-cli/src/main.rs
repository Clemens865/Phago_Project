//! Phago CLI - Command-line interface for biological computing.

mod commands;
mod config;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "phago")]
#[command(author, version, about = "Phago - Self-evolving knowledge substrates", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Phago project
    Init {
        /// Project directory (default: current directory)
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Ingest documents into the colony
    Ingest {
        /// File or directory to ingest
        path: String,

        /// Number of ticks to run after ingestion
        #[arg(short, long, default_value = "30")]
        ticks: u64,

        /// File extensions to include (e.g., "txt,md")
        #[arg(short, long, default_value = "txt,md")]
        extensions: String,
    },

    /// Run the colony simulation
    Run {
        /// Number of ticks to run
        #[arg(short, long, default_value = "50")]
        ticks: u64,
    },

    /// Query the knowledge graph
    Query {
        /// Search query
        query: String,

        /// Maximum results to return
        #[arg(short, long, default_value = "10")]
        max_results: usize,

        /// Alpha value (0.0 = pure graph, 1.0 = pure TF-IDF)
        #[arg(short, long, default_value = "0.5")]
        alpha: f64,
    },

    /// Explore graph structure
    Explore {
        #[command(subcommand)]
        command: ExploreCommands,
    },

    /// Export the knowledge graph
    Export {
        /// Output file path
        output: String,

        /// Export format
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// Manage sessions
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },

    /// Show colony statistics
    Stats,

    /// Start the MCP server (delegates to phago-mcp binary)
    Mcp {
        /// Path to SQLite database for persistent knowledge storage
        #[arg(long)]
        db: Option<String>,
    },

    /// Distributed cluster management
    #[cfg(feature = "distributed")]
    Cluster {
        #[command(subcommand)]
        command: ClusterCommands,
    },
}

#[derive(Subcommand)]
enum ExploreCommands {
    /// Show most central concepts
    Centrality {
        /// Number of top concepts
        #[arg(short, long, default_value = "10")]
        top: usize,
    },

    /// Show bridge concepts between clusters
    Bridges {
        /// Number of top bridges
        #[arg(short, long, default_value = "10")]
        top: usize,
    },

    /// Find shortest path between concepts
    Path {
        /// Source concept
        from: String,
        /// Target concept
        to: String,
    },

    /// Count connected components
    Components,
}

#[cfg(feature = "distributed")]
#[derive(Subcommand)]
enum ClusterCommands {
    /// Start a coordinator node
    StartCoordinator {
        /// Port to listen on
        #[arg(short, long, default_value = "9000")]
        port: u16,

        /// Number of shards in the cluster
        #[arg(short, long, default_value = "3")]
        num_shards: u32,
    },

    /// Start a shard node
    StartShard {
        /// Port for this shard to listen on
        #[arg(short, long)]
        port: u16,

        /// Coordinator address (host:port)
        #[arg(short, long, default_value = "127.0.0.1:9000")]
        coordinator: String,

        /// Shard ID
        #[arg(short, long)]
        id: u32,
    },

    /// Show cluster status
    Status {
        /// Coordinator address (host:port)
        #[arg(short, long, default_value = "127.0.0.1:9000")]
        coordinator: String,
    },

    /// Run distributed benchmarks
    Bench {
        /// Benchmark mode: quick, full, or scaling
        #[arg(default_value = "quick")]
        mode: String,
    },
}

#[derive(Subcommand)]
enum SessionCommands {
    /// Save current session
    Save {
        /// Session name
        name: String,
    },

    /// Load a saved session
    Load {
        /// Session name
        name: String,
    },

    /// List saved sessions
    List,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => commands::init::run(path),
        Commands::Ingest { path, ticks, extensions } => {
            commands::ingest::run(&path, ticks, &extensions, cli.verbose)
        }
        Commands::Run { ticks } => commands::run::run(ticks, cli.verbose),
        Commands::Query { query, max_results, alpha } => {
            commands::query::run(&query, max_results, alpha)
        }
        Commands::Explore { command } => match command {
            ExploreCommands::Centrality { top } => commands::explore::centrality(top),
            ExploreCommands::Bridges { top } => commands::explore::bridges(top),
            ExploreCommands::Path { from, to } => commands::explore::path(&from, &to),
            ExploreCommands::Components => commands::explore::components(),
        },
        Commands::Export { output, format } => commands::export::run(&output, &format),
        Commands::Session { command } => match command {
            SessionCommands::Save { name } => commands::session::save(&name),
            SessionCommands::Load { name } => commands::session::load(&name),
            SessionCommands::List => commands::session::list(),
        },
        Commands::Stats => commands::stats::run(),
        Commands::Mcp { db } => {
            let mut cmd = std::process::Command::new("phago-mcp");
            if let Some(path) = db {
                cmd.arg("--db").arg(path);
            }
            let status = cmd.status().map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!(
                        "phago-mcp binary not found. Install with: cargo install phago-mcp"
                    )
                } else {
                    anyhow::anyhow!("Failed to start MCP server: {e}")
                }
            })?;
            std::process::exit(status.code().unwrap_or(1));
        }

        #[cfg(feature = "distributed")]
        Commands::Cluster { command } => match command {
            ClusterCommands::StartCoordinator { port, num_shards } => {
                commands::cluster::start_coordinator(port, num_shards)
            }
            ClusterCommands::StartShard {
                port,
                coordinator,
                id,
            } => commands::cluster::start_shard(port, &coordinator, id),
            ClusterCommands::Status { coordinator } => {
                commands::cluster::status(&coordinator)
            }
            ClusterCommands::Bench { mode } => commands::cluster::bench(&mode),
        },
    }
}
