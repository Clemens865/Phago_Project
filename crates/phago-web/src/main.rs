//! Phago Web Dashboard - Real-time colony visualization.

use anyhow::Result;
use clap::Parser;

mod routes;
mod state;

pub use state::AppState;

#[derive(Parser, Debug)]
#[command(name = "phago-web")]
#[command(about = "Phago Web Dashboard - Real-time colony visualization")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Path to SQLite database (optional persistence)
    #[arg(short, long)]
    db: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let addr = format!("{}:{}", cli.host, cli.port);

    println!("Starting Phago Web Dashboard...");
    println!("Open http://{} in your browser", addr);

    // Create app state
    let state = AppState::new(cli.db)?;

    // Build router
    let app = routes::create_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
