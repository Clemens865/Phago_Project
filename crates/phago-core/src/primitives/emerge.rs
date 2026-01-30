//! EMERGE — Quorum Sensing + Phase Transitions
//!
//! Individual bacteria are weak. But when population density exceeds a
//! threshold, they detect each other's autoinducer molecules and undergo
//! coordinated behavioral changes: biofilm formation, bioluminescence,
//! virulence factor production.
//!
//! This is a **phase transition** — a qualitative change triggered by a
//! quantitative threshold. The collective exhibits behaviors that no
//! individual possesses.

use crate::substrate::Substrate;
use crate::types::*;

/// Detect quorum and activate collective behaviors.
///
/// Emergence is discrete, not gradual. Below the threshold, agents
/// operate individually. Above it, collective behaviors unlock that
/// no single agent can perform.
pub trait Emerge {
    /// The collective behavior that emerges at quorum.
    type EmergentBehavior;

    /// Measure signal density in the local region.
    ///
    /// This is the agent counting autoinducer concentration —
    /// how many nearby agents are emitting presence signals.
    fn signal_density(&self, substrate: &dyn Substrate) -> f64;

    /// The threshold for phase transition.
    ///
    /// This can be fixed or adaptive (shifting based on environment).
    fn quorum_threshold(&self) -> f64;

    /// Whether quorum has been reached.
    fn quorum_reached(&self, substrate: &dyn Substrate) -> bool {
        self.signal_density(substrate) >= self.quorum_threshold()
    }

    /// The emergent behavior that activates at quorum.
    ///
    /// Returns `None` if quorum is not reached. The behavior itself
    /// is a collective computation — it requires contributions from
    /// multiple agents and cannot be performed by any single one.
    fn emergent_behavior(&self) -> Option<Self::EmergentBehavior>;

    /// Contribute to a collective computation.
    ///
    /// Each agent contributes its local perspective. The collective
    /// synthesizes these into something none of them could produce alone.
    fn contribute(&self) -> Contribution;
}
