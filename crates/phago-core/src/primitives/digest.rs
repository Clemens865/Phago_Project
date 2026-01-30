//! DIGEST — Phagocytosis
//!
//! A macrophage engulfs foreign material, destroys it in a lysosome,
//! and presents fragments on its surface via MHC proteins. Other immune
//! cells read these fragments and mount targeted responses.
//!
//! **Key insight**: Destruction is the primary learning pathway. Every
//! problem consumed becomes a lesson distributed to the network.
//!
//! In Rust, `engulf` takes ownership of the input — it is truly consumed,
//! not copied. This is not a simulation of phagocytosis; Rust's move
//! semantics ARE phagocytosis.

use crate::types::DigestionResult;

/// Consume input, break it down, and present extracted fragments.
///
/// The digestion cycle:
/// 1. **Engulf** — take ownership of input (input ceases to exist)
/// 2. **Lyse** — break engulfed material into structural fragments
/// 3. **Present** — expose fragments for other agents to read
pub trait Digest {
    /// The raw input to consume. Ownership is transferred — the input is destroyed.
    type Input;
    /// Structural fragments extracted during digestion.
    type Fragment;
    /// What is presented to other agents after digestion.
    type Presentation;

    /// Engulf: take ownership of input.
    ///
    /// After this call, the input no longer exists independently.
    /// The agent holds it internally for processing.
    fn engulf(&mut self, input: Self::Input) -> DigestionResult;

    /// Lyse: break the engulfed material into fragments.
    ///
    /// Analogous to lysosomal degradation. The internal material is
    /// broken into structural pieces that can be analyzed and presented.
    fn lyse(&mut self) -> Vec<Self::Fragment>;

    /// Present: expose fragments on the agent's surface.
    ///
    /// Other agents can read this presentation (like T-cells reading
    /// MHC-presented antigens). The presentation is a read-only view.
    fn present(&self) -> Self::Presentation;

    /// Run the full digestion cycle: engulf → lyse → present.
    fn digest(&mut self, input: Self::Input) -> Self::Presentation {
        self.engulf(input);
        self.lyse();
        self.present()
    }
}
