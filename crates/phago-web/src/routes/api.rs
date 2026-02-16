//! REST API endpoints for colony interaction.

use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use phago_core::types::Position;
use phago_runtime::colony::{
    AgentSnapshot, ColonySnapshot, ColonyStats, EdgeSnapshot, NodeSnapshot,
};
use serde::{Deserialize, Serialize};

/// Get colony statistics.
pub async fn get_stats(State(state): State<AppState>) -> Json<ColonyStats> {
    Json(state.stats().await)
}

/// Get all graph nodes.
pub async fn get_nodes(State(state): State<AppState>) -> Json<Vec<NodeSnapshot>> {
    let snapshot = state.snapshot().await;
    Json(snapshot.nodes)
}

/// Get all graph edges.
pub async fn get_edges(State(state): State<AppState>) -> Json<Vec<EdgeSnapshot>> {
    let snapshot = state.snapshot().await;
    Json(snapshot.edges)
}

/// Get all active agents.
pub async fn get_agents(State(state): State<AppState>) -> Json<Vec<AgentSnapshot>> {
    let snapshot = state.snapshot().await;
    Json(snapshot.agents)
}

/// Get full colony snapshot.
pub async fn get_snapshot(State(state): State<AppState>) -> Json<ColonySnapshot> {
    Json(state.snapshot().await)
}

/// Query request body.
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_alpha")]
    pub alpha: f64,
}

fn default_max_results() -> usize {
    10
}
fn default_alpha() -> f64 {
    0.5
}

/// Query result.
#[derive(Debug, Serialize)]
pub struct QueryResultItem {
    pub label: String,
    pub score: f64,
    pub tfidf_score: f64,
    pub graph_score: f64,
}

/// Query response.
#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub results: Vec<QueryResultItem>,
    pub total_nodes: usize,
    pub total_edges: usize,
}

/// Query the knowledge graph.
pub async fn query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Json<QueryResponse> {
    let result = state.query(req.query, req.max_results, req.alpha).await;

    Json(QueryResponse {
        results: result
            .results
            .into_iter()
            .map(|r| QueryResultItem {
                label: r.label,
                score: r.score,
                tfidf_score: r.tfidf_score,
                graph_score: r.graph_score,
            })
            .collect(),
        total_nodes: result.total_nodes,
        total_edges: result.total_edges,
    })
}

/// Ingest request body.
#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub position: Option<(f64, f64)>,
    #[serde(default = "default_ticks")]
    pub ticks: u64,
}

fn default_ticks() -> u64 {
    15
}

/// Ingest response.
#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub document_id: String,
    pub nodes_created: usize,
    pub edges_created: usize,
    pub tick: u64,
}

/// Ingest a document.
pub async fn ingest(
    State(state): State<AppState>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, StatusCode> {
    let pos = req
        .position
        .map(|(x, y)| Position::new(x, y))
        .unwrap_or_else(|| Position::new(0.0, 0.0));

    let result = state.ingest(req.title, req.content, pos, req.ticks).await;

    Ok(Json(IngestResponse {
        document_id: result.document_id,
        nodes_created: result.nodes_created,
        edges_created: result.edges_created,
        tick: result.tick,
    }))
}

/// Tick request body.
#[derive(Debug, Deserialize)]
pub struct TickRequest {
    #[serde(default = "default_tick_count")]
    pub count: u64,
}

fn default_tick_count() -> u64 {
    1
}

/// Run tick(s).
pub async fn tick(
    State(state): State<AppState>,
    Json(req): Json<TickRequest>,
) -> Json<ColonyStats> {
    state.run(req.count).await;
    Json(state.stats().await)
}

/// Run request body.
#[derive(Debug, Deserialize)]
pub struct RunRequest {
    pub ticks: u64,
}

/// Run multiple ticks.
pub async fn run(State(state): State<AppState>, Json(req): Json<RunRequest>) -> Json<ColonyStats> {
    state.run(req.ticks).await;
    Json(state.stats().await)
}
