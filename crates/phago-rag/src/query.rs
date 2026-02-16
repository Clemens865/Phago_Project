//! Query engine — traverses the Hebbian knowledge graph to retrieve relevant concepts.
//!
//! The query engine:
//! 1. Tokenizes the query into terms
//! 2. Finds matching nodes in the graph (exact label match)
//! 3. Traverses outward following strongest edges (BFS weighted by edge weight)
//! 4. Collects and ranks results by path weight × access count
//! 5. Optionally reinforces traversed paths (the graph learns from queries)

use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use phago_runtime::colony::Colony;
use serde::Serialize;

/// A query to the knowledge graph.
#[derive(Debug, Clone)]
pub struct Query {
    /// The raw query text.
    pub text: String,
    /// Maximum number of results to return.
    pub max_results: usize,
    /// Maximum traversal depth from seed nodes.
    pub max_depth: usize,
    /// Whether to reinforce traversed paths (learning from queries).
    pub reinforce: bool,
}

impl Query {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            max_results: 10,
            max_depth: 3,
            reinforce: true,
        }
    }

    pub fn with_max_results(mut self, n: usize) -> Self {
        self.max_results = n;
        self
    }

    pub fn with_max_depth(mut self, d: usize) -> Self {
        self.max_depth = d;
        self
    }

    pub fn without_reinforcement(mut self) -> Self {
        self.reinforce = false;
        self
    }
}

/// A single result from a query.
#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    /// The concept label.
    pub label: String,
    /// Node type (Concept, Insight, Anomaly).
    pub node_type: NodeType,
    /// How many times this node has been accessed/reinforced.
    pub access_count: u64,
    /// Relevance score (path weight × access count).
    pub score: f64,
    /// The path from a seed term to this result.
    pub path: Vec<String>,
    /// Node ID for further traversal.
    pub node_id: NodeId,
}

/// The query engine — traverses the Hebbian graph.
pub struct QueryEngine;

