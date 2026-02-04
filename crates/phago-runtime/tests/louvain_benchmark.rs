//! Louvain Community Detection Benchmarks
//!
//! Tests the quality of Louvain community detection against known ground truth.

use phago_core::louvain::{louvain_communities, compute_modularity, LouvainResult};
use phago_core::types::NodeId;
use std::collections::HashMap;
use std::time::Instant;

/// Compute Normalized Mutual Information between two partitions.
/// NMI ranges from 0 (no correlation) to 1 (perfect match).
fn normalized_mutual_information(
    ground_truth: &[usize],
    predicted: &[usize],
) -> f64 {
    let n = ground_truth.len();
    if n == 0 || predicted.len() != n {
        return 0.0;
    }

    // Count occurrences
    let mut gt_counts: HashMap<usize, usize> = HashMap::new();
    let mut pred_counts: HashMap<usize, usize> = HashMap::new();
    let mut joint_counts: HashMap<(usize, usize), usize> = HashMap::new();

    for i in 0..n {
        *gt_counts.entry(ground_truth[i]).or_insert(0) += 1;
        *pred_counts.entry(predicted[i]).or_insert(0) += 1;
        *joint_counts.entry((ground_truth[i], predicted[i])).or_insert(0) += 1;
    }

    let n_f = n as f64;

    // Compute entropy H(X)
    let h_gt: f64 = gt_counts
        .values()
        .map(|&c| {
            let p = c as f64 / n_f;
            if p > 0.0 { -p * p.log2() } else { 0.0 }
        })
        .sum();

    // Compute entropy H(Y)
    let h_pred: f64 = pred_counts
        .values()
        .map(|&c| {
            let p = c as f64 / n_f;
            if p > 0.0 { -p * p.log2() } else { 0.0 }
        })
        .sum();

    // Compute mutual information I(X;Y)
    let mut mi = 0.0;
    for (&(gt, pred), &count) in &joint_counts {
        let p_xy = count as f64 / n_f;
        let p_x = *gt_counts.get(&gt).unwrap() as f64 / n_f;
        let p_y = *pred_counts.get(&pred).unwrap() as f64 / n_f;
        if p_xy > 0.0 && p_x > 0.0 && p_y > 0.0 {
            mi += p_xy * (p_xy / (p_x * p_y)).log2();
        }
    }

    // NMI = 2 * I(X;Y) / (H(X) + H(Y))
    if h_gt + h_pred > 0.0 {
        2.0 * mi / (h_gt + h_pred)
    } else {
        1.0 // Both are trivial (single community)
    }
}

/// Generate a synthetic graph with planted communities.
/// Returns (node_ids, edges, ground_truth_partition).
fn generate_planted_partition(
    num_communities: usize,
    nodes_per_community: usize,
    p_in: f64,  // Probability of edge within community
    p_out: f64, // Probability of edge between communities
    seed: u64,
) -> (Vec<NodeId>, Vec<(usize, usize, f64)>, Vec<usize>) {
    let total_nodes = num_communities * nodes_per_community;
    let node_ids: Vec<NodeId> = (0..total_nodes)
        .map(|i| NodeId::from_seed(i as u64 + seed))
        .collect();

    let mut edges = Vec::new();
    let mut rng_state = seed;

    // Simple PRNG
    let mut rand = || {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_state >> 33) as f64 / (1u64 << 31) as f64
    };

    // Generate edges
    for i in 0..total_nodes {
        let ci = i / nodes_per_community;
        for j in (i + 1)..total_nodes {
            let cj = j / nodes_per_community;
            let p = if ci == cj { p_in } else { p_out };
            if rand() < p {
                edges.push((i, j, 1.0));
            }
        }
    }

    // Ground truth: node i belongs to community i / nodes_per_community
    let ground_truth: Vec<usize> = (0..total_nodes)
        .map(|i| i / nodes_per_community)
        .collect();

    (node_ids, edges, ground_truth)
}

