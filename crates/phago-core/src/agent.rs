//! Agent — the fundamental computational cell.
//!
//! Every agent in Phago is a cell: an isolated unit that senses its
//! environment, processes inputs, and acts on the substrate. The base
//! Agent trait requires the three most fundamental biological capabilities:
//! - DIGEST: the ability to consume and process input
//! - APOPTOSE: the ability to self-assess and gracefully die
//! - SENSE: the ability to detect environmental signals

use crate::primitives::symbiose::AgentProfile;
use crate::primitives::{Apoptose, Digest, Sense};
use crate::substrate::Substrate;
use crate::types::*;

/// The fundamental unit of computation in Phago — a biological cell.
///
/// Every agent must be able to:
/// - **Digest** input (consume, break down, present fragments)
/// - **Apoptose** (self-assess health, gracefully die when compromised)
/// - **Sense** the substrate (detect signals, follow gradients)
///
/// Additional biological capabilities (Wire, Transfer, Emerge, etc.)
/// are opt-in through additional trait implementations.
pub trait Agent: Digest + Apoptose + Sense {
    /// The agent's unique identity.
    fn id(&self) -> AgentId;

    /// The agent's current position in the substrate.
    fn position(&self) -> Position;

    /// Set the agent's position.
    fn set_position(&mut self, position: Position);

    /// The agent's type name (for display and logging).
    fn agent_type(&self) -> &str;

    /// Execute one tick of the agent's lifecycle.
    ///
    /// This is the main loop body. Each tick, the agent:
    /// 1. Senses the environment
    /// 2. Decides what to do
    /// 3. Returns an action for the runtime to execute
    ///
    /// The runtime calls this once per simulation tick.
    fn tick(&mut self, substrate: &dyn Substrate) -> AgentAction;

    /// How many ticks this agent has been alive.
    fn age(&self) -> Tick;

    // --- Transfer (Horizontal Gene Transfer) default methods ---

    /// Export this agent's vocabulary as serialized bytes.
    /// Returns None if the agent has no vocabulary to export.
    fn export_vocabulary(&self) -> Option<Vec<u8>> {
        None
    }

    /// Integrate foreign vocabulary from serialized bytes.
    /// Returns true if integration succeeded.
    fn integrate_vocabulary(&mut self, _data: &[u8]) -> bool {
        false
    }

    // --- Symbiose (Endosymbiosis) default methods ---

    /// Build a profile describing this agent's capabilities.
    fn profile(&self) -> AgentProfile {
        AgentProfile {
            id: self.id(),
            agent_type: self.agent_type().to_string(),
            capabilities: Vec::new(),
            health: CellHealth::Healthy,
        }
    }

    /// Evaluate whether to absorb another agent as a symbiont.
    fn evaluate_symbiosis(&self, _other: &AgentProfile) -> Option<SymbiosisEval> {
        None
    }

    /// Absorb another agent's profile and vocabulary data as a symbiont.
    /// Returns true if absorption succeeded.
    fn absorb_symbiont(&mut self, _profile: AgentProfile, _data: Vec<u8>) -> bool {
        false
    }

    // --- Dissolve (Holobiont) default methods ---

    /// Current boundary permeability (0.0 = rigid, 1.0 = fully dissolved).
    fn permeability(&self) -> f64 {
        0.0
    }

    /// Adjust boundary permeability based on environmental context.
    fn modulate_boundary(&mut self, _context: &BoundaryContext) {}

    /// Return vocabulary terms to externalize (reinforce in the substrate).
    fn externalize_vocabulary(&self) -> Vec<String> {
        Vec::new()
    }

    /// Absorb nearby concept terms from the substrate.
    fn internalize_vocabulary(&mut self, _terms: &[String]) {}

    /// Return the size of this agent's vocabulary (for metrics).
    fn vocabulary_size(&self) -> usize {
        0
    }
}
