//! Colony worker thread.
//!
//! Colony contains `Box<dyn Agent>` (not Send+Sync), so it must live
//! on a dedicated thread. Commands are sent via `mpsc`, responses via
//! `oneshot`.

use phago_rag::mcp::{
    ExploreRequest, ExploreResponse, RecallRequest, RecallResponse, RememberRequest,
    RememberResponse,
};
use phago_runtime::colony::Colony;
use std::sync::mpsc;
use tokio::sync::oneshot;

/// Commands sent to the colony worker thread.
pub enum ColonyCommand {
    Remember {
        req: RememberRequest,
        tx: oneshot::Sender<RememberResponse>,
    },
    Recall {
        req: RecallRequest,
        tx: oneshot::Sender<RecallResponse>,
    },
    Explore {
        req: ExploreRequest,
        tx: oneshot::Sender<ExploreResponse>,
    },
}

/// Handle to the colony worker thread.
#[derive(Clone)]
pub struct ColonyHandle {
    cmd_tx: mpsc::Sender<ColonyCommand>,
}

impl ColonyHandle {
    /// Spawn a new colony worker thread and return a handle.
    pub fn spawn(db_path: Option<String>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();

        std::thread::spawn(move || {
            let mut colony = if let Some(ref _path) = db_path {
                // SQLite persistence requires phago-runtime's "sqlite" feature.
                // ColonyBuilder::with_persistence is a no-op without it.
                use phago_runtime::colony_builder::ColonyBuilder;
                match ColonyBuilder::new()
                    .with_persistence(_path)
                    .auto_save(true)
                    .build()
                {
                    Ok(pc) => pc.into_inner(),
                    Err(e) => {
                        eprintln!("Warning: Failed to open database: {e}. Using in-memory colony.");
                        Colony::new()
                    }
                }
            } else {
                Colony::new()
            };

            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    ColonyCommand::Remember { req, tx } => {
                        let resp = phago_rag::mcp::phago_remember(&mut colony, &req);
                        let _ = tx.send(resp);
                    }
                    ColonyCommand::Recall { req, tx } => {
                        let resp = phago_rag::mcp::phago_recall(&colony, &req);
                        let _ = tx.send(resp);
                    }
                    ColonyCommand::Explore { req, tx } => {
                        let resp = phago_rag::mcp::phago_explore(&colony, &req);
                        let _ = tx.send(resp);
                    }
                }
            }
        });

        Self { cmd_tx }
    }

    /// Ingest a document into the colony.
    pub async fn remember(&self, req: RememberRequest) -> anyhow::Result<RememberResponse> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(ColonyCommand::Remember { req, tx })
            .map_err(|_| anyhow::anyhow!("Colony worker thread has shut down"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Colony worker dropped response"))
    }

    /// Query the knowledge graph.
    pub async fn recall(&self, req: RecallRequest) -> anyhow::Result<RecallResponse> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(ColonyCommand::Recall { req, tx })
            .map_err(|_| anyhow::anyhow!("Colony worker thread has shut down"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Colony worker dropped response"))
    }

    /// Explore the graph structure.
    pub async fn explore(&self, req: ExploreRequest) -> anyhow::Result<ExploreResponse> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(ColonyCommand::Explore { req, tx })
            .map_err(|_| anyhow::anyhow!("Colony worker thread has shut down"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Colony worker dropped response"))
    }
}
