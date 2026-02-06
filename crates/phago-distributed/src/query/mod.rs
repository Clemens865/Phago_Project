//! Distributed query implementation.
//!
//! This module handles queries that span multiple shards using a
//! two-phase TF-IDF approach:
//!
//! 1. **Scatter (Phase 1)**: Get local term frequencies from all shards
//! 2. **Gather (Phase 2)**: Aggregate to global document frequencies
//! 3. **Scatter (Phase 3)**: Compute TF-IDF with global DF
//! 4. **Gather (Phase 4)**: Merge top-k results

mod distributed;

pub use distributed::{DistributedHybridConfig, DistributedQueryEngine};

use crate::types::*;

/// Tokenizer matching phago-rag's tokenizer.
///
/// Performs case-insensitive tokenization with stopword removal
/// and minimum token length filtering.
pub fn tokenize(text: &str) -> Vec<String> {
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

/// Merge scored results from multiple shards.
///
/// Combines results from multiple shards, sorts by score (descending),
/// and truncates to the specified maximum.
pub fn merge_results(results: Vec<Vec<ScoredNode>>, max_results: usize) -> Vec<ScoredNode> {
    let mut all: Vec<ScoredNode> = results.into_iter().flatten().collect();
    all.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all.truncate(max_results);
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use phago_core::types::NodeId;

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("The cell membrane");
        assert!(tokens.contains(&"cell".to_string()));
        assert!(tokens.contains(&"membrane".to_string()));
        // "The" should be filtered as stopword
        assert!(!tokens.contains(&"the".to_string()));
    }

    #[test]
    fn test_tokenize_filters_short_words() {
        let tokens = tokenize("a is the on by");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_trims_punctuation() {
        let tokens = tokenize("cell, membrane.");
        assert!(tokens.contains(&"cell".to_string()));
        assert!(tokens.contains(&"membrane".to_string()));
    }

    #[test]
    fn test_tokenize_lowercase() {
        let tokens = tokenize("CELL Membrane");
        assert!(tokens.contains(&"cell".to_string()));
        assert!(tokens.contains(&"membrane".to_string()));
    }

    #[test]
    fn test_merge_results_empty() {
        let results: Vec<Vec<ScoredNode>> = vec![];
        let merged = merge_results(results, 10);
        assert!(merged.is_empty());
    }

    #[test]
    fn test_merge_results_sorting() {
        let results = vec![
            vec![ScoredNode {
                node_id: NodeId::from_seed(1),
                label: "low".to_string(),
                score: 0.3,
                shard_id: ShardId::new(0),
            }],
            vec![ScoredNode {
                node_id: NodeId::from_seed(2),
                label: "high".to_string(),
                score: 0.9,
                shard_id: ShardId::new(1),
            }],
        ];
        let merged = merge_results(results, 10);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].label, "high");
        assert_eq!(merged[1].label, "low");
    }

    #[test]
    fn test_merge_results_truncates() {
        let results = vec![vec![
            ScoredNode {
                node_id: NodeId::from_seed(1),
                label: "a".to_string(),
                score: 0.9,
                shard_id: ShardId::new(0),
            },
            ScoredNode {
                node_id: NodeId::from_seed(2),
                label: "b".to_string(),
                score: 0.8,
                shard_id: ShardId::new(0),
            },
            ScoredNode {
                node_id: NodeId::from_seed(3),
                label: "c".to_string(),
                score: 0.7,
                shard_id: ShardId::new(0),
            },
        ]];
        let merged = merge_results(results, 2);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].label, "a");
        assert_eq!(merged[1].label, "b");
    }
}
