//! Spike-Timing-Dependent Plasticity (STDP) — directed temporal edges.
//!
//! Maintains a parallel directed graph alongside the existing undirected
//! co-occurrence graph. Directed edges encode temporal/causal relationships:
//! "what comes next?" rather than "what co-occurs?"
//!
//! # How It Works
//!
//! When nodes are activated in sequence [A, B, C], STDP creates directed
//! edges A→B and B→C with weights that decay exponentially with temporal
//! distance. Repeated sequences strengthen these predictive edges.
//!
//! The existing undirected graph is untouched — STDP is an additive layer.

use petgraph::graph::{DiGraph, NodeIndex};
use phago_core::types::NodeId;
use std::collections::HashMap;

/// A directed edge in the STDP graph.
#[derive(Debug, Clone)]
pub struct DirectedEdge {
    /// Predictive weight (higher = stronger temporal association).
    pub weight: f64,
    /// Number of times this sequence was observed.
    pub count: u64,
    /// Tick when this edge was last reinforced.
    pub last_reinforced_tick: u64,
}

impl DirectedEdge {
    fn new(weight: f64, tick: u64) -> Self {
        Self {
            weight,
            count: 1,
            last_reinforced_tick: tick,
        }
    }
}

/// STDP configuration.
pub struct StdpConfig {
    /// Maximum temporal window for STDP wiring (in sequence positions).
    /// Nodes more than `window` positions apart won't be wired.
    pub window: usize,
    /// Base weight for a direct successor (distance=1).
    pub base_weight: f64,
    /// Exponential decay factor per position of distance.
    pub distance_decay: f64,
    /// Reinforcement factor for existing edges.
    pub reinforcement: f64,
    /// Decay rate per tick for edge weights.
    pub decay_rate: f64,
    /// Prune edges below this weight threshold.
    pub prune_threshold: f64,
}

impl Default for StdpConfig {
    fn default() -> Self {
        Self {
            window: 3,
            base_weight: 1.0,
            distance_decay: 0.5,
            reinforcement: 0.2,
            decay_rate: 0.01,
            prune_threshold: 0.05,
        }
    }
}

/// Directed temporal graph for STDP predictions.
pub struct StdpGraph {
    graph: DiGraph<NodeId, DirectedEdge>,
    node_index: HashMap<NodeId, NodeIndex>,
    config: StdpConfig,
}

