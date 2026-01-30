//! DISSOLVE — Holobiont
//!
//! A human is not one organism. It is a holobiont — ~30 trillion human
//! cells and ~38 trillion microbial cells in symbiosis. The gut microbiome
//! influences immune function, metabolism, neurotransmitter production,
//! and even behavior.
//!
//! The boundary between "organism" and "environment" is not a wall —
//! it is a gradient. The most robust systems are those where this
//! boundary is fluid, not rigid.
//!
//! In Phago, agents can modulate their boundary with the substrate.
//! Mature, trusted agents dissolve into the substrate — their knowledge
//! becomes ambient, indistinguishable from substrate-native data.

use crate::substrate::Substrate;
use crate::types::*;

/// Modulate the boundary between agent and substrate.
///
/// Dissolution is the endpoint of agent maturity. An agent that has
/// contributed valuable, reinforced knowledge to the substrate gradually
/// loses its boundary. Its internal state becomes substrate state.
/// The agent and the substrate co-constitute each other.
pub trait Dissolve {
    /// Current boundary permeability (0.0 = rigid wall, 1.0 = no boundary).
    ///
    /// At 0.0, the agent is fully isolated — classic WASM sandboxing.
    /// At 1.0, the agent's internal state is fully exposed to the substrate.
    fn permeability(&self) -> f64;

    /// Adjust boundary permeability based on context.
    ///
    /// Permeability increases when:
    /// - The agent's contributions are reinforced by others
    /// - The agent has been alive for many cycles
    /// - Trust from the colony is high
    ///
    /// Permeability decreases when:
    /// - The agent detects anomalies (defensive contraction)
    /// - Trust signals are absent
    fn modulate_boundary(&mut self, context: &BoundaryContext);

    /// Expose an aspect of internal state to the substrate.
    ///
    /// Partial dissolution: the agent selectively externalizes
    /// specific internal data, making it available to all agents
    /// through the substrate.
    fn externalize(&self, aspect: &str, substrate: &mut dyn Substrate);

    /// Absorb substrate state into internal processing.
    ///
    /// Partial absorption: the agent internalizes substrate data,
    /// incorporating external knowledge into its own processing.
    fn internalize(&mut self, aspect: &str, substrate: &dyn Substrate);
}
