//! Application state for the web server.
//!
//! Uses a dedicated thread for Colony operations since Colony contains
//! trait objects that are not Send+Sync.

use anyhow::Result;
use phago_core::types::Position;
use phago_runtime::colony::{Colony, ColonyConfig, ColonyEvent, ColonySnapshot, ColonyStats};
use std::sync::mpsc;
use std::thread;
use tokio::sync::{broadcast, oneshot};

/// Commands sent to the colony worker thread.
enum ColonyCommand {
    GetStats(oneshot::Sender<ColonyStats>),
    GetSnapshot(oneshot::Sender<ColonySnapshot>),
    RunTicks(u64, oneshot::Sender<Vec<Vec<ColonyEvent>>>),
    Ingest {
        title: String,
        content: String,
        position: Position,
        ticks: u64,
        response: oneshot::Sender<IngestResult>,
    },
    Query {
        query: String,
        max_results: usize,
        alpha: f64,
        response: oneshot::Sender<QueryResult>,
    },
}

/// Result of an ingest operation.
pub struct IngestResult {
    pub document_id: String,
    pub nodes_created: usize,
    pub edges_created: usize,
    pub tick: u64,
}

/// Result of a query operation.
pub struct QueryResult {
    pub results: Vec<QueryHit>,
    pub total_nodes: usize,
    pub total_edges: usize,
}

pub struct QueryHit {
    pub label: String,
    pub score: f64,
    pub tfidf_score: f64,
    pub graph_score: f64,
}

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Channel to send commands to the colony worker.
    cmd_tx: mpsc::Sender<ColonyCommand>,
    /// Broadcast channel for colony events.
    pub event_tx: broadcast::Sender<ColonyEvent>,
}

impl AppState {
    /// Create a new app state, optionally with SQLite persistence.
    pub fn new(_db_path: Option<String>) -> Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (event_tx, _) = broadcast::channel(1000);
        let event_tx_clone = event_tx.clone();

        // Spawn dedicated thread for Colony operations
        thread::spawn(move || {
            let mut colony = Colony::from_config(ColonyConfig::default());

            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    ColonyCommand::GetStats(response) => {
                        let _ = response.send(colony.stats());
                    }
                    ColonyCommand::GetSnapshot(response) => {
                        let _ = response.send(colony.snapshot());
                    }
                    ColonyCommand::RunTicks(ticks, response) => {
                        let all_events = colony.run(ticks);
                        // Broadcast events
                        for events in &all_events {
                            for event in events {
                                let _ = event_tx_clone.send(event.clone());
                            }
                        }
                        let _ = response.send(all_events);
                    }
                    ColonyCommand::Ingest {
                        title,
                        content,
                        position,
                        ticks,
                        response,
                    } => {
                        use phago::prelude::Digester;

                        let before_nodes = colony.stats().graph_nodes;
                        let before_edges = colony.stats().graph_edges;

                        let doc_id = colony.ingest_document(&title, &content, position);
                        colony.spawn(Box::new(Digester::new(position).with_max_idle(30)));

                        let all_events = colony.run(ticks);
                        for events in &all_events {
                            for event in events {
                                let _ = event_tx_clone.send(event.clone());
                            }
                        }

                        let after_nodes = colony.stats().graph_nodes;
                        let after_edges = colony.stats().graph_edges;

                        let _ = response.send(IngestResult {
                            document_id: format!("{}", doc_id.0),
                            nodes_created: after_nodes.saturating_sub(before_nodes),
                            edges_created: after_edges.saturating_sub(before_edges),
                            tick: colony.stats().tick,
                        });
                    }
                    ColonyCommand::Query {
                        query,
                        max_results,
                        alpha,
                        response,
                    } => {
                        use phago::rag::{hybrid_query, HybridConfig};

                        let config = HybridConfig {
                            alpha,
                            max_results,
                            candidate_multiplier: 3,
                        };
                        let results = hybrid_query(&colony, &query, &config);
                        let stats = colony.stats();

                        let _ = response.send(QueryResult {
                            results: results
                                .into_iter()
                                .map(|r| QueryHit {
                                    label: r.label,
                                    score: r.final_score,
                                    tfidf_score: r.tfidf_score,
                                    graph_score: r.graph_score,
                                })
                                .collect(),
                            total_nodes: stats.graph_nodes,
                            total_edges: stats.graph_edges,
                        });
                    }
                }
            }
        });

        Ok(Self { cmd_tx, event_tx })
    }

    /// Get colony statistics.
    pub async fn stats(&self) -> ColonyStats {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(ColonyCommand::GetStats(tx));
        rx.await.unwrap_or_else(|_| ColonyStats {
            tick: 0,
            agents_alive: 0,
            agents_died: 0,
            total_spawned: 0,
            graph_nodes: 0,
            graph_edges: 0,
            total_signals: 0,
            documents_total: 0,
            documents_digested: 0,
        })
    }

    /// Get colony snapshot.
    pub async fn snapshot(&self) -> ColonySnapshot {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(ColonyCommand::GetSnapshot(tx));
        rx.await.unwrap_or_else(|_| ColonySnapshot {
            tick: 0,
            agents: vec![],
            nodes: vec![],
            edges: vec![],
            stats: ColonyStats {
                tick: 0,
                agents_alive: 0,
                agents_died: 0,
                total_spawned: 0,
                graph_nodes: 0,
                graph_edges: 0,
                total_signals: 0,
                documents_total: 0,
                documents_digested: 0,
            },
        })
    }

    /// Run N ticks.
    pub async fn run(&self, ticks: u64) -> Vec<Vec<ColonyEvent>> {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(ColonyCommand::RunTicks(ticks, tx));
        rx.await.unwrap_or_default()
    }

    /// Ingest a document.
    pub async fn ingest(
        &self,
        title: String,
        content: String,
        position: Position,
        ticks: u64,
    ) -> IngestResult {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(ColonyCommand::Ingest {
            title,
            content,
            position,
            ticks,
            response: tx,
        });
        rx.await.unwrap_or_else(|_| IngestResult {
            document_id: "error".to_string(),
            nodes_created: 0,
            edges_created: 0,
            tick: 0,
        })
    }

    /// Query the knowledge graph.
    pub async fn query(&self, query: String, max_results: usize, alpha: f64) -> QueryResult {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(ColonyCommand::Query {
            query,
            max_results,
            alpha,
            response: tx,
        });
        rx.await.unwrap_or_else(|_| QueryResult {
            results: vec![],
            total_nodes: 0,
            total_edges: 0,
        })
    }

    /// Subscribe to events.
    pub fn subscribe(&self) -> broadcast::Receiver<ColonyEvent> {
        self.event_tx.subscribe()
    }
}
