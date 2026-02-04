//! HTTP and WebSocket routes for the web dashboard.

mod api;
mod ws;

use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::path::PathBuf;
use tower_http::{cors::CorsLayer, services::ServeDir};

/// Create the main router with all routes.
pub fn create_router(state: AppState) -> Router {
    // Determine static file directory
    let static_dir = std::env::var("PHAGO_STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            manifest.join("static")
        });

    Router::new()
        // API routes
        .route("/api/stats", get(api::get_stats))
        .route("/api/nodes", get(api::get_nodes))
        .route("/api/edges", get(api::get_edges))
        .route("/api/agents", get(api::get_agents))
        .route("/api/query", post(api::query))
        .route("/api/ingest", post(api::ingest))
        .route("/api/tick", post(api::tick))
        .route("/api/run", post(api::run))
        .route("/api/snapshot", get(api::get_snapshot))
        // WebSocket for live events
        .route("/ws/events", get(ws::events_handler))
        // Static files (serve index.html as fallback)
        .fallback_service(ServeDir::new(static_dir).append_index_html_on_directories(true))
        // CORS for development
        .layer(CorsLayer::permissive())
        // State
        .with_state(state)
}
