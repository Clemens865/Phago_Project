//! # Phago Runtime
//!
//! Colony management, scheduling, and host runtime.
//!
//! The runtime is the "organism" — it manages the lifecycle of agents
//! (cells), runs the tick-based simulation, and maintains the substrate
//! (shared environment).
//!
//! ## Quick Start
//!
//! ```rust
//! use phago_runtime::prelude::*;
//!
//! // Create a colony
//! let mut colony = Colony::new();
//!
//! // Ingest a document
//! colony.ingest_document("title", "content", Position::new(0.0, 0.0));
//!
//! // Spawn a digester
//! colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0))));
//!
//! // Run the simulation
//! colony.run(50);
//! ```

pub mod substrate_impl;
pub mod topology_impl;
pub mod colony;
pub mod colony_builder;
pub mod metrics;
pub mod corpus;
pub mod bench;
pub mod export;
pub mod community;
pub mod curriculum;
pub mod training_format;
pub mod session;
pub mod project_context;
pub mod backend;
pub mod prelude;

#[cfg(feature = "sqlite")]
pub mod sqlite_topology;

#[cfg(feature = "async")]
pub mod async_runtime;

#[cfg(feature = "streaming")]
pub mod streaming;
