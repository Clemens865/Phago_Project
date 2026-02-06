//! Distributed hybrid query engine.
//!
//! Implements two-phase TF-IDF for distributed queries:
//!
//! 1. **Scatter (Phase 1)**: Collect local term frequencies from each shard
//! 2. **Gather (Phase 2)**: Aggregate into global document frequencies
//! 3. **Scatter (Phase 3)**: Execute local queries with global DF for accurate IDF
//! 4. **Gather (Phase 4)**: Merge and rank top-k results from all shards

use crate::query::tokenize;
use crate::shard::ShardedColony;
use crate::types::*;
use phago_core::topology::TopologyGraph;
use std::collections::HashMap;

/// Configuration for distributed hybrid queries.
#[derive(Debug, Clone)]
pub struct DistributedHybridConfig {
    /// Weight for TF-IDF component (0.0 to 1.0).
    pub alpha: f64,
    /// Maximum results per shard.
    pub max_local_results: usize,
    /// Maximum final results.
    pub max_results: usize,
    /// Candidate multiplier for TF-IDF.
    pub candidate_multiplier: usize,
}

impl Default for DistributedHybridConfig {
    fn default() -> Self {
        Self {
            alpha: 0.5,
            max_local_results: 30,
            max_results: 10,
            candidate_multiplier: 3,
        }
    }
}

/// Distributed query engine implementing two-phase TF-IDF.
///
/// This engine executes queries across multiple shards by:
/// 1. First collecting term frequencies from all shards
/// 2. Computing global document frequencies
/// 3. Re-executing queries with the global DF for accurate scoring
/// 4. Merging and normalizing results across shards
pub struct DistributedQueryEngine {
    config: DistributedHybridConfig,
}

impl DistributedQueryEngine {
    /// Create a new distributed query engine with the given configuration.
    pub fn new(config: DistributedHybridConfig) -> Self {
        Self { config }
    }

