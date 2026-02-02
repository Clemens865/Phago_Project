//! Information retrieval scoring metrics.
//!
//! Implements standard IR metrics: Precision@k, Recall@k, MRR, NDCG@k.
//! Used to benchmark bio-rag query performance against baselines.

use serde::Serialize;
use std::collections::HashSet;

/// Scores for a single query.
#[derive(Debug, Clone, Serialize)]
pub struct QueryScores {
    pub query: String,
    pub precision_at_5: f64,
    pub precision_at_10: f64,
    pub recall_at_5: f64,
    pub recall_at_10: f64,
    pub mrr: f64,
    pub ndcg_at_10: f64,
}

/// Aggregate scores across multiple queries.
#[derive(Debug, Clone, Serialize)]
pub struct AggregateScores {
    pub mean_precision_at_5: f64,
    pub mean_precision_at_10: f64,
    pub mean_recall_at_5: f64,
    pub mean_recall_at_10: f64,
    pub mean_mrr: f64,
    pub mean_ndcg_at_10: f64,
    pub query_count: usize,
}

/// Compute precision@k: fraction of top-k results that are relevant.
pub fn precision_at_k(retrieved: &[String], relevant: &HashSet<String>, k: usize) -> f64 {
    let top_k: Vec<&String> = retrieved.iter().take(k).collect();
    if top_k.is_empty() {
        return 0.0;
    }
    let hits = top_k.iter().filter(|r| relevant.contains(r.as_str())).count();
    hits as f64 / top_k.len() as f64
}

/// Compute recall@k: fraction of relevant items found in top-k.
pub fn recall_at_k(retrieved: &[String], relevant: &HashSet<String>, k: usize) -> f64 {
    if relevant.is_empty() {
        return 0.0;
    }
    let top_k: Vec<&String> = retrieved.iter().take(k).collect();
    let hits = top_k.iter().filter(|r| relevant.contains(r.as_str())).count();
    hits as f64 / relevant.len() as f64
}

/// Compute Mean Reciprocal Rank: 1/rank of first relevant result.
pub fn mrr(retrieved: &[String], relevant: &HashSet<String>) -> f64 {
    for (i, item) in retrieved.iter().enumerate() {
        if relevant.contains(item.as_str()) {
            return 1.0 / (i as f64 + 1.0);
        }
    }
    0.0
}

/// Compute NDCG@k: Normalized Discounted Cumulative Gain.
pub fn ndcg_at_k(retrieved: &[String], relevant: &HashSet<String>, k: usize) -> f64 {
    let dcg = dcg(retrieved, relevant, k);
    let ideal_dcg = ideal_dcg(relevant.len(), k);
    if ideal_dcg == 0.0 {
        return 0.0;
    }
    dcg / ideal_dcg
}

fn dcg(retrieved: &[String], relevant: &HashSet<String>, k: usize) -> f64 {
    retrieved
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, item)| {
            let rel = if relevant.contains(item.as_str()) { 1.0 } else { 0.0 };
            rel / (i as f64 + 2.0).log2()
        })
        .sum()
}

fn ideal_dcg(num_relevant: usize, k: usize) -> f64 {
    (0..num_relevant.min(k))
        .map(|i| 1.0 / (i as f64 + 2.0).log2())
        .sum()
}

/// Compute all scores for a single query.
pub fn score_query(
    query_text: &str,
    retrieved: &[String],
    relevant: &HashSet<String>,
) -> QueryScores {
    QueryScores {
        query: query_text.to_string(),
        precision_at_5: precision_at_k(retrieved, relevant, 5),
        precision_at_10: precision_at_k(retrieved, relevant, 10),
        recall_at_5: recall_at_k(retrieved, relevant, 5),
        recall_at_10: recall_at_k(retrieved, relevant, 10),
        mrr: mrr(retrieved, relevant),
        ndcg_at_10: ndcg_at_k(retrieved, relevant, 10),
    }
}

/// Aggregate scores across multiple queries.
pub fn aggregate(scores: &[QueryScores]) -> AggregateScores {
    let n = scores.len();
    if n == 0 {
        return AggregateScores {
            mean_precision_at_5: 0.0,
            mean_precision_at_10: 0.0,
            mean_recall_at_5: 0.0,
            mean_recall_at_10: 0.0,
            mean_mrr: 0.0,
            mean_ndcg_at_10: 0.0,
            query_count: 0,
        };
    }

    let nf = n as f64;
    AggregateScores {
        mean_precision_at_5: scores.iter().map(|s| s.precision_at_5).sum::<f64>() / nf,
        mean_precision_at_10: scores.iter().map(|s| s.precision_at_10).sum::<f64>() / nf,
        mean_recall_at_5: scores.iter().map(|s| s.recall_at_5).sum::<f64>() / nf,
        mean_recall_at_10: scores.iter().map(|s| s.recall_at_10).sum::<f64>() / nf,
        mean_mrr: scores.iter().map(|s| s.mrr).sum::<f64>() / nf,
        mean_ndcg_at_10: scores.iter().map(|s| s.ndcg_at_10).sum::<f64>() / nf,
        query_count: n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_precision() {
        let relevant: HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        let retrieved: Vec<String> = ["a", "b", "c", "d", "e"].iter().map(|s| s.to_string()).collect();
        assert!((precision_at_k(&retrieved, &relevant, 3) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn zero_precision_when_no_relevant() {
        let relevant: HashSet<String> = ["x", "y"].iter().map(|s| s.to_string()).collect();
        let retrieved: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert!((precision_at_k(&retrieved, &relevant, 3) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn mrr_first_position() {
        let relevant: HashSet<String> = ["a"].iter().map(|s| s.to_string()).collect();
        let retrieved: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert!((mrr(&retrieved, &relevant) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn mrr_second_position() {
        let relevant: HashSet<String> = ["b"].iter().map(|s| s.to_string()).collect();
        let retrieved: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert!((mrr(&retrieved, &relevant) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn ndcg_perfect() {
        let relevant: HashSet<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
        let retrieved: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert!((ndcg_at_k(&retrieved, &relevant, 3) - 1.0).abs() < 1e-10);
    }
}
