//! NEGATE — Negative Selection
//!
//! During T-cell maturation in the thymus, developing cells are exposed
//! to the body's own proteins (self-antigens). Any T-cell that strongly
//! reacts to self is destroyed. Only cells that **ignore self** and
//! **react to non-self** survive.
//!
//! The immune system doesn't learn what threats look like — that space
//! is infinite. It learns what **self** looks like — that space is finite.
//! Everything else is potentially foreign.
//!
//! This is anomaly detection by exclusion: define normal, flag everything else.

use crate::types::Classification;

/// Define identity through exclusion and detect anomalies.
///
/// The agent builds a model of "self" (normal/expected patterns) and
/// classifies new observations as self (normal) or non-self (anomalous).
/// The self-model is finite and learnable; the threat space is infinite
/// and unknowable.
pub trait Negate {
    /// An observation that can be classified as self or non-self.
    type Observation;

    /// The internal self-model (what the agent considers "normal").
    type SelfModel;

    /// Build or update the self-model from observations.
    ///
    /// During "maturation", the agent is shown normal data and builds
    /// a statistical model of what normal looks like. This is the
    /// training phase — like T-cell education in the thymus.
    fn learn_self(&mut self, observations: &[Self::Observation]);

    /// Get a reference to the current self-model.
    fn self_model(&self) -> &Self::SelfModel;

    /// Whether the self-model has been sufficiently trained.
    ///
    /// An immature detector should not classify — it needs more
    /// training data. This prevents false positives during early life.
    fn is_mature(&self) -> bool;

    /// Classify an observation as self (normal) or non-self (anomalous).
    ///
    /// Returns `Classification::IsSelf` for normal observations,
    /// `Classification::NonSelf(deviation)` for anomalies (with degree),
    /// or `Classification::Unknown` if the self-model is insufficient.
    fn classify(&self, observation: &Self::Observation) -> Classification;
}
