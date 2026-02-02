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
/// Finds the node matching the query and returns its neighbors
/// sorted by edge weight. This surfaces related types, functions,
/// and concepts.
pub fn code_query(colony: &Colony, query: &str, max_results: usize) -> Vec<CodeQueryResult> {
    let graph = colony.substrate().graph();

    // Find seed nodes
    let seed_ids = graph.find_nodes_by_label(query);
    if seed_ids.is_empty() {
        return Vec::new();
    }

    let mut results: Vec<CodeQueryResult> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for seed_id in &seed_ids {
        let node = match graph.get_node(seed_id) {
            Some(n) => n,
            None => continue,
        };

        // Get neighbors sorted by weight
        let mut neighbors: Vec<_> = graph.neighbors(seed_id)
            .into_iter()
            .filter_map(|(nid, edge)| {
                let n = graph.get_node(&nid)?;
                Some((n.label.clone(), edge.weight, edge.co_activations))
            })
            .collect();
        neighbors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let related: Vec<String> = neighbors.iter()
            .take(10)
            .map(|(label, _, _)| label.clone())
            .collect();

        if !seen.contains(&node.label) {
            seen.insert(node.label.clone());
            results.push(CodeQueryResult {
                label: node.label.clone(),
                score: node.access_count as f64,
                related,
            });
        }
    }

    results.truncate(max_results);
    results
}
