//! Signal utilities — helpers for working with signals and gradients.

use crate::types::*;

impl Signal {
    /// Create a new signal.
    pub fn new(
        signal_type: SignalType,
        intensity: f64,
        position: Position,
        emitter: AgentId,
        tick: Tick,
    ) -> Self {
        Self {
            signal_type,
            intensity,
            position,
            emitter,
            tick,
        }
    }

    /// Apply decay to this signal's intensity.
    pub fn decay(&mut self, rate: f64) {
        self.intensity *= 1.0 - rate;
    }

    /// Whether this signal has decayed below a threshold.
    pub fn is_below_threshold(&self, threshold: f64) -> bool {
        self.intensity < threshold
    }
}

impl Gradient {
    /// Create a new gradient.
    pub fn new(signal_type: SignalType, direction: Position, magnitude: f64) -> Self {
        Self {
            signal_type,
            direction,
            magnitude,
        }
    }
}

/// Compute the gradient direction from a set of nearby signals.
///
/// Returns the weighted average direction toward higher signal concentration.
/// This is the computational analog of how a cell senses a chemical gradient
/// by comparing receptor binding rates across its surface.
pub fn compute_gradient(signals: &[&Signal], from: &Position) -> Option<Gradient> {
    if signals.is_empty() {
        return None;
    }

    let signal_type = signals[0].signal_type.clone();
    let mut weighted_x = 0.0;
    let mut weighted_y = 0.0;
    let mut total_intensity = 0.0;

    for signal in signals {
        let dx = signal.position.x - from.x;
        let dy = signal.position.y - from.y;
        let dist = (dx * dx + dy * dy).sqrt().max(0.001); // Avoid division by zero

        // Weight by intensity, inversely by distance
        let weight = signal.intensity / dist;
        weighted_x += dx * weight;
        weighted_y += dy * weight;
        total_intensity += signal.intensity;
    }

    if total_intensity < f64::EPSILON {
        return None;
    }

    let magnitude = (weighted_x * weighted_x + weighted_y * weighted_y).sqrt();
    if magnitude < f64::EPSILON {
        return None;
    }

    Some(Gradient::new(
        signal_type,
        Position::new(weighted_x / magnitude, weighted_y / magnitude),
        magnitude,
    ))
}
