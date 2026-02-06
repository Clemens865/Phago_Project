//! # Phago
//!
//! Self-evolving knowledge substrates through biological computing primitives.
//!
//! Phago is a framework that maps cellular biology mechanisms to computational
//! operations for autonomous knowledge graph construction. Agents digest documents,
//! wire concepts through Hebbian learning, and evolve through fitness-directed mutation.
//!
//! ## Quick Start
//!
//! ```rust
//! use phago::prelude::*;
//!
//! // Create a colony
//! let mut colony = Colony::new();
//!
//! // Ingest documents
//! colony.ingest_document("Biology 101", "The cell membrane controls transport.", Position::new(0.0, 0.0));
//!
//! // Spawn a digester agent
//! colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0)).with_max_idle(30)));
//!
//! // Run the simulation
//! colony.run(30);
//!
//! // Query with hybrid scoring
//! let results = hybrid_query(&colony, "cell membrane", &HybridConfig {
//!     alpha: 0.5,
//!     max_results: 5,
//!     candidate_multiplier: 3,
//! });
//!
//! for r in results {
//!     println!("{} (score: {:.3})", r.label, r.final_score);
//! }
//! ```
//!
//! ## Architecture
//!
//! Phago is organized into several crates:
//!
//! - [`phago_core`] - Core traits (10 biological primitives) and shared types
//! - [`phago_agents`] - Agent implementations (Digester, Sentinel, Synthesizer)
//! - [`phago_runtime`] - Colony management, substrate, sessions
//! - [`phago_rag`] - Query engine, hybrid scoring, MCP adapter
//!
//! ## Key Concepts
//!
//! ### Biological Primitives
//!
//! | Primitive | Biological Analog | What It Does |
//! |-----------|-------------------|--------------|
//! | DIGEST | Phagocytosis | Consume input, extract fragments |
//! | APOPTOSE | Programmed cell death | Self-assess, gracefully terminate |
//! | SENSE | Chemotaxis | Detect signals, follow gradients |
//! | WIRE | Hebbian learning | Strengthen used connections |
//! | EMERGE | Quorum sensing | Collective behavior at threshold |
//! | NEGATE | Negative selection | Detect anomalies by exclusion |
//!
//! ### Agent Types
//!
//! - **Digester** - Consumes documents, extracts keywords, wires concepts
//! - **Sentinel** - Learns "normal", flags anomalies
//! - **Synthesizer** - Detects cross-document patterns, generates insights
//!
//! ### Hebbian Learning
//!
//! "Neurons that fire together wire together."
//!
//! - First co-occurrence: edge at 0.1 weight (tentative)
//! - Subsequent co-occurrences: +0.1 weight (reinforcement)
//! - Unused edges decay and are pruned
//!
//! ## MCP Integration
//!
//! Phago provides an MCP adapter for external LLM/agent integration:
//!
//! ```rust
//! use phago::prelude::*;
//!
//! let mut colony = Colony::new();
//!
//! // Ingest via MCP
//! let resp = phago_remember(&mut colony, &RememberRequest {
//!     title: "Doc".into(),
//!     content: "Content here".into(),
//!     ticks: Some(15),
//! });
//!
//! // Query via MCP
//! let results = phago_recall(&colony, &RecallRequest {
//!     query: "search terms".into(),
//!     max_results: 5,
//!     alpha: 0.5,
//! });
//!
//! // Explore graph structure
//! let stats = phago_explore(&colony, &ExploreRequest::Stats);
//! ```
//!
//! ## Session Persistence
//!
//! Save and restore colony state:
//!
//! ```rust,ignore
//! use phago::prelude::*;
//! use std::path::Path;
//!
//! let colony = Colony::new();
//!
//! // Save session
//! save_session(&colony, Path::new("session.json"), &["doc1.txt".to_string()]).unwrap();
//!
//! // Load session
//! let state = load_session(Path::new("session.json")).unwrap();
//! let mut restored = Colony::new();
//! restore_into_colony(&mut restored, &state);
//! ```

// Re-export all subcrates
pub use phago_core as core;
pub use phago_runtime as runtime;
pub use phago_agents as agents;
pub use phago_rag as rag;

#[cfg(feature = "semantic")]
pub use phago_embeddings as embeddings;

#[cfg(feature = "llm")]
pub use phago_llm as llm;

#[cfg(feature = "distributed")]
pub use phago_distributed as distributed;

/// Prelude module for convenient imports.
///
/// ```rust
/// use phago::prelude::*;
/// ```
pub mod prelude {
    // Core types
    pub use phago_core::types::{
        AgentId, NodeId, DocumentId,
        Position, Document,
        Signal, SignalType, Gradient,
        Trace, TraceType,
        CellHealth, DeathSignal, DeathCause,
        NodeData, NodeType, EdgeData,
        AgentAction, FragmentPresentation,
        DigestionResult,
        SymbiosisEval, SymbiontInfo,
        Classification,
        BoundaryContext,
        Orientation,
        Tick,
    };

    // Core traits
    pub use phago_core::agent::Agent;
    pub use phago_core::substrate::Substrate;
    pub use phago_core::topology::TopologyGraph;

    // Error types
    pub use phago_core::error::{PhagoError, Result};

    // Agents
    pub use phago_agents::digester::Digester;
    pub use phago_agents::sentinel::Sentinel;
    pub use phago_agents::synthesizer::Synthesizer;
    pub use phago_agents::genome::AgentGenome;
    pub use phago_agents::fitness::{AgentFitness, FitnessTracker};

    // Runtime
    pub use phago_runtime::colony::{Colony, ColonyEvent, ColonyStats};
    pub use phago_runtime::session::{
        save_session, load_session, restore_into_colony,
        GraphState, SessionMetadata,
    };
    pub use phago_runtime::metrics::ColonyMetrics;

    // RAG
    pub use phago_rag::{hybrid_query, HybridConfig, HybridResult};
    pub use phago_rag::query::{Query, QueryResult};
    pub use phago_rag::mcp::{
        phago_remember, phago_recall, phago_explore,
        RememberRequest, RememberResponse,
        RecallRequest, RecallResponse,
        ExploreRequest, ExploreResponse,
    };

    // Semantic embeddings (requires "semantic" feature)
    #[cfg(feature = "semantic")]
    pub use phago_agents::semantic_digester::{SemanticDigester, SemanticConcept, SemanticConfig};

    #[cfg(feature = "semantic")]
    pub use phago_embeddings::{
        Embedder, EmbeddingError, EmbeddingResult,
        SimpleEmbedder, Chunker, ChunkConfig,
        cosine_similarity, euclidean_distance, normalize_l2,
    };

    // LLM integration (requires "llm" feature)
    #[cfg(feature = "llm")]
    pub use phago_llm::{
        LlmBackend, LlmError, LlmResult,
        Concept, Relationship, ConceptType, RelationType,
        PromptTemplate,
    };

    #[cfg(feature = "llm-local")]
    pub use phago_llm::OllamaBackend;

    #[cfg(feature = "llm-api")]
    pub use phago_llm::{ClaudeBackend, OpenAiBackend};

    // Distributed colony (requires "distributed" feature)
    #[cfg(feature = "distributed")]
    pub use phago_distributed::{
        Coordinator, ShardedColony, ConsistentHashRing,
        DistributedQueryEngine, DistributedHybridConfig,
        DistributedRunner, RunnerConfig,
        ShardId, DistributedConfig, DistributedError, DistributedResult,
        TickPhase, CrossShardEdge, GhostNode, ShardInfo,
    };
}

/// Version information.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
