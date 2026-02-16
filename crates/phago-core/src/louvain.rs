//! Louvain community detection algorithm implementation.
//!
//! The Louvain algorithm is a greedy optimization method for detecting
//! communities in large networks. It optimizes modularity through a
//! two-phase iterative process:
//!
//! 1. **Local Moving**: Greedily move nodes to maximize modularity gain
//! 2. **Aggregation**: Build a new graph with communities as nodes
//!
//! Reference: Blondel et al. (2008) "Fast unfolding of communities in large networks"

use crate::types::NodeId;
use std::collections::HashMap;

/// Result of Louvain community detection.
#[derive(Debug, Clone)]
pub struct LouvainResult {
    /// Communities as vectors of node IDs.
    pub communities: Vec<Vec<NodeId>>,
    /// Final modularity score (0.0 to 1.0, higher = better structure).
    pub modularity: f64,
    /// Number of Louvain passes performed.
    pub passes: usize,
}

/// Internal node representation for the algorithm.
#[derive(Clone)]
struct LouvainNode {
    /// Original node ID (or aggregated community ID).
    #[allow(dead_code)]
    id: usize,
    /// Current community assignment.
    community: usize,
    /// Weighted degree (sum of incident edge weights).
    weighted_degree: f64,
    /// Self-loop weight (for aggregated nodes).
    self_loop: f64,
}

/// Graph representation optimized for Louvain.
struct LouvainGraph {
    /// Nodes with their community assignments.
    nodes: Vec<LouvainNode>,
    /// Adjacency list: node_idx -> [(neighbor_idx, weight)].
    adj: Vec<Vec<(usize, f64)>>,
    /// Total edge weight (sum of all weights, counting undirected edges once).
    total_weight: f64,
}

