//! Phago Runtime Prelude — convenient imports for common usage.
//!
//! ```rust
//! use phago_runtime::prelude::*;
//! ```

// Re-export colony
pub use crate::colony::{Colony, ColonyEvent, ColonyStats, ColonySnapshot};

// Re-export session
pub use crate::session::{
    GraphState, SerializedNode, SerializedEdge, SessionMetadata,
    save_session, load_session, restore_into_colony, verify_fidelity,
};

// Re-export metrics
pub use crate::metrics::{ColonyMetrics, TransferMetrics, DissolutionMetrics, GraphRichnessMetrics};

// Re-export from agents
pub use phago_agents::prelude::*;
