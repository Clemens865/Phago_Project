//! SENSE — Chemotaxis
//!
//! Cells navigate by sensing chemical gradients. A neutrophil detects
//! increasing concentrations of inflammatory signals and moves toward
//! the source. The cell doesn't know where the problem is — it follows
//! the gradient.
//!
//! Agents also emit signals (autoinducers) for quorum sensing. When
//! enough agents emit presence signals, the collective detects its own
//! density and can trigger phase transitions.

use crate::substrate::Substrate;
use crate::types::*;

/// Detect environmental signals, compute gradients, and emit signals.
///
/// Sensing is local, not global. An agent only perceives signals within
/// its sensing radius — it has no access to the full substrate state.
/// Navigation emerges from following local gradients, not from knowing
/// the global map.
pub trait Sense {
    /// The radius within which this agent can sense signals.
    fn sense_radius(&self) -> f64;

    /// Read signals from the local environment.
    ///
    /// Returns only signals within `sense_radius` of the agent's position.
    fn sense(&self, substrate: &dyn Substrate) -> Vec<Signal> {
        let position = self.sense_position();
        let radius = self.sense_radius();
        substrate
            .signals_near(&position, radius)
            .into_iter()
            .cloned()
            .collect()
    }

    /// The position from which this agent senses.
    fn sense_position(&self) -> Position;

    /// Compute the gradient — direction of increasing signal strength.
    ///
    /// Like a cell comparing receptor binding rates across its surface.
    fn gradient(&self, substrate: &dyn Substrate) -> Vec<Gradient>;

    /// Emit a signal into the substrate.
    ///
    /// Used for quorum sensing (presence signals), alerting
    /// (anomaly signals), and coordination (capability signals).
    fn emit(&self, signal: Signal, substrate: &mut dyn Substrate) {
        substrate.emit_signal(signal);
    }

    /// Determine next orientation based on sensed gradients.
    ///
    /// Returns where the agent should move or what it should prioritize.
    fn orient(&self, gradients: &[Gradient]) -> Orientation;
}
