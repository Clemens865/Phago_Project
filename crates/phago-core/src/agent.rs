//! Agent — the fundamental computational cell.
//!
//! Every agent in Phago is a cell: an isolated unit that senses its
//! environment, processes inputs, and acts on the substrate. The base
//! Agent trait requires the three most fundamental biological capabilities:
//! - DIGEST: the ability to consume and process input
//! - APOPTOSE: the ability to self-assess and gracefully die
//! - SENSE: the ability to detect environmental signals

use crate::primitives::{Apoptose, Digest, Sense};
use crate::substrate::Substrate;
use crate::types::*;

/// The fundamental unit of computation in Phago — a biological cell.
///
/// Every agent must be able to:
/// - **Digest** input (consume, break down, present fragments)
/// - **Apoptose** (self-assess health, gracefully die when compromised)
/// - **Sense** the substrate (detect signals, follow gradients)
///
/// Additional biological capabilities (Wire, Transfer, Emerge, etc.)
/// are opt-in through additional trait implementations.
pub trait Agent: Digest + Apoptose + Sense {
    /// The agent's unique identity.
    fn id(&self) -> AgentId;

    /// The agent's current position in the substrate.
    fn position(&self) -> Position;

    /// Set the agent's position.
    fn set_position(&mut self, position: Position);

    /// The agent's type name (for display and logging).
    fn agent_type(&self) -> &str;

    /// Execute one tick of the agent's lifecycle.
    ///
    /// This is the main loop body. Each tick, the agent:
    /// 1. Senses the environment
    /// 2. Decides what to do
    /// 3. Returns an action for the runtime to execute
    ///
    /// The runtime calls this once per simulation tick.
    fn tick(&mut self, substrate: &dyn Substrate) -> AgentAction;

    /// How many ticks this agent has been alive.
    fn age(&self) -> Tick;
}
