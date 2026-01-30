//! Concrete implementation of the Substrate trait.
//!
//! In-memory substrate with:
//! - Signal field stored as a Vec (linear scan with distance filtering)
//! - Knowledge graph backed by PetTopologyGraph
//! - Trace storage as a HashMap keyed by SubstrateLocation
//! - Serialization support for persistence across restarts

use crate::topology_impl::PetTopologyGraph;
use phago_core::substrate::Substrate;
use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// In-memory substrate implementation.
///
/// The substrate is the extracellular matrix — the shared environment
/// all agents sense and modify. It holds signals (for chemotaxis),
/// a knowledge graph (for stigmergy and Hebbian wiring), and traces
/// (for indirect coordination).
pub struct SubstrateImpl {
    signals: Vec<Signal>,
    graph: PetTopologyGraph,
    traces: HashMap<TraceLocationKey, Vec<Trace>>,
    documents: HashMap<DocumentId, Document>,
    tick: Tick,
}

/// Key for trace storage. We need something hashable for SubstrateLocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum TraceLocationKey {
    Spatial(OrderedPosition),
    GraphNode(NodeId),
}

/// Position with Eq/Hash for use as HashMap key (quantized to grid).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct OrderedPosition {
    /// Quantized to 0.1 grid units for hashing.
    x: i64,
    y: i64,
}

impl From<&Position> for OrderedPosition {
    fn from(p: &Position) -> Self {
        Self {
            x: (p.x * 10.0).round() as i64,
            y: (p.y * 10.0).round() as i64,
        }
    }
}

impl From<&SubstrateLocation> for TraceLocationKey {
    fn from(loc: &SubstrateLocation) -> Self {
        match loc {
            SubstrateLocation::Spatial(pos) => TraceLocationKey::Spatial(OrderedPosition::from(pos)),
            SubstrateLocation::GraphNode(id) => TraceLocationKey::GraphNode(*id),
        }
    }
}

impl SubstrateImpl {
    pub fn new() -> Self {
        Self {
            signals: Vec::new(),
            graph: PetTopologyGraph::new(),
            traces: HashMap::new(),
            documents: HashMap::new(),
            tick: 0,
        }
    }

    /// Get a document by ID (convenience method bypassing trait).
    pub fn get_document(&self, id: &DocumentId) -> Option<&Document> {
        self.documents.get(id)
    }

    /// Get all documents (convenience method).
    pub fn all_documents(&self) -> Vec<&Document> {
        self.documents.values().collect()
    }

    /// Get a reference to the underlying topology graph.
    pub fn graph(&self) -> &PetTopologyGraph {
        &self.graph
    }

    /// Get a mutable reference to the underlying topology graph.
    pub fn graph_mut(&mut self) -> &mut PetTopologyGraph {
        &mut self.graph
    }

    /// Get all signals (for diagnostics/visualization).
    pub fn all_signals(&self) -> &[Signal] {
        &self.signals
    }

    /// Total number of traces across all locations.
    pub fn total_trace_count(&self) -> usize {
        self.traces.values().map(|v| v.len()).sum()
    }
}

impl Default for SubstrateImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl Substrate for SubstrateImpl {
    // --- Signal field ---

    fn signals_near(&self, position: &Position, radius: f64) -> Vec<&Signal> {
        let r2 = radius * radius;
        self.signals
            .iter()
            .filter(|s| {
                let dx = s.position.x - position.x;
                let dy = s.position.y - position.y;
                dx * dx + dy * dy <= r2
            })
            .collect()
    }

    fn emit_signal(&mut self, signal: Signal) {
        self.signals.push(signal);
    }

    fn decay_signals(&mut self, rate: f64, removal_threshold: f64) {
        for signal in &mut self.signals {
            signal.decay(rate);
        }
        self.signals
            .retain(|s| !s.is_below_threshold(removal_threshold));
    }

    // --- Knowledge graph ---

    fn add_node(&mut self, data: NodeData) -> NodeId {
        self.graph.add_node(data)
    }

    fn get_node(&self, id: &NodeId) -> Option<&NodeData> {
        self.graph.get_node(id)
    }

    fn set_edge(&mut self, from: NodeId, to: NodeId, data: EdgeData) {
        self.graph.set_edge(from, to, data);
    }

    fn get_edge(&self, from: &NodeId, to: &NodeId) -> Option<&EdgeData> {
        self.graph.get_edge(from, to)
    }

    fn neighbors(&self, node: &NodeId) -> Vec<(NodeId, &EdgeData)> {
        self.graph.neighbors(node)
    }

    fn remove_edge(&mut self, from: &NodeId, to: &NodeId) {
        self.graph.remove_edge(from, to);
    }

    fn all_nodes(&self) -> Vec<NodeId> {
        self.graph.all_nodes()
    }

    fn all_edges(&self) -> Vec<(NodeId, NodeId, &EdgeData)> {
        self.graph.all_edges()
    }

    fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    // --- Trace storage ---

    fn deposit_trace(&mut self, location: &SubstrateLocation, trace: Trace) {
        let key = TraceLocationKey::from(location);
        self.traces.entry(key).or_default().push(trace);
    }

