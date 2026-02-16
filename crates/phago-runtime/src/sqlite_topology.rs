//! SQLite-backed implementation of the TopologyGraph trait.
//!
//! Provides persistent storage for large knowledge graphs that don't fit in memory.
//! Uses SQLite for storage with indexes on node labels and edge endpoints.

#![cfg(feature = "sqlite")]

use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use rusqlite::{params, Connection, Result as SqlResult};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// SQLite-backed implementation of the topology graph.
///
/// Stores nodes and edges in SQLite tables with appropriate indexes.
/// Supports both in-memory and file-backed databases.
pub struct SqliteTopologyGraph {
    conn: Arc<Mutex<Connection>>,
    /// Cache for frequently accessed nodes (LRU-style, limited size)
    node_cache: HashMap<NodeId, NodeData>,
    cache_size: usize,
}

impl SqliteTopologyGraph {
    /// Create a new in-memory SQLite graph.
    pub fn new_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init_with_connection(conn)
    }

    /// Create or open a file-backed SQLite graph.
    pub fn open<P: AsRef<Path>>(path: P) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        Self::init_with_connection(conn)
    }

    fn init_with_connection(conn: Connection) -> SqlResult<Self> {
        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        // Create tables
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                node_type TEXT NOT NULL,
                position_x REAL NOT NULL,
                position_y REAL NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 1,
                created_tick INTEGER NOT NULL DEFAULT 0,
                embedding BLOB
            );

            CREATE TABLE IF NOT EXISTS edges (
                from_id TEXT NOT NULL,
                to_id TEXT NOT NULL,
                weight REAL NOT NULL,
                co_activations INTEGER NOT NULL DEFAULT 1,
                created_tick INTEGER NOT NULL DEFAULT 0,
                last_activated_tick INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (from_id, to_id),
                FOREIGN KEY (from_id) REFERENCES nodes(id),
                FOREIGN KEY (to_id) REFERENCES nodes(id)
            );

            CREATE INDEX IF NOT EXISTS idx_nodes_label ON nodes(label);
            CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_id);
            CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id);
            "#,
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            node_cache: HashMap::new(),
            cache_size: 1000,
        })
    }

    /// Set the cache size for frequently accessed nodes.
    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    /// Get database statistics.
    pub fn stats(&self) -> SqlResult<(usize, usize)> {
        let conn = self.conn.lock().unwrap();
        let node_count: usize =
            conn.query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))?;
        let edge_count: usize =
            conn.query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;
        Ok((node_count, edge_count))
    }

    /// Clear the in-memory cache.
    pub fn clear_cache(&mut self) {
        self.node_cache.clear();
    }

    /// Iterate over all nodes with their data.
    ///
    /// This is useful for bulk loading from SQLite into another graph backend.
    pub fn iter_nodes(&self) -> impl Iterator<Item = NodeData> + '_ {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, label, node_type, position_x, position_y, access_count, created_tick, embedding FROM nodes")
            .expect("Failed to prepare statement");

        let nodes: Vec<NodeData> = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let embedding_bytes: Option<Vec<u8>> = row.get(7)?;
                Ok(NodeData {
                    id: NodeId(uuid::Uuid::parse_str(&id_str).unwrap_or_default()),
                    label: row.get(1)?,
                    node_type: Self::string_to_node_type(&row.get::<_, String>(2)?),
                    position: Position::new(row.get(3)?, row.get(4)?),
                    access_count: row.get(5)?,
                    created_tick: row.get(6)?,
                    embedding: Self::deserialize_embedding(embedding_bytes),
                })
            })
            .expect("Failed to query nodes")
            .filter_map(|r| r.ok())
            .collect();

        nodes.into_iter()
    }

    /// Iterate over all edges with their data.
    ///
    /// Returns (from_id, to_id, edge_data) tuples.
    pub fn iter_edges(&self) -> impl Iterator<Item = (NodeId, NodeId, EdgeData)> + '_ {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT from_id, to_id, weight, co_activations, created_tick, last_activated_tick FROM edges")
            .expect("Failed to prepare statement");

        let edges: Vec<(NodeId, NodeId, EdgeData)> = stmt
            .query_map([], |row| {
                let from_str: String = row.get(0)?;
                let to_str: String = row.get(1)?;
                Ok((
                    NodeId(uuid::Uuid::parse_str(&from_str).unwrap_or_default()),
                    NodeId(uuid::Uuid::parse_str(&to_str).unwrap_or_default()),
                    EdgeData {
                        weight: row.get(2)?,
                        co_activations: row.get(3)?,
                        created_tick: row.get(4)?,
                        last_activated_tick: row.get(5)?,
                    },
                ))
            })
            .expect("Failed to query edges")
            .filter_map(|r| r.ok())
            .collect();

        edges.into_iter()
    }

    fn node_type_to_string(nt: &NodeType) -> &'static str {
        match nt {
            NodeType::Concept => "Concept",
            NodeType::Insight => "Insight",
            NodeType::Anomaly => "Anomaly",
            NodeType::Document => "Document",
        }
    }

    fn string_to_node_type(s: &str) -> NodeType {
        match s {
            "Insight" => NodeType::Insight,
            "Anomaly" => NodeType::Anomaly,
            "Document" => NodeType::Document,
            _ => NodeType::Concept,
        }
    }

    fn serialize_embedding(embedding: &Option<Vec<f32>>) -> Option<Vec<u8>> {
        embedding
            .as_ref()
            .map(|e| e.iter().flat_map(|f| f.to_le_bytes()).collect())
    }

    fn deserialize_embedding(bytes: Option<Vec<u8>>) -> Option<Vec<f32>> {
        bytes.map(|b| {
            b.chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect()
        })
    }
}

