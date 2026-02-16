//! Phago MCP Server binary.
//!
//! Speaks the Model Context Protocol over stdio, allowing Claude Desktop,
//! Cursor, or any MCP client to interact with the biological knowledge graph.
//!
//! Usage:
//!   phago-mcp [--db path/to/knowledge.db]
//!
//! Claude Desktop config example:
//! ```json
//! {
//!   "mcpServers": {
//!     "phago": {
//!       "command": "phago-mcp",
//!       "args": ["--db", "~/phago-knowledge.db"]
//!     }
//!   }
//! }
//! ```

use anyhow::Result;
use clap::Parser;
use phago_mcp::tools::PhagoTools;
use phago_mcp::worker::ColonyHandle;
use rmcp::{transport::stdio, ServiceExt};

#[derive(Parser)]
#[command(name = "phago-mcp")]
#[command(about = "Phago MCP Server — biological knowledge graph for AI agents")]
struct Args {
    /// Path to SQLite database for persistent knowledge storage.
    /// If omitted, knowledge is stored in memory only.
    #[arg(long)]
    db: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let handle = ColonyHandle::spawn(args.db);
    let tools = PhagoTools::new(handle);

    let service = tools.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