impl StdpGraph {
    /// Create a new empty STDP graph.
    pub fn new() -> Self {
        Self::with_config(StdpConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: StdpConfig) -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
            config,
        }
    }

    /// Ensure a node exists in the directed graph.
    fn ensure_node(&mut self, id: NodeId) -> NodeIndex {
        if let Some(&idx) = self.node_index.get(&id) {
            idx
        } else {
            let idx = self.graph.add_node(id);
            self.node_index.insert(id, idx);
            idx
        }
    }

    /// Apply STDP wiring to an activation sequence.
    ///
    /// Given a sequence of node activations [A, B, C, ...], creates directed
    /// edges between nodes within the temporal window. Existing edges are
    /// reinforced rather than replaced.
    pub fn apply_sequence(&mut self, sequence: &[NodeId], tick: u64) {
        if sequence.len() < 2 {
            return;
        }

        for i in 0..sequence.len() {
            let from = sequence[i];
            let from_idx = self.ensure_node(from);

            // Wire to successors within the window
            let end = (i + 1 + self.config.window).min(sequence.len());
            for j in (i + 1)..end {
                let to = sequence[j];
                let to_idx = self.ensure_node(to);
                let distance = (j - i) as f64;

                // Weight decays exponentially with distance
                let weight =
                    self.config.base_weight * self.config.distance_decay.powf(distance - 1.0);

                // Check if edge already exists
                if let Some(edge_idx) = self.graph.find_edge(from_idx, to_idx) {
                    let edge = &mut self.graph[edge_idx];
                    edge.weight += weight * self.config.reinforcement;
                    edge.count += 1;
                    edge.last_reinforced_tick = tick;
                } else {
                    self.graph
                        .add_edge(from_idx, to_idx, DirectedEdge::new(weight, tick));
                }
            }
        }
    }

    /// Get successors of a node (what comes after it?).
    ///
    /// Returns (NodeId, weight) pairs sorted by descending weight.
    pub fn successors(&self, node: &NodeId) -> Vec<(NodeId, f64)> {
        let Some(&idx) = self.node_index.get(node) else {
            return vec![];
        };

        let mut results: Vec<(NodeId, f64)> = self
            .graph
            .neighbors_directed(idx, petgraph::Direction::Outgoing)
            .filter_map(|neighbor_idx| {
                let edge_idx = self.graph.find_edge(idx, neighbor_idx)?;
                let edge = &self.graph[edge_idx];
                let target_id = self.graph[neighbor_idx];
                Some((target_id, edge.weight))
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Get predecessors of a node (what comes before it?).
    pub fn predecessors(&self, node: &NodeId) -> Vec<(NodeId, f64)> {
        let Some(&idx) = self.node_index.get(node) else {
            return vec![];
        };

        let mut results: Vec<(NodeId, f64)> = self
            .graph
            .neighbors_directed(idx, petgraph::Direction::Incoming)
            .filter_map(|neighbor_idx| {
                let edge_idx = self.graph.find_edge(neighbor_idx, idx)?;
                let edge = &self.graph[edge_idx];
                let source_id = self.graph[neighbor_idx];
                Some((source_id, edge.weight))
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Find directed shortest path from `from` to `to`.
    ///
    /// Uses inverse weight as cost (stronger edges = cheaper paths).
    pub fn directed_shortest_path(&self, from: &NodeId, to: &NodeId) -> Option<(Vec<NodeId>, f64)> {
        let from_idx = *self.node_index.get(from)?;
        let to_idx = *self.node_index.get(to)?;

        // Dijkstra with inverse weight
        let costs = petgraph::algo::dijkstra(&self.graph, from_idx, Some(to_idx), |e| {
            let w = e.weight().weight;
            if w > 0.0 {
                1.0 / w
            } else {
                f64::INFINITY
            }
        });

        let cost = *costs.get(&to_idx)?;
        if cost.is_infinite() {
            return None;
        }

        // Reconstruct path using BFS (Dijkstra doesn't return predecessors)
        let path = self.reconstruct_path(from_idx, to_idx)?;
        let node_ids: Vec<NodeId> = path.iter().map(|&idx| self.graph[idx]).collect();

        Some((node_ids, cost))
    }

    /// Reconstruct path using BFS (simple approach for small graphs).
    fn reconstruct_path(&self, from: NodeIndex, to: NodeIndex) -> Option<Vec<NodeIndex>> {
        use std::collections::VecDeque;

        let mut visited = HashMap::new();
        let mut queue = VecDeque::new();
        queue.push_back(from);
        visited.insert(from, None);

        while let Some(current) = queue.pop_front() {
            if current == to {
                // Reconstruct path
                let mut path = vec![to];
                let mut node = to;
                while let Some(Some(prev)) = visited.get(&node) {
                    path.push(*prev);
                    node = *prev;
                }
                path.reverse();
                return Some(path);
            }

            for neighbor in self
                .graph
                .neighbors_directed(current, petgraph::Direction::Outgoing)
            {
                if !visited.contains_key(&neighbor) {
                    visited.insert(neighbor, Some(current));
                    queue.push_back(neighbor);
                }
            }
        }

        None
    }

    /// Decay all edge weights and prune weak edges.
    pub fn decay_and_prune(&mut self, current_tick: u64) -> usize {
        let mut to_remove = vec![];

        for edge_idx in self.graph.edge_indices() {
            let edge = &mut self.graph[edge_idx];
            let ticks_since = current_tick.saturating_sub(edge.last_reinforced_tick);
            edge.weight *= (1.0 - self.config.decay_rate).powi(ticks_since as i32);

            if edge.weight < self.config.prune_threshold {
                to_remove.push(edge_idx);
            }
        }

        let pruned = to_remove.len();
        // Remove in reverse order to keep indices valid
        to_remove.sort_by(|a, b| b.cmp(a));
        for idx in to_remove {
            self.graph.remove_edge(idx);
        }
        pruned
    }

    /// Number of directed edges.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Number of nodes in the directed graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Set a directed edge explicitly.
    pub fn set_directed_edge(&mut self, from: NodeId, to: NodeId, weight: f64, tick: u64) {
        let from_idx = self.ensure_node(from);
        let to_idx = self.ensure_node(to);

        if let Some(edge_idx) = self.graph.find_edge(from_idx, to_idx) {
            let edge = &mut self.graph[edge_idx];
            edge.weight = weight;
            edge.last_reinforced_tick = tick;
        } else {
            self.graph
                .add_edge(from_idx, to_idx, DirectedEdge::new(weight, tick));
        }
    }
}

impl Default for StdpGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(seed: u64) -> NodeId {
        NodeId::from_seed(seed)
    }

    #[test]
    fn sequence_creates_directed_edges() {
        let mut stdp = StdpGraph::new();
        let a = node(1);
        let b = node(2);
        let c = node(3);

        stdp.apply_sequence(&[a, b, c], 0);

        // A→B should exist
        let succ_a = stdp.successors(&a);
        assert!(
            succ_a.iter().any(|(id, _)| *id == b),
            "A should have successor B"
        );

        // B→C should exist
        let succ_b = stdp.successors(&b);
        assert!(
            succ_b.iter().any(|(id, _)| *id == c),
            "B should have successor C"
        );

        // C should have no successors
        assert!(stdp.successors(&c).is_empty());

        // Predecessors
        let pred_c = stdp.predecessors(&c);
        assert!(
            pred_c.iter().any(|(id, _)| *id == b),
            "C should have predecessor B"
        );
    }

    #[test]
    fn directed_shortest_path() {
        // Use window=1 so only direct successors are wired: A→B, B→C
        let config = StdpConfig {
            window: 1,
            ..Default::default()
        };
        let mut stdp = StdpGraph::with_config(config);
        let a = node(1);
        let b = node(2);
        let c = node(3);

        stdp.apply_sequence(&[a, b, c], 0);

        let path = stdp.directed_shortest_path(&a, &c);
        assert!(path.is_some(), "Should find path A→B→C");
        let (nodes, _cost) = path.unwrap();
        assert_eq!(nodes, vec![a, b, c]);
    }

    #[test]
    fn reinforcement_strengthens_edges() {
        let mut stdp = StdpGraph::new();
        let a = node(1);
        let b = node(2);

        stdp.apply_sequence(&[a, b], 0);
        let w1 = stdp.successors(&a)[0].1;

        stdp.apply_sequence(&[a, b], 1);
        let w2 = stdp.successors(&a)[0].1;

        assert!(w2 > w1, "Repeated sequence should strengthen edge");
    }

    #[test]
    fn decay_weakens_edges() {
        let mut stdp = StdpGraph::new();
        let a = node(1);
        let b = node(2);

        stdp.apply_sequence(&[a, b], 0);
        let initial_count = stdp.edge_count();
        assert_eq!(initial_count, 1);

        // Decay after many ticks
        let pruned = stdp.decay_and_prune(1000);

        // With default config (0.01 decay rate, 0.05 threshold), after 1000 ticks
        // the edge should be pruned
        assert!(
            pruned > 0 || stdp.edge_count() < initial_count || {
                // Check if weight is significantly reduced
                let succ = stdp.successors(&a);
                succ.is_empty() || succ[0].1 < 0.1
            }
        );
    }

    #[test]
    fn no_reverse_edge() {
        let mut stdp = StdpGraph::new();
        let a = node(1);
        let b = node(2);

        stdp.apply_sequence(&[a, b], 0);

        // B→A should NOT exist (directed)
        let path_ba = stdp.directed_shortest_path(&b, &a);
        assert!(path_ba.is_none(), "B→A should not exist (directed graph)");
    }

    #[test]
    fn window_limits_range() {
        let config = StdpConfig {
            window: 1, // Only direct successors
            ..Default::default()
        };
        let mut stdp = StdpGraph::with_config(config);
        let a = node(1);
        let b = node(2);
        let c = node(3);

        stdp.apply_sequence(&[a, b, c], 0);

        // A→C should NOT exist with window=1
        let succ_a = stdp.successors(&a);
        assert!(
            !succ_a.iter().any(|(id, _)| *id == c),
            "Window=1 should not create A→C edge"
        );
    }
}
