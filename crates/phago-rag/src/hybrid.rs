//! Hybrid scoring — TF-IDF candidate selection + graph structural re-ranking.
//!
//! Strategy:
//! 1. TF-IDF generates a broad candidate set (2x max_results)
//! 2. Graph structure re-ranks candidates using:
//!    - Edge weight to query seed nodes (direct connectivity)
//!    - Co-activation count (reinforcement signal)
//!    - Node degree / centrality (hub importance)
//!    - Access count (usage frequency)
//! 3. Final score = alpha * tfidf_score + (1 - alpha) * graph_score

use phago_core::topology::TopologyGraph;
use phago_runtime::colony::Colony;
use std::collections::HashMap;

/// Configuration for hybrid scoring.
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Weight for TF-IDF component (0.0 to 1.0). Graph weight = 1.0 - alpha.
    pub alpha: f64,
    /// Maximum results to return.
    pub max_results: usize,
    /// Size of TF-IDF candidate pool (multiplier on max_results).
    pub candidate_multiplier: usize,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            alpha: 0.5,
            max_results: 10,
            candidate_multiplier: 3,
        }
    }
}

/// A hybrid query result with component scores.
#[derive(Debug, Clone)]
pub struct HybridResult {
    pub label: String,
    pub tfidf_score: f64,
    pub graph_score: f64,
    pub final_score: f64,
}

/// Execute a hybrid query: TF-IDF candidates re-ranked by graph structure.
pub fn hybrid_query(colony: &Colony, query_text: &str, config: &HybridConfig) -> Vec<HybridResult> {
    let query_terms = tokenize(query_text);
    if query_terms.is_empty() {
        return Vec::new();
    }

    let graph = colony.substrate().graph();
    let all_nodes = graph.all_nodes();
    let total_docs = all_nodes.len().max(1) as f64;

    // Phase 1: TF-IDF scoring for all nodes
    let mut df: HashMap<String, usize> = HashMap::new();
    for nid in &all_nodes {
        if let Some(node) = graph.get_node(nid) {
            let unique: std::collections::HashSet<String> = node
                .label
                .to_lowercase()
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| w.len() >= 3)
                .map(|w| w.to_string())
                .collect();
            for term in unique {
                *df.entry(term).or_insert(0) += 1;
            }
        }
    }

    let mut tfidf_scores: Vec<(phago_core::types::NodeId, String, f64)> = Vec::new();
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
            // Exact match boost
            for qt in &query_terms {
                if label_lower == *qt {
                    score += 10.0;
                }
            }

            if score > 0.0 {
                tfidf_scores.push((*nid, node.label.clone(), score));
            }
        }
    }

    // Sort by TF-IDF score and take top candidates
    tfidf_scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let candidate_count = config.max_results * config.candidate_multiplier;
    tfidf_scores.truncate(candidate_count);

    if tfidf_scores.is_empty() {
        return Vec::new();
    }

    // Normalize TF-IDF scores to [0, 1]
    let max_tfidf = tfidf_scores.first().map(|s| s.2).unwrap_or(1.0).max(0.001);

    // Phase 2: Find seed nodes (query terms that exactly match graph nodes)
    let seed_ids: Vec<phago_core::types::NodeId> = query_terms
        .iter()
        .flat_map(|t| graph.find_nodes_by_exact_label(t).to_vec())
        .collect();

    // Phase 3: Graph structural scoring for each candidate
    let mut results: Vec<HybridResult> = Vec::new();

    for (nid, label, tfidf_raw) in &tfidf_scores {
        let tfidf_norm = tfidf_raw / max_tfidf;

        // Graph score components:
        let mut graph_score = 0.0;

        // (a) Direct edge connectivity to seed nodes
        let mut max_edge_weight = 0.0_f64;
        let mut total_co_activations = 0_u64;
        for seed in &seed_ids {
            if seed == nid {
                continue;
            }
            if let Some(edge) = graph.get_edge(seed, nid) {
                max_edge_weight = max_edge_weight.max(edge.weight);
                total_co_activations += edge.co_activations;
            }
        }
        // Direct connectivity: 0-1 based on strongest seed edge
        graph_score += max_edge_weight * 0.4;
        // Co-activation bonus: diminishing returns
        graph_score += (total_co_activations as f64).ln().max(0.0) * 0.1;

        // (b) Node importance: degree-based (hub nodes are more central)
        if let Some(node) = graph.get_node(nid) {
            let degree = graph.neighbors(nid).len();
            let degree_score = (degree as f64).ln().max(0.0) / 5.0; // normalize
            graph_score += degree_score.min(1.0) * 0.2;

            // (c) Access count (usage frequency, Hebbian reinforcement)
            let access_score = (node.access_count as f64).ln().max(0.0) / 5.0;
            graph_score += access_score.min(1.0) * 0.3;
        }

        // Clamp graph_score to [0, 1]
        let graph_score_norm = graph_score.min(1.0);

        // Final blended score
        let final_score = config.alpha * tfidf_norm + (1.0 - config.alpha) * graph_score_norm;

        results.push(HybridResult {
            label: label.clone(),
            tfidf_score: tfidf_norm,
            graph_score: graph_score_norm,
            final_score,
        });
    }

    // Sort by final score and take top results
    results.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(config.max_results);
    results
}