impl TopologyGraph for SqliteTopologyGraph {
    fn add_node(&mut self, data: NodeData) -> NodeId {
        let id = data.id;
        let conn = self.conn.lock().unwrap();

        let embedding_bytes = Self::serialize_embedding(&data.embedding);

        conn.execute(
            "INSERT OR REPLACE INTO nodes (id, label, node_type, position_x, position_y, access_count, created_tick, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id.0.to_string(),
                data.label,
                Self::node_type_to_string(&data.node_type),
                data.position.x,
                data.position.y,
                data.access_count,
                data.created_tick,
                embedding_bytes,
            ],
        ).expect("Failed to insert node");

        // Add to cache if there's room
        if self.node_cache.len() < self.cache_size {
            drop(conn);
            self.node_cache.insert(id, data);
        }

        id
    }

    fn get_node(&self, id: &NodeId) -> Option<&NodeData> {
        // Check cache first
        self.node_cache.get(id)
    }

    fn get_node_mut(&mut self, id: &NodeId) -> Option<&mut NodeData> {
        // For mutable access, we need to load into cache first
        if !self.node_cache.contains_key(id) {
            let conn = self.conn.lock().unwrap();
            let node: Option<NodeData> = conn
                .query_row(
                    "SELECT id, label, node_type, position_x, position_y, access_count, created_tick, embedding
                     FROM nodes WHERE id = ?1",
                    params![id.0.to_string()],
                    |row| {
                        let id_str: String = row.get(0)?;
                        let embedding_bytes: Option<Vec<u8>> = row.get(7)?;
                        Ok(NodeData {
                            id: NodeId(uuid::Uuid::parse_str(&id_str).unwrap_or_default()),
                            label: row.get(1)?,
                            node_type: Self::string_to_node_type(&row.get::<_, String>(2)?),
                            position: Position::new(row.get(3)?, row.get(4)?),
                            access_count: row.get(5)?,
                            created_tick: row.get(6)?,
                            embedding: Self::deserialize_embedding(embedding_bytes),
                        })
                    },
                )
                .ok();

            if let Some(node) = node {
                drop(conn);
                self.node_cache.insert(*id, node);
            }
        }

        self.node_cache.get_mut(id)
    }

    fn set_edge(&mut self, from: NodeId, to: NodeId, data: EdgeData) {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO edges (from_id, to_id, weight, co_activations, created_tick, last_activated_tick)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                from.0.to_string(),
                to.0.to_string(),
                data.weight,
                data.co_activations,
                data.created_tick,
                data.last_activated_tick,
            ],
        ).expect("Failed to insert edge");
    }

    fn get_edge(&self, _from: &NodeId, _to: &NodeId) -> Option<&EdgeData> {
        // SQLite backend returns owned data, so we can't return a reference
        // This is a limitation - for persistent backends, we'd need a different pattern
        None
    }

    fn get_edge_mut(&mut self, _from: &NodeId, _to: &NodeId) -> Option<&mut EdgeData> {
        // Same limitation as get_edge
        None
    }

    fn neighbors(&self, _node: &NodeId) -> Vec<(NodeId, &EdgeData)> {
        // Can't return references to edges from SQLite
        // Users should use all_edges or a custom query
        Vec::new()
    }

    fn remove_edge(&mut self, from: &NodeId, to: &NodeId) -> Option<EdgeData> {
        let conn = self.conn.lock().unwrap();

        // First get the edge data
        let edge: Option<EdgeData> = conn
            .query_row(
                "SELECT weight, co_activations, created_tick, last_activated_tick FROM edges WHERE from_id = ?1 AND to_id = ?2",
                params![from.0.to_string(), to.0.to_string()],
                |row| {
                    Ok(EdgeData {
                        weight: row.get(0)?,
                        co_activations: row.get(1)?,
                        created_tick: row.get(2)?,
                        last_activated_tick: row.get(3)?,
                    })
                },
            )
            .ok();

        // Then delete it
        conn.execute(
            "DELETE FROM edges WHERE from_id = ?1 AND to_id = ?2",
            params![from.0.to_string(), to.0.to_string()],
        )
        .ok();

        edge
    }

    fn all_nodes(&self) -> Vec<NodeId> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id FROM nodes")
            .expect("Failed to prepare statement");
        let nodes: Vec<NodeId> = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                Ok(NodeId(uuid::Uuid::parse_str(&id_str).unwrap_or_default()))
            })
            .expect("Failed to query nodes")
            .filter_map(|r| r.ok())
            .collect();
        nodes
    }

    fn all_edges(&self) -> Vec<(NodeId, NodeId, &EdgeData)> {
        // Can't return references from SQLite
        Vec::new()
    }

    fn node_count(&self) -> usize {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))
            .unwrap_or(0)
    }

    fn edge_count(&self) -> usize {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
            .unwrap_or(0)
    }

    fn decay_edges(&mut self, rate: f64, prune_threshold: f64) -> Vec<PrunedConnection> {
        let conn = self.conn.lock().unwrap();

        // Decay all edge weights
        conn.execute(
            "UPDATE edges SET weight = weight * (1.0 - ?1)",
            params![rate],
        )
        .expect("Failed to decay edges");

        // Get edges below threshold
        let mut stmt = conn
            .prepare("SELECT from_id, to_id, weight FROM edges WHERE weight < ?1")
            .expect("Failed to prepare statement");

        let pruned: Vec<PrunedConnection> = stmt
            .query_map(params![prune_threshold], |row| {
                let from_str: String = row.get(0)?;
                let to_str: String = row.get(1)?;
                Ok(PrunedConnection {
                    from: NodeId(uuid::Uuid::parse_str(&from_str).unwrap_or_default()),
                    to: NodeId(uuid::Uuid::parse_str(&to_str).unwrap_or_default()),
                    final_weight: row.get(2)?,
                })
            })
            .expect("Failed to query pruned edges")
            .filter_map(|r| r.ok())
            .collect();

        // Delete pruned edges
        conn.execute(
            "DELETE FROM edges WHERE weight < ?1",
            params![prune_threshold],
        )
        .expect("Failed to delete pruned edges");

        pruned
    }

    fn decay_edges_activity(
        &mut self,
        base_rate: f64,
        prune_threshold: f64,
        current_tick: u64,
        staleness_factor: f64,
        maturation_ticks: u64,
    ) -> Vec<PrunedConnection> {
        let conn = self.conn.lock().unwrap();

        // For SQLite, we use a simplified approach:
        // 1. Decay mature edges faster if they haven't been activated recently
        // 2. Use base_rate for young edges

        // Calculate decay for each edge based on staleness
        // This is done in Rust since SQLite doesn't have complex math functions
        let mut stmt = conn
            .prepare("SELECT from_id, to_id, weight, co_activations, created_tick, last_activated_tick FROM edges")
            .expect("Failed to prepare statement");

        let edges: Vec<(String, String, f64, u64, u64, u64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, u64>(3)?,
                    row.get::<_, u64>(4)?,
                    row.get::<_, u64>(5)?,
                ))
            })
            .expect("Failed to query edges")
            .filter_map(|r| r.ok())
            .collect();

        drop(stmt);

        let mut pruned = Vec::new();

        for (from_str, to_str, weight, co_activations, created_tick, last_activated_tick) in edges {
            let edge_age = current_tick.saturating_sub(created_tick);

            let decay_rate = if edge_age < maturation_ticks {
                base_rate
            } else {
                let staleness = current_tick.saturating_sub(last_activated_tick);
                let activity_factor = 1.0 / (1.0 + co_activations as f64);
                base_rate * (1.0 + staleness_factor * staleness as f64 * activity_factor / 100.0)
            };

            let new_weight = weight * (1.0 - decay_rate.min(0.5));

            if new_weight < prune_threshold {
                pruned.push(PrunedConnection {
                    from: NodeId(uuid::Uuid::parse_str(&from_str).unwrap_or_default()),
                    to: NodeId(uuid::Uuid::parse_str(&to_str).unwrap_or_default()),
                    final_weight: new_weight,
                });
                conn.execute(
                    "DELETE FROM edges WHERE from_id = ?1 AND to_id = ?2",
                    params![from_str, to_str],
                )
                .ok();
            } else {
                conn.execute(
                    "UPDATE edges SET weight = ?1 WHERE from_id = ?2 AND to_id = ?3",
                    params![new_weight, from_str, to_str],
                )
                .ok();
            }
        }

        pruned
    }

    fn prune_to_max_degree(&mut self, max_degree: usize) -> Vec<PrunedConnection> {
        let conn = self.conn.lock().unwrap();
        let mut pruned = Vec::new();

        // Get all nodes
        let nodes: Vec<String> = conn
            .prepare("SELECT id FROM nodes")
            .expect("prepare")
            .query_map([], |row| row.get(0))
            .expect("query")
            .filter_map(|r| r.ok())
            .collect();

        for node_id in nodes {
            // Get edges sorted by weight (ascending, so weakest first)
            let edges: Vec<(String, String, f64)> = conn
                .prepare(
                    "SELECT from_id, to_id, weight FROM edges
                     WHERE from_id = ?1 OR to_id = ?1
                     ORDER BY weight ASC",
                )
                .expect("prepare")
                .query_map(params![&node_id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })
                .expect("query")
                .filter_map(|r| r.ok())
                .collect();

            if edges.len() > max_degree {
                // Prune the weakest edges
                let to_prune = edges.len() - max_degree;
                for (from_str, to_str, weight) in edges.into_iter().take(to_prune) {
                    pruned.push(PrunedConnection {
                        from: NodeId(uuid::Uuid::parse_str(&from_str).unwrap_or_default()),
                        to: NodeId(uuid::Uuid::parse_str(&to_str).unwrap_or_default()),
                        final_weight: weight,
                    });
                    conn.execute(
                        "DELETE FROM edges WHERE from_id = ?1 AND to_id = ?2",
                        params![from_str, to_str],
                    )
                    .ok();
                }
            }
        }

        pruned
    }

    fn find_nodes_by_label(&self, query: &str) -> Vec<NodeId> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = conn
            .prepare("SELECT id FROM nodes WHERE LOWER(label) LIKE ?1")
            .expect("Failed to prepare statement");

        stmt.query_map(params![pattern], |row| {
            let id_str: String = row.get(0)?;
            Ok(NodeId(uuid::Uuid::parse_str(&id_str).unwrap_or_default()))
        })
        .expect("Failed to query nodes")
        .filter_map(|r| r.ok())
        .collect()
    }

    fn find_nodes_by_exact_label(&self, label: &str) -> Vec<NodeId> {
        let conn = self.conn.lock().unwrap();
        let label_lower = label.to_lowercase();
        let mut stmt = conn
            .prepare("SELECT id FROM nodes WHERE LOWER(label) = ?1")
            .expect("Failed to prepare statement");

        stmt.query_map(params![label_lower], |row| {
            let id_str: String = row.get(0)?;
            Ok(NodeId(uuid::Uuid::parse_str(&id_str).unwrap_or_default()))
        })
        .expect("Failed to query nodes")
        .filter_map(|r| r.ok())
        .collect()
    }

    fn shortest_path(&self, _from: &NodeId, _to: &NodeId) -> Option<(Vec<NodeId>, f64)> {
        // SQLite doesn't support graph algorithms directly
        // Would need to implement Dijkstra in Rust with SQL queries
        None
    }

    fn betweenness_centrality(&self, _sample_size: usize) -> Vec<(NodeId, f64)> {
        // Complex to implement with SQL
        Vec::new()
    }

    fn bridge_nodes(&self, _top_k: usize) -> Vec<(NodeId, f64)> {
        Vec::new()
    }

    fn connected_components(&self) -> usize {
        // Would require union-find or BFS implementation
        1
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;

    #[test]
    fn create_in_memory() {
        let graph = SqliteTopologyGraph::new_in_memory().unwrap();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn add_and_count_nodes() {
        let mut graph = SqliteTopologyGraph::new_in_memory().unwrap();

        for i in 0..100 {
            graph.add_node(NodeData {
                id: NodeId::new(),
                label: format!("node_{}", i),
                node_type: NodeType::Concept,
                position: Position::new(i as f64, 0.0),
                access_count: 1,
                created_tick: 0,
                embedding: None,
            });
        }

        assert_eq!(graph.node_count(), 100);
    }

    #[test]
    fn add_and_count_edges() {
        let mut graph = SqliteTopologyGraph::new_in_memory().unwrap();

        let n1 = graph.add_node(NodeData {
            id: NodeId::new(),
            label: "a".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(0.0, 0.0),
            access_count: 1,
            created_tick: 0,
            embedding: None,
        });

        let n2 = graph.add_node(NodeData {
            id: NodeId::new(),
            label: "b".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(1.0, 0.0),
            access_count: 1,
            created_tick: 0,
            embedding: None,
        });

        graph.set_edge(
            n1,
            n2,
            EdgeData {
                weight: 0.5,
                co_activations: 1,
                created_tick: 0,
                last_activated_tick: 0,
            },
        );

        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn find_nodes_by_label() {
        let mut graph = SqliteTopologyGraph::new_in_memory().unwrap();

        graph.add_node(NodeData {
            id: NodeId::new(),
            label: "cell membrane".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(0.0, 0.0),
            access_count: 1,
            created_tick: 0,
            embedding: None,
        });

        graph.add_node(NodeData {
            id: NodeId::new(),
            label: "protein".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(1.0, 0.0),
            access_count: 1,
            created_tick: 0,
            embedding: None,
        });

        let results = graph.find_nodes_by_label("cell");
        assert_eq!(results.len(), 1);

        let results = graph.find_nodes_by_label("membrane");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn decay_removes_weak_edges() {
        let mut graph = SqliteTopologyGraph::new_in_memory().unwrap();

        let n1 = graph.add_node(NodeData {
            id: NodeId::new(),
            label: "a".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(0.0, 0.0),
            access_count: 1,
            created_tick: 0,
            embedding: None,
        });

        let n2 = graph.add_node(NodeData {
            id: NodeId::new(),
            label: "b".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(1.0, 0.0),
            access_count: 1,
            created_tick: 0,
            embedding: None,
        });

        // Add a weak edge
        graph.set_edge(
            n1,
            n2,
            EdgeData {
                weight: 0.1,
                co_activations: 1,
                created_tick: 0,
                last_activated_tick: 0,
            },
        );

        assert_eq!(graph.edge_count(), 1);

        // Decay aggressively
        let pruned = graph.decay_edges(0.9, 0.05);
        assert_eq!(pruned.len(), 1);
        assert_eq!(graph.edge_count(), 0);
    }
}
