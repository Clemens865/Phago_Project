//! Code-aware query interface.
//!
//! Queries the knowledge graph for code-related concepts like
//! function names, type references, and file associations.

use phago_core::topology::TopologyGraph;
use phago_runtime::colony::Colony;

/// A code query result.
#[derive(Debug, Clone)]
pub struct CodeQueryResult {
    pub label: String,
    pub score: f64,
    pub related: Vec<String>,
}

/// Query the code knowledge graph for a term.
///
/// Performs a 2-3 hop BFS traversal from seed nodes with cumulative
/// edge weight scoring. This surfaces related types, functions,
/// and concepts beyond immediate neighbors.
pub fn code_query(colony: &Colony, query: &str, max_results: usize) -> Vec<CodeQueryResult> {
    let graph = colony.substrate().graph();

    // Find seed nodes
    let seed_ids = graph.find_nodes_by_label(query);
    if seed_ids.is_empty() {
        return Vec::new();
    }

    let max_depth: usize = 3;
    let max_expansions: usize = 150;

    // BFS with cumulative edge weight scoring
    // (cumulative_weight, node_id, depth)
    let mut frontier: Vec<(f64, phago_core::types::NodeId, usize)> = Vec::new();
    let mut visited: std::collections::HashSet<phago_core::types::NodeId> =
        std::collections::HashSet::new();
    let mut scored: Vec<(String, f64, phago_core::types::NodeId)> = Vec::new();

    for seed_id in &seed_ids {
        visited.insert(*seed_id);
        if let Some(node) = graph.get_node(seed_id) {
            scored.push((
                node.label.clone(),
                (node.access_count as f64 + 1.0) * 10.0,
                *seed_id,
            ));
        }
        frontier.push((1.0, *seed_id, 0));
    }

    let mut expansions = 0;
    while !frontier.is_empty() && expansions < max_expansions {
        // Sort ascending, pop best from end
        frontier.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let (weight, current, depth) = frontier.pop().unwrap();
        expansions += 1;

        if depth >= max_depth {
            continue;
        }

        let neighbors = graph.neighbors(&current);
        for (nid, edge) in &neighbors {
            if visited.contains(nid) {
                continue;
            }
            visited.insert(*nid);

            if let Some(node) = graph.get_node(nid) {
                let cumulative_weight = weight * edge.weight;
                let score = cumulative_weight * (1.0 + (node.access_count as f64).ln().max(0.0));
                scored.push((node.label.clone(), score, *nid));
                frontier.push((cumulative_weight, *nid, depth + 1));
            }
        }
    }

    // Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Build results with related nodes
    let mut results: Vec<CodeQueryResult> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (label, score, nid) in &scored {
        if seen.contains(label) {
            continue;
        }
        seen.insert(label.clone());

        // Get neighbors for "related" field
        let mut neighbors: Vec<_> = graph
            .neighbors(nid)
            .into_iter()
            .filter_map(|(neighbor_id, edge)| {
                let n = graph.get_node(&neighbor_id)?;
                Some((n.label.clone(), edge.weight))
            })
            .collect();
        neighbors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let related: Vec<String> = neighbors.iter().take(10).map(|(l, _)| l.clone()).collect();

        results.push(CodeQueryResult {
            label: label.clone(),
            score: *score,
            related,
        });

        if results.len() >= max_results {
            break;
        }
    }

    results
}
