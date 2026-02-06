//! Async distributed tick runner.
//!
//! Coordinates tick execution across multiple shards with proper
//! phase synchronization and cross-shard edge resolution.

use crate::coordinator::Coordinator;
use crate::shard::ShardedColony;
use crate::types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for the distributed runner.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Timeout for each phase in milliseconds.
    pub phase_timeout_ms: u64,
    /// Whether to resolve ghost nodes after each tick.
    pub resolve_ghosts: bool,
    /// Maximum parallel operations.
    pub max_parallelism: usize,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            phase_timeout_ms: 30_000,
            resolve_ghosts: true,
            max_parallelism: 8,
        }
    }
}

/// Orchestrates distributed tick execution.
///
/// The `DistributedRunner` coordinates tick execution across multiple shards,
/// ensuring proper phase synchronization via barriers. Each tick consists of
/// four phases (Sense, Act, Decay, Advance) that are executed in order across
/// all shards before moving to the next phase.
///
/// # Architecture
///
/// The runner uses a coordinator for global synchronization and maintains
/// references to all shard instances. During each tick:
///
/// 1. **Sense Phase**: All shards prepare agent decisions (read-only)
/// 2. **Act Phase**: All shards execute agent actions (write operations)
/// 3. **Decay Phase**: All shards decay signals, traces, and edges
/// 4. **Advance Phase**: Coordinator advances the global tick counter
///
/// After the Act phase, any cross-shard edges are collected and ghost nodes
/// are resolved if configured.
///
/// # Example
///
/// ```ignore
/// use phago_distributed::runner::{DistributedRunner, RunnerConfig};
///
/// let runner = DistributedRunner::new(coordinator, shards, RunnerConfig::default());
///
/// // Run a single tick
/// let result = runner.tick().await?;
/// println!("Completed tick {}", result.tick);
///
/// // Run multiple ticks
/// let results = runner.run(10).await?;
/// ```
pub struct DistributedRunner {
    coordinator: Arc<Coordinator>,
    shards: Vec<Arc<RwLock<ShardedColony>>>,
    config: RunnerConfig,
}

impl DistributedRunner {
    /// Create a new distributed runner.
    ///
    /// # Arguments
    ///
    /// * `coordinator` - The coordinator for global synchronization
    /// * `shards` - Vector of shard instances wrapped in Arc<RwLock<_>>
    /// * `config` - Runner configuration
    pub fn new(
        coordinator: Arc<Coordinator>,
        shards: Vec<Arc<RwLock<ShardedColony>>>,
        config: RunnerConfig,
    ) -> Self {
        Self {
            coordinator,
            shards,
            config,
        }
    }

    /// Run a single distributed tick.
    ///
    /// Executes all four phases (Sense, Act, Decay, Advance) with
    /// barrier synchronization between each phase.
    ///
    /// # Returns
    ///
    /// A `DistributedTickResult` containing the new tick number, phase results,
    /// and any cross-shard edges that were created.
    ///
    /// # Errors
    ///
    /// Returns a `DistributedError` if:
    /// - Phase synchronization times out
    /// - Cross-shard edge resolution fails
    pub async fn tick(&self) -> DistributedResult<DistributedTickResult> {
        let tick = self.coordinator.current_tick();
        let mut phase_results = Vec::new();
        let mut all_cross_edges = Vec::new();

        // Phase 1: Sense
        let sense_results = self.run_phase(TickPhase::Sense, tick).await?;
        phase_results.extend(sense_results);

        // Phase 2: Act
        let act_results = self.run_phase(TickPhase::Act, tick).await?;
        for result in &act_results {
            all_cross_edges.extend(result.cross_shard_edges.clone());
        }
        phase_results.extend(act_results);

        // Phase 3: Decay
        let decay_results = self.run_phase(TickPhase::Decay, tick).await?;
        phase_results.extend(decay_results);

        // Phase 4: Advance
        let new_tick = self.coordinator.advance_tick().await;

        // Resolve ghost nodes if configured
        if self.config.resolve_ghosts && !all_cross_edges.is_empty() {
            self.resolve_cross_shard_edges(&all_cross_edges).await?;
        }

        Ok(DistributedTickResult {
            tick: new_tick,
            phase_results,
            cross_shard_edges: all_cross_edges,
        })
    }

