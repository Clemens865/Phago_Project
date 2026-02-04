//! Python bindings for Phago biological computing framework.
//!
//! This crate provides Python bindings via PyO3 for the core Phago
//! functionality including Colony management, document ingestion,
//! hybrid queries, and graph exploration.

use phago_agents::digester::Digester;
use phago_core::types::Position as CorePosition;
use phago_rag::{hybrid_query, HybridConfig};
use phago_runtime::colony::{Colony as RustColony, ColonyConfig as RustColonyConfig};
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

/// Python-friendly Position class.
#[pyclass]
#[derive(Clone)]
pub struct Position {
    #[pyo3(get, set)]
    pub x: f64,
    #[pyo3(get, set)]
    pub y: f64,
}

#[pymethods]
impl Position {
    #[new]
    fn new(x: f64, y: f64) -> Self {
        Position { x, y }
    }

    fn __repr__(&self) -> String {
        format!("Position(x={}, y={})", self.x, self.y)
    }
}

impl From<Position> for CorePosition {
    fn from(p: Position) -> Self {
        CorePosition::new(p.x, p.y)
    }
}

impl From<CorePosition> for Position {
    fn from(p: CorePosition) -> Self {
        Position { x: p.x, y: p.y }
    }
}

/// Colony configuration.
#[pyclass]
#[derive(Clone)]
pub struct ColonyConfig {
    #[pyo3(get, set)]
    pub signal_decay_rate: f64,
    #[pyo3(get, set)]
    pub trace_decay_rate: f64,
    #[pyo3(get, set)]
    pub edge_decay_rate: f64,
    #[pyo3(get, set)]
    pub edge_prune_threshold: f64,
    #[pyo3(get, set)]
    pub staleness_factor: f64,
    #[pyo3(get, set)]
    pub maturation_ticks: u64,
    #[pyo3(get, set)]
    pub max_edge_degree: usize,
}

#[pymethods]
impl ColonyConfig {
    #[new]
    fn new() -> Self {
        let default = RustColonyConfig::default();
        ColonyConfig {
            signal_decay_rate: default.signal_decay_rate,
            trace_decay_rate: default.trace_decay_rate,
            edge_decay_rate: default.edge_decay_rate,
            edge_prune_threshold: default.edge_prune_threshold,
            staleness_factor: default.staleness_factor,
            maturation_ticks: default.maturation_ticks,
            max_edge_degree: default.max_edge_degree,
        }
    }
}

/// Query result item.
#[pyclass]
pub struct QueryResult {
    #[pyo3(get)]
    pub label: String,
    #[pyo3(get)]
    pub score: f64,
    #[pyo3(get)]
    pub tfidf_score: f64,
    #[pyo3(get)]
    pub graph_score: f64,
}

#[pymethods]
impl QueryResult {
    fn __repr__(&self) -> String {
        format!("QueryResult(label='{}', score={:.3})", self.label, self.score)
    }
}

/// Colony statistics.
#[pyclass]
pub struct ColonyStats {
    #[pyo3(get)]
    pub tick: u64,
    #[pyo3(get)]
    pub agents_alive: usize,
    #[pyo3(get)]
    pub agents_died: usize,
    #[pyo3(get)]
    pub total_spawned: usize,
    #[pyo3(get)]
    pub graph_nodes: usize,
    #[pyo3(get)]
    pub graph_edges: usize,
    #[pyo3(get)]
    pub total_signals: usize,
    #[pyo3(get)]
    pub documents_total: usize,
    #[pyo3(get)]
    pub documents_digested: usize,
}

#[pymethods]
impl ColonyStats {
    fn __repr__(&self) -> String {
        format!(
            "ColonyStats(tick={}, nodes={}, edges={}, agents={})",
            self.tick, self.graph_nodes, self.graph_edges, self.agents_alive
        )
    }
}

/// Main Colony class - the Phago biological computing environment.
///
/// Example:
///     >>> from phago import Colony, Position
///     >>> colony = Colony()
///     >>> colony.ingest_document("Title", "Content about cells and proteins")
///     >>> colony.run(50)
///     >>> results = colony.query("cells")
///     >>> for r in results:
///     ...     print(f"{r.label}: {r.score:.3f}")
///
/// Note: This class is not thread-safe. Use only from the thread it was created on.
#[pyclass(unsendable)]
pub struct Colony {
    inner: RustColony,
}

