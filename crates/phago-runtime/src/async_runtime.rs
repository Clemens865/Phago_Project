//! Async runtime support for Phago Colony operations.
//!
//! This module provides async variants of key Colony operations for improved
//! throughput in I/O-bound scenarios (document ingestion, API calls, etc.).
//!
//! # Feature Flag
//!
//! This module requires the `async` feature:
//! ```toml
//! phago-runtime = { version = "0.1", features = ["async"] }
//! ```
//!
//! # Note on Send bounds
//!
//! Colony contains `Box<dyn Agent>` which is not `Send`, so spawned tasks
//! must use `spawn_local` within a `LocalSet`. For multi-threaded scenarios,
//! consider running separate colonies on separate threads.
//!
//! # Example
//!
//! ```rust,ignore
//! use phago_runtime::async_runtime::AsyncColony;
//! use phago_runtime::prelude::*;
//! use tokio::task::LocalSet;
//!
//! #[tokio::main]
//! async fn main() {
//!     let local = LocalSet::new();
//!     local.run_until(async {
//!         let colony = Colony::new();
//!         let async_colony = AsyncColony::new(colony);
//!
//!         // Ingest documents
//!         async_colony.ingest_documents_parallel(&[
//!             ("doc1", "content1", Position::new(0.0, 0.0)),
//!             ("doc2", "content2", Position::new(5.0, 0.0)),
//!         ]).await;
//!
//!         // Run simulation with async tick
//!         async_colony.run_async(50).await;
//!     }).await;
//! }
//! ```

#![cfg(feature = "async")]

use crate::colony::{Colony, ColonyEvent, ColonyStats, ColonySnapshot};
use phago_core::types::{DocumentId, Position};
use std::cell::RefCell;
use std::rc::Rc;
use tokio::task::JoinHandle;

/// Async wrapper around Colony for concurrent operations.
///
/// This wrapper provides async versions of Colony methods that can benefit
/// from concurrent execution, such as document ingestion and simulation ticks.
///
/// Since Colony is not `Send` (it contains `Box<dyn Agent>`), this wrapper
/// uses `Rc<RefCell<>>` and is designed for use within a `LocalSet`.
pub struct AsyncColony {
    colony: Rc<RefCell<Colony>>,
}

impl AsyncColony {
    /// Create a new AsyncColony wrapping an existing Colony.
    pub fn new(colony: Colony) -> Self {
        Self {
            colony: Rc::new(RefCell::new(colony)),
        }
    }

    /// Get a clone of the inner Rc for spawning local tasks.
    pub fn inner(&self) -> Rc<RefCell<Colony>> {
        Rc::clone(&self.colony)
    }

    /// Take ownership of the inner Colony, consuming the AsyncColony.
    ///
    /// # Panics
    /// Panics if there are other references to the colony.
    pub fn into_inner(self) -> Colony {
        match Rc::try_unwrap(self.colony) {
            Ok(cell) => cell.into_inner(),
            Err(_) => panic!("Cannot unwrap AsyncColony: other references exist"),
        }
    }

    /// Ingest a single document asynchronously.
    pub async fn ingest_document(
        &self,
        title: &str,
        content: &str,
        position: Position,
    ) -> DocumentId {
        let id = self.colony.borrow_mut().ingest_document(title, content, position);
        // Yield to allow other tasks to progress
        tokio::task::yield_now().await;
        id
    }

    /// Ingest multiple documents, yielding between each.
    ///
    /// This allows other async tasks to make progress during batch ingestion.
    pub async fn ingest_documents_parallel(
        &self,
        documents: &[(&str, &str, Position)],
    ) -> Vec<DocumentId> {
        let mut ids = Vec::with_capacity(documents.len());
        for (title, content, position) in documents {
            let id = self.colony.borrow_mut().ingest_document(title, content, *position);
            ids.push(id);
            // Yield to allow other tasks to progress
            tokio::task::yield_now().await;
        }
        ids
    }

    /// Run a single simulation tick asynchronously.
    ///
    /// This yields control after the tick to allow other async tasks to progress.
    pub async fn tick_async(&self) -> Vec<ColonyEvent> {
        let events = self.colony.borrow_mut().tick();
        tokio::task::yield_now().await;
        events
    }