impl LouvainGraph {
    /// Create from a list of edges with weights.
    fn from_edges(node_count: usize, edges: &[(usize, usize, f64)]) -> Self {
        let mut nodes: Vec<LouvainNode> = (0..node_count)
            .map(|i| LouvainNode {
                id: i,
                community: i, // Each node starts in its own community
                weighted_degree: 0.0,
                self_loop: 0.0,
            })
            .collect();

        let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); node_count];
        let mut total_weight = 0.0;

        for &(from, to, weight) in edges {
            if from == to {
                // Self-loop
                nodes[from].self_loop += weight;
                nodes[from].weighted_degree += 2.0 * weight;
                total_weight += weight;
            } else {
                adj[from].push((to, weight));
                adj[to].push((from, weight));
                nodes[from].weighted_degree += weight;
                nodes[to].weighted_degree += weight;
                total_weight += weight;
            }
        }

        Self {
            nodes,
            adj,
            total_weight,
        }
    }

    /// Compute the modularity of the current partition.
    fn modularity(&self) -> f64 {
        if self.total_weight == 0.0 {
            return 0.0;
        }

        let m2 = 2.0 * self.total_weight;
        let mut q = 0.0;

        // Sum of weights and degrees per community
        let mut comm_internal: HashMap<usize, f64> = HashMap::new();
        let mut comm_degree: HashMap<usize, f64> = HashMap::new();

        for node in &self.nodes {
            let c = node.community;
            *comm_degree.entry(c).or_insert(0.0) += node.weighted_degree;
            *comm_internal.entry(c).or_insert(0.0) += node.self_loop;
        }

        // Add internal edge weights
        for (i, neighbors) in self.adj.iter().enumerate() {
            let ci = self.nodes[i].community;
            for &(j, weight) in neighbors {
                if i < j && self.nodes[j].community == ci {
                    *comm_internal.entry(ci).or_insert(0.0) += weight;
                }
            }
        }

        // Q = Σc [ (internal_c / m) - (degree_c / 2m)^2 ]
        for (c, internal) in &comm_internal {
            let degree = comm_degree.get(c).unwrap_or(&0.0);
            q += internal / self.total_weight - (degree / m2).powi(2);
        }

        q
    }

    /// Compute modularity gain of moving node i to community c.
    fn modularity_gain(&self, node_idx: usize, target_community: usize) -> f64 {
        if self.total_weight == 0.0 {
            return 0.0;
        }

        let node = &self.nodes[node_idx];
        if node.community == target_community {
            return 0.0;
        }

        let m2 = 2.0 * self.total_weight;
        let ki = node.weighted_degree;

        // Sum of weights to nodes in target community
        let mut ki_in = 0.0;
        for &(j, weight) in &self.adj[node_idx] {
            if self.nodes[j].community == target_community {
                ki_in += weight;
            }
        }

        // Sum of weighted degrees in target community
        let mut sigma_tot = 0.0;
        for other in &self.nodes {
            if other.community == target_community {
                sigma_tot += other.weighted_degree;
            }
        }

        // ΔQ = ki_in/m - (sigma_tot * ki) / (2m^2)
        ki_in / self.total_weight - (sigma_tot * ki) / (m2 * self.total_weight)
    }

    /// Phase 1: Local moving of nodes to maximize modularity.
    /// Returns true if any improvement was made.
    fn local_moving(&mut self) -> bool {
        let n = self.nodes.len();
        let mut improved = false;
        let mut changed = true;

        while changed {
            changed = false;

            for i in 0..n {
                let current_community = self.nodes[i].community;

                // Find neighboring communities
                let mut neighbor_communities: HashMap<usize, f64> = HashMap::new();
                for &(j, weight) in &self.adj[i] {
                    let c = self.nodes[j].community;
                    *neighbor_communities.entry(c).or_insert(0.0) += weight;
                }

                // Always consider staying in current community
                neighbor_communities.entry(current_community).or_insert(0.0);

                // Find best community
                let mut best_community = current_community;
                let mut best_gain = 0.0;

                for &c in neighbor_communities.keys() {
                    // Temporarily remove from current community for gain calculation
                    self.nodes[i].community = current_community; // Ensure starting state
                    let gain = if c == current_community {
                        0.0
                    } else {
                        // Gain of moving to c
                        let gain_to_c = self.modularity_gain(i, c);
                        // Loss of leaving current
                        self.nodes[i].community = c; // Temporarily move
                        let loss_from_current = -self.modularity_gain(i, current_community);
                        self.nodes[i].community = current_community; // Restore
                        gain_to_c + loss_from_current
                    };

                    if gain > best_gain + 1e-10 {
                        best_gain = gain;
                        best_community = c;
                    }
                }

                if best_community != current_community {
                    self.nodes[i].community = best_community;
                    changed = true;
                    improved = true;
                }
            }
        }

        improved
    }

    /// Phase 2: Aggregate communities into super-nodes.
    fn aggregate(&self) -> (Self, Vec<Vec<usize>>) {
        // Map old communities to new node indices
        let mut comm_to_new: HashMap<usize, usize> = HashMap::new();
        let mut communities: Vec<Vec<usize>> = Vec::new();

        for (i, node) in self.nodes.iter().enumerate() {
            let c = node.community;
            if let Some(&new_idx) = comm_to_new.get(&c) {
                communities[new_idx].push(i);
            } else {
                let new_idx = communities.len();
                comm_to_new.insert(c, new_idx);
                communities.push(vec![i]);
            }
        }

        let new_node_count = communities.len();

        // Compute edges between new nodes
        let mut new_edges: HashMap<(usize, usize), f64> = HashMap::new();

        for (i, neighbors) in self.adj.iter().enumerate() {
            let new_i = comm_to_new[&self.nodes[i].community];
            for &(j, weight) in neighbors {
                let new_j = comm_to_new[&self.nodes[j].community];
                let key = if new_i <= new_j {
                    (new_i, new_j)
                } else {
                    (new_j, new_i)
                };
                *new_edges.entry(key).or_insert(0.0) += weight;
            }
        }

        // Add self-loops from original nodes
        for node in &self.nodes {
            let new_idx = comm_to_new[&node.community];
            *new_edges.entry((new_idx, new_idx)).or_insert(0.0) += node.self_loop;
        }

        // Convert to edge list (halve weights for undirected, except self-loops)
        let edges: Vec<(usize, usize, f64)> = new_edges
            .into_iter()
            .map(|((a, b), w)| {
                if a == b {
                    (a, b, w) // Self-loop: don't halve
                } else {
                    (a, b, w / 2.0) // Undirected edge counted twice
                }
            })
            .collect();

        let new_graph = LouvainGraph::from_edges(new_node_count, &edges);
        (new_graph, communities)
    }
}

