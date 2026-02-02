//! Baseline retrieval methods for comparison against bio-rag.
//!
//! Provides:
//! - TF-IDF keyword baseline (no graph, just term matching)
//! - Static graph baseline (same BFS, no reinforcement)
//! - Random baseline (random selection from all concepts)

use phago_core::topology::TopologyGraph;
use phago_runtime::colony::Colony;
use std::collections::HashMap;

/// TF-IDF baseline: score concepts by term frequency overlap with query.
///
/// Returns concept labels ranked by TF-IDF similarity to query terms.
pub fn tfidf_query(colony: &Colony, query_text: &str, max_results: usize) -> Vec<String> {
    let query_terms = tokenize(query_text);
    if query_terms.is_empty() {
        return Vec::new();
    }

    let graph = colony.substrate().graph();
    let all_nodes = graph.all_nodes();
    let total_docs = all_nodes.len().max(1) as f64;

    // Build document frequency for each term across all node labels
    let mut df: HashMap<String, usize> = HashMap::new();
    for nid in &all_nodes {
        if let Some(node) = graph.get_node(nid) {
            let label_terms: Vec<String> = node
                .label
                .to_lowercase()
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| w.len() >= 3)
                .map(|w| w.to_string())
                .collect();
            let unique: std::collections::HashSet<_> = label_terms.into_iter().collect();
            for term in unique {
                *df.entry(term).or_insert(0) += 1;
            }
        }
    }

    // Score each node by TF-IDF overlap with query
    let mut scores: Vec<(String, f64)> = Vec::new();
    for nid in &all_nodes {
        if let Some(node) = graph.get_node(nid) {
            let label_lower = node.label.to_lowercase();
            let label_terms: Vec<String> = label_lower
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| w.len() >= 3)
                .map(|w| w.to_string())
                .collect();

            let mut score = 0.0;
            for qt in &query_terms {
                let tf = label_terms.iter().filter(|t| *t == qt).count() as f64;
                if tf > 0.0 {
                    let idf = (total_docs / (*df.get(qt).unwrap_or(&1) as f64)).ln() + 1.0;
                    score += tf * idf;
                }
            }

            // Boost exact matches significantly
            for qt in &query_terms {
                if label_lower == *qt {
                    score += 10.0;
                }
            }

            if score > 0.0 {
                scores.push((node.label.clone(), score));
            }
        }
    }

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores
        .into_iter()
        .take(max_results)
        .map(|(label, _)| label)
        .collect()
}

/// Static graph baseline: same BFS traversal as QueryEngine but without
/// reinforcing traversed paths. Uses a fresh clone-like approach by
/// simply not calling reinforce.
pub fn static_graph_query(
    colony: &mut Colony,
    query_text: &str,
    max_results: usize,
) -> Vec<String> {
    use crate::query::{Query, QueryEngine};

    let q = Query::new(query_text)
        .with_max_results(max_results)
        .without_reinforcement();

    let results = QueryEngine::query(colony, &q);
    results.into_iter().map(|r| r.label).collect()
}

/// Random baseline: return random concept labels from the graph.
pub fn random_query(colony: &Colony, max_results: usize, seed: u64) -> Vec<String> {
    let graph = colony.substrate().graph();
    let all_nodes = graph.all_nodes();

    if all_nodes.is_empty() {
        return Vec::new();
    }

    // Simple deterministic shuffle using seed
    let mut indices: Vec<usize> = (0..all_nodes.len()).collect();
    let mut rng_state = seed;
    for i in (1..indices.len()).rev() {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (rng_state >> 33) as usize % (i + 1);
        indices.swap(i, j);
    }

    indices
        .into_iter()
        .take(max_results)
        .filter_map(|i| {
            all_nodes.get(i).and_then(|nid| {
                graph.get_node(nid).map(|n| n.label.clone())
            })
        })
        .collect()
}

/// Simple tokenizer matching the one in query.rs.
fn tokenize(text: &str) -> Vec<String> {
    let stopwords: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "need", "to", "of", "in",
        "for", "on", "with", "at", "by", "from", "as", "into", "through",
        "during", "before", "after", "above", "below", "between", "out",
        "off", "over", "under", "again", "further", "then", "once", "and",
        "but", "or", "if", "while", "what", "which", "who", "this", "that",
        "these", "those", "it", "its", "how",
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
    use phago_core::types::Position;

    fn setup_colony() -> Colony {
        let mut colony = Colony::new();
        colony.ingest_document(
            "Bio",
            "The cell membrane controls transport of molecules. Proteins serve as channels.",
            Position::new(0.0, 0.0),
        );
        colony.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(80),
        ));
        colony.run(15);
        colony
    }

    #[test]
    fn tfidf_returns_results() {
        let colony = setup_colony();
        let results = tfidf_query(&colony, "cell membrane", 5);
        assert!(!results.is_empty());
    }

    #[test]
    fn random_returns_results() {
        let colony = setup_colony();
        let results = random_query(&colony, 5, 42);
        assert!(!results.is_empty());
    }
}