    /// Run multiple simulation ticks asynchronously.
    ///
    /// Yields between ticks to allow other async tasks to make progress.
    pub async fn run_async(&self, ticks: u64) -> Vec<Vec<ColonyEvent>> {
        let mut all_events = Vec::with_capacity(ticks as usize);
        for _ in 0..ticks {
            let events = self.colony.borrow_mut().tick();
            all_events.push(events);
            tokio::task::yield_now().await;
        }
        all_events
    }

    /// Run simulation ticks with a callback after each tick.
    ///
    /// Useful for progress reporting or early termination conditions.
    /// The callback receives (tick_number, events) and returns whether to continue.
    pub async fn run_with_callback<F>(
        &self,
        ticks: u64,
        mut callback: F,
    ) -> Vec<Vec<ColonyEvent>>
    where
        F: FnMut(u64, &[ColonyEvent]) -> bool,
    {
        let mut all_events = Vec::with_capacity(ticks as usize);
        for tick in 0..ticks {
            let events = self.colony.borrow_mut().tick();
            let should_continue = callback(tick, &events);
            all_events.push(events);
            if !should_continue {
                break;
            }
            tokio::task::yield_now().await;
        }
        all_events
    }

    /// Get colony statistics.
    pub fn stats(&self) -> ColonyStats {
        self.colony.borrow().stats()
    }

    /// Get a snapshot of the colony.
    pub fn snapshot(&self) -> ColonySnapshot {
        self.colony.borrow().snapshot()
    }

    /// Get the number of alive agents.
    pub fn alive_count(&self) -> usize {
        self.colony.borrow().alive_count()
    }
}

/// Spawn a local task that runs the colony simulation.
///
/// Must be called within a `LocalSet` context.
///
/// # Example
///
/// ```rust,ignore
/// use phago_runtime::async_runtime::{AsyncColony, spawn_simulation_local};
/// use tokio::task::LocalSet;
///
/// let local = LocalSet::new();
/// local.run_until(async {
///     let colony = Colony::new();
///     let rc = Rc::new(RefCell::new(colony));
///
///     let handle = spawn_simulation_local(Rc::clone(&rc), 100);
///     let events = handle.await.unwrap();
/// }).await;
/// ```
pub fn spawn_simulation_local(
    colony: Rc<RefCell<Colony>>,
    ticks: u64,
) -> JoinHandle<Vec<Vec<ColonyEvent>>> {
    tokio::task::spawn_local(async move {
        let mut all_events = Vec::with_capacity(ticks as usize);
        for _ in 0..ticks {
            let events = colony.borrow_mut().tick();
            all_events.push(events);
            tokio::task::yield_now().await;
        }
        all_events
    })
}

/// Batch document ingestion with controlled concurrency.
///
/// Processes documents in batches, yielding between batches.
pub async fn batch_ingest(
    colony: Rc<RefCell<Colony>>,
    documents: Vec<(String, String, Position)>,
    batch_size: usize,
) -> Vec<DocumentId> {
    let mut ids = Vec::with_capacity(documents.len());

    for batch in documents.chunks(batch_size) {
        for (title, content, position) in batch {
            let id = colony.borrow_mut().ingest_document(title, content, *position);
            ids.push(id);
        }
        // Yield between batches
        tokio::task::yield_now().await;
    }

    ids
}

/// A ticker that runs simulation steps at a controlled rate.
///
/// Useful for real-time simulations or visualizations where you want
/// to pace the simulation at a specific rate.
pub struct TickTimer {
    interval: tokio::time::Interval,
}

impl TickTimer {
    /// Create a new tick timer with the specified interval in milliseconds.
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval: tokio::time::interval(tokio::time::Duration::from_millis(interval_ms)),
        }
    }

    /// Wait for the next tick interval.
    pub async fn tick(&mut self) {
        self.interval.tick().await;
    }

    /// Run a colony at a controlled tick rate.
    pub async fn run_timed(
        &mut self,
        colony: &AsyncColony,
        ticks: u64,
    ) -> Vec<Vec<ColonyEvent>> {
        let mut all_events = Vec::with_capacity(ticks as usize);
        for _ in 0..ticks {
            self.tick().await;
            let events = colony.tick_async().await;
            all_events.push(events);
        }
        all_events
    }
}

