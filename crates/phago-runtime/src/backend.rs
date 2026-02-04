//! Graph backend configuration and factory.
//!
//! Provides a unified interface for selecting and configuring graph storage backends.

use crate::topology_impl::PetTopologyGraph;
use phago_core::topology::TopologyGraph;

#[cfg(feature = "sqlite")]
use std::path::PathBuf;

/// Configuration for graph backend selection.
#[derive(Debug, Clone)]
pub enum BackendConfig {
    /// In-memory petgraph backend (default, fast, no persistence).
    InMemory,

    /// SQLite-backed persistent storage.
    #[cfg(feature = "sqlite")]
    Sqlite {
        /// Path to the SQLite database file.
        /// If None, uses an in-memory SQLite database.
        path: Option<PathBuf>,
        /// Node cache size for frequently accessed nodes.
        cache_size: usize,
    },
}

impl Default for BackendConfig {
    fn default() -> Self {
        BackendConfig::InMemory
    }
}

impl BackendConfig {
    /// Create an in-memory backend configuration.
    pub fn in_memory() -> Self {
        BackendConfig::InMemory
    }

    /// Create an SQLite backend configuration with a file path.
    #[cfg(feature = "sqlite")]
    pub fn sqlite(path: impl Into<PathBuf>) -> Self {
        BackendConfig::Sqlite {
            path: Some(path.into()),
            cache_size: 1000,
        }
    }

    /// Create an SQLite backend configuration with in-memory storage.
    #[cfg(feature = "sqlite")]
    pub fn sqlite_in_memory() -> Self {
        BackendConfig::Sqlite {
            path: None,
            cache_size: 1000,
        }
    }

    /// Set the cache size for SQLite backend.
    #[cfg(feature = "sqlite")]
    pub fn with_cache_size(mut self, size: usize) -> Self {
        if let BackendConfig::Sqlite { cache_size, .. } = &mut self {
            *cache_size = size;
        }
        self
    }
}

/// Trait object for graph backends.
///
/// This allows storing different backend implementations behind a single type.
pub type DynTopologyGraph = Box<dyn TopologyGraph + Send + Sync>;

/// Create a graph backend from configuration.
///
/// # Errors
/// Returns an error if the backend cannot be created (e.g., SQLite file issues).
pub fn create_backend(config: &BackendConfig) -> Result<DynTopologyGraph, BackendError> {
    match config {
        BackendConfig::InMemory => Ok(Box::new(PetTopologyGraph::new())),

        #[cfg(feature = "sqlite")]
        BackendConfig::Sqlite { path, cache_size } => {
            use crate::sqlite_topology::SqliteTopologyGraph;

            let graph = if let Some(p) = path {
                SqliteTopologyGraph::open(p)
                    .map_err(|e| BackendError::SqliteError(e.to_string()))?
            } else {
                SqliteTopologyGraph::new_in_memory()
                    .map_err(|e| BackendError::SqliteError(e.to_string()))?
            };

            Ok(Box::new(graph.with_cache_size(*cache_size)))
        }
    }
}

/// Errors that can occur when creating graph backends.
#[derive(Debug, Clone)]
pub enum BackendError {
    /// SQLite-specific error.
    #[cfg(feature = "sqlite")]
    SqliteError(String),

    /// Generic backend error.
    Other(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "sqlite")]
            BackendError::SqliteError(msg) => write!(f, "SQLite error: {}", msg),
            BackendError::Other(msg) => write!(f, "Backend error: {}", msg),
        }
    }
}

impl std::error::Error for BackendError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_in_memory_backend() {
        let config = BackendConfig::in_memory();
        let backend = create_backend(&config).unwrap();
        assert_eq!(backend.node_count(), 0);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn create_sqlite_in_memory_backend() {
        let config = BackendConfig::sqlite_in_memory();
        let backend = create_backend(&config).unwrap();
        assert_eq!(backend.node_count(), 0);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn create_sqlite_file_backend() {
        let tmp = std::env::temp_dir().join("phago_backend_test.db");
        let config = BackendConfig::sqlite(&tmp);
        let backend = create_backend(&config).unwrap();
        assert_eq!(backend.node_count(), 0);
        std::fs::remove_file(&tmp).ok();
    }
}