    fn traces_at(&self, location: &SubstrateLocation) -> Vec<&Trace> {
        let key = TraceLocationKey::from(location);
        self.traces
            .get(&key)
            .map(|traces| traces.iter().collect())
            .unwrap_or_default()
    }

    fn decay_traces(&mut self, rate: f64, removal_threshold: f64) {
        for traces in self.traces.values_mut() {
            for trace in traces.iter_mut() {
                trace.intensity *= 1.0 - rate;
            }
            traces.retain(|t| t.intensity >= removal_threshold);
        }
        // Remove empty locations
        self.traces.retain(|_, v| !v.is_empty());
    }

    // --- Document storage ---

    fn add_document(&mut self, doc: Document) {
        self.documents.insert(doc.id, doc);
    }

    fn get_document(&self, id: &DocumentId) -> Option<&Document> {
        self.documents.get(id)
    }

    fn undigested_documents(&self) -> Vec<&Document> {
        self.documents.values().filter(|d| !d.digested).collect()
    }

    fn consume_document(&mut self, id: &DocumentId) -> Option<String> {
        if let Some(doc) = self.documents.get_mut(id) {
            if !doc.digested {
                doc.digested = true;
                return Some(doc.content.clone());
            }
        }
        None
    }

    fn all_documents(&self) -> Vec<&Document> {
        self.documents.values().collect()
    }

    // --- Lifecycle ---

    fn current_tick(&self) -> Tick {
        self.tick
    }

    fn advance_tick(&mut self) {
        self.tick += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_signal(x: f64, y: f64, intensity: f64) -> Signal {
        Signal::new(
            SignalType::Input,
            intensity,
            Position::new(x, y),
            AgentId::new(),
            0,
        )
    }

    #[test]
    fn signals_near_filters_by_distance() {
        let mut sub = SubstrateImpl::new();
        sub.emit_signal(make_signal(1.0, 1.0, 1.0)); // Close
        sub.emit_signal(make_signal(100.0, 100.0, 1.0)); // Far

        let near = sub.signals_near(&Position::new(0.0, 0.0), 5.0);
        assert_eq!(near.len(), 1);
    }

    #[test]
    fn signal_decay_removes_weak_signals() {
        let mut sub = SubstrateImpl::new();
        sub.emit_signal(make_signal(0.0, 0.0, 1.0));
        sub.emit_signal(make_signal(1.0, 1.0, 0.05));

        // Decay by 50%, remove below 0.04
        sub.decay_signals(0.5, 0.04);
        assert_eq!(sub.all_signals().len(), 1); // Only the strong one survives
    }

    #[test]
    fn trace_deposit_and_retrieve() {
        let mut sub = SubstrateImpl::new();
        let loc = SubstrateLocation::Spatial(Position::new(5.0, 5.0));
        let trace = Trace {
            agent_id: AgentId::new(),
            trace_type: TraceType::Digestion,
            intensity: 1.0,
            tick: 0,
            payload: vec![],
        };
        sub.deposit_trace(&loc, trace);

        let traces = sub.traces_at(&loc);
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].trace_type, TraceType::Digestion);
    }

    #[test]
    fn trace_decay_removes_weak_traces() {
        let mut sub = SubstrateImpl::new();
        let loc = SubstrateLocation::Spatial(Position::new(0.0, 0.0));
        sub.deposit_trace(&loc, Trace {
            agent_id: AgentId::new(),
            trace_type: TraceType::Visit,
            intensity: 1.0,
            tick: 0,
            payload: vec![],
        });
        sub.deposit_trace(&loc, Trace {
            agent_id: AgentId::new(),
            trace_type: TraceType::Visit,
            intensity: 0.02,
            tick: 0,
            payload: vec![],
        });

        sub.decay_traces(0.5, 0.02);
        // Strong trace decays to 0.5, weak to 0.01 (removed)
        assert_eq!(sub.traces_at(&loc).len(), 1);
    }

    #[test]
    fn graph_operations_through_substrate() {
        let mut sub = SubstrateImpl::new();

        let n1 = sub.add_node(NodeData {
            id: NodeId::new(),
            label: "cell".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(0.0, 0.0),
            access_count: 0,
            created_tick: 0,
        });
        let n2 = sub.add_node(NodeData {
            id: NodeId::new(),
            label: "membrane".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(1.0, 0.0),
            access_count: 0,
            created_tick: 0,
        });

        sub.set_edge(n1, n2, EdgeData {
            weight: 0.8,
            co_activations: 1,
            created_tick: 0,
            last_activated_tick: 0,
        });

        assert_eq!(sub.node_count(), 2);
        assert_eq!(sub.edge_count(), 1);
        assert_eq!(sub.get_node(&n1).unwrap().label, "cell");
    }

    #[test]
    fn tick_advances() {
        let mut sub = SubstrateImpl::new();
        assert_eq!(sub.current_tick(), 0);
        sub.advance_tick();
        sub.advance_tick();
        assert_eq!(sub.current_tick(), 2);
    }
}