/// Convert LouvainResult to a partition vector (node_idx -> community_idx).
fn result_to_partition(result: &LouvainResult, node_ids: &[NodeId]) -> Vec<usize> {
    let id_to_idx: HashMap<NodeId, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();

    let mut partition = vec![0; node_ids.len()];
    for (comm_idx, community) in result.communities.iter().enumerate() {
        for &node_id in community {
            if let Some(&idx) = id_to_idx.get(&node_id) {
                partition[idx] = comm_idx;
            }
        }
    }
    partition
}

#[test]
fn benchmark_small_synthetic() {
    // 4 communities, 10 nodes each = 40 nodes
    // p_in = 0.7, p_out = 0.05
    let (node_ids, edges, ground_truth) = generate_planted_partition(4, 10, 0.7, 0.05, 42);

    let start = Instant::now();
    let result = louvain_communities(&node_ids, &edges);
    let elapsed = start.elapsed();

    let predicted = result_to_partition(&result, &node_ids);
    let nmi = normalized_mutual_information(&ground_truth, &predicted);

    println!("\n=== Small Synthetic Benchmark (40 nodes) ===");
    println!("Ground truth communities: 4");
    println!("Detected communities: {}", result.communities.len());
    println!("Modularity: {:.4}", result.modularity);
    println!("NMI: {:.4}", nmi);
    println!("Passes: {}", result.passes);
    println!("Time: {:?}", elapsed);

    // Target: NMI > 0.3
    assert!(nmi > 0.3, "NMI {} should be > 0.3", nmi);
    // Modularity should be positive
    assert!(result.modularity > 0.0, "Modularity should be positive");
}

#[test]
fn benchmark_medium_synthetic() {
    // 8 communities, 25 nodes each = 200 nodes
    let (node_ids, edges, ground_truth) = generate_planted_partition(8, 25, 0.6, 0.02, 123);

    let start = Instant::now();
    let result = louvain_communities(&node_ids, &edges);
    let elapsed = start.elapsed();

    let predicted = result_to_partition(&result, &node_ids);
    let nmi = normalized_mutual_information(&ground_truth, &predicted);

    println!("\n=== Medium Synthetic Benchmark (200 nodes) ===");
    println!("Ground truth communities: 8");
    println!("Detected communities: {}", result.communities.len());
    println!("Modularity: {:.4}", result.modularity);
    println!("NMI: {:.4}", nmi);
    println!("Passes: {}", result.passes);
    println!("Time: {:?}", elapsed);
    println!("Edges: {}", edges.len());

    // Target: NMI > 0.3
    assert!(nmi > 0.3, "NMI {} should be > 0.3", nmi);
}

#[test]
fn benchmark_large_synthetic() {
    // 10 communities, 100 nodes each = 1000 nodes
    let (node_ids, edges, ground_truth) = generate_planted_partition(10, 100, 0.5, 0.01, 456);

    let start = Instant::now();
    let result = louvain_communities(&node_ids, &edges);
    let elapsed = start.elapsed();

    let predicted = result_to_partition(&result, &node_ids);
    let nmi = normalized_mutual_information(&ground_truth, &predicted);

    println!("\n=== Large Synthetic Benchmark (1000 nodes) ===");
    println!("Ground truth communities: 10");
    println!("Detected communities: {}", result.communities.len());
    println!("Modularity: {:.4}", result.modularity);
    println!("NMI: {:.4}", nmi);
    println!("Passes: {}", result.passes);
    println!("Time: {:?}", elapsed);
    println!("Edges: {}", edges.len());

    // Target: NMI > 0.3
    assert!(nmi > 0.3, "NMI {} should be > 0.3", nmi);
    // Runtime should be reasonable (< 5 seconds for 1000 nodes)
    assert!(elapsed.as_secs() < 5, "Should complete in < 5 seconds");
}

