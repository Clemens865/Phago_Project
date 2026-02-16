//! Error types for Phago operations.
//!
//! Provides structured error handling instead of panics.

use std::error::Error;
use std::fmt;

/// Result type for Phago operations.
pub type Result<T> = std::result::Result<T, PhagoError>;

/// Errors that can occur during Phago operations.
#[derive(Debug, Clone)]
pub enum PhagoError {
    /// Document-related errors.
    Document(DocumentError),
    /// Graph-related errors.
    Graph(GraphError),
    /// Agent-related errors.
    Agent(AgentError),
    /// Session-related errors.
    Session(SessionError),
    /// Query-related errors.
    Query(QueryError),
    /// Configuration errors.
    Config(ConfigError),
    /// I/O errors (wrapped).
    Io(String),
    /// Serialization errors.
    Serialization(String),
}

impl fmt::Display for PhagoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhagoError::Document(e) => write!(f, "Document error: {}", e),
            PhagoError::Graph(e) => write!(f, "Graph error: {}", e),
            PhagoError::Agent(e) => write!(f, "Agent error: {}", e),
            PhagoError::Session(e) => write!(f, "Session error: {}", e),
            PhagoError::Query(e) => write!(f, "Query error: {}", e),
            PhagoError::Config(e) => write!(f, "Config error: {}", e),
            PhagoError::Io(msg) => write!(f, "I/O error: {}", msg),
            PhagoError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl Error for PhagoError {}

impl From<std::io::Error> for PhagoError {
    fn from(e: std::io::Error) -> Self {
        PhagoError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for PhagoError {
    fn from(e: serde_json::Error) -> Self {
        PhagoError::Serialization(e.to_string())
    }
}

/// Document-related errors.
#[derive(Debug, Clone)]
pub enum DocumentError {
    /// Document not found.
    NotFound(String),
    /// Document already digested.
    AlreadyDigested(String),
    /// Empty content.
    EmptyContent,
    /// Invalid format.
    InvalidFormat(String),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocumentError::NotFound(id) => write!(f, "Document not found: {}", id),
            DocumentError::AlreadyDigested(id) => write!(f, "Document already digested: {}", id),
            DocumentError::EmptyContent => write!(f, "Document content is empty"),
            DocumentError::InvalidFormat(msg) => write!(f, "Invalid document format: {}", msg),
        }
    }
}

/// Graph-related errors.
#[derive(Debug, Clone)]
pub enum GraphError {
    /// Node not found.
    NodeNotFound(String),
    /// Edge not found.
    EdgeNotFound(String, String),
    /// Duplicate node.
    DuplicateNode(String),
    /// Invalid weight (must be 0.0-1.0).
    InvalidWeight(f64),
    /// Graph is empty.
    EmptyGraph,
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            GraphError::EdgeNotFound(from, to) => write!(f, "Edge not found: {} -> {}", from, to),
            GraphError::DuplicateNode(label) => write!(f, "Duplicate node: {}", label),
            GraphError::InvalidWeight(w) => write!(f, "Invalid weight: {} (must be 0.0-1.0)", w),
            GraphError::EmptyGraph => write!(f, "Graph is empty"),
        }
    }
}

/// Agent-related errors.
#[derive(Debug, Clone)]
pub enum AgentError {
    /// Agent not found.
    NotFound(String),
    /// Agent already exists.
    AlreadyExists(String),
    /// Agent is busy.
    Busy(String),
    /// Agent is dead.
    Dead(String),
    /// Invalid action.
    InvalidAction(String),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::NotFound(id) => write!(f, "Agent not found: {}", id),
            AgentError::AlreadyExists(id) => write!(f, "Agent already exists: {}", id),
            AgentError::Busy(id) => write!(f, "Agent is busy: {}", id),
            AgentError::Dead(id) => write!(f, "Agent is dead: {}", id),
            AgentError::InvalidAction(msg) => write!(f, "Invalid action: {}", msg),
        }
    }
}

/// Session-related errors.
#[derive(Debug, Clone)]
pub enum SessionError {
    /// Session not found.
    NotFound(String),
    /// Session file corrupt.
    Corrupt(String),
    /// Version mismatch.
    VersionMismatch { expected: String, found: String },
    /// Save failed.
    SaveFailed(String),
    /// Load failed.
    LoadFailed(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::NotFound(path) => write!(f, "Session not found: {}", path),
            SessionError::Corrupt(msg) => write!(f, "Session file corrupt: {}", msg),
            SessionError::VersionMismatch { expected, found } => {
                write!(
                    f,
                    "Version mismatch: expected {}, found {}",
                    expected, found
                )
            }
            SessionError::SaveFailed(msg) => write!(f, "Save failed: {}", msg),
            SessionError::LoadFailed(msg) => write!(f, "Load failed: {}", msg),
        }
    }
}

/// Query-related errors.
#[derive(Debug, Clone)]
pub enum QueryError {
    /// Empty query.
    EmptyQuery,
    /// No results found.
    NoResults,
    /// Invalid parameters.
    InvalidParameters(String),
    /// Timeout.
    Timeout,
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::EmptyQuery => write!(f, "Query is empty"),
            QueryError::NoResults => write!(f, "No results found"),
            QueryError::InvalidParameters(msg) => write!(f, "Invalid parameters: {}", msg),
            QueryError::Timeout => write!(f, "Query timed out"),
        }
    }
}

/// Configuration errors.
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// Invalid value.
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },
    /// Missing required field.
    MissingField(String),
    /// Out of range.
    OutOfRange {
        field: String,
        min: f64,
        max: f64,
        value: f64,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InvalidValue {
                field,
                value,
                reason,
            } => {
                write!(f, "Invalid value for {}: {} ({})", field, value, reason)
            }
            ConfigError::MissingField(field) => write!(f, "Missing required field: {}", field),
            ConfigError::OutOfRange {
                field,
                min,
                max,
                value,
            } => {
                write!(
                    f,
                    "{} out of range: {} (must be {}-{})",
                    field, value, min, max
                )
            }
        }
    }
}

// Convenience constructors
impl PhagoError {
    pub fn document_not_found(id: impl Into<String>) -> Self {
        PhagoError::Document(DocumentError::NotFound(id.into()))
    }

    pub fn node_not_found(id: impl Into<String>) -> Self {
        PhagoError::Graph(GraphError::NodeNotFound(id.into()))
    }

    pub fn agent_not_found(id: impl Into<String>) -> Self {
        PhagoError::Agent(AgentError::NotFound(id.into()))
    }

    pub fn empty_query() -> Self {
        PhagoError::Query(QueryError::EmptyQuery)
    }

    pub fn invalid_config(
        field: impl Into<String>,
        value: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        PhagoError::Config(ConfigError::InvalidValue {
            field: field.into(),
            value: value.into(),
            reason: reason.into(),
        })
    }
}
