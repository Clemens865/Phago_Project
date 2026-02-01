//! # Phago Runtime
//!
//! Colony management, scheduling, and host runtime.
//!
//! The runtime is the "organism" — it manages the lifecycle of agents
//! (cells), runs the tick-based simulation, and maintains the substrate
//! (shared environment).

pub mod substrate_impl;
pub mod topology_impl;
pub mod colony;
pub mod metrics;