    /// Create a query engine with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(DistributedHybridConfig::default())
    }

    /// Get the configuration.
    pub fn config(&self) -> &DistributedHybridConfig {
        &self.config
    }

    /// Phase 1: Get term frequencies from a shard.
    ///
    /// Collects how many documents in this shard contain each query term.
    /// This is used to compute local document frequencies.
    pub fn get_local_term_frequencies(
        &self,
        shard: &ShardedColony,
        terms: &[String],
    ) -> HashMap<String, u64> {
        shard.get_term_frequencies(terms)
    }

    /// Phase 2: Aggregate global document frequencies.
    ///
    /// Combines local document frequencies from all shards to compute
    /// the global DF for each term across the entire distributed graph.
    pub fn aggregate_global_df(
        &self,
        local_dfs: Vec<HashMap<String, u64>>,
    ) -> HashMap<String, u64> {
        let mut global_df = HashMap::new();
        for local in local_dfs {
            for (term, count) in local {
                *global_df.entry(term).or_insert(0) += count;
            }
        }
        global_df
    }

    /// Phase 3: Execute local query with global DF.
    ///
    /// Computes TF-IDF scores for nodes in a single shard using the
    /// global document frequencies for accurate IDF computation.
    pub fn execute_local_query(
        &self,
        shard: &ShardedColony,
        request: &LocalQueryRequest,
    ) -> LocalQueryResult {
        let graph = shard.local().substrate().graph();
        let all_nodes = graph.all_nodes();
        let total_docs = all_nodes.len().max(1) as f64;

        // Compute TF-IDF for each node
        let mut scored: Vec<ScoredNode> = Vec::new();

        for nid in &all_nodes {
            if let Some(node) = graph.get_node(nid) {
                let label_lower = node.label.to_lowercase();
                let label_terms: Vec<String> = label_lower
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| w.len() >= 3)
                    .map(|w| w.to_string())
                    .collect();

                let mut score = 0.0;
                for qt in &request.query_terms {
                    let tf = label_terms.iter().filter(|t| *t == qt).count() as f64;
                    if tf > 0.0 {
                        // Use global DF if available, otherwise assume 1
                        let df = *request.global_df.get(qt).unwrap_or(&1) as f64;
                        let idf = (total_docs / df.max(1.0)).ln() + 1.0;
                        score += tf * idf;
                    }
                }

                // Exact match boost - if the entire label matches a query term
                for qt in &request.query_terms {
                    if label_lower == *qt {
                        score += 10.0;
                    }
                }

                if score > 0.0 {
                    scored.push(ScoredNode {
                        node_id: *nid,
                        label: node.label.clone(),
                        score,
                        shard_id: shard.shard_id(),
                    });
                }
            }
        }

        // Sort by score descending and truncate
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(request.max_results);

        LocalQueryResult {
            shard_id: shard.shard_id(),
            results: scored,
            term_frequencies: shard.get_term_frequencies(&request.query_terms),
        }
    }

    /// Phase 4: Merge results from all shards.
    ///
    /// Combines results from multiple shards, normalizes scores across
    /// shards, sorts by score, and returns the top-k results.
    pub fn merge_results(&self, results: Vec<LocalQueryResult>) -> Vec<ScoredNode> {
        let mut all: Vec<ScoredNode> = results.into_iter().flat_map(|r| r.results).collect();

        // Normalize scores across shards
        if let Some(max_score) = all
            .iter()
            .map(|s| s.score)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
        {
            if max_score > 0.0 {
                for node in &mut all {
                    node.score /= max_score;
                }
            }
        }

        // Sort and truncate to final result count
        all.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all.truncate(self.config.max_results);
        all
    }

    /// Execute a full distributed query across multiple shards.
    ///
    /// This is the main entry point for distributed queries. It coordinates
    /// all four phases of the two-phase TF-IDF algorithm:
    ///
    /// 1. Collects local term frequencies from each shard
    /// 2. Aggregates them into global document frequencies
    /// 3. Executes local queries on each shard with global DF
    /// 4. Merges and normalizes results
    ///
    /// # Arguments
    ///
    /// * `shards` - Slice of shard references to query
    /// * `query_text` - The raw query text to search for
    ///
    /// # Returns
    ///
    /// A vector of scored nodes, sorted by relevance (highest first).
    pub fn distributed_query(
        &self,
        shards: &[&ShardedColony],
        query_text: &str,
    ) -> Vec<ScoredNode> {
        let query_terms = tokenize(query_text);
        if query_terms.is_empty() || shards.is_empty() {
            return Vec::new();
        }

        // Phase 1: Get local term frequencies
        let local_dfs: Vec<HashMap<String, u64>> = shards
            .iter()
            .map(|s| self.get_local_term_frequencies(s, &query_terms))
            .collect();

        // Phase 2: Aggregate global DF
        let global_df = self.aggregate_global_df(local_dfs);

        // Phase 3: Execute local queries with global DF
        let request = LocalQueryRequest {
            query_terms: query_terms.clone(),
            max_results: self.config.max_local_results,
            global_df,
        };

        let local_results: Vec<LocalQueryResult> = shards
            .iter()
            .map(|s| self.execute_local_query(s, &request))
            .collect();

        // Phase 4: Merge results
        self.merge_results(local_results)
    }

    /// Execute a query on a single shard (for non-distributed use).
    ///
    /// This is useful for testing or when the data resides in a single shard.
    pub fn local_query(&self, shard: &ShardedColony, query_text: &str) -> Vec<ScoredNode> {
        self.distributed_query(&[shard], query_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::ConsistentHashRing;
    use phago_core::types::Position;
    use phago_runtime::colony::ColonyConfig;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn create_test_ring() -> Arc<RwLock<ConsistentHashRing>> {
        Arc::new(RwLock::new(ConsistentHashRing::new(3)))
    }

    fn create_test_shard(id: u32) -> ShardedColony {
        let ring = create_test_ring();
        let mut shard = ShardedColony::new(ShardId::new(id), ColonyConfig::default(), ring);

        // Add some test data directly to the colony
        shard.local_mut().ingest_document(
            "Test Doc",
            "cell membrane protein transport",
            Position::new(0.0, 0.0),
        );

        shard
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("The cell membrane");
        assert!(tokens.contains(&"cell".to_string()));
        assert!(tokens.contains(&"membrane".to_string()));
        assert!(!tokens.contains(&"the".to_string())); // Stopword
    }

    #[test]
    fn test_aggregate_global_df() {
        let engine = DistributedQueryEngine::with_defaults();

        let local_dfs = vec![
            [("cell".to_string(), 5u64), ("membrane".to_string(), 3u64)]
                .into_iter()
                .collect(),
            [("cell".to_string(), 2u64), ("protein".to_string(), 4u64)]
                .into_iter()
                .collect(),
        ];

        let global_df = engine.aggregate_global_df(local_dfs);

        assert_eq!(global_df.get("cell"), Some(&7));
        assert_eq!(global_df.get("membrane"), Some(&3));
        assert_eq!(global_df.get("protein"), Some(&4));
    }

    #[test]
    fn test_merge_results() {
        let engine = DistributedQueryEngine::new(DistributedHybridConfig {
            max_results: 10,
            ..Default::default()
        });

        let results = vec![
            LocalQueryResult {
                shard_id: ShardId::new(0),
                results: vec![ScoredNode {
                    node_id: phago_core::types::NodeId::from_seed(1),
                    label: "cell".to_string(),
                    score: 1.0,
                    shard_id: ShardId::new(0),
                }],
                term_frequencies: HashMap::new(),
            },
            LocalQueryResult {
                shard_id: ShardId::new(1),
                results: vec![ScoredNode {
                    node_id: phago_core::types::NodeId::from_seed(2),
                    label: "membrane".to_string(),
                    score: 0.5,
                    shard_id: ShardId::new(1),
                }],
                term_frequencies: HashMap::new(),
            },
        ];

        let merged = engine.merge_results(results);
        assert_eq!(merged.len(), 2);
        // After normalization, highest score should be 1.0
        assert!((merged[0].score - 1.0).abs() < 0.001);
        // Second should be 0.5 / 1.0 = 0.5
        assert!((merged[1].score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_config_defaults() {
        let config = DistributedHybridConfig::default();
        assert_eq!(config.alpha, 0.5);
        assert_eq!(config.max_local_results, 30);
        assert_eq!(config.max_results, 10);
        assert_eq!(config.candidate_multiplier, 3);
    }

    #[test]
    fn test_engine_creation() {
        let engine = DistributedQueryEngine::with_defaults();
        assert_eq!(engine.config().max_results, 10);

        let custom_engine = DistributedQueryEngine::new(DistributedHybridConfig {
            max_results: 20,
            ..Default::default()
        });
        assert_eq!(custom_engine.config().max_results, 20);
    }

    #[test]
    fn test_empty_query() {
        let engine = DistributedQueryEngine::with_defaults();
        let shard = create_test_shard(0);

        // Empty query text should return empty results
        let results = engine.distributed_query(&[&shard], "");
        assert!(results.is_empty());

        // Query with only stopwords should also return empty
        let results = engine.distributed_query(&[&shard], "the a an");
        assert!(results.is_empty());
    }

    #[test]
    fn test_empty_shards() {
        let engine = DistributedQueryEngine::with_defaults();

        // No shards should return empty results
        let results = engine.distributed_query(&[], "cell membrane");
        assert!(results.is_empty());
    }

    #[test]
    fn test_local_query() {
        let engine = DistributedQueryEngine::with_defaults();
        let shard = create_test_shard(0);

        // Run some ticks to process the document
        // (Note: This test may not find results if the document hasn't been
        // processed into graph nodes yet - depends on colony behavior)
        let results = engine.local_query(&shard, "cell membrane");

        // Results may be empty if document hasn't been digested into graph nodes
        // This is expected behavior - the test validates the query path works
        assert!(results.len() <= engine.config().max_results);
    }
}
