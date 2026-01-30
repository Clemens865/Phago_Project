//! WIRE — Hebbian Learning + Synaptic Pruning
//!
//! "Neurons that fire together, wire together." Synaptic connections that
//! are frequently used grow stronger (long-term potentiation). Connections
//! that are rarely used weaken and are pruned (synaptic pruning).
//!
//! The critical insight: **the structure IS the memory**. The network
//! doesn't store knowledge in nodes and query through edges. The edges
//! themselves — their weights, their topology — encode what the system
//! has learned. Topology is knowledge.

use crate::topology::TopologyGraph;
use crate::types::*;

/// Strengthen used connections and prune unused ones.
///
/// Wire operates on the shared topology graph in the substrate.
/// Multiple agents read and modify the same graph — wiring is a
/// collective activity, like neural plasticity across a brain region.
pub trait Wire {
    /// Strengthen the connection between two nodes.
    ///
    /// Called when two concepts are observed together (co-activation).
    /// The weight increase is proportional to the strength parameter.
    fn strengthen(&self, from: NodeId, to: NodeId, weight: f64, graph: &mut dyn TopologyGraph);

    /// Record a co-activation event for a set of nodes.
    ///
    /// When multiple concepts appear together (e.g., in the same document),
    /// all pairwise connections are strengthened. This is Hebbian learning:
    /// nodes that activate together wire together.
    fn co_activate(&self, nodes: &[NodeId], graph: &mut dyn TopologyGraph);

    /// Prune connections below a weight threshold.
    ///
    /// Weak connections are removed — synaptic pruning. This prevents
    /// the graph from growing without bound and ensures that only
    /// genuinely related concepts remain connected.
    fn prune(&self, threshold: f64, graph: &mut dyn TopologyGraph) -> Vec<PrunedConnection>;

    /// Decay all connection weights.
    ///
    /// Time-based weakening: all connections lose strength unless
    /// they are re-activated. This ensures the graph reflects recent
    /// relevance, not just historical co-occurrence.
    fn decay(&self, rate: f64, graph: &mut dyn TopologyGraph);
}