/// Run Louvain community detection on a graph.
///
/// # Arguments
/// * `node_ids` - The original node IDs in order
/// * `edges` - Edges as (from_idx, to_idx, weight) where indices correspond to node_ids
///
/// # Returns
/// A `LouvainResult` containing communities (as NodeIds), modularity score, and pass count.
pub fn louvain_communities(node_ids: &[NodeId], edges: &[(usize, usize, f64)]) -> LouvainResult {
    if node_ids.is_empty() {
        return LouvainResult {
            communities: Vec::new(),
            modularity: 0.0,
            passes: 0,
        };
    }

    // Single node case
    if node_ids.len() == 1 {
        return LouvainResult {
            communities: vec![vec![node_ids[0]]],
            modularity: 0.0,
            passes: 0,
        };
    }

    let mut graph = LouvainGraph::from_edges(node_ids.len(), edges);
    let mut dendrogram: Vec<Vec<Vec<usize>>> = Vec::new();
    let mut passes = 0;
    const MAX_PASSES: usize = 100;

    loop {
        passes += 1;

        // Phase 1: Local moving
        let improved = graph.local_moving();

        if !improved || passes >= MAX_PASSES {
            // No improvement or max passes reached
            break;
        }

        // Phase 2: Aggregate
        let (new_graph, communities) = graph.aggregate();

        // If only one community (or no change in community count), stop
        if communities.len() == graph.nodes.len() || communities.len() == 1 {
            break;
        }

        dendrogram.push(communities);
        graph = new_graph;
    }

    // Extract final communities
    let final_modularity = graph.modularity();

    // Build community mapping from dendrogram
    // Start with each node in its own group
    let mut node_to_community: Vec<usize> = (0..node_ids.len()).collect();

    // Apply each level of the dendrogram
    for level in &dendrogram {
        let mut new_mapping = vec![0; node_ids.len()];
        for (new_comm, old_indices) in level.iter().enumerate() {
            for &old_idx in old_indices {
                // Find all original nodes that mapped to old_idx
                for (orig, &comm) in node_to_community.iter().enumerate() {
                    if comm == old_idx {
                        new_mapping[orig] = new_comm;
                    }
                }
            }
        }
        node_to_community = new_mapping;
    }

    // Apply final community assignments from the last graph
    let mut final_mapping = vec![0; node_ids.len()];
    if dendrogram.is_empty() {
        // No aggregation happened, use direct community assignments
        for (orig, &_) in node_to_community.iter().enumerate() {
            final_mapping[orig] = graph.nodes[orig].community;
        }
    } else {
        // Map through dendrogram to final communities
        let last_level = &graph.nodes;
        for (orig, &intermediate) in node_to_community.iter().enumerate() {
            if intermediate < last_level.len() {
                final_mapping[orig] = last_level[intermediate].community;
            }
        }
    }

    // Group nodes by community
    let mut community_map: HashMap<usize, Vec<NodeId>> = HashMap::new();
    for (i, &comm) in final_mapping.iter().enumerate() {
        community_map.entry(comm).or_default().push(node_ids[i]);
    }

    let communities: Vec<Vec<NodeId>> = community_map.into_values().collect();

    LouvainResult {
        communities,
        modularity: final_modularity,
        passes,
    }
}

