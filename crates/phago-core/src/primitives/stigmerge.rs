//! STIGMERGE — Stigmergy
//!
//! Termites build cathedral-like mounds without blueprints or central
//! coordination. Each termite follows a simple rule: if pheromone
//! concentration is high here, deposit material. The deposited material
//! changes the environment, which changes other termites' behavior.
//!
//! **The artifact under construction is simultaneously the product,
//! the plan, and the communication medium.**
//!
//! In Phago, agents deposit traces on the substrate. These traces
//! influence other agents' behavior. The knowledge graph itself becomes
//! the coordination mechanism — no direct agent-to-agent communication needed.

use crate::substrate::Substrate;
use crate::types::*;

/// Coordinate through environmental modification.
///
/// Stigmergy enables indirect coordination: agents modify the substrate,
/// and those modifications guide subsequent agent behavior. The substrate
/// IS the shared plan.
pub trait Stigmerge {
    /// Deposit a trace at a substrate location.
    ///
    /// Like a termite depositing pheromone-laden material. The trace
    /// carries information (type, intensity, payload) that other agents
    /// can read and respond to.
    fn deposit(&self, location: &SubstrateLocation, trace: Trace, substrate: &mut dyn Substrate) {
        substrate.deposit_trace(location, trace);
    }

    /// Read traces at a substrate location.
    ///
    /// Returns all traces deposited at this location, including
    /// traces from other agents. Traces decay over time (like
    /// pheromone evaporation).
    fn read_traces(&self, location: &SubstrateLocation, substrate: &dyn Substrate) -> Vec<Trace> {
        substrate.traces_at(location).into_iter().cloned().collect()
    }

    /// Decide how to respond to traces at the current location.
    ///
    /// High trace density may attract (positive feedback — like ant trails).
    /// Certain trace types may repel (negative feedback — "already explored").
    /// The agent's response creates the feedback loop that drives
    /// self-organization.
    fn respond_to_traces(&self, traces: &[Trace]) -> StigmergicResponse;
}
