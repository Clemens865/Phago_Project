//! SYMBIOSE — Endosymbiosis
//!
//! ~1.5 billion years ago, a cell engulfed an alpha-proteobacterium.
//! Instead of digesting it, the two formed a permanent partnership.
//! The engulfed cell became the mitochondrion — the energy-producing
//! organelle in every complex cell alive today.
//!
//! **The most important moment in the evolution of complex life was a
//! failed act of destruction that became collaboration.**
//!
//! In Phago, symbiosis occurs when an agent evaluates another's output
//! during digestion and determines it is more valuable intact than
//! broken down. The other agent is integrated as a permanent sub-component.

use crate::types::*;

/// Metadata about another agent, used to evaluate symbiosis potential.
#[derive(Debug, Clone)]
pub struct AgentProfile {
    pub id: AgentId,
    pub agent_type: String,
    pub capabilities: Vec<CapabilityDescriptor>,
    pub health: CellHealth,
}

/// Integrate another agent instead of consuming it.
///
/// Symbiosis is a DIGEST that pivots: during evaluation, the agent
/// detects that the other is more valuable as a permanent partner
/// than as fragments. The symbiont retains its own module (like
/// mitochondria retaining their own DNA).
pub trait Symbiose {
    /// Evaluate whether integration is more valuable than digestion.
    ///
    /// Called during the DIGEST process when the agent encounters
    /// another agent's output. Returns whether to digest, integrate,
    /// or leave independent.
    fn evaluate_for_symbiosis(&self, other: &AgentProfile) -> SymbiosisEval;

    /// Merge another agent's module into this one as a permanent sub-component.
    ///
    /// The symbiont becomes an internal module. The host gains all
    /// of the symbiont's capabilities without reimplementing them.
    /// The `module_bytes` represent the symbiont's portable WASM module.
    fn integrate_symbiont(
        &mut self,
        profile: AgentProfile,
        module_bytes: Vec<u8>,
    ) -> Result<(), SymbiosisFailure>;

    /// List current symbionts.
    fn symbionts(&self) -> Vec<SymbiontInfo>;

    /// Delegate a task to an integrated symbiont.
    ///
    /// The host passes input to the symbiont and receives output.
    /// Like a eukaryotic cell delegating energy production to its
    /// mitochondria.
    fn delegate_to_symbiont(&self, symbiont_id: &AgentId, input: &[u8]) -> Option<Vec<u8>>;
}