impl QueryEngine {
    /// Execute a query against the colony's knowledge graph.
    ///
    /// Returns results ranked by score (highest first).
    pub fn query(colony: &mut Colony, q: &Query) -> Vec<QueryResult> {
        let terms = tokenize(&q.text);
        let graph = colony.substrate().graph();

        // Phase 1: Find seed nodes (fuzzy substring matching)
        // Uses substring matching so queries like "membrane" find nodes
        // labeled "cell_membrane", "membrane_proteins", etc.
        let mut seed_nodes: Vec<(NodeId, String)> = Vec::new();
        let mut seed_seen: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
        for term in &terms {
            // First try exact match
            for nid in graph.find_nodes_by_exact_label(term) {
                if seed_seen.insert(*nid) {
                    if let Some(node) = graph.get_node(nid) {
                        seed_nodes.push((*nid, node.label.clone()));
                    }
                }
            }
            // Then try substring match for broader coverage
            for nid in graph.find_nodes_by_label(term) {
                if seed_seen.insert(nid) {
                    if let Some(node) = graph.get_node(&nid) {
                        seed_nodes.push((nid, node.label.clone()));
                    }
                }
            }
        }

        if seed_nodes.is_empty() {
            return Vec::new();
        }

        // Phase 2: Priority-queue traversal weighted by edge weight.
        // Uses a sorted frontier (best-first search) with limited expansion budget.
        // Stronger edges get explored first, so reinforcement directly affects
        // which nodes appear in results.
        let mut results: Vec<QueryResult> = Vec::new();
        let mut visited: std::collections::HashSet<NodeId> = std::collections::HashSet::new();

        // (priority=cumulative_weight, node_id, path, depth)
        // Using Vec as a max-heap (sort and pop from end)
        let mut frontier: Vec<(f64, NodeId, Vec<String>, usize)> = Vec::new();
        let max_expansions: usize = 200; // Budget limits how much of the graph we explore
        let mut expansions = 0;

        // Compute median edge weight to filter out weak edges during traversal
        let all_edges = graph.all_edges();
        let edge_threshold = if all_edges.is_empty() {
            0.0
        } else {
            let mut weights: Vec<f64> = all_edges.iter().map(|(_, _, e)| e.weight).collect();
            weights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            weights[weights.len() * 75 / 100]
        };

        for (nid, label) in &seed_nodes {
            visited.insert(*nid);
            if let Some(node) = graph.get_node(nid) {
                // Seeds that match query terms get a high base score
                let term_overlap = terms
                    .iter()
                    .filter(|t| node.label.to_lowercase().contains(t.as_str()))
                    .count() as f64;
                results.push(QueryResult {
                    label: node.label.clone(),
                    node_type: node.node_type.clone(),
                    access_count: node.access_count,
                    score: 10.0 + term_overlap * 5.0,
                    path: vec![label.clone()],
                    node_id: *nid,
                });
            }
            frontier.push((1.0, *nid, vec![label.clone()], 0));
        }

        // Best-first search with edge filtering and additive hop-decay scoring.
        // Only follows edges above the 75th percentile weight to avoid noise
        // in dense graphs. Uses additive scoring (base_weight * decay^depth)
        // instead of multiplicative cumulative weight which decays too fast.
        while !frontier.is_empty() && expansions < max_expansions {
            frontier.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let (weight, current, path, depth) = frontier.pop().unwrap();
            expansions += 1;

            if depth >= q.max_depth {
                continue;
            }

            let neighbors = graph.neighbors(&current);
            for (nid, edge) in &neighbors {
                if visited.contains(nid) {
                    continue;
                }
                // Only follow strong edges (above 75th percentile)
                if edge.weight < edge_threshold {
                    continue;
                }
                visited.insert(*nid);

                if let Some(node) = graph.get_node(nid) {
                    // Additive scoring: edge weight decays per hop, term overlap dominates
                    let hop_decay = 0.5_f64.powi((depth + 1) as i32);
                    let graph_score =
                        edge.weight * hop_decay * (1.0 + edge.co_activations as f64 * 0.1);
                    let term_overlap = terms
                        .iter()
                        .filter(|t| node.label.to_lowercase().contains(t.as_str()))
                        .count() as f64;
                    let term_bonus = term_overlap * 5.0;
                    let score = graph_score + term_bonus;

                    let mut node_path = path.clone();
                    node_path.push(node.label.clone());

                    results.push(QueryResult {
                        label: node.label.clone(),
                        node_type: node.node_type.clone(),
                        access_count: node.access_count,
                        score,
                        path: node_path.clone(),
                        node_id: *nid,
                    });

                    frontier.push((weight * edge.weight, *nid, node_path, depth + 1));
                }
            }
        }

        // Phase 3: Rank by score
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(q.max_results);

        // Phase 4: Reinforce traversed paths (the graph learns from queries)
        // True Hebbian: "neurons that fire together wire together"
        //
        // Key insight: A result node that connects to MULTIPLE seed terms
        // is more likely relevant (it bridges query concepts). We reinforce
        // these multi-seed results more heavily.
        if q.reinforce && !results.is_empty() {
            let seed_ids: Vec<NodeId> = seed_nodes.iter().map(|(nid, _)| *nid).collect();

            // Count how many seed nodes each result is connected to
            let graph_ref = colony.substrate().graph();
            let mut result_seed_connections: Vec<(NodeId, usize)> = Vec::new();
            for result in &results {
                let mut seed_count = 0;
                for seed_id in &seed_ids {
                    if *seed_id == result.node_id {
                        seed_count += 1;
                    } else if graph_ref.get_edge(seed_id, &result.node_id).is_some() {
                        seed_count += 1;
                    }
                }
                result_seed_connections.push((result.node_id, seed_count));
            }

            let graph_mut = colony.substrate_mut().graph_mut();

            // Reinforce based on seed connectivity (Hebbian correlation)
            for (result_id, seed_count) in &result_seed_connections {
                if *seed_count == 0 {
                    continue;
                }
                let multi_seed_bonus = *seed_count as f64;

                // Boost access count proportional to seed connectivity
                if let Some(node) = graph_mut.get_node_mut(result_id) {
                    node.access_count += (*seed_count as u64) * 2;
                }

                // Strengthen all seed↔result edges
                for seed_id in &seed_ids {
                    if seed_id == result_id {
                        continue;
                    }
                    if let Some(edge) = graph_mut.get_edge_mut(seed_id, result_id) {
                        let boost = 0.05 * multi_seed_bonus;
                        edge.weight = (edge.weight + boost).min(1.0);
                        edge.co_activations += 1;
                    }
                }
            }
        }

        results
    }
}