/// Compute modularity of a given partition.
///
/// # Arguments
/// * `node_count` - Number of nodes
/// * `edges` - Edges as (from_idx, to_idx, weight)
/// * `partition` - Community assignment for each node (indexed by node index)
///
/// # Returns
/// Modularity score (typically 0.3-0.7 for good community structure).
pub fn compute_modularity(
    node_count: usize,
    edges: &[(usize, usize, f64)],
    partition: &[usize],
) -> f64 {
    if node_count == 0 || edges.is_empty() {
        return 0.0;
    }

    let mut graph = LouvainGraph::from_edges(node_count, edges);

    // Apply the given partition
    for (i, &comm) in partition.iter().enumerate() {
        if i < graph.nodes.len() {
            graph.nodes[i].community = comm;
        }
    }

    graph.modularity()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(seed: u64) -> NodeId {
        NodeId::from_seed(seed)
    }

    #[test]
    fn empty_graph() {
        let result = louvain_communities(&[], &[]);
        assert!(result.communities.is_empty());
        assert_eq!(result.modularity, 0.0);
    }

    #[test]
    fn single_node() {
        let nodes = vec![node_id(1)];
        let result = louvain_communities(&nodes, &[]);
        assert_eq!(result.communities.len(), 1);
        assert_eq!(result.communities[0].len(), 1);
    }

    #[test]
    fn two_disconnected_nodes() {
        let nodes = vec![node_id(1), node_id(2)];
        let result = louvain_communities(&nodes, &[]);
        // Each node in its own community
        assert_eq!(result.communities.len(), 2);
    }

    #[test]
    fn two_connected_nodes() {
        let nodes = vec![node_id(1), node_id(2)];
        let edges = vec![(0, 1, 1.0)];
        let result = louvain_communities(&nodes, &edges);
        // Should merge into one community
        assert_eq!(result.communities.len(), 1);
        assert_eq!(result.communities[0].len(), 2);
    }

    #[test]
    fn triangle() {
        let nodes = vec![node_id(1), node_id(2), node_id(3)];
        let edges = vec![(0, 1, 1.0), (1, 2, 1.0), (0, 2, 1.0)];
        let result = louvain_communities(&nodes, &edges);
        // All in one community
        assert_eq!(result.communities.len(), 1);
        assert_eq!(result.communities[0].len(), 3);
    }

    #[test]
    fn two_triangles_weakly_connected() {
        // Triangle 1: 0-1-2
        // Triangle 2: 3-4-5
        // Weak bridge: 2-3
        let nodes: Vec<NodeId> = (0..6).map(|i| node_id(i as u64)).collect();
        let edges = vec![
            // Triangle 1
            (0, 1, 1.0),
            (1, 2, 1.0),
            (0, 2, 1.0),
            // Triangle 2
            (3, 4, 1.0),
            (4, 5, 1.0),
            (3, 5, 1.0),
            // Weak bridge
            (2, 3, 0.1),
        ];
        let result = louvain_communities(&nodes, &edges);

        // Should detect 2 communities
        assert_eq!(result.communities.len(), 2);

        // Each community should have 3 nodes
        let sizes: Vec<usize> = result.communities.iter().map(|c| c.len()).collect();
        assert!(sizes.contains(&3));
        assert_eq!(sizes.iter().sum::<usize>(), 6);

        // Modularity should be positive
        assert!(
            result.modularity > 0.0,
            "modularity = {}",
            result.modularity
        );
    }

    #[test]
    fn karate_club_style() {
        // Simplified Zachary's karate club: two dense groups with sparse connections
        // Group A: 0, 1, 2, 3 (densely connected)
        // Group B: 4, 5, 6, 7 (densely connected)
        // Sparse inter-group connections
        let nodes: Vec<NodeId> = (0..8).map(|i| node_id(i as u64)).collect();
        let edges = vec![
            // Group A internal (complete)
            (0, 1, 1.0),
            (0, 2, 1.0),
            (0, 3, 1.0),
            (1, 2, 1.0),
            (1, 3, 1.0),
            (2, 3, 1.0),
            // Group B internal (complete)
            (4, 5, 1.0),
            (4, 6, 1.0),
            (4, 7, 1.0),
            (5, 6, 1.0),
            (5, 7, 1.0),
            (6, 7, 1.0),
            // Inter-group (sparse)
            (3, 4, 0.2),
        ];

        let result = louvain_communities(&nodes, &edges);

        // Should find 2 communities
        assert_eq!(
            result.communities.len(),
            2,
            "found {} communities",
            result.communities.len()
        );

        // Modularity for this structure should be high (> 0.3)
        assert!(
            result.modularity > 0.3,
            "modularity {} should be > 0.3",
            result.modularity
        );
    }

    #[test]
    fn modularity_calculation() {
        // Simple case: 4 nodes in 2 pairs
        let edges = vec![
            (0, 1, 1.0), // Pair 1
            (2, 3, 1.0), // Pair 2
        ];
        let partition = vec![0, 0, 1, 1]; // Each pair is a community

        let q = compute_modularity(4, &edges, &partition);
        // With this partition, modularity should be 0.5
        // Q = 2 * (1/2 - (2/4)^2) = 2 * (0.5 - 0.25) = 0.5
        assert!((q - 0.5).abs() < 0.01, "modularity = {}, expected ~0.5", q);
    }

    #[test]
    fn modularity_all_one_community() {
        // All nodes in one community should give Q = 0
        let edges = vec![(0, 1, 1.0), (1, 2, 1.0), (0, 2, 1.0)];
        let partition = vec![0, 0, 0]; // All in same community

        let q = compute_modularity(3, &edges, &partition);
        assert!(q.abs() < 0.01, "modularity = {}, expected ~0", q);
    }

    #[test]
    fn weighted_edges() {
        // Strong edges within communities, weak between
        let nodes: Vec<NodeId> = (0..4).map(|i| node_id(i as u64)).collect();
        let edges = vec![
            (0, 1, 5.0), // Strong within group 1
            (2, 3, 5.0), // Strong within group 2
            (1, 2, 0.1), // Weak between groups
        ];

        let result = louvain_communities(&nodes, &edges);

        // Should detect 2 communities
        assert_eq!(result.communities.len(), 2);

        // Check that strong pairs are together
        let comm_map: HashMap<NodeId, usize> = result
            .communities
            .iter()
            .enumerate()
            .flat_map(|(i, c)| c.iter().map(move |&n| (n, i)))
            .collect();

        assert_eq!(
            comm_map.get(&node_id(0)),
            comm_map.get(&node_id(1)),
            "nodes 0 and 1 should be in same community"
        );
        assert_eq!(
            comm_map.get(&node_id(2)),
            comm_map.get(&node_id(3)),
            "nodes 2 and 3 should be in same community"
        );
    }
}
