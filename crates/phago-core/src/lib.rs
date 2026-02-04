//! # Phago Core
//!
//! Core traits and types for Phago biological computing primitives.
//!
//! This crate defines the ten biological primitives as Rust traits,
//! along with shared types used across the entire framework:
//!
//! - **DIGEST** — Consume input, extract fragments, present learnings (phagocytosis)
//! - **APOPTOSE** — Self-assess integrity, gracefully self-terminate (programmed cell death)
//! - **SENSE** — Detect environmental signals, follow gradients (chemotaxis)
//! - **TRANSFER** — Export/import capabilities across agents (horizontal gene transfer)
//! - **EMERGE** — Detect quorum, activate collective behaviors (phase transitions)
//! - **WIRE** — Strengthen used connections, prune unused ones (Hebbian learning)
//! - **SYMBIOSE** — Integrate another agent instead of consuming it (endosymbiosis)
//! - **STIGMERGE** — Coordinate through environmental modification (stigmergy)
//! - **NEGATE** — Define identity by exclusion, detect anomalies (negative selection)
//! - **DISSOLVE** — Modulate agent-substrate boundaries (holobiont)
//!
//! ## Quick Start
//!
//! ```rust
//! use phago_core::prelude::*;
//!
//! // Create a position
//! let pos = Position::new(0.0, 0.0);
//!
//! // Create a deterministic agent ID (for testing)
//! let id = AgentId::from_seed(42);
//! ```

pub mod primitives;
pub mod types;
pub mod agent;
pub mod substrate;
pub mod signal;
pub mod topology;
pub mod error;
pub mod semantic;
pub mod prelude;