/// Simple tokenizer — lowercase, split on whitespace, filter stopwords and short words.
fn tokenize(text: &str) -> Vec<String> {
    let stopwords: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "shall", "can",
        "need", "dare", "ought", "used", "to", "of", "in", "for", "on", "with", "at", "by", "from",
        "as", "into", "through", "during", "before", "after", "above", "below", "between", "out",
        "off", "over", "under", "again", "further", "then", "once", "here", "there", "when",
        "where", "why", "how", "all", "each", "every", "both", "few", "more", "most", "other",
        "some", "such", "no", "nor", "not", "only", "own", "same", "so", "than", "too", "very",
        "and", "but", "or", "if", "while", "what", "which", "who", "this", "that", "these",
        "those", "it", "its",
    ]
    .iter()
    .cloned()
    .collect();

    text.to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() >= 3 && !stopwords.contains(w))
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| w.len() >= 3)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use phago_agents::digester::Digester;

    #[test]
    fn query_returns_results_from_digested_documents() {
        let mut colony = Colony::new();
        colony.ingest_document(
            "Biology",
            "The cell membrane controls transport of molecules. Proteins serve as channels \
             and receptors for signaling cascades in the cellular environment.",
            Position::new(0.0, 0.0),
        );
        colony.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(80),
        ));
        colony.run(15);

        let q = Query::new("cell membrane").without_reinforcement();
        let results = QueryEngine::query(&mut colony, &q);

        assert!(!results.is_empty(), "query should return results");
        assert!(
            results[0].score > 0.0,
            "results should have positive scores"
        );
    }

    #[test]
    fn query_reinforces_traversed_nodes() {
        let mut colony = Colony::new();
        colony.ingest_document(
            "Biology",
            "The cell membrane controls transport of molecules. Proteins serve as channels.",
            Position::new(0.0, 0.0),
        );
        colony.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(80),
        ));
        colony.run(15);

        // Get initial access count
        let q = Query::new("cell").without_reinforcement();
        let results_before = QueryEngine::query(&mut colony, &q);
        let initial_access = results_before.first().map(|r| r.access_count).unwrap_or(0);

        // Query with reinforcement
        let q = Query::new("cell");
        let _ = QueryEngine::query(&mut colony, &q);

        // Check access count increased
        let q = Query::new("cell").without_reinforcement();
        let results_after = QueryEngine::query(&mut colony, &q);
        let after_access = results_after.first().map(|r| r.access_count).unwrap_or(0);

        assert!(
            after_access > initial_access,
            "reinforcement should increase access count"
        );
    }

    #[test]
    fn empty_query_returns_empty() {
        let mut colony = Colony::new();
        let q = Query::new("nonexistent term xyz");
        let results = QueryEngine::query(&mut colony, &q);
        assert!(results.is_empty());
    }

    #[test]
    fn tokenizer_filters_stopwords() {
        let tokens = tokenize("the cell is a membrane");
        assert!(tokens.contains(&"cell".to_string()));
        assert!(tokens.contains(&"membrane".to_string()));
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"is".to_string()));
    }
}
