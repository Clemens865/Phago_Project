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

        // Phase 1: Find seed nodes (exact label matches)
        let mut seed_nodes: Vec<(NodeId, String)> = Vec::new();
        for term in &terms {
            for nid in graph.find_nodes_by_exact_label(term) {
                if let Some(node) = graph.get_node(nid) {
                    seed_nodes.push((*nid, node.label.clone()));
                }
            }
        }

        if seed_nodes.is_empty() {
            return Vec::new();
        }

        // Phase 2: BFS traversal weighted by edge weight
        let mut results: Vec<QueryResult> = Vec::new();
        let mut visited: std::collections::HashSet<NodeId> = std::collections::HashSet::new();

        // (node_id, cumulative_weight, path, depth)
        let mut frontier: Vec<(NodeId, f64, Vec<String>, usize)> = Vec::new();

        for (nid, label) in &seed_nodes {
            visited.insert(*nid);
            if let Some(node) = graph.get_node(nid) {
                results.push(QueryResult {
                    label: node.label.clone(),
                    node_type: node.node_type.clone(),
                    access_count: node.access_count,
                    score: node.access_count as f64 * 10.0, // Seed nodes get high base score
                    path: vec![label.clone()],
                    node_id: *nid,
                });
            }
            frontier.push((*nid, 1.0, vec![label.clone()], 0));
        }

        // BFS outward
        while let Some((current, weight, path, depth)) = frontier.pop() {
            if depth >= q.max_depth {
                continue;
            }

            let neighbors = graph.neighbors(&current);
            let mut scored_neighbors: Vec<_> = neighbors.iter()
                .filter(|(nid, _)| !visited.contains(nid))
                .map(|(nid, edge)| (*nid, edge.weight, edge.co_activations))
                .collect();

            // Sort by edge weight descending — explore strongest connections first
            scored_neighbors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (nid, edge_weight, _co_act) in scored_neighbors {
                if visited.contains(&nid) {
                    continue;
                }
                visited.insert(nid);

                if let Some(node) = graph.get_node(&nid) {
                    let cumulative_weight = weight * edge_weight;
                    let score = cumulative_weight * node.access_count as f64;

                    let mut node_path = path.clone();
                    node_path.push(node.label.clone());

                    results.push(QueryResult {
                        label: node.label.clone(),
                        node_type: node.node_type.clone(),
                        access_count: node.access_count,
                        score,
                        path: node_path.clone(),
                        node_id: nid,
                    });

                    frontier.push((nid, cumulative_weight, node_path, depth + 1));
                }
            }
        }

        // Phase 3: Rank by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(q.max_results);

        // Phase 4: Reinforce traversed paths (the graph learns from queries)
        if q.reinforce && !results.is_empty() {
            let result_node_ids: Vec<NodeId> = results.iter().map(|r| r.node_id).collect();
            let graph_mut = colony.substrate_mut().graph_mut();
            for nid in &result_node_ids {
                if let Some(node) = graph_mut.get_node_mut(nid) {
                    node.access_count += 1;
                }
            }
        }

        results
    }
}

/// Simple tokenizer — lowercase, split on whitespace, filter stopwords and short words.
fn tokenize(text: &str) -> Vec<String> {
    let stopwords: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "need", "dare", "ought",
        "used", "to", "of", "in", "for", "on", "with", "at", "by", "from",
        "as", "into", "through", "during", "before", "after", "above", "below",
        "between", "out", "off", "over", "under", "again", "further", "then",
        "once", "here", "there", "when", "where", "why", "how", "all", "each",
        "every", "both", "few", "more", "most", "other", "some", "such", "no",
        "nor", "not", "only", "own", "same", "so", "than", "too", "very",
        "and", "but", "or", "if", "while", "what", "which", "who", "this",
        "that", "these", "those", "it", "its",
    ].iter().cloned().collect();

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
        colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0)).with_max_idle(80)));
        colony.run(15);

        let q = Query::new("cell membrane").without_reinforcement();
        let results = QueryEngine::query(&mut colony, &q);

        assert!(!results.is_empty(), "query should return results");
        assert!(results[0].score > 0.0, "results should have positive scores");
    }

    #[test]
    fn query_reinforces_traversed_nodes() {
        let mut colony = Colony::new();
        colony.ingest_document(
            "Biology",
            "The cell membrane controls transport of molecules. Proteins serve as channels.",
            Position::new(0.0, 0.0),
        );
        colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0)).with_max_idle(80)));
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

        assert!(after_access > initial_access, "reinforcement should increase access count");
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
