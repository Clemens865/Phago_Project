//! APOPTOSE — Programmed Cell Death
//!
//! Apoptosis is genetically programmed, orderly cell death. Unlike necrosis
//! (accidental death causing inflammation), apoptosis is clean: the cell
//! shrinks, packages its contents into membrane-bound bodies, and signals
//! for recycling. No collateral damage.
//!
//! The decision to die is **intrinsic**. The cell evaluates its own state
//! and initiates its own death. In Rust, `trigger_apoptosis` takes `self`
//! by value — the agent is consumed by its own death. The `Drop` trait
//! handles resource cleanup, providing deterministic death without GC.

use crate::types::{CellHealth, DeathSignal};

/// Self-assess integrity and gracefully self-terminate when compromised.
///
/// Every agent must be able to honestly evaluate whether it is still
/// useful to the colony. Agents that are stuck, redundant, or compromised
/// should die — releasing resources and final learnings.
pub trait Apoptose {
    /// Evaluate own health and usefulness.
    ///
    /// This is introspection: the agent examines its own state,
    /// recent output quality, resource consumption, and whether
    /// other agents already cover its function.
    fn self_assess(&self) -> CellHealth;

    /// Whether the agent should initiate death.
    ///
    /// Default implementation triggers death for Compromised,
    /// Redundant, or Senescent states.
    fn should_die(&self) -> bool {
        self.self_assess().should_die()
    }

    /// Package final state into a death signal before destruction.
    ///
    /// This prepares the apoptotic bodies — the final learnings that
    /// other agents can digest. The colony runtime calls this before
    /// dropping the agent. Works with trait objects (`&dyn Apoptose`).
    fn prepare_death_signal(&self) -> DeathSignal;

    /// Initiate orderly self-destruction (for concrete types).
    ///
    /// This method takes `self` by value — the agent is consumed.
    /// After this call, the agent no longer exists. Rust's ownership
    /// system guarantees this — no dangling references, no zombie agents.
    ///
    /// Default implementation calls `prepare_death_signal` then drops self.
    fn trigger_apoptosis(self) -> DeathSignal
    where
        Self: Sized,
    {
        self.prepare_death_signal()
    }
}
