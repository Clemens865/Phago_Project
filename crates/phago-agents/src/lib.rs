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
//!
//! ## Quick Start
//!
//! ```rust
//! use phago_agents::prelude::*;
//!
//! let digester = Digester::new(Position::new(0.0, 0.0));
//! let sentinel = Sentinel::new(Position::new(1.0, 1.0));
//! let synthesizer = Synthesizer::new(Position::new(2.0, 2.0));
//! ```

pub mod code_digester;
pub mod digester;
pub mod fitness;
pub mod genome;
pub mod prelude;
pub mod sentinel;
pub mod serialize;
pub mod spawn;
pub mod synthesizer;

pub mod lamarckian;

#[cfg(feature = "semantic")]
pub mod semantic_digester;

#[cfg(feature = "ast")]
pub mod ast_digester;