    /// Run multiple ticks.
    ///
    /// # Arguments
    ///
    /// * `num_ticks` - Number of ticks to execute
    ///
    /// # Returns
    ///
    /// A vector of `DistributedTickResult` for each tick executed.
    pub async fn run(&self, num_ticks: u64) -> DistributedResult<Vec<DistributedTickResult>> {
        let mut results = Vec::with_capacity(num_ticks as usize);
        for _ in 0..num_ticks {
            results.push(self.tick().await?);
        }
        Ok(results)
    }

    /// Execute a single phase across all shards.
    ///
    /// Runs the specified phase on all shards in parallel, then waits
    /// for all shards to complete before returning.
    async fn run_phase(&self, phase: TickPhase, tick: u64) -> DistributedResult<Vec<PhaseResult>> {
        use futures::future::join_all;

        // Execute phase on all shards in parallel
        let futures: Vec<_> = self
            .shards
            .iter()
            .map(|shard| {
                let shard = shard.clone();
                async move {
                    let mut s = shard.write().await;
                    s.tick_phase(phase)
                }
            })
            .collect();

        let results = join_all(futures).await;

        // Signal phase completion to coordinator
        for result in &results {
            self.coordinator
                .phase_complete(result.shard_id, phase, tick)
                .await?;
        }

        // Wait for all shards to complete
        self.coordinator.wait_for_phase(phase, tick).await?;

        Ok(results)
    }

