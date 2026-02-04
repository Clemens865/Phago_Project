//! Colony builder with configurable persistence.
//!
//! Provides a builder pattern for creating colonies with optional
//! SQLite-backed persistence for the knowledge graph.
//!
//! # Architecture
//!
//! The Colony uses PetTopologyGraph internally for simulation (required for
//! reference-based operations). Persistence is handled by:
//! - Loading initial state from SQLite on creation
//! - Saving state to SQLite on explicit save or drop
//!
//! This gives the benefits of persistence without compromising simulation performance.
//!
//! # Example
//!
//! ```rust,ignore
//! use phago_runtime::colony_builder::ColonyBuilder;
//! use phago_runtime::backend::BackendConfig;
//!
//! // Create a colony with SQLite persistence
//! let mut colony = ColonyBuilder::new()
//!     .with_persistence("knowledge.db")
//!     .auto_save(true)
//!     .build()?;
//!
//! // Run simulation
//! colony.run(100);
//!
//! // Explicitly save (also happens on drop if auto_save is enabled)
//! colony.save()?;
//! ```

use crate::colony::{Colony, ColonyConfig};
use crate::topology_impl::PetTopologyGraph;
use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use std::path::{Path, PathBuf};

#[cfg(feature = "sqlite")]
use crate::sqlite_topology::SqliteTopologyGraph;

/// Error type for colony builder operations.
#[derive(Debug)]
pub enum BuilderError {
    /// SQLite feature not enabled.
    SqliteNotEnabled,
    /// Failed to open or create database.
    DatabaseError(String),
    /// Failed to load or save state.
    PersistenceError(String),
}

impl std::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuilderError::SqliteNotEnabled => {
                write!(f, "SQLite feature not enabled. Add features = [\"sqlite\"] to Cargo.toml")
            }
            BuilderError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            BuilderError::PersistenceError(msg) => write!(f, "Persistence error: {}", msg),
        }
    }
}

impl std::error::Error for BuilderError {}

/// Builder for creating colonies with optional persistence.
pub struct ColonyBuilder {
    persistence_path: Option<PathBuf>,
    auto_save: bool,
    cache_size: usize,
    colony_config: ColonyConfig,
}

impl Default for ColonyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ColonyBuilder {
    /// Create a new colony builder with default settings.
    pub fn new() -> Self {
        Self {
            persistence_path: None,
            auto_save: false,
            cache_size: 1000,
            colony_config: ColonyConfig::default(),
        }
    }

    /// Set the colony configuration for simulation parameters.
    ///
    /// This allows customizing decay rates, pruning thresholds, and
    /// semantic wiring settings.
    pub fn with_config(mut self, config: ColonyConfig) -> Self {
        self.colony_config = config;
        self
    }

    /// Enable SQLite persistence at the given path.
    ///
    /// The database will be created if it doesn't exist.
    /// If it exists, the graph state will be loaded on build.
    #[cfg(feature = "sqlite")]
    pub fn with_persistence<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.persistence_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Stub for when SQLite is not enabled.
    #[cfg(not(feature = "sqlite"))]
    pub fn with_persistence<P: AsRef<Path>>(self, _path: P) -> Self {
        // Will error on build
        self
    }

    /// Enable automatic saving on colony drop.
    ///
    /// When enabled, the colony state will be persisted when the
    /// `PersistentColony` is dropped.
    pub fn auto_save(mut self, enabled: bool) -> Self {
        self.auto_save = enabled;
        self
    }

    /// Set the cache size for SQLite operations.
    pub fn cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    /// Build a standard Colony (no persistence).
    pub fn build_simple(self) -> Colony {
        Colony::from_config(self.colony_config)
    }

    /// Build a PersistentColony with SQLite backing.
    #[cfg(feature = "sqlite")]
    pub fn build(self) -> Result<PersistentColony, BuilderError> {
        let mut colony = Colony::from_config(self.colony_config);

        let persistence = if let Some(path) = self.persistence_path {
            let db = SqliteTopologyGraph::open(&path)
                .map_err(|e| BuilderError::DatabaseError(e.to_string()))?
                .with_cache_size(self.cache_size);

            // Load existing nodes into the colony's graph
            load_from_sqlite(&db, colony.substrate_mut().graph_mut())?;

            Some(PersistenceState {
                db,
                path,
                auto_save: self.auto_save,
            })
        } else {
            None
        };

        Ok(PersistentColony {
            colony,
            persistence,
        })
    }

