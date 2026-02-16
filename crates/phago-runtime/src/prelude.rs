//! Phago Runtime Prelude — convenient imports for common usage.
//!
//! ```rust
//! use phago_runtime::prelude::*;
//! ```

// Re-export colony
pub use crate::colony::{Colony, ColonyConfig, ColonyEvent, ColonySnapshot, ColonyStats};

// Re-export colony builder
pub use crate::colony_builder::{BuilderError, ColonyBuilder, PersistentColony};

// Re-export session
pub use crate::session::{
    load_session, restore_into_colony, save_session, verify_fidelity, GraphState, SerializedEdge,
    SerializedNode, SessionMetadata,
};

// Re-export metrics
pub use crate::metrics::{
    ColonyMetrics, DissolutionMetrics, GraphRichnessMetrics, TransferMetrics,
};

// Re-export backend configuration
pub use crate::backend::{create_backend, BackendConfig, BackendError, DynTopologyGraph};

// Re-export SQLite backend when feature is enabled
#[cfg(feature = "sqlite")]
pub use crate::sqlite_topology::SqliteTopologyGraph;

// Re-export async runtime when feature is enabled
#[cfg(feature = "async")]
pub use crate::async_runtime::{
    batch_ingest, run_in_local, spawn_simulation_local, AsyncColony, TickTimer,
};

// Re-export streaming when feature is enabled
#[cfg(feature = "streaming")]
pub use crate::streaming::{
    streaming_from_directory, watch_directory_to_channel, DocumentChannel, FileWatcher,
    IngestDocument, StreamingColony, StreamingConfig, StreamingMetrics, WatchEvent,
};

// Re-export from agents
pub use phago_agents::prelude::*;

// Re-export Louvain community detection
pub use phago_core::louvain::LouvainResult;
