//! Concrete implementation of the TopologyGraph trait using petgraph.
//!
//! The knowledge graph is the substrate's structural backbone.
//! This implementation uses petgraph's `Graph` as the backing store
//! with HashMap indices for O(1) node/edge lookup by ID.

use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use std::cmp::Ordering;
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

    fn decay_edges_activity(
        &mut self,
        base_rate: f64,
        prune_threshold: f64,
        current_tick: u64,
        staleness_factor: f64,
        maturation_ticks: u64,
    ) -> Vec<PrunedConnection> {
        let mut to_remove = Vec::new();

        // Decay pass: compute per-edge effective rate
        for edge_idx in self.graph.edge_indices() {
            let edge = &self.graph[edge_idx];
            let age = current_tick.saturating_sub(edge.created_tick);

            let effective_rate = if age < maturation_ticks {
                // Young edges: base rate only (maturation window)
                base_rate
            } else {
                let staleness = current_tick.saturating_sub(edge.last_activated_tick) as f64;
                let activity_factor = 1.0 / (1.0 + edge.co_activations as f64 * 0.5);
                base_rate * (1.0 + staleness_factor * (staleness / 100.0) * activity_factor)
            };

            self.graph[edge_idx].weight *= 1.0 - effective_rate.min(0.5); // cap at 50% per tick
        }

        // Prune pass: only prune mature edges (young edges get a grace period)
        for edge_idx in self.graph.edge_indices() {
            let edge = &self.graph[edge_idx];
            let age = current_tick.saturating_sub(edge.created_tick);
            if age >= maturation_ticks && edge.weight < prune_threshold {
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

        let mut indices: Vec<_> = to_remove.into_iter().map(|(idx, _, _, _)| idx).collect();
        indices.sort();
        for idx in indices.into_iter().rev() {
            self.graph.remove_edge(idx);
        }

        pruned
    }

    fn prune_to_max_degree(&mut self, max_degree: usize) -> Vec<PrunedConnection> {
        use std::collections::HashSet;

        // For each over-degree node, identify which edges to drop (weakest beyond top-K).
        // An edge is removed if ANY of its endpoints wants to drop it.
        let mut dropped_edges: HashSet<petgraph::graph::EdgeIndex> = HashSet::new();

        for node_idx in self.graph.node_indices() {
            let mut edges: Vec<(petgraph::graph::EdgeIndex, f64)> = self
                .graph
                .edges(node_idx)
                .map(|e| (e.id(), e.weight().weight))
                .collect();

            if edges.len() > max_degree {
                // Sort descending by weight, drop everything beyond top-K
                edges.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                for (eidx, _) in edges.iter().skip(max_degree) {
                    dropped_edges.insert(*eidx);
                }
            }
        }

        // Remove all dropped edges
        let mut to_remove = Vec::new();
        for &edge_idx in &dropped_edges {
            if let Some((a, b)) = self.graph.edge_endpoints(edge_idx) {
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

    fn shortest_path(&self, from: &NodeId, to: &NodeId) -> Option<(Vec<NodeId>, f64)> {
        use std::collections::BinaryHeap;
        use std::cmp::Ordering;

        let from_idx = *self.node_index.get(from)?;
        let to_idx = *self.node_index.get(to)?;

        // Dijkstra with inverse weight as cost (prefer stronger edges)
        #[derive(PartialEq)]
        struct State {
            cost: f64,
            node: NodeIndex,
        }
        impl Eq for State {}
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                other.cost.partial_cmp(&self.cost) // min-heap
            }
        }
        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                self.partial_cmp(other).unwrap_or(Ordering::Equal)
            }
        }

        let mut dist: HashMap<NodeIndex, f64> = HashMap::new();
        let mut prev: HashMap<NodeIndex, NodeIndex> = HashMap::new();
        let mut heap = BinaryHeap::new();

        dist.insert(from_idx, 0.0);
        heap.push(State { cost: 0.0, node: from_idx });

        while let Some(State { cost, node }) = heap.pop() {
            if node == to_idx {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current = to_idx;
                while current != from_idx {
                    path.push(self.graph[current].id);
                    current = prev[&current];
                }
                path.push(self.graph[from_idx].id);
                path.reverse();
                return Some((path, cost));
            }

            if cost > *dist.get(&node).unwrap_or(&f64::INFINITY) {
                continue;
            }

            for edge in self.graph.edges(node) {
                let next = if edge.source() == node { edge.target() } else { edge.source() };
                // Cost = 1/weight so strong edges are "shorter"
                let edge_cost = 1.0 / edge.weight().weight.max(0.001);
                let next_cost = cost + edge_cost;

                if next_cost < *dist.get(&next).unwrap_or(&f64::INFINITY) {
                    dist.insert(next, next_cost);
                    prev.insert(next, node);
                    heap.push(State { cost: next_cost, node: next });
                }
            }
        }
        None
    }

    fn betweenness_centrality(&self, sample_size: usize) -> Vec<(NodeId, f64)> {
        let nodes: Vec<NodeIndex> = self.graph.node_indices().collect();
        let n = nodes.len();
        if n < 2 {
            return Vec::new();
        }

        let mut centrality: HashMap<NodeIndex, f64> = HashMap::new();
        for &idx in &nodes {
            centrality.insert(idx, 0.0);
        }

        // Sample pairs for approximate centrality
        let pairs_to_sample = sample_size.min(n * (n - 1) / 2);
        let mut seed: u64 = 42;
        let mut sampled = 0;

        while sampled < pairs_to_sample {
            // Simple PRNG for pair selection
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let i = (seed >> 33) as usize % n;
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let j = (seed >> 33) as usize % n;
            if i == j { continue; }
            sampled += 1;

            let from_id = self.graph[nodes[i]].id;
            let to_id = self.graph[nodes[j]].id;

            if let Some((path, _)) = self.shortest_path(&from_id, &to_id) {
                // Credit intermediate nodes (not endpoints)
                for node_id in &path[1..path.len().saturating_sub(1)] {
                    if let Some(&idx) = self.node_index.get(node_id) {
                        *centrality.entry(idx).or_insert(0.0) += 1.0;
                    }
                }
            }
        }

        // Normalize by number of pairs sampled
        let norm = if sampled > 0 { sampled as f64 } else { 1.0 };
        let mut result: Vec<(NodeId, f64)> = centrality
            .into_iter()
            .map(|(idx, c)| (self.graph[idx].id, c / norm))
            .collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        result
    }

    fn bridge_nodes(&self, top_k: usize) -> Vec<(NodeId, f64)> {
        let base_components = self.connected_components();
        let nodes: Vec<NodeIndex> = self.graph.node_indices().collect();

        let mut fragility: Vec<(NodeId, f64)> = Vec::new();

        for &node_idx in &nodes {
            let degree = self.graph.edges(node_idx).count();
            if degree == 0 { continue; }

            // Simulate removal: count how many components would result
            // by doing BFS on the remaining graph
            let mut visited = std::collections::HashSet::new();
            visited.insert(node_idx);
            let mut components = 0;

            for &start in &nodes {
                if visited.contains(&start) { continue; }
                components += 1;
                // BFS from start, skipping node_idx
                let mut queue = std::collections::VecDeque::new();
                queue.push_back(start);
                visited.insert(start);
                while let Some(current) = queue.pop_front() {
                    for edge in self.graph.edges(current) {
                        let next = if edge.source() == current { edge.target() } else { edge.source() };
                        if !visited.contains(&next) {
                            visited.insert(next);
                            queue.push_back(next);
                        }
                    }
                }
            }

            let delta = components as f64 - base_components as f64;
            let score = delta / degree as f64;
            if score > 0.0 {
                fragility.push((self.graph[node_idx].id, score));
            }
        }

        fragility.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        fragility.truncate(top_k);
        fragility
    }

    fn connected_components(&self) -> usize {
        let mut visited = std::collections::HashSet::new();
        let mut components = 0;

        for node_idx in self.graph.node_indices() {
            if visited.contains(&node_idx) { continue; }
            components += 1;
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(node_idx);
            visited.insert(node_idx);
            while let Some(current) = queue.pop_front() {
                for edge in self.graph.edges(current) {
                    let next = if edge.source() == current { edge.target() } else { edge.source() };
                    if !visited.contains(&next) {
                        visited.insert(next);
                        queue.push_back(next);
                    }
                }
            }
        }

        components
    }

    fn find_nodes_by_exact_label(&self, label: &str) -> Vec<NodeId> {
        // Use O(1) index lookup
        self.label_index
            .get(&label.to_lowercase())
            .map(|v| v.clone())
            .unwrap_or_default()
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
            embedding: None,
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

    #[test]
    fn activity_decay_penalizes_stale_edges() {
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

        // Recently activated edge
        graph.set_edge(id1, id2, EdgeData {
            weight: 0.5,
            co_activations: 5,
            created_tick: 0,
            last_activated_tick: 95, // activated 5 ticks ago
        });
        // Stale edge (same initial weight, same co_activations but not activated recently)
        graph.set_edge(id2, id3, EdgeData {
            weight: 0.5,
            co_activations: 1,
            created_tick: 0,
            last_activated_tick: 10, // activated 90 ticks ago
        });

        let current_tick = 100;
        graph.decay_edges_activity(0.005, 0.01, current_tick, 4.0, 30);

        let fresh_weight = graph.get_edge(&id1, &id2).unwrap().weight;
        let stale_weight = graph.get_edge(&id2, &id3).unwrap().weight;

        // Stale edge should have decayed more than fresh edge
        assert!(
            stale_weight < fresh_weight,
            "stale edge ({stale_weight}) should be lighter than fresh edge ({fresh_weight})"
        );
    }

    #[test]
    fn competitive_pruning_enforces_max_degree() {
        let mut graph = PetTopologyGraph::new();

        // Create a hub node connected to 30 neighbors
        let hub = make_node("hub", 0);
        let hub_id = hub.id;
        graph.add_node(hub);

        let mut neighbor_ids = Vec::new();
        for i in 0..30 {
            let n = make_node(&format!("n{i}"), 0);
            let nid = n.id;
            graph.add_node(n);
            // Assign decreasing weights so we know which survive
            graph.set_edge(hub_id, nid, EdgeData {
                weight: 1.0 - (i as f64 * 0.02), // 1.0, 0.98, 0.96, ...
                co_activations: 1,
                created_tick: 0,
                last_activated_tick: 0,
            });
            neighbor_ids.push(nid);
        }

        assert_eq!(graph.edge_count(), 30);

        let max_degree = 10;
        let pruned = graph.prune_to_max_degree(max_degree);

        // Hub had 30 edges, each neighbor had 1 edge.
        // Hub keeps top 10. Each neighbor keeps its 1 edge (it's their only one).
        // Hub had 30 edges, max_degree=10. Hub drops its weakest 20.
        // With "any endpoint drops" policy, those 20 edges are removed
        // even though each spoke only has degree 1.
        assert_eq!(pruned.len(), 20, "hub should drop its 20 weakest edges");
        assert_eq!(graph.edge_count(), 10);

        // Verify the 10 surviving edges are the strongest ones
        let hub_neighbors = graph.neighbors(&hub_id);
        assert_eq!(hub_neighbors.len(), 10);
        for (_, edge) in &hub_neighbors {
            assert!(edge.weight >= 0.8, "surviving edges should be the strongest, got {:.2}", edge.weight);
        }

        // Now test with a clique where all nodes exceed max_degree
        // Use uniform weights so each node's top-K is deterministic
        let mut clique = PetTopologyGraph::new();
        let mut ids = Vec::new();
        let n_nodes = 15;
        let max_k = 5;
        for i in 0..n_nodes {
            let n = make_node(&format!("c{i}"), 0);
            ids.push(n.id);
            clique.add_node(n);
        }
        // Connect with distinct weights: edge (i,j) gets weight = 1.0 - distance/n_nodes
        // so each node's nearest-index neighbors have highest weight
        for i in 0..n_nodes {
            for j in (i + 1)..n_nodes {
                let dist = (j - i) as f64;
                clique.set_edge(ids[i], ids[j], EdgeData {
                    weight: 1.0 - dist / n_nodes as f64,
                    co_activations: 1,
                    created_tick: 0,
                    last_activated_tick: 0,
                });
            }
        }
        let initial_edges = clique.edge_count();
        assert_eq!(initial_edges, n_nodes * (n_nodes - 1) / 2); // 105

        clique.prune_to_max_degree(max_k);

        // "Any endpoint drops" policy: edges pruned if either endpoint is over-degree.
        // Verify total edges dropped significantly.
        assert!(
            clique.edge_count() < initial_edges,
            "edge count {} should be less than initial {initial_edges}",
            clique.edge_count()
        );
        // With max_k=5 and 15 nodes, each node keeps top 5.
        // Total kept edges <= n_nodes * max_k (counting each edge from both endpoints)
        // = 75, but duplicates mean actual unique edges <= 75.
        // Still a significant reduction from 105.
        assert!(
            clique.edge_count() <= n_nodes * max_k,
            "edge count {} should be at most {}",
            clique.edge_count(),
            n_nodes * max_k
        );
    }

    #[test]
    fn co_activation_gate_protects_strong_edges() {
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

        // High co_activation edge, but stale
        graph.set_edge(id1, id2, EdgeData {
            weight: 0.3,
            co_activations: 20, // well-established connection
            created_tick: 0,
            last_activated_tick: 10,
        });
        // Low co_activation edge, equally stale
        graph.set_edge(id2, id3, EdgeData {
            weight: 0.3,
            co_activations: 1, // weak connection
            created_tick: 0,
            last_activated_tick: 10,
        });

        // Run many decay rounds
        for tick in 100..120 {
            graph.decay_edges_activity(0.005, 0.05, tick, 4.0, 30);
        }

        let strong_edge = graph.get_edge(&id1, &id2);
        let weak_edge = graph.get_edge(&id2, &id3);

        // The high co_activation edge should survive; the weak one may be pruned
        assert!(
            strong_edge.is_some(),
            "high co_activation edge should survive activity decay"
        );
        // The weak edge should be pruned or at least much lighter
        if let Some(weak) = weak_edge {
            assert!(
                weak.weight < strong_edge.unwrap().weight,
                "weak edge should be lighter than strong edge"
            );
        }
        // (If weak_edge is None, it was pruned — also a valid outcome)
    }
}