    /// Build without SQLite feature - returns error if persistence was requested.
    #[cfg(not(feature = "sqlite"))]
    pub fn build(self) -> Result<PersistentColony, BuilderError> {
        if self.persistence_path.is_some() {
            return Err(BuilderError::SqliteNotEnabled);
        }
        Ok(PersistentColony {
            colony: Colony::from_config(self.colony_config),
            persistence: None,
        })
    }
}

/// Internal state for persistence.
#[cfg(feature = "sqlite")]
struct PersistenceState {
    db: SqliteTopologyGraph,
    path: PathBuf,
    auto_save: bool,
}

#[cfg(not(feature = "sqlite"))]
struct PersistenceState;

/// A Colony with optional SQLite persistence.
///
/// Wraps a standard Colony and provides automatic loading/saving
/// of the knowledge graph to SQLite.
pub struct PersistentColony {
    colony: Colony,
    #[cfg(feature = "sqlite")]
    persistence: Option<PersistenceState>,
    #[cfg(not(feature = "sqlite"))]
    persistence: Option<PersistenceState>,
}

impl PersistentColony {
    /// Get a reference to the inner colony.
    pub fn colony(&self) -> &Colony {
        &self.colony
    }

    /// Get a mutable reference to the inner colony.
    pub fn colony_mut(&mut self) -> &mut Colony {
        &mut self.colony
    }

    /// Consume self and return the inner colony.
    ///
    /// Note: This will NOT trigger auto-save. Call `save()` first if needed.
    pub fn into_inner(mut self) -> Colony {
        // Disable auto_save to prevent save on drop
        #[cfg(feature = "sqlite")]
        if let Some(ref mut state) = self.persistence {
            state.auto_save = false;
        }
        // Use ManuallyDrop to prevent Drop from running
        let colony = std::mem::replace(&mut self.colony, Colony::new());
        std::mem::forget(self); // Don't run Drop
        colony
    }

    /// Check if persistence is enabled.
    pub fn has_persistence(&self) -> bool {
        self.persistence.is_some()
    }

    /// Save the current graph state to SQLite.
    #[cfg(feature = "sqlite")]
    pub fn save(&mut self) -> Result<(), BuilderError> {
        if let Some(ref mut state) = self.persistence {
            save_to_sqlite(self.colony.substrate().graph(), &mut state.db)?;
        }
        Ok(())
    }

    #[cfg(not(feature = "sqlite"))]
    pub fn save(&mut self) -> Result<(), BuilderError> {
        Ok(())
    }

    /// Get the persistence path if configured.
    #[cfg(feature = "sqlite")]
    pub fn persistence_path(&self) -> Option<&Path> {
        self.persistence.as_ref().map(|s| s.path.as_path())
    }

    #[cfg(not(feature = "sqlite"))]
    pub fn persistence_path(&self) -> Option<&Path> {
        None
    }
}

// Delegate common Colony methods
impl PersistentColony {
    /// Run simulation for N ticks.
    pub fn run(&mut self, ticks: u64) -> Vec<Vec<crate::colony::ColonyEvent>> {
        self.colony.run(ticks)
    }

    /// Run a single tick.
    pub fn tick(&mut self) -> Vec<crate::colony::ColonyEvent> {
        self.colony.tick()
    }

    /// Ingest a document.
    pub fn ingest_document(&mut self, title: &str, content: &str, position: Position) -> DocumentId {
        self.colony.ingest_document(title, content, position)
    }

    /// Get colony statistics.
    pub fn stats(&self) -> crate::colony::ColonyStats {
        self.colony.stats()
    }

    /// Get a snapshot of the colony.
    pub fn snapshot(&self) -> crate::colony::ColonySnapshot {
        self.colony.snapshot()
    }

    /// Spawn an agent.
    pub fn spawn(
        &mut self,
        agent: Box<dyn phago_core::agent::Agent<Input = String, Fragment = String, Presentation = Vec<String>>>,
    ) -> AgentId {
        self.colony.spawn(agent)
    }

    /// Number of alive agents.
    pub fn alive_count(&self) -> usize {
        self.colony.alive_count()
    }
}