#[test]
fn benchmark_weighted_edges() {
    // Test with weighted edges (strong within, weak between)
    let num_communities = 5;
    let nodes_per_community = 20;
    let total_nodes = num_communities * nodes_per_community;

    let node_ids: Vec<NodeId> = (0..total_nodes)
        .map(|i| NodeId::from_seed(i as u64 + 789))
        .collect();

    let mut edges = Vec::new();
    for i in 0..total_nodes {
        let ci = i / nodes_per_community;
        for j in (i + 1)..total_nodes {
            let cj = j / nodes_per_community;
            if ci == cj {
                // Strong edge within community
                edges.push((i, j, 5.0));
            } else if (i + j) % 7 == 0 {
                // Sparse weak edges between communities
                edges.push((i, j, 0.5));
            }
        }
    }

    let ground_truth: Vec<usize> = (0..total_nodes)
        .map(|i| i / nodes_per_community)
        .collect();

    let start = Instant::now();
    let result = louvain_communities(&node_ids, &edges);
    let elapsed = start.elapsed();

    let predicted = result_to_partition(&result, &node_ids);
    let nmi = normalized_mutual_information(&ground_truth, &predicted);

    println!("\n=== Weighted Edges Benchmark (100 nodes) ===");
    println!("Ground truth communities: {}", num_communities);
    println!("Detected communities: {}", result.communities.len());
    println!("Modularity: {:.4}", result.modularity);
    println!("NMI: {:.4}", nmi);
    println!("Time: {:?}", elapsed);

    // With strong edge weighting, should get near-perfect recovery
    assert!(nmi > 0.8, "NMI {} should be > 0.8 with weighted edges", nmi);
}

#[test]
fn benchmark_modularity_quality() {
    // Test modularity values for known structures

    // Two disconnected cliques - modularity should be ~0.5
    let n = 10;
    let node_ids: Vec<NodeId> = (0..2*n).map(|i| NodeId::from_seed(i as u64)).collect();
    let mut edges = Vec::new();

    // Clique 1: nodes 0..n
    for i in 0..n {
        for j in (i+1)..n {
            edges.push((i, j, 1.0));
        }
    }
    // Clique 2: nodes n..2n
    for i in n..2*n {
        for j in (i+1)..2*n {
            edges.push((i, j, 1.0));
        }
    }

    let result = louvain_communities(&node_ids, &edges);

    println!("\n=== Two Disconnected Cliques (modularity quality) ===");
    println!("Detected communities: {}", result.communities.len());
    println!("Modularity: {:.4}", result.modularity);

    assert_eq!(result.communities.len(), 2, "Should detect exactly 2 communities");
    assert!(result.modularity > 0.4, "Modularity should be > 0.4 for disconnected cliques");
}

#[test]
fn benchmark_summary() {
    // Run multiple tests and compute average metrics
    let configs = vec![
        (4, 10, 0.7, 0.05),   // Small, well-separated
        (6, 20, 0.6, 0.03),   // Medium
        (8, 25, 0.5, 0.02),   // Larger
        (10, 30, 0.4, 0.01),  // Large, harder
    ];

    println!("\n=== Louvain Benchmark Summary ===");
    println!("{:<20} {:>12} {:>12} {:>12} {:>12}",
             "Config", "Nodes", "NMI", "Modularity", "Time (ms)");
    println!("{}", "-".repeat(70));

    let mut total_nmi = 0.0;
    let mut count = 0;

    for (seed_offset, (num_comm, nodes_per, p_in, p_out)) in configs.iter().enumerate() {
        let (node_ids, edges, ground_truth) = generate_planted_partition(
            *num_comm, *nodes_per, *p_in, *p_out, (seed_offset * 1000) as u64
        );

        let total_nodes = num_comm * nodes_per;
        let start = Instant::now();
        let result = louvain_communities(&node_ids, &edges);
        let elapsed = start.elapsed();

        let predicted = result_to_partition(&result, &node_ids);
        let nmi = normalized_mutual_information(&ground_truth, &predicted);

        println!("{:<20} {:>12} {:>12.4} {:>12.4} {:>12.2}",
                 format!("{}x{}", num_comm, nodes_per),
                 total_nodes,
                 nmi,
                 result.modularity,
                 elapsed.as_secs_f64() * 1000.0);

        total_nmi += nmi;
        count += 1;
    }

    let avg_nmi = total_nmi / count as f64;
    println!("{}", "-".repeat(70));
    println!("Average NMI: {:.4}", avg_nmi);

    // Target: Average NMI > 0.3
    assert!(avg_nmi > 0.3, "Average NMI {} should be > 0.3", avg_nmi);
}
