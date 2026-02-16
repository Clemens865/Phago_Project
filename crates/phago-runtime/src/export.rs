//! Triple exporter — extract knowledge graph triples with Hebbian weights.
//!
//! Exports the colony's knowledge graph as (subject, predicate, object, weight)
//! triples, suitable for downstream processing into training data.

use crate::colony::Colony;
use phago_core::topology::TopologyGraph;
use serde::Serialize;

/// A knowledge graph triple with weight.
#[derive(Debug, Clone, Serialize)]
pub struct WeightedTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub weight: f64,
    pub co_activations: u64,
}

/// Export all edges as weighted triples.
pub fn export_triples(colony: &Colony) -> Vec<WeightedTriple> {
    let graph = colony.substrate().graph();
    let mut triples = Vec::new();

    for (from_id, to_id, edge) in graph.all_edges() {
        let from_label = graph
            .get_node(&from_id)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| "?".to_string());
        let to_label = graph
            .get_node(&to_id)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| "?".to_string());

        triples.push(WeightedTriple {
            subject: from_label,
            predicate: "related_to".to_string(),
            object: to_label,
            weight: edge.weight,
            co_activations: edge.co_activations,
        });
    }

    // Sort by weight descending (most important triples first)
    triples.sort_by(|a, b| {
        b.weight
            .partial_cmp(&a.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    triples
}

/// Export triples with weight statistics.
pub fn triple_stats(triples: &[WeightedTriple]) -> TripleStats {
    if triples.is_empty() {
        return TripleStats {
            total: 0,
            mean_weight: 0.0,
            median_weight: 0.0,
            max_weight: 0.0,
            min_weight: 0.0,
            mean_co_activations: 0.0,
        };
    }

    let weights: Vec<f64> = triples.iter().map(|t| t.weight).collect();
    let total = triples.len();
    let mean_weight = weights.iter().sum::<f64>() / total as f64;
    let max_weight = weights.iter().cloned().fold(0.0f64, f64::max);
    let min_weight = weights.iter().cloned().fold(f64::MAX, f64::min);

    let mut sorted = weights.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_weight = sorted[sorted.len() / 2];

    let mean_co_activations =
        triples.iter().map(|t| t.co_activations as f64).sum::<f64>() / total as f64;

    TripleStats {
        total,
        mean_weight,
        median_weight,
        max_weight,
        min_weight,
        mean_co_activations,
    }
}

/// Statistics about exported triples.
#[derive(Debug, Clone, Serialize)]
pub struct TripleStats {
    pub total: usize,
    pub mean_weight: f64,
    pub median_weight: f64,
    pub max_weight: f64,
    pub min_weight: f64,
    pub mean_co_activations: f64,
}
