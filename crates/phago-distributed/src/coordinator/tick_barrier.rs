//! Tick barrier for phase synchronization.
//!
//! This module implements a barrier mechanism that ensures all shards
//! complete each phase of a tick before any shard proceeds to the next phase.
//! This is essential for maintaining consistency in the distributed colony.

use crate::types::*;
use phago_core::types::Tick;
use std::collections::HashSet;
use tokio::sync::{Mutex, Notify};

/// Barrier ensuring all shards complete a phase before any proceeds.
///
/// The tick barrier coordinates the phases within each simulation tick:
/// 1. Sense - agents sense the substrate (read-only phase)
/// 2. Act - process agent actions (write phase)
/// 3. Decay - decay signals, traces, and edges (maintenance phase)
/// 4. Advance - advance tick counter (finalization phase)
///
/// Each shard must signal completion of each phase, and all shards must
/// complete before any can proceed to the next phase.
pub struct TickBarrier {
    /// Number of shards expected to participate.
    shard_count: Mutex<usize>,
    /// Set of (shard, phase, tick) tuples that have completed.
    completed: Mutex<HashSet<(ShardId, TickPhase, Tick)>>,
    /// Notification channel for waiters.
    notify: Notify,
    /// Default timeout for phase completion in seconds.
    phase_timeout_secs: u64,
}

impl TickBarrier {
    /// Create a new tick barrier for the specified number of shards.
    pub fn new(shard_count: usize) -> Self {
        Self {
            shard_count: Mutex::new(shard_count),
            completed: Mutex::new(HashSet::new()),
            notify: Notify::new(),
            phase_timeout_secs: 30,
        }
    }

    /// Create a tick barrier with custom timeout.
    pub fn with_timeout(shard_count: usize, timeout_secs: u64) -> Self {
        Self {
            shard_count: Mutex::new(shard_count),
            completed: Mutex::new(HashSet::new()),
            notify: Notify::new(),
            phase_timeout_secs: timeout_secs,
        }
    }

    /// Update the expected shard count.
    ///
    /// This should be called when shards are added or removed from the cluster.
    pub async fn set_shard_count(&self, count: usize) {
        let mut sc = self.shard_count.lock().await;
        *sc = count;
    }

    /// Get the current shard count.
    pub async fn shard_count(&self) -> usize {
        *self.shard_count.lock().await
    }

    /// Mark a shard as having completed a phase.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - The shard that completed
    /// * `phase` - The phase that was completed
    /// * `tick` - The tick number
    pub async fn complete(
        &self,
        shard_id: ShardId,
        phase: TickPhase,
        tick: Tick,
    ) -> DistributedResult<()> {
        let mut completed = self.completed.lock().await;
        completed.insert((shard_id, phase, tick));
        drop(completed);

        // Notify all waiters that progress was made
        self.notify.notify_waiters();

        Ok(())
    }

    /// Check if a specific shard has completed a phase.
    pub async fn is_complete(&self, shard_id: ShardId, phase: TickPhase, tick: Tick) -> bool {
        let completed = self.completed.lock().await;
        completed.contains(&(shard_id, phase, tick))
    }

    /// Get the number of shards that have completed a phase.
    pub async fn completed_count(&self, phase: TickPhase, tick: Tick) -> usize {
        let completed = self.completed.lock().await;
        completed
            .iter()
            .filter(|(_, p, t)| *p == phase && *t == tick)
            .count()
    }

    /// Wait for all shards to complete a phase.
    ///
    /// This will block until all registered shards have signaled completion
    /// of the specified phase, or until the timeout is reached.
    ///
    /// # Arguments
    ///
    /// * `phase` - The phase to wait for
    /// * `tick` - The tick number
    ///
    /// # Errors
    ///
    /// Returns `DistributedError::PhaseTimeout` if the timeout is reached
    /// before all shards complete.
    pub async fn wait_all(&self, phase: TickPhase, tick: Tick) -> DistributedResult<()> {
        let timeout = tokio::time::Duration::from_secs(self.phase_timeout_secs);

        loop {
            // Check if all shards have completed
            {
                let completed = self.completed.lock().await;
                let shard_count = *self.shard_count.lock().await;

                let count = completed
                    .iter()
                    .filter(|(_, p, t)| *p == phase && *t == tick)
                    .count();

                if count >= shard_count && shard_count > 0 {
                    return Ok(());
                }
            }

            // Wait for notification or timeout
            tokio::select! {
                _ = self.notify.notified() => {
                    // A shard completed, loop to check if all are done
                    continue;
                }
                _ = tokio::time::sleep(timeout) => {
                    return Err(DistributedError::PhaseTimeout(phase));
                }
            }
        }
    }

