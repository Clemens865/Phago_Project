//! Community detection via label propagation.
//!
//! Detects communities in the knowledge graph using a simple
//! label propagation algorithm. Used to evaluate whether the
//! self-organized Hebbian graph recovers ground-truth topic clusters.

use crate::colony::Colony;
use phago_core::topology::TopologyGraph;
use phago_core::types::NodeId;
use serde::Serialize;
use std::collections::HashMap;

/// A detected community in the knowledge graph.
#[derive(Debug, Clone, Serialize)]
pub struct Community {
    pub id: usize,
    pub members: Vec<String>,
    pub size: usize,
}

/// Result of community detection.
#[derive(Debug, Clone, Serialize)]
pub struct CommunityResult {
    pub communities: Vec<Community>,
    /// Node label → community ID mapping.
    pub assignments: HashMap<String, usize>,
    pub total_nodes: usize,
    pub num_communities: usize,
}

/// Run label propagation community detection.
///
/// Each node starts with its own label. In each iteration, each node
/// adopts the label most common among its neighbors (weighted by edge weight).
/// Converges when no labels change.
///
/// Uses edge weight thresholding: only edges above the median weight are
/// considered during neighbor voting. This prunes weak cross-topic edges
/// and preserves within-topic clusters, improving NMI.
pub fn detect_communities(colony: &Colony, max_iterations: usize) -> CommunityResult {
    let graph = colony.substrate().graph();
    let all_nodes = graph.all_nodes();

    if all_nodes.is_empty() {
        return CommunityResult {
            communities: Vec::new(),
            assignments: HashMap::new(),
            total_nodes: 0,
            num_communities: 0,
        };
    }

    // Compute edge weight threshold adaptively based on graph density.
    // Dense graphs need aggressive pruning (90th percentile) to reveal
    // community structure; sparse graphs use median.
    let all_edges = graph.all_edges();
    let weight_threshold = if all_edges.is_empty() {
        0.0
    } else {
        let mut weights: Vec<f64> = all_edges.iter().map(|(_, _, e)| e.weight).collect();
        weights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = all_nodes.len() as f64;
        let density = if n > 1.0 {
            (2.0 * all_edges.len() as f64) / (n * (n - 1.0))
        } else {
            0.0
        };
        // For dense graphs (density > 0.05), use 90th percentile to aggressively
        // prune weak edges. Otherwise use 75th percentile.
        let percentile = if density > 0.05 { 90 } else { 75 };
        let idx = (weights.len() * percentile / 100).min(weights.len() - 1);
        weights[idx]
    };

    // Initialize: each node gets its own label
    let mut labels: HashMap<NodeId, usize> = HashMap::new();
    let node_list: Vec<NodeId> = all_nodes.clone();
    for (i, nid) in node_list.iter().enumerate() {
        labels.insert(*nid, i);
    }

    // Iterate with shuffled node order per iteration (asynchronous LP)
    for iter in 0..max_iterations {
        let mut changed = false;

        // Shuffle node processing order using Fisher-Yates with deterministic seed
        let mut order: Vec<usize> = (0..node_list.len()).collect();
        let mut seed: u64 = (iter as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        for i in (1..order.len()).rev() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let j = (seed >> 33) as usize % (i + 1);
            order.swap(i, j);
        }

        for &idx in &order {
            let nid = &node_list[idx];
            let neighbors = graph.neighbors(nid);
            if neighbors.is_empty() {
                continue;
            }

            // Weight-vote for neighbor labels, only considering edges above the median weight
            let mut label_weights: HashMap<usize, f64> = HashMap::new();
            for (neighbor_id, edge) in &neighbors {
                if edge.weight < weight_threshold {
                    continue; // Skip weak cross-topic edges
                }
                if let Some(&label) = labels.get(neighbor_id) {
                    *label_weights.entry(label).or_insert(0.0) += edge.weight;
                }
            }

            if label_weights.is_empty() {
                continue; // No strong neighbors, keep current label
            }

            // Adopt the highest-weighted label
            if let Some((&best_label, _)) = label_weights.iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            {
                let current = labels.get(nid).copied().unwrap_or(0);
                if best_label != current {
                    labels.insert(*nid, best_label);
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    // Build communities from final labels
    let mut community_members: HashMap<usize, Vec<String>> = HashMap::new();
    let mut assignments: HashMap<String, usize> = HashMap::new();

    for nid in &node_list {
        if let (Some(&label), Some(node)) = (labels.get(nid), graph.get_node(nid)) {
            community_members.entry(label).or_default().push(node.label.clone());
            assignments.insert(node.label.clone(), label);
        }
    }

    // Renumber communities 0, 1, 2, ...
    let mut renumber: HashMap<usize, usize> = HashMap::new();
    let mut next_id = 0;
    for old_id in community_members.keys() {
        renumber.entry(*old_id).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            id
        });
    }

    let mut communities: Vec<Community> = community_members.into_iter()
        .map(|(old_id, members)| {
            let new_id = renumber[&old_id];
            Community {
                id: new_id,
                size: members.len(),
                members,
            }
        })
        .collect();
    communities.sort_by(|a, b| b.size.cmp(&a.size));

    // Update assignments with new IDs
    for val in assignments.values_mut() {
        *val = renumber[val];
    }

    CommunityResult {
        num_communities: communities.len(),
        total_nodes: node_list.len(),
        communities,
        assignments,
    }
}

/// Compute Normalized Mutual Information (NMI) between detected and ground-truth labels.
///
/// NMI ranges from 0 (no correlation) to 1 (perfect match).
pub fn compute_nmi(
    assignments: &HashMap<String, usize>,
    ground_truth: &HashMap<String, String>,
) -> f64 {
    // Map ground truth categories to numeric IDs
    let mut gt_labels: HashMap<String, usize> = HashMap::new();
    let mut gt_next = 0;
    let mut gt_assignments: HashMap<String, usize> = HashMap::new();
    for (node, category) in ground_truth {
        if !gt_labels.contains_key(category) {
            gt_labels.insert(category.clone(), gt_next);
            gt_next += 1;
        }
        gt_assignments.insert(node.clone(), gt_labels[category]);
    }

    // Find nodes present in both
    let common_nodes: Vec<&String> = assignments.keys()
        .filter(|k| gt_assignments.contains_key(*k))
        .collect();

    if common_nodes.is_empty() {
        return 0.0;
    }

    let n = common_nodes.len() as f64;

    // Count label co-occurrences
    let mut detected_counts: HashMap<usize, f64> = HashMap::new();
    let mut gt_counts: HashMap<usize, f64> = HashMap::new();
    let mut joint_counts: HashMap<(usize, usize), f64> = HashMap::new();

    for node in &common_nodes {
        let d = assignments[*node];
        let g = gt_assignments[*node];
        *detected_counts.entry(d).or_insert(0.0) += 1.0;
        *gt_counts.entry(g).or_insert(0.0) += 1.0;
        *joint_counts.entry((d, g)).or_insert(0.0) += 1.0;
    }

    // Compute mutual information
    let mut mi = 0.0;
    for (&(d, g), &nij) in &joint_counts {
        if nij > 0.0 {
            let ni = detected_counts[&d];
            let nj = gt_counts[&g];
            mi += (nij / n) * ((n * nij) / (ni * nj)).ln();
        }
    }

    // Compute entropies
    let h_detected: f64 = detected_counts.values()
        .map(|&c| if c > 0.0 { -(c / n) * (c / n).ln() } else { 0.0 })
        .sum();
    let h_gt: f64 = gt_counts.values()
        .map(|&c| if c > 0.0 { -(c / n) * (c / n).ln() } else { 0.0 })
        .sum();

    // NMI = 2 * MI / (H_detected + H_gt)
    let denominator = h_detected + h_gt;
    if denominator < 1e-10 {
        0.0
    } else {
        (2.0 * mi / denominator).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nmi_perfect_match() {
        let mut detected: HashMap<String, usize> = HashMap::new();
        let mut gt: HashMap<String, String> = HashMap::new();
        for i in 0..10 {
            let name = format!("node_{}", i);
            let cluster = i / 5;
            let category = format!("cat_{}", cluster);
            detected.insert(name.clone(), cluster);
            gt.insert(name, category);
        }
        let nmi = compute_nmi(&detected, &gt);
        assert!(nmi > 0.99, "NMI should be ~1.0 for perfect match: {}", nmi);
    }
}