/// Simple tokenizer matching the ones in query.rs and baseline.rs.
fn tokenize(text: &str) -> Vec<String> {
    let stopwords: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "shall", "can",
        "need", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into", "through",
        "during", "before", "after", "above", "below", "between", "out", "off", "over", "under",
        "again", "further", "then", "once", "and", "but", "or", "if", "while", "what", "which",
        "who", "this", "that", "these", "those", "it", "its", "how",
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
        // Two docs with shared terms to create reinforced edges
        colony.ingest_document(
            "Bio1",
            "The cell membrane controls transport of molecules. Proteins serve as channels.",
            Position::new(0.0, 0.0),
        );
        colony.ingest_document(
            "Bio2",
            "Cell signaling through membrane receptors activates protein cascades.",
            Position::new(1.0, 0.0),
        );
        colony.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(80),
        ));
        colony.spawn(Box::new(
            Digester::new(Position::new(1.0, 0.0)).with_max_idle(80),
        ));
        colony.run(20);
        colony
    }

    #[test]
    fn hybrid_returns_results() {
        let colony = setup_colony();
        let config = HybridConfig::default();
        let results = hybrid_query(&colony, "cell membrane", &config);
        assert!(!results.is_empty(), "hybrid should return results");
        assert!(results[0].final_score > 0.0);
    }

    #[test]
    fn hybrid_blends_scores() {
        let colony = setup_colony();
        let config = HybridConfig {
            alpha: 0.5,
            ..Default::default()
        };
        let results = hybrid_query(&colony, "cell membrane", &config);

        for r in &results {
            // Final score should be between tfidf and graph components
            let min = r.tfidf_score.min(r.graph_score) * 0.5;
            let max = r.tfidf_score.max(r.graph_score);
            assert!(
                r.final_score >= min * 0.9 && r.final_score <= max * 1.1,
                "final_score {:.3} should blend tfidf {:.3} and graph {:.3}",
                r.final_score,
                r.tfidf_score,
                r.graph_score
            );
        }
    }

    #[test]
    fn alpha_1_equals_pure_tfidf() {
        let colony = setup_colony();
        let config = HybridConfig {
            alpha: 1.0,
            max_results: 5,
            candidate_multiplier: 3,
        };
        let results = hybrid_query(&colony, "cell", &config);

        for r in &results {
            assert!(
                (r.final_score - r.tfidf_score).abs() < 1e-10,
                "alpha=1.0 should give pure TF-IDF: final={:.4}, tfidf={:.4}",
                r.final_score,
                r.tfidf_score
            );
        }
    }
}