#[pymethods]
impl Colony {
    /// Create a new Colony with optional configuration.
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(config: Option<ColonyConfig>) -> PyResult<Self> {
        let rust_config = if let Some(cfg) = config {
            use phago_core::semantic::SemanticWiringConfig;
            RustColonyConfig {
                signal_decay_rate: cfg.signal_decay_rate,
                signal_removal_threshold: 0.01,
                trace_decay_rate: cfg.trace_decay_rate,
                trace_removal_threshold: 0.01,
                edge_decay_rate: cfg.edge_decay_rate,
                edge_prune_threshold: cfg.edge_prune_threshold,
                staleness_factor: cfg.staleness_factor,
                maturation_ticks: cfg.maturation_ticks,
                max_edge_degree: cfg.max_edge_degree,
                semantic_wiring: SemanticWiringConfig::default(),
            }
        } else {
            RustColonyConfig::default()
        };

        Ok(Colony {
            inner: RustColony::from_config(rust_config),
        })
    }

    /// Ingest a document into the colony.
    ///
    /// Args:
    ///     title: Document title
    ///     content: Document content
    ///     position: Optional Position (default: (0, 0))
    ///
    /// Returns:
    ///     Document ID string
    #[pyo3(signature = (title, content, position=None))]
    fn ingest_document(&mut self, title: &str, content: &str, position: Option<Position>) -> String {
        let pos = position.map(|p| p.into()).unwrap_or(CorePosition::new(0.0, 0.0));
        let doc_id = self.inner.ingest_document(title, content, pos);

        // Spawn a digester to process the document
        self.inner.spawn(Box::new(
            Digester::new(pos).with_max_idle(30),
        ));

        format!("{}", doc_id.0)
    }

    /// Run the simulation for N ticks.
    ///
    /// Args:
    ///     ticks: Number of simulation ticks to run
    fn run(&mut self, ticks: u64) {
        self.inner.run(ticks);
    }

    /// Run a single simulation tick.
    fn tick(&mut self) {
        self.inner.tick();
    }

    /// Query the knowledge graph using hybrid scoring.
    ///
    /// Args:
    ///     query: Search query string
    ///     alpha: Balance between TF-IDF (0) and graph (1) scores (default: 0.5)
    ///     max_results: Maximum number of results (default: 10)
    ///
    /// Returns:
    ///     List of QueryResult objects
    #[pyo3(signature = (query, alpha=0.5, max_results=10))]
    fn query(&self, query: &str, alpha: f64, max_results: usize) -> Vec<QueryResult> {
        let config = HybridConfig {
            alpha,
            max_results,
            candidate_multiplier: 3,
        };

        hybrid_query(&self.inner, query, &config)
            .into_iter()
            .map(|r| QueryResult {
                label: r.label,
                score: r.final_score,
                tfidf_score: r.tfidf_score,
                graph_score: r.graph_score,
            })
            .collect()
    }

    /// Get colony statistics.
    ///
    /// Returns:
    ///     ColonyStats object with current metrics
    fn stats(&self) -> ColonyStats {
        let s = self.inner.stats();
        ColonyStats {
            tick: s.tick,
            agents_alive: s.agents_alive,
            agents_died: s.agents_died,
            total_spawned: s.total_spawned,
            graph_nodes: s.graph_nodes,
            graph_edges: s.graph_edges,
            total_signals: s.total_signals,
            documents_total: s.documents_total,
            documents_digested: s.documents_digested,
        }
    }

    /// Get the number of alive agents.
    fn alive_count(&self) -> usize {
        self.inner.alive_count()
    }

    /// Get a JSON snapshot of the colony state.
    fn snapshot_json(&self) -> PyResult<String> {
        let snapshot = self.inner.snapshot();
        serde_json::to_string(&snapshot)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize snapshot: {}", e)))
    }

    fn __repr__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "Colony(tick={}, nodes={}, edges={}, agents={})",
            stats.tick, stats.graph_nodes, stats.graph_edges, stats.agents_alive
        )
    }
}

/// Python module definition.
#[pymodule]
fn _phago(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Colony>()?;
    m.add_class::<ColonyConfig>()?;
    m.add_class::<ColonyStats>()?;
    m.add_class::<Position>()?;
    m.add_class::<QueryResult>()?;
    Ok(())
}
