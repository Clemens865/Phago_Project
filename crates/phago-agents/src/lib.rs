//! # Phago Agents
//!
//! Reference agent implementations using Phago biological primitives.
//!
//! Each agent type implements a subset of the ten primitives, giving it
//! specific biological capabilities:
//!
//! - **Digester** — DIGEST + SENSE + APOPTOSE — consumes and processes text input
//! - **Synthesizer** — EMERGE + SENSE + APOPTOSE — collective intelligence through quorum sensing
//! - **Sentinel** — NEGATE + SENSE + APOPTOSE — anomaly detection through negative selection

pub mod digester;
pub mod sentinel;
pub mod synthesizer;
pub mod genome;
pub mod fitness;
pub mod spawn;
pub mod code_digester;