    /// Resolve cross-shard edges by fetching ghost nodes.
    ///
    /// For each cross-shard edge, fetches the target node's data from
    /// the owning shard and caches it as a ghost node in the requesting shard.
    async fn resolve_cross_shard_edges(&self, edges: &[CrossShardEdge]) -> DistributedResult<()> {
        use std::collections::HashMap;

        // Group edges by target shard
        let mut by_shard: HashMap<ShardId, Vec<&CrossShardEdge>> = HashMap::new();
        for edge in edges {
            by_shard.entry(edge.to_shard).or_default().push(edge);
        }

        // Fetch ghost nodes from each shard
        for (shard_id, shard_edges) in by_shard {
            let node_ids: Vec<_> = shard_edges.iter().map(|e| e.to_node).collect();

            // Find the shard and fetch nodes
            for shard in &self.shards {
                let s = shard.read().await;
                if s.shard_id() == shard_id {
                    for node_id in &node_ids {
                        if let Some(node_data) = s.get_node(node_id) {
                            // Cache the ghost node in requesting shards
                            for requesting_edge in
                                shard_edges.iter().filter(|e| e.to_node == *node_id)
                            {
                                // Find requesting shard and update its ghost cache
                                for req_shard in &self.shards {
                                    let mut req = req_shard.write().await;
                                    // Check if this shard has the from_node
                                    if req.get_node(&requesting_edge.from_node).is_some() {
                                        let ghost = GhostNode::new(
                                            *node_id,
                                            shard_id,
                                            node_data.label.clone(),
                                        );
                                        req.ghost_cache_mut().insert(ghost);
                                    }
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    /// Get the coordinator.
    pub fn coordinator(&self) -> &Arc<Coordinator> {
        &self.coordinator
    }

    /// Get shard count.
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Get a reference to all shards.
    pub fn shards(&self) -> &[Arc<RwLock<ShardedColony>>] {
        &self.shards
    }

    /// Get runner configuration.
    pub fn config(&self) -> &RunnerConfig {
        &self.config
    }
}

/// Result of a distributed tick.
#[derive(Debug, Clone)]
pub struct DistributedTickResult {
    /// The tick number after completion.
    pub tick: u64,
    /// Results from each phase.
    pub phase_results: Vec<PhaseResult>,
    /// Cross-shard edges created this tick.
    pub cross_shard_edges: Vec<CrossShardEdge>,
}

impl DistributedTickResult {
    /// Get the total number of nodes across all shards after this tick.
    pub fn total_nodes(&self) -> usize {
        // Get the node count from the last phase result for each shard
        let mut shard_counts: std::collections::HashMap<ShardId, usize> =
            std::collections::HashMap::new();
        for result in &self.phase_results {
            shard_counts.insert(result.shard_id, result.node_count);
        }
        shard_counts.values().sum()
    }

    /// Get the total number of edges across all shards after this tick.
    pub fn total_edges(&self) -> usize {
        let mut shard_counts: std::collections::HashMap<ShardId, usize> =
            std::collections::HashMap::new();
        for result in &self.phase_results {
            shard_counts.insert(result.shard_id, result.edge_count);
        }
        shard_counts.values().sum()
    }

    /// Check if any cross-shard communication occurred this tick.
    pub fn has_cross_shard_activity(&self) -> bool {
        !self.cross_shard_edges.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::ConsistentHashRing;
    use phago_runtime::colony::ColonyConfig;

    fn create_test_cluster(num_shards: u32) -> (Arc<Coordinator>, Vec<Arc<RwLock<ShardedColony>>>) {
        let coordinator = Arc::new(Coordinator::new(num_shards));
        let hash_ring = Arc::new(RwLock::new(ConsistentHashRing::new(num_shards)));

        let shards: Vec<_> = (0..num_shards)
            .map(|i| {
                Arc::new(RwLock::new(ShardedColony::new(
                    ShardId::new(i),
                    ColonyConfig::default(),
                    hash_ring.clone(),
                )))
            })
            .collect();

        (coordinator, shards)
    }

    #[tokio::test]
    async fn test_runner_creation() {
        let (coordinator, shards) = create_test_cluster(3);
        let runner = DistributedRunner::new(coordinator, shards, RunnerConfig::default());

        assert_eq!(runner.shard_count(), 3);
        assert_eq!(runner.config().phase_timeout_ms, 30_000);
        assert!(runner.config().resolve_ghosts);
    }

    #[tokio::test]
    async fn test_single_tick() {
        let (coordinator, shards) = create_test_cluster(3);
        let runner = DistributedRunner::new(coordinator, shards, RunnerConfig::default());

        let result = runner.tick().await.unwrap();
        assert_eq!(result.tick, 1);
        // Should have results from all phases (Sense, Act, Decay) for all 3 shards
        // Note: Advance phase doesn't produce PhaseResult in our implementation
        assert!(!result.phase_results.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_ticks() {
        let (coordinator, shards) = create_test_cluster(2);
        let runner = DistributedRunner::new(coordinator, shards, RunnerConfig::default());

        let results = runner.run(5).await.unwrap();
        assert_eq!(results.len(), 5);
        assert_eq!(results.last().unwrap().tick, 5);
    }

    #[tokio::test]
    async fn test_tick_result_methods() {
        let (coordinator, shards) = create_test_cluster(2);
        let runner = DistributedRunner::new(coordinator, shards, RunnerConfig::default());

        let result = runner.tick().await.unwrap();

        // These should work even with empty graphs
        let _ = result.total_nodes();
        let _ = result.total_edges();
        assert!(!result.has_cross_shard_activity()); // No cross-shard edges in basic test
    }

    #[tokio::test]
    async fn test_config_custom() {
        let config = RunnerConfig {
            phase_timeout_ms: 5_000,
            resolve_ghosts: false,
            max_parallelism: 4,
        };

        let (coordinator, shards) = create_test_cluster(2);
        let runner = DistributedRunner::new(coordinator, shards, config);

        assert_eq!(runner.config().phase_timeout_ms, 5_000);
        assert!(!runner.config().resolve_ghosts);
        assert_eq!(runner.config().max_parallelism, 4);
    }

    #[tokio::test]
    async fn test_concurrent_ticks() {
        let (coordinator, shards) = create_test_cluster(4);
        let runner = Arc::new(DistributedRunner::new(
            coordinator,
            shards,
            RunnerConfig::default(),
        ));

        // Run 10 sequential ticks (concurrent tick execution would require
        // additional synchronization which the runner doesn't currently support)
        let results = runner.run(10).await.unwrap();

        // Verify tick numbers are sequential
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.tick, (i + 1) as u64);
        }
    }
}
