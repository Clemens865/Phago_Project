//! Substrate — the shared environment all agents operate within.
//!
//! The substrate is the extracellular matrix: it holds the signal field,
//! the knowledge graph, and trace deposits. Agents read from and write to
//! the substrate, but never directly to each other.

use crate::types::*;

/// The shared environment that all agents sense and modify.
///
/// The substrate is the computational analog of the extracellular matrix
/// in biology. It holds:
/// - A **signal field** for chemotaxis (agents sense gradients)
/// - A **knowledge graph** for stigmergy (agents deposit and read structure)
/// - **Trace storage** for indirect coordination
pub trait Substrate {
    // --- Signal field ---

    /// Read all signals within a radius of a position.
    fn signals_near(&self, position: &Position, radius: f64) -> Vec<&Signal>;

    /// Emit a signal into the substrate.
    fn emit_signal(&mut self, signal: Signal);

    /// Decay all signals by a rate (0.0-1.0). Signals below threshold are removed.
    fn decay_signals(&mut self, rate: f64, removal_threshold: f64);

    // --- Knowledge graph ---

    /// Add a node to the knowledge graph.
    fn add_node(&mut self, data: NodeData) -> NodeId;

    /// Get a node by ID.
    fn get_node(&self, id: &NodeId) -> Option<&NodeData>;

    /// Add or update an edge between two nodes.
    fn set_edge(&mut self, from: NodeId, to: NodeId, data: EdgeData);

    /// Get edge data between two nodes.
    fn get_edge(&self, from: &NodeId, to: &NodeId) -> Option<&EdgeData>;

    /// Get all neighbors of a node with their edge data.
    fn neighbors(&self, node: &NodeId) -> Vec<(NodeId, &EdgeData)>;

    /// Remove an edge.
    fn remove_edge(&mut self, from: &NodeId, to: &NodeId);

    /// Get all node IDs.
    fn all_nodes(&self) -> Vec<NodeId>;

    /// Get all edges as (from, to, data) triples.
    fn all_edges(&self) -> Vec<(NodeId, NodeId, &EdgeData)>;

    /// Number of nodes in the graph.
    fn node_count(&self) -> usize;

    /// Number of edges in the graph.
    fn edge_count(&self) -> usize;

    // --- Trace storage ---

    /// Deposit a trace at a location.
    fn deposit_trace(&mut self, location: &SubstrateLocation, trace: Trace);

    /// Read traces at a location.
    fn traces_at(&self, location: &SubstrateLocation) -> Vec<&Trace>;

    /// Decay all traces by a rate. Traces below threshold are removed.
    fn decay_traces(&mut self, rate: f64, removal_threshold: f64);

    // --- Document storage ---

    /// Place a document in the substrate.
    fn add_document(&mut self, doc: Document);

    /// Get a document by ID.
    fn get_document(&self, id: &DocumentId) -> Option<&Document>;

    /// Get all undigested documents.
    fn undigested_documents(&self) -> Vec<&Document>;

    /// Mark a document as digested and return its content.
    fn consume_document(&mut self, id: &DocumentId) -> Option<String>;

    /// Get all documents.
    fn all_documents(&self) -> Vec<&Document>;

    // --- Lifecycle ---

    /// Current simulation tick.
    fn current_tick(&self) -> Tick;

    /// Advance the tick counter.
    fn advance_tick(&mut self);
}
