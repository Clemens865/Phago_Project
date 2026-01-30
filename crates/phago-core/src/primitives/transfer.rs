//! TRANSFER — Horizontal Gene Transfer
//!
//! Bacteria share genetic material across species boundaries through:
//! - **Conjugation**: direct cell-to-cell transfer via pili
//! - **Transformation**: uptake of free DNA from the environment
//! - **Transduction**: transfer via bacteriophage intermediaries
//!
//! This is how antibiotic resistance spreads rapidly. In Phago, agents
//! export learned capabilities as portable WASM modules that other agents
//! can integrate at runtime — instant skill acquisition from strangers.

use crate::types::*;

/// Export and import capabilities across agent boundaries.
///
/// Capabilities are portable WASM modules — truly cross-agent, cross-type.
/// Not all transfers succeed; incompatibility is expected and biological.
pub trait Transfer {
    /// A portable capability (conceptually a WASM module).
    type Capability;

    /// List capabilities this agent can export.
    fn available_capabilities(&self) -> Vec<CapabilityDescriptor>;

    /// Export a capability as a portable module.
    ///
    /// The exported capability can be deposited in the substrate
    /// (transformation), passed directly to another agent (conjugation),
    /// or relayed through a third party (transduction).
    fn export_capability(&self, id: &CapabilityId) -> Option<Self::Capability>;

    /// Evaluate whether a foreign capability is compatible.
    ///
    /// Like immune compatibility — not all genes can be integrated
    /// into all organisms.
    fn evaluate_foreign(&self, cap: &CapabilityDescriptor) -> Compatibility;

    /// Integrate a foreign capability into this agent.
    ///
    /// If successful, the agent permanently gains the new capability.
    /// If rejected, returns the reason for incompatibility.
    fn integrate(&mut self, capability: Self::Capability) -> Result<(), RejectionReason>;
}
