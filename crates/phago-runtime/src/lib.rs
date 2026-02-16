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

pub mod backend;
pub mod bench;
pub mod colony;
pub mod colony_builder;
pub mod community;
pub mod corpus;
pub mod curriculum;
pub mod diff;
pub mod export;
pub mod metrics;
pub mod prelude;
pub mod project_context;
pub mod session;
pub mod stdp;
pub mod substrate_impl;
pub mod topology_impl;
pub mod training_format;

#[cfg(feature = "sqlite")]
pub mod sqlite_topology;

#[cfg(feature = "async")]
pub mod async_runtime;

#[cfg(feature = "streaming")]
pub mod streaming;

#[cfg(feature = "vectors")]
pub mod vector_integration;
