//! Phago Runtime Prelude — convenient imports for common usage.
//!
//! ```rust
//! use phago_runtime::prelude::*;
//! ```

// Re-export colony
pub use crate::colony::{Colony, ColonyConfig, ColonyEvent, ColonyStats, ColonySnapshot};

// Re-export colony builder
pub use crate::colony_builder::{ColonyBuilder, PersistentColony, BuilderError};

// Re-export session
pub use crate::session::{
    GraphState, SerializedNode, SerializedEdge, SessionMetadata,
    save_session, load_session, restore_into_colony, verify_fidelity,
};

// Re-export metrics
pub use crate::metrics::{ColonyMetrics, TransferMetrics, DissolutionMetrics, GraphRichnessMetrics};

// Re-export backend configuration
pub use crate::backend::{BackendConfig, BackendError, DynTopologyGraph, create_backend};

// Re-export SQLite backend when feature is enabled
#[cfg(feature = "sqlite")]
pub use crate::sqlite_topology::SqliteTopologyGraph;

// Re-export async runtime when feature is enabled
#[cfg(feature = "async")]
pub use crate::async_runtime::{
    AsyncColony, TickTimer,
    spawn_simulation_local, batch_ingest, run_in_local,
};

// Re-export streaming when feature is enabled
#[cfg(feature = "streaming")]
pub use crate::streaming::{
    StreamingColony, StreamingConfig, StreamingMetrics,
    IngestDocument, DocumentChannel, FileWatcher, WatchEvent,
    watch_directory_to_channel, streaming_from_directory,
};

// Re-export from agents
pub use phago_agents::prelude::*;

// Re-export Louvain community detection
pub use phago_core::louvain::LouvainResult;
