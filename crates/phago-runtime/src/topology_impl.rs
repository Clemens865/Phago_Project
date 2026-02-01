//! Concrete implementation of the TopologyGraph trait using petgraph.
//!
//! The knowledge graph is the substrate's structural backbone.
//! This implementation uses petgraph's `Graph` as the backing store
//! with HashMap indices for O(1) node/edge lookup by ID.

use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use std::collections::HashMap;

/// Petgraph-backed implementation of the topology graph.
pub struct PetTopologyGraph {
    graph: Graph<NodeData, EdgeData, petgraph::Undirected>,
    /// Map from our NodeId to petgraph's internal index.
    node_index: HashMap<NodeId, NodeIndex>,
    /// Index from lowercase label to node IDs for O(1) exact lookup.
    label_index: HashMap<String, Vec<NodeId>>,
}

impl PetTopologyGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new_undirected(),
            node_index: HashMap::new(),
            label_index: HashMap::new(),
        }
    }

    /// O(1) exact label lookup (case-insensitive).
    pub fn find_nodes_by_exact_label(&self, label: &str) -> &[NodeId] {
        self.label_index
            .get(&label.to_lowercase())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

impl Default for PetTopologyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl TopologyGraph for PetTopologyGraph {
    fn add_node(&mut self, data: NodeData) -> NodeId {
        let id = data.id;
        let label_key = data.label.to_lowercase();
        let idx = self.graph.add_node(data);
        self.node_index.insert(id, idx);
        self.label_index.entry(label_key).or_default().push(id);
        id
    }

    fn get_node(&self, id: &NodeId) -> Option<&NodeData> {
        self.node_index.get(id).map(|idx| &self.graph[*idx])
    }

    fn get_node_mut(&mut self, id: &NodeId) -> Option<&mut NodeData> {
        self.node_index
            .get(id)
            .copied()
            .map(|idx| &mut self.graph[idx])
    }

    fn set_edge(&mut self, from: NodeId, to: NodeId, data: EdgeData) {
        let Some(&from_idx) = self.node_index.get(&from) else {
            return;
        };
        let Some(&to_idx) = self.node_index.get(&to) else {
            return;
        };

        // Check if edge already exists
        if let Some(edge_idx) = self.graph.find_edge(from_idx, to_idx) {
            self.graph[edge_idx] = data;
        } else {
            self.graph.add_edge(from_idx, to_idx, data);
        }
    }

    fn get_edge(&self, from: &NodeId, to: &NodeId) -> Option<&EdgeData> {
        let from_idx = self.node_index.get(from)?;
        let to_idx = self.node_index.get(to)?;
        let edge_idx = self.graph.find_edge(*from_idx, *to_idx)?;
        Some(&self.graph[edge_idx])
    }

    fn get_edge_mut(&mut self, from: &NodeId, to: &NodeId) -> Option<&mut EdgeData> {
        let from_idx = *self.node_index.get(from)?;
        let to_idx = *self.node_index.get(to)?;
        let edge_idx = self.graph.find_edge(from_idx, to_idx)?;
        Some(&mut self.graph[edge_idx])
    }

    fn neighbors(&self, node: &NodeId) -> Vec<(NodeId, &EdgeData)> {
        let Some(&node_idx) = self.node_index.get(node) else {
            return Vec::new();
        };

        self.graph
            .edges(node_idx)
            .map(|edge| {
                let other_idx = if edge.source() == node_idx {
                    edge.target()
                } else {
                    edge.source()
                };
                let other_data = &self.graph[other_idx];
                (other_data.id, edge.weight())
            })
            .collect()
    }

    fn remove_edge(&mut self, from: &NodeId, to: &NodeId) -> Option<EdgeData> {
        let from_idx = *self.node_index.get(from)?;
        let to_idx = *self.node_index.get(to)?;
        let edge_idx = self.graph.find_edge(from_idx, to_idx)?;
        self.graph.remove_edge(edge_idx)
    }

    fn all_nodes(&self) -> Vec<NodeId> {
        self.graph
            .node_indices()
            .map(|idx| self.graph[idx].id)
            .collect()
    }

    fn all_edges(&self) -> Vec<(NodeId, NodeId, &EdgeData)> {
        self.graph
            .edge_indices()
            .map(|idx| {
                let (a, b) = self.graph.edge_endpoints(idx).unwrap();
                (self.graph[a].id, self.graph[b].id, &self.graph[idx])
            })
            .collect()
    }

    fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    fn decay_edges(&mut self, rate: f64, prune_threshold: f64) -> Vec<PrunedConnection> {
        // First pass: decay all weights and collect edges to prune
        let mut to_remove = Vec::new();

        for edge_idx in self.graph.edge_indices() {
            self.graph[edge_idx].weight *= 1.0 - rate;
        }

        // Second pass: identify edges below threshold
        for edge_idx in self.graph.edge_indices() {
            if self.graph[edge_idx].weight < prune_threshold {
                let (a, b) = self.graph.edge_endpoints(edge_idx).unwrap();
                to_remove.push((edge_idx, self.graph[a].id, self.graph[b].id, self.graph[edge_idx].weight));
            }
        }

        let pruned: Vec<PrunedConnection> = to_remove
            .iter()
            .map(|(_, from, to, weight)| PrunedConnection {
                from: *from,
                to: *to,
                final_weight: *weight,
            })
            .collect();

        // Remove in reverse order to avoid index invalidation
        let mut indices: Vec<_> = to_remove.into_iter().map(|(idx, _, _, _)| idx).collect();
        indices.sort();
        for idx in indices.into_iter().rev() {
            self.graph.remove_edge(idx);
        }

        pruned
    }

    fn find_nodes_by_label(&self, query: &str) -> Vec<NodeId> {
        let query_lower = query.to_lowercase();
        self.graph
            .node_indices()
            .filter(|&idx| self.graph[idx].label.to_lowercase().contains(&query_lower))
            .map(|idx| self.graph[idx].id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(label: &str, tick: u64) -> NodeData {
        NodeData {
            id: NodeId::new(),
            label: label.to_string(),
            node_type: NodeType::Concept,
            position: Position::new(0.0, 0.0),
            access_count: 0,
            created_tick: tick,
        }
    }

    fn make_edge(tick: u64) -> EdgeData {
        EdgeData {
            weight: 1.0,
            co_activations: 1,
            created_tick: tick,
            last_activated_tick: tick,
        }
    }

    #[test]
    fn add_and_retrieve_nodes() {
        let mut graph = PetTopologyGraph::new();
        let node = make_node("cell", 0);
        let id = node.id;
        graph.add_node(node);

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.get_node(&id).unwrap().label, "cell");
    }

    #[test]
    fn add_and_retrieve_edges() {
        let mut graph = PetTopologyGraph::new();
        let n1 = make_node("cell", 0);
        let n2 = make_node("membrane", 0);
        let id1 = n1.id;
        let id2 = n2.id;
        graph.add_node(n1);
        graph.add_node(n2);
        graph.set_edge(id1, id2, make_edge(0));

        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.get_edge(&id1, &id2).unwrap().weight, 1.0);
    }

    #[test]
    fn decay_and_prune_edges() {
        let mut graph = PetTopologyGraph::new();
        let n1 = make_node("a", 0);
        let n2 = make_node("b", 0);
        let n3 = make_node("c", 0);
        let id1 = n1.id;
        let id2 = n2.id;
        let id3 = n3.id;
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);

        // Strong edge
        graph.set_edge(id1, id2, EdgeData {
            weight: 1.0,
            co_activations: 10,
            created_tick: 0,
            last_activated_tick: 0,
        });
        // Weak edge
        graph.set_edge(id2, id3, EdgeData {
            weight: 0.1,
            co_activations: 1,
            created_tick: 0,
            last_activated_tick: 0,
        });

        // Decay by 50% — strong edge survives, weak edge gets pruned
        let pruned = graph.decay_edges(0.5, 0.08);
        assert_eq!(pruned.len(), 1);
        assert_eq!(graph.edge_count(), 1);
        // Strong edge decayed to 0.5
        assert!((graph.get_edge(&id1, &id2).unwrap().weight - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn find_nodes_by_label() {
        let mut graph = PetTopologyGraph::new();
        graph.add_node(make_node("cell membrane", 0));
        graph.add_node(make_node("cell wall", 0));
        graph.add_node(make_node("protein", 0));

        let found = graph.find_nodes_by_label("cell");
        assert_eq!(found.len(), 2);

        let found = graph.find_nodes_by_label("protein");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn neighbors() {
        let mut graph = PetTopologyGraph::new();
        let n1 = make_node("a", 0);
        let n2 = make_node("b", 0);
        let n3 = make_node("c", 0);
        let id1 = n1.id;
        let id2 = n2.id;
        let id3 = n3.id;
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);
        graph.set_edge(id1, id2, make_edge(0));
        graph.set_edge(id1, id3, make_edge(0));

        let neighbors = graph.neighbors(&id1);
        assert_eq!(neighbors.len(), 2);
    }
}
