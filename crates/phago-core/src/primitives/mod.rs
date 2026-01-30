//! The ten biological computing primitives.
//!
//! Each primitive is a Rust trait representing a cellular mechanism.
//! Agents implement these traits to gain biological behaviors.

pub mod digest;
pub mod apoptose;
pub mod sense;
pub mod transfer;
pub mod emerge;
pub mod wire;
pub mod symbiose;
pub mod stigmerge;
pub mod negate;
pub mod dissolve;

// Re-export all traits at the primitives level
pub use digest::Digest;
pub use apoptose::Apoptose;
pub use sense::Sense;
pub use transfer::Transfer;
pub use emerge::Emerge;
pub use wire::Wire;
pub use symbiose::Symbiose;
pub use stigmerge::Stigmerge;
pub use negate::Negate;
pub use dissolve::Dissolve;