#[cfg(feature = "sqlite")]
impl Drop for PersistentColony {
    fn drop(&mut self) {
        if let Some(ref state) = self.persistence {
            if state.auto_save {
                // Best-effort save on drop
                let _ = save_to_sqlite(self.colony.substrate().graph(), &mut self.persistence.as_mut().unwrap().db);
            }
        }
    }
}

/// Load nodes and edges from SQLite into PetTopologyGraph.
#[cfg(feature = "sqlite")]
fn load_from_sqlite(
    source: &SqliteTopologyGraph,
    target: &mut PetTopologyGraph,
) -> Result<(), BuilderError> {
    // Load all nodes using the iterator
    let mut node_count = 0;
    for node in source.iter_nodes() {
        target.add_node(node);
        node_count += 1;
    }

    // Load all edges
    let mut edge_count = 0;
    for (from, to, edge) in source.iter_edges() {
        target.set_edge(from, to, edge);
        edge_count += 1;
    }

    if node_count > 0 || edge_count > 0 {
        eprintln!(
            "Loaded {} nodes and {} edges from SQLite database.",
            node_count, edge_count
        );
    }

    Ok(())
}

/// Save nodes and edges from PetTopologyGraph to SQLite.
#[cfg(feature = "sqlite")]
fn save_to_sqlite(
    source: &PetTopologyGraph,
    target: &mut SqliteTopologyGraph,
) -> Result<(), BuilderError> {
    // Save all nodes
    for node_id in source.all_nodes() {
        if let Some(node) = source.get_node(&node_id) {
            target.add_node(node.clone());
        }
    }

    // Save all edges
    for (from, to, edge) in source.all_edges() {
        target.set_edge(from, to, edge.clone());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_simple_colony() {
        let colony = ColonyBuilder::new().build_simple();
        assert_eq!(colony.alive_count(), 0);
    }

    #[test]
    fn build_without_persistence() {
        let colony = ColonyBuilder::new().build().unwrap();
        assert!(!colony.has_persistence());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn build_with_persistence() {
        let tmp = std::env::temp_dir().join("phago_builder_test.db");
        let _ = std::fs::remove_file(&tmp); // Clean up from previous runs

        let mut colony = ColonyBuilder::new()
            .with_persistence(&tmp)
            .build()
            .unwrap();

        assert!(colony.has_persistence());
        assert_eq!(colony.persistence_path(), Some(tmp.as_path()));

        // Ingest a document and save
        colony.ingest_document("Test", "Content", Position::new(0.0, 0.0));
        colony.run(5);
        colony.save().unwrap();

        // Clean up
        let _ = std::fs::remove_file(&tmp);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn auto_save_on_drop() {
        let tmp = std::env::temp_dir().join("phago_autosave_test.db");
        let _ = std::fs::remove_file(&tmp);

        {
            let mut colony = ColonyBuilder::new()
                .with_persistence(&tmp)
                .auto_save(true)
                .build()
                .unwrap();

            colony.ingest_document("Test", "Content", Position::new(0.0, 0.0));
            colony.run(5);
            // Drop should trigger save
        }

        // Verify file exists
        assert!(tmp.exists());
        let _ = std::fs::remove_file(&tmp);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn roundtrip_save_load() {
        use phago_agents::digester::Digester;

        let tmp = std::env::temp_dir().join("phago_roundtrip_test.db");
        let _ = std::fs::remove_file(&tmp);

        // Create colony, add data, save
        let (node_count, edge_count) = {
            let mut colony = ColonyBuilder::new()
                .with_persistence(&tmp)
                .build()
                .unwrap();

            colony.ingest_document("Biology 101", "Cell membrane proteins transport molecules", Position::new(0.0, 0.0));
            colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0)).with_max_idle(50)));
            colony.run(15);

            let stats = colony.stats();
            colony.save().unwrap();
            (stats.graph_nodes, stats.graph_edges)
        };

        // Load into new colony
        let colony2 = ColonyBuilder::new()
            .with_persistence(&tmp)
            .build()
            .unwrap();

        let stats2 = colony2.stats();

        // Verify data was loaded
        assert_eq!(stats2.graph_nodes, node_count, "Node count should match after reload");
        assert_eq!(stats2.graph_edges, edge_count, "Edge count should match after reload");

        let _ = std::fs::remove_file(&tmp);
    }
}
