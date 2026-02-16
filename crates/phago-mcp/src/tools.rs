//! MCP tool definitions for the Phago knowledge graph.
//!
//! Wraps the existing `phago_rag::mcp` functions as MCP tools
//! accessible via the rmcp protocol.

use crate::worker::ColonyHandle;
use rmcp::{
    handler::server::router::tool::ToolRouter, handler::server::wrapper::Parameters, model::*,
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use serde::Deserialize;
use std::borrow::Cow;

type McpError = rmcp::model::ErrorData;

/// MCP tool router for Phago operations.
#[derive(Clone)]
pub struct PhagoTools {
    tool_router: ToolRouter<PhagoTools>,
    handle: ColonyHandle,
}

impl std::fmt::Debug for PhagoTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhagoTools").finish()
    }
}

// === Tool parameter types (JSON Schema via schemars) ===

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RememberParams {
    /// Title or label for the document being ingested.
    pub title: String,
    /// The text content to ingest into the knowledge graph.
    pub content: String,
    /// Number of simulation ticks to run for digestion (default: 15).
    pub ticks: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RecallParams {
    /// Search query to find relevant concepts in the knowledge graph.
    pub query: String,
    /// Maximum number of results to return (default: 10).
    pub max_results: Option<usize>,
    /// Balance between TF-IDF (1.0) and graph-based (0.0) scoring (default: 0.5).
    pub alpha: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExploreParams {
    /// Type of structural query: "path", "centrality", "bridges", or "stats".
    #[serde(rename = "type")]
    pub query_type: String,
    /// Source concept (required for "path" queries).
    pub from: Option<String>,
    /// Target concept (required for "path" queries).
    pub to: Option<String>,
    /// Number of top results (for "centrality" and "bridges", default: 10).
    pub top_k: Option<usize>,
}

#[tool_router]
impl PhagoTools {
    pub fn new(handle: ColonyHandle) -> Self {
        Self {
            tool_router: Self::tool_router(),
            handle,
        }
    }

    /// Ingest a document into the biological knowledge graph. The colony's
    /// agents will digest the text into concepts, wire them via Hebbian
    /// co-activation, and build a self-organizing knowledge structure.
    #[tool(
        name = "phago_remember",
        description = "Ingest a document into the biological knowledge graph. Agents digest text into concepts and wire them via Hebbian co-activation learning."
    )]
    async fn remember(
        &self,
        params: Parameters<RememberParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let req = phago_rag::mcp::RememberRequest {
            title: params.title,
            content: params.content,
            ticks: params.ticks,
        };

        let resp = self.handle.remember(req).await.map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Remember failed: {e}")),
            data: None,
        })?;

        let json = serde_json::to_string_pretty(&resp).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Query the knowledge graph using hybrid TF-IDF + graph-topology scoring.
    /// Returns concepts ranked by relevance, combining text matching with
    /// structural importance in the knowledge graph.
    #[tool(
        name = "phago_recall",
        description = "Query the knowledge graph with hybrid TF-IDF + graph-topology scoring. Returns concepts ranked by combined text and structural relevance."
    )]
    async fn recall(&self, params: Parameters<RecallParams>) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let req = phago_rag::mcp::RecallRequest {
            query: params.query,
            max_results: params.max_results.unwrap_or(10),
            alpha: params.alpha.unwrap_or(0.5),
        };

        let resp = self.handle.recall(req).await.map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Recall failed: {e}")),
            data: None,
        })?;

        let json = serde_json::to_string_pretty(&resp).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Explore the graph structure: find shortest paths between concepts,
    /// discover high-centrality hub nodes, identify bridge concepts between
    /// clusters, or get colony statistics.
    #[tool(
        name = "phago_explore",
        description = "Explore the graph structure. Supports: 'path' (shortest path between concepts), 'centrality' (hub nodes), 'bridges' (cross-cluster connectors), 'stats' (colony metrics)."
    )]
    async fn explore(&self, params: Parameters<ExploreParams>) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let req = match params.query_type.as_str() {
            "path" => {
                let from = params.from.ok_or_else(|| McpError {
                    code: ErrorCode::INVALID_PARAMS,
                    message: Cow::from("'from' is required for path queries"),
                    data: None,
                })?;
                let to = params.to.ok_or_else(|| McpError {
                    code: ErrorCode::INVALID_PARAMS,
                    message: Cow::from("'to' is required for path queries"),
                    data: None,
                })?;
                phago_rag::mcp::ExploreRequest::ShortestPath { from, to }
            }
            "centrality" => phago_rag::mcp::ExploreRequest::Centrality {
                top_k: params.top_k.unwrap_or(10),
            },
            "bridges" => phago_rag::mcp::ExploreRequest::Bridges {
                top_k: params.top_k.unwrap_or(10),
            },
            "stats" => phago_rag::mcp::ExploreRequest::Stats,
            other => {
                return Err(McpError {
                    code: ErrorCode::INVALID_PARAMS,
                    message: Cow::from(format!(
                        "Unknown explore type '{other}'. Use: path, centrality, bridges, stats"
                    )),
                    data: None,
                });
            }
        };

        let resp = self.handle.explore(req).await.map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Explore failed: {e}")),
            data: None,
        })?;

        let json = serde_json::to_string_pretty(&resp).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

#[tool_handler]
impl ServerHandler for PhagoTools {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "phago".into(),
                title: Some("Phago Knowledge Graph".into()),
                version: env!("CARGO_PKG_VERSION").into(),
                description: Some(
                    "Self-evolving biological knowledge graph with Hebbian learning".into(),
                ),
                icons: None,
                website_url: Some("https://github.com/Clemens865/Phago_Project".into()),
            },
            instructions: Some(
                "Phago biological knowledge graph. Use phago_remember to ingest documents, \
                 phago_recall to query knowledge, and phago_explore to analyze graph structure."
                    .into(),
            ),
        }
    }
}
