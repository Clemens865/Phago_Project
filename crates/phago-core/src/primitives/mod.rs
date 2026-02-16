//! The ten biological computing primitives.
//!
//! Each primitive is a Rust trait representing a cellular mechanism.
//! Agents implement these traits to gain biological behaviors.

pub mod apoptose;
pub mod digest;
pub mod dissolve;
pub mod emerge;
pub mod negate;
pub mod sense;
pub mod stigmerge;
pub mod symbiose;
pub mod transfer;
pub mod wire;

// Re-export all traits at the primitives level
pub use apoptose::Apoptose;
pub use digest::Digest;
pub use dissolve::Dissolve;
pub use emerge::Emerge;
pub use negate::Negate;
pub use sense::Sense;
pub use stigmerge::Stigmerge;
pub use symbiose::Symbiose;
pub use transfer::Transfer;
pub use wire::Wire;