    /// Wait for all shards with custom timeout.
    pub async fn wait_all_with_timeout(
        &self,
        phase: TickPhase,
        tick: Tick,
        timeout: tokio::time::Duration,
    ) -> DistributedResult<()> {
        loop {
            {
                let completed = self.completed.lock().await;
                let shard_count = *self.shard_count.lock().await;

                let count = completed
                    .iter()
                    .filter(|(_, p, t)| *p == phase && *t == tick)
                    .count();

                if count >= shard_count && shard_count > 0 {
                    return Ok(());
                }
            }

            tokio::select! {
                _ = self.notify.notified() => continue,
                _ = tokio::time::sleep(timeout) => {
                    return Err(DistributedError::PhaseTimeout(phase));
                }
            }
        }
    }

    /// Reset the barrier for a new tick.
    ///
    /// This clears all completion records. Should be called before
    /// starting a new tick.
    pub async fn reset_for_tick(&self, _tick: Tick) {
        let mut completed = self.completed.lock().await;
        completed.clear();
    }

    /// Get all shards that have completed a specific phase.
    pub async fn get_completed_shards(&self, phase: TickPhase, tick: Tick) -> Vec<ShardId> {
        let completed = self.completed.lock().await;
        completed
            .iter()
            .filter(|(_, p, t)| *p == phase && *t == tick)
            .map(|(s, _, _)| *s)
            .collect()
    }

    /// Get all shards that have NOT completed a specific phase.
    pub async fn get_pending_shards(
        &self,
        phase: TickPhase,
        tick: Tick,
        all_shards: &[ShardId],
    ) -> Vec<ShardId> {
        let completed = self.completed.lock().await;
        let completed_set: HashSet<_> = completed
            .iter()
            .filter(|(_, p, t)| *p == phase && *t == tick)
            .map(|(s, _, _)| *s)
            .collect();

        all_shards
            .iter()
            .filter(|s| !completed_set.contains(s))
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_barrier_creation() {
        let barrier = TickBarrier::new(3);
        assert_eq!(barrier.shard_count().await, 3);
    }

    #[tokio::test]
    async fn test_phase_completion() {
        let barrier = TickBarrier::new(2);

        // Complete phase for shard 0
        barrier
            .complete(ShardId::new(0), TickPhase::Sense, 1)
            .await
            .unwrap();
        assert!(
            barrier
                .is_complete(ShardId::new(0), TickPhase::Sense, 1)
                .await
        );
        assert!(
            !barrier
                .is_complete(ShardId::new(1), TickPhase::Sense, 1)
                .await
        );

        // Complete phase for shard 1
        barrier
            .complete(ShardId::new(1), TickPhase::Sense, 1)
            .await
            .unwrap();
        assert!(
            barrier
                .is_complete(ShardId::new(1), TickPhase::Sense, 1)
                .await
        );
    }

    #[tokio::test]
    async fn test_wait_all_completes() {
        let barrier = TickBarrier::with_timeout(2, 5);

        // Spawn task to complete both shards
        let barrier_clone = std::sync::Arc::new(barrier);
        let barrier_ref = barrier_clone.clone();

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            barrier_ref
                .complete(ShardId::new(0), TickPhase::Sense, 1)
                .await
                .unwrap();
            barrier_ref
                .complete(ShardId::new(1), TickPhase::Sense, 1)
                .await
                .unwrap();
        });

        // Wait should succeed
        barrier_clone.wait_all(TickPhase::Sense, 1).await.unwrap();
    }

    #[tokio::test]
    async fn test_reset_for_tick() {
        let barrier = TickBarrier::new(1);

        barrier
            .complete(ShardId::new(0), TickPhase::Sense, 1)
            .await
            .unwrap();
        assert!(
            barrier
                .is_complete(ShardId::new(0), TickPhase::Sense, 1)
                .await
        );

        barrier.reset_for_tick(2).await;
        assert!(
            !barrier
                .is_complete(ShardId::new(0), TickPhase::Sense, 1)
                .await
        );
    }

    #[tokio::test]
    async fn test_completed_count() {
        let barrier = TickBarrier::new(3);

        assert_eq!(barrier.completed_count(TickPhase::Sense, 1).await, 0);

        barrier
            .complete(ShardId::new(0), TickPhase::Sense, 1)
            .await
            .unwrap();
        assert_eq!(barrier.completed_count(TickPhase::Sense, 1).await, 1);

        barrier
            .complete(ShardId::new(1), TickPhase::Sense, 1)
            .await
            .unwrap();
        assert_eq!(barrier.completed_count(TickPhase::Sense, 1).await, 2);
    }

    #[tokio::test]
    async fn test_update_shard_count() {
        let barrier = TickBarrier::new(2);
        assert_eq!(barrier.shard_count().await, 2);

        barrier.set_shard_count(5).await;
        assert_eq!(barrier.shard_count().await, 5);
    }
}