/// Run a colony simulation within a LocalSet.
///
/// This is a convenience function for running async colony operations
/// without manually setting up the LocalSet.
///
/// # Example
///
/// ```rust,ignore
/// use phago_runtime::async_runtime::run_in_local;
/// use phago_runtime::prelude::*;
///
/// #[tokio::main]
/// async fn main() {
///     let colony = Colony::new();
///     let events = run_in_local(colony, |async_colony| async move {
///         async_colony.run_async(50).await
///     }).await;
/// }
/// ```
pub async fn run_in_local<F, Fut, T>(colony: Colony, f: F) -> T
where
    F: FnOnce(AsyncColony) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let local = tokio::task::LocalSet::new();
    local.run_until(async move {
        let async_colony = AsyncColony::new(colony);
        f(async_colony).await
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::task::LocalSet;

    async fn run_test<F, Fut>(f: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let local = LocalSet::new();
        local.run_until(f()).await;
    }

    #[tokio::test]
    async fn async_colony_basic_operations() {
        run_test(|| async {
            let colony = Colony::new();
            let async_colony = AsyncColony::new(colony);

            // Ingest a document
            let _doc_id = async_colony
                .ingest_document("Test", "Cell membrane protein", Position::new(0.0, 0.0))
                .await;

            let stats = async_colony.stats();
            assert_eq!(stats.documents_total, 1);

            // Run simulation
            let events = async_colony.run_async(5).await;
            assert_eq!(events.len(), 5);
        }).await;
    }

    #[tokio::test]
    async fn async_colony_parallel_documents() {
        run_test(|| async {
            let colony = Colony::new();
            let async_colony = AsyncColony::new(colony);

            let docs = vec![
                ("Doc 1", "Content about cells", Position::new(0.0, 0.0)),
                ("Doc 2", "Content about proteins", Position::new(5.0, 0.0)),
                ("Doc 3", "Content about membranes", Position::new(10.0, 0.0)),
            ];

            let ids = async_colony.ingest_documents_parallel(&docs).await;
            assert_eq!(ids.len(), 3);

            let stats = async_colony.stats();
            assert_eq!(stats.documents_total, 3);
        }).await;
    }

    #[tokio::test]
    async fn async_colony_with_callback() {
        run_test(|| async {
            let mut colony = Colony::new();
            colony.ingest_document("Test", "Cell membrane", Position::new(0.0, 0.0));

            let async_colony = AsyncColony::new(colony);

            let mut tick_count = 0;
            let events = async_colony
                .run_with_callback(10, |tick, _events| {
                    tick_count = tick + 1;
                    tick < 5 // Stop after 5 ticks
                })
                .await;

            assert_eq!(events.len(), 6); // 0..5 inclusive = 6 ticks
            assert_eq!(tick_count, 6);
        }).await;
    }

    #[tokio::test]
    async fn spawn_simulation_local_works() {
        let local = LocalSet::new();
        local.run_until(async {
            let mut colony = Colony::new();
            colony.ingest_document("Test", "Content", Position::new(0.0, 0.0));

            let rc = Rc::new(RefCell::new(colony));
            let handle = spawn_simulation_local(Rc::clone(&rc), 10);

            let events = handle.await.unwrap();
            assert_eq!(events.len(), 10);
        }).await;
    }

    #[tokio::test]
    async fn batch_ingest_works() {
        run_test(|| async {
            let colony = Colony::new();
            let rc = Rc::new(RefCell::new(colony));

            let docs: Vec<_> = (0..10)
                .map(|i| (format!("Doc {}", i), format!("Content {}", i), Position::new(i as f64, 0.0)))
                .collect();

            let ids = batch_ingest(Rc::clone(&rc), docs, 3).await;
            assert_eq!(ids.len(), 10);

            let stats = rc.borrow().stats();
            assert_eq!(stats.documents_total, 10);
        }).await;
    }

    #[tokio::test]
    async fn tick_timer_controlled_rate() {
        run_test(|| async {
            let colony = Colony::new();
            let async_colony = AsyncColony::new(colony);

            let mut timer = TickTimer::new(10); // 10ms interval
            let start = tokio::time::Instant::now();

            let events = timer.run_timed(&async_colony, 5).await;

            let elapsed = start.elapsed();
            assert_eq!(events.len(), 5);
            // Should have taken at least 40ms (5 ticks * ~10ms, minus first immediate)
            assert!(elapsed.as_millis() >= 40);
        }).await;
    }

    #[tokio::test]
    async fn run_in_local_convenience() {
        let colony = Colony::new();
        let events = run_in_local(colony, |async_colony| async move {
            async_colony.run_async(5).await
        }).await;
        assert_eq!(events.len(), 5);
    }
}
