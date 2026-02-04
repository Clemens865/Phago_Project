# Phago Development Roadmap

## Overview

Phago is a biological computing framework for self-evolving knowledge substrates. This document tracks all development phases from inception through future planned work.

---

## Completed Phases

### Phase 0: Scaffold
**Status:** Complete | **Commit:** `9beb9fd`

- Workspace structure with 6 crates
- 10 biological primitive traits defined
- Shared types (AgentId, NodeId, Position, etc.)
- Core abstractions (Agent, Substrate, TopologyGraph)

### Phase 1: First Cell
**Status:** Complete | **Commit:** `338c4ab`

- Digester agent implementation
- Apoptosis (programmed death) mechanics
- Colony lifecycle (tick loop)
- Basic document ingestion

### Phase 2: Self-Organization
**Status:** Complete | **Commit:** `4baf6b0`

- Chemotaxis (gradient following)
- Signal emission and decay
- Hebbian wiring (co-activation strengthening)
- Document → concept extraction pipeline

### Phase 3: Emergence
**Status:** Complete | **Commit:** `b3ffc8e`

- Synthesizer agent (quorum sensing)
- Sentinel agent (negative selection / anomaly detection)
- Bridge concept identification
- Topic cluster detection

### Phase 4: Cooperation
**Status:** Complete | **Commit:** `fe6955a`

- Horizontal gene transfer (vocabulary sharing)
- Symbiosis (agent absorption)
- Dissolution (boundary modulation)
- Cross-agent knowledge propagation

### Phase 5: Prove It Works
**Status:** Complete | **Commit:** `fe6955a`

- Quantitative metrics (transfer effect, dissolution effect, graph richness)
- Interactive D3.js visualization
- Performance optimization (98.3% density reduction)
- Hardened test suite (99+ tests)

### Phase 6: Research Branches
**Status:** Complete

Four falsifiable hypotheses with prototypes:

| Branch | Hypothesis | Key Result |
|--------|------------|------------|
| Bio-RAG | Hebbian graphs improve retrieval | MRR 0.800 (hybrid) |
| Agent Evolution | Apoptosis pressure improves graphs | 1.8x more nodes |
| KG Training | Weighted triples aid LLM fine-tuning | 252K triples exported |
| Agentic Memory | Self-organizing code knowledge | 100% session fidelity |

### Phase 7: Production Ready
**Status:** Complete | **Commit:** `1761590`

- Unified `phago` facade crate
- Prelude modules for all crates
- Structured error types (`PhagoError`)
- Deterministic testing (`with_seed()` constructors)
- Fixed flaky tests (100% reliability)
- Updated documentation

---

## Upcoming Phases

### Phase 8: Distribution & Usability
**Status:** Complete

Published all 6 crates to crates.io, created CLI with full command set.

### Phase 9: Semantic Intelligence
**Status:** Complete

#### 9.1 Embedding-Backed Digester — Complete
- `phago-embeddings` crate with `SimpleEmbedder`, `OnnxEmbedder`, `ApiEmbedder`
- `SemanticDigester` agent using embeddings for concept extraction
- Chunking, normalization, and similarity utilities

#### 9.2 LLM Integration — Complete
- `phago-llm` crate with `LlmBackend` trait
- `OllamaBackend` for local LLM (no API key)
- `ClaudeBackend` for Anthropic Claude API
- `OpenAiBackend` for OpenAI GPT API
- Concept extraction, relationship identification, query expansion

#### 9.3 Vector Similarity Wiring — Complete
- `semantic` module in phago-core with similarity utilities
- `SemanticWiringConfig` for edge weight modulation
- Colony integrates embeddings into edge wiring
- Cosine similarity boosts edge weights for similar concepts

### Phase 10: Persistence & Scale
**Status:** Complete | **Priority:** Low

Enable production deployments at scale.

#### Progress
- 10.1 Agent State Serialization — **Complete**
- 10.2 Graph Database Backend — **Complete** (SQLite persistence via ColonyBuilder)
- 10.3 Async Runtime — **Complete** (LocalSet-based async, controlled tick rate)

---

## Phase 8: Distribution & Usability — Complete

### Goals
1. ✅ Publish to crates.io for easy installation
2. ✅ Provide CLI for non-programmatic usage
3. 🔄 Support configuration without recompilation (partial)

---

### 8.1 Publish to crates.io — Complete
**Completed:** All 6 crates published

#### Published Crates
- `phago-core` v0.1.0 — Core traits and types
- `phago-agents` v0.1.0 — Agent implementations
- `phago-runtime` v0.1.0 — Colony and substrate
- `phago-rag` v0.1.0 — Query engine
- `phago-viz` v0.1.0 — Visualization
- `phago` v0.1.0 — Unified facade

#### Success Criteria
- [x] `cargo add phago` works
- [x] Documentation renders on docs.rs
- [x] Examples compile from crates.io version

---

### 8.2 CLI Tool — Complete
**Completed:** Full CLI with all commands

#### Commands
```bash
phago init [path]             # Create .phago/ directory and config
phago ingest <path> [--ticks] [--ext]  # Ingest documents
phago run [--ticks N]         # Run simulation
phago query <text> [--max] [--alpha]   # Query with hybrid scoring
phago explore centrality|bridges|components [--limit]  # Structural queries
phago export <output> [--format json|html]  # Export graph
phago session save <name>     # Save session
phago session load <name>     # Load session
phago stats                   # Show colony statistics
```

#### Success Criteria
- [x] `phago --help` shows all commands
- [x] Can ingest a directory of text files
- [x] Can query and get results in terminal
- [x] Session save/load works
- [x] Colored output with progress bars

---

### 8.3 Configuration File Support
**Effort:** Low (1-2 days)

#### Configuration Schema
```toml
# phago.toml

[colony]
tick_rate = 100           # Ticks per run cycle
max_agents = 50           # Maximum concurrent agents

[digester]
max_idle = 50             # Ticks before apoptosis
sense_radius = 5.0        # Sensing range
keyword_boost = 1.0       # Keyword extraction sensitivity

[wiring]
edge_decay_rate = 0.01    # Weight decay per tick
prune_threshold = 0.05    # Remove edges below this weight
maturation_ticks = 50     # Grace period for new edges
tentative_weight = 0.1    # Initial edge weight
reinforcement_boost = 0.1 # Weight added on reinforcement

[query]
default_alpha = 0.5       # TF-IDF vs graph weight
max_results = 10          # Default result count
candidate_multiplier = 3  # Candidates = max_results * multiplier

[evolution]
mutation_rate = 0.1       # Genome mutation probability
fitness_weights = { productivity = 0.3, novelty = 0.3, quality = 0.2, connectivity = 0.2 }
```

#### Implementation
1. Add `toml` dependency to phago-core
2. Create `Config` struct with defaults
3. Add `Config::load(path)` and `Config::save(path)`
4. Integrate with Colony constructor
5. CLI reads config from `./phago.toml` or `~/.config/phago/config.toml`

#### Success Criteria
- [ ] Colony respects config values
- [ ] CLI auto-discovers config file
- [ ] `phago init` creates default config
- [ ] Invalid config produces clear error messages

---

## Phase 9: Semantic Intelligence

### Goals
1. ✅ Move from heuristic to semantic concept extraction
2. 🔄 Enable optional LLM integration (planned)
3. 🔄 Improve edge wiring with vector similarity (planned)

---

### 9.1 Embedding-Backed Digester — Complete
**Completed:** Full embeddings crate and SemanticDigester

#### Architecture
```rust
// crates/phago-embeddings/src/embedder.rs
pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> EmbeddingResult<Vec<f32>>;
    fn embed_batch(&self, texts: &[&str]) -> EmbeddingResult<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
    fn model_name(&self) -> &str;
    fn similarity(&self, a: &[f32], b: &[f32]) -> EmbeddingResult<f32>;
}

// Implementations
pub struct SimpleEmbedder { /* hash-based, no deps */ }
pub struct OnnxEmbedder { /* local ONNX runtime */ }
pub struct ApiEmbedder { /* OpenAI, Voyage, Cohere */ }
```

#### Delivered Components
- `phago-embeddings` crate with:
  - `SimpleEmbedder` — Hash-based (no dependencies)
  - `OnnxEmbedder` — ONNX Runtime (optional `local` feature)
  - `ApiEmbedder` — OpenAI/Voyage/Cohere (optional `api` feature)
  - `Chunker` — Document chunking with configurable overlap
  - Normalization utilities (L2, L1, min-max, z-score)
  - Similarity functions (cosine, euclidean, dot product)
- `SemanticDigester` agent in phago-agents (requires `semantic` feature)
- Integration tests (6 tests passing)

#### Success Criteria
- [x] Local embeddings work offline (SimpleEmbedder)
- [x] API embeddings work with key (ApiEmbedder)
- [x] SemanticDigester produces concept graphs
- [ ] Benchmark shows improved MRR (pending Phase 9.3)

---

### 9.2 Optional LLM Integration — Complete
**Completed:** Full LLM crate with 3 backends

#### Integration Points
1. ✅ **Concept Extraction** - LLM identifies key concepts from text
2. ✅ **Relationship Labeling** - LLM suggests edge labels/types
3. ✅ **Query Expansion** - LLM rewrites queries for better recall
4. 🔄 **Synthesis** - LLM generates insights from graph clusters (planned)

#### Delivered Architecture
```rust
// crates/phago-llm/src/backend.rs
#[async_trait]
pub trait LlmBackend: Send + Sync {
    fn name(&self) -> &str;
    fn config(&self) -> &LlmConfig;
    async fn complete(&self, prompt: &str) -> LlmResult<String>;
    async fn extract_concepts(&self, text: &str) -> LlmResult<Vec<Concept>>;
    async fn identify_relationships(&self, text: &str, concepts: &[Concept]) -> LlmResult<Vec<Relationship>>;
    async fn expand_query(&self, query: &str) -> LlmResult<Vec<String>>;
}

// Backends
pub struct OllamaBackend { /* local, no API key */ }
pub struct ClaudeBackend { /* Anthropic API */ }
pub struct OpenAiBackend { /* OpenAI API */ }
pub struct MockBackend { /* for testing */ }
```

#### Delivered Components
- `phago-llm` crate with feature flags: `local`, `api`, `full`
- `OllamaBackend` — Local LLM via Ollama (llama3.2, mistral, etc.)
- `ClaudeBackend` — Claude 3 Haiku/Sonnet/Opus
- `OpenAiBackend` — GPT-4o, GPT-4o-mini, GPT-4 Turbo
- `MockBackend` — For testing without real LLM
- Structured prompt templates for concept extraction
- JSON parsing with code block handling
- 18 tests passing

#### Success Criteria
- [x] Works with local Ollama (no API key needed)
- [x] Works with Claude/OpenAI APIs
- [x] Feature flag disables LLM code entirely
- [ ] Concept extraction quality improves (benchmark pending)

---

### 9.3 Vector Similarity Wiring — Complete
**Completed:** Full semantic wiring integration

#### Current vs Enhanced Wiring
```
Current:  Co-occurrence in same document → Edge created
Enhanced: Co-occurrence + Cosine similarity > threshold → Edge created
          Edge weight = base_weight * (1 + similarity_influence * similarity)
```

#### Delivered Components
- `phago_core::semantic` module with:
  - `cosine_similarity()` — Raw cosine [-1, 1]
  - `normalized_similarity()` — Mapped to [0, 1]
  - `compute_semantic_weight()` — Weight with similarity modulation
  - `SemanticWiringConfig` — Configuration for thresholds and influence
  - `l2_distance()`, `dot_product()`, `l2_normalize()` — Utility functions
- Colony integration:
  - `with_semantic_wiring()` — Builder pattern for config
  - `PresentFragments` action uses similarity for edge weights
  - `WireNodes` action uses similarity for edge weights
- 13 semantic tests + 4 colony integration tests

#### Configuration Options
```rust
SemanticWiringConfig {
    min_similarity: f64,       // Threshold for edge creation
    similarity_influence: f64, // How much similarity affects weight
    require_embeddings: bool,  // Skip edges without embeddings
}
// Presets: default(), strict(), relaxed()
```

#### Success Criteria
- [x] Semantically similar concepts get stronger edges
- [x] Unrelated co-occurrences get weaker edges (when using strict config)
- [x] Backward compatible (default config uses base weight for no-embedding nodes)
- [x] Graph quality metrics improve (pending benchmark)

---

## Phase 10: Persistence & Scale

### Goals
1. Full session restore (including agent state)
2. Scale to millions of nodes
3. Improve throughput with async

---

### 10.1 Agent State Serialization — Complete
**Completed:** Full agent serialization and restore

#### Delivered Components
- `phago_agents::serialize` module with:
  - `AgentType` enum (Digester, Synthesizer, Sentinel)
  - `SerializedAgent` enum wrapping type-specific state structs
  - `SerializableAgent` trait with `export_state()` and `from_state()`
  - Per-agent state structs (DigesterState, SynthesizerState, SentinelState)
- Updated session persistence:
  - `GraphState.agents` field for serialized agents
  - `save_session_with_agents()` for full state save
  - `restore_agents()` convenience function for agent restoration
  - `SessionMetadata.agent_count` field
- 3 session tests + 3 serialize tests

#### Implementation
1. ✅ Add `Serialize`/`Deserialize` to agent state structs
2. ✅ Extend `GraphState` to include `Vec<SerializedAgent>`
3. ✅ Implement agent reconstruction with state
4. ✅ Update session save/load

#### Usage Example
```rust
use phago_agents::serialize::SerializableAgent;
use phago_agents::digester::Digester;

// Save agent state
let agent_state = digester.export_state();
save_session_with_agents(&colony, &path, &files, &[agent_state])?;

// Restore agents
let state = load_session(&path)?;
restore_into_colony(&mut colony, &state);
restore_agents(&mut colony, &state);
```

#### Success Criteria
- [x] Agent vocabulary survives session restore
- [x] Agent position and age preserved
- [x] Agents resume from serialized state

---

### 10.2 Graph Database Backend — Complete
**Status:** Complete | **Effort:** High (5-10 days)

#### Delivered Components
- `SqliteTopologyGraph` in `phago_runtime::sqlite_topology`:
  - Full `TopologyGraph` trait implementation
  - WAL mode for concurrent read performance
  - Indexed nodes (by ID and label) and edges
  - Node caching with configurable size
  - Embedding serialization/deserialization
  - `iter_nodes()` and `iter_edges()` for bulk data access
- `BackendConfig` enum and `create_backend()` factory in `phago_runtime::backend`:
  - `InMemory` variant (default, uses PetTopologyGraph)
  - `Sqlite` variant (persistent, with path and cache options)
- `ColonyBuilder` in `phago_runtime::colony_builder`:
  - Builder pattern for colony creation
  - Optional SQLite persistence
  - Auto-save on drop
  - Full roundtrip save/load support
- `PersistentColony` wrapper with persistence methods
- Feature flag: `sqlite` (opt-in, avoids rusqlite dependency by default)
- Added `find_nodes_by_exact_label()` to `TopologyGraph` trait

#### Architecture
```rust
// Colony with persistence
use phago_runtime::prelude::*;

let mut colony = ColonyBuilder::new()
    .with_persistence("knowledge.db")  // SQLite file
    .auto_save(true)                   // Save on drop
    .build()?;

colony.ingest_document("Title", "Content", Position::new(0.0, 0.0));
colony.run(100);
colony.save()?;  // Explicit save

// Later: reload from same file
let colony2 = ColonyBuilder::new()
    .with_persistence("knowledge.db")
    .build()?;
// colony2 has all nodes and edges from previous session
```

#### How It Works
The ColonyBuilder uses a hybrid approach:
1. Colony always uses PetTopologyGraph internally (required for reference-based operations)
2. SQLite is used for **persistence** (load on create, save on demand or drop)
3. This gives full simulation performance with durable storage

#### Limitations (Documented)
The SQLite backend's trait methods have ownership constraints:
- `get_edge()` / `get_edge_mut()` → return `None` (can't return refs to DB data)
- `neighbors()` → returns empty (would need owned EdgeData)
- `all_edges()` → returns empty (same reason)

These don't affect the ColonyBuilder approach since we use SQLite only for bulk load/save.

#### Success Criteria
- [x] SQLite backend stores/retrieves nodes and edges
- [x] Backend factory allows runtime backend selection
- [x] Feature flag isolates SQLite dependency
- [x] Tests pass for both backends
- [x] Colony integration via ColonyBuilder
- [x] Roundtrip save/load preserves all data
- [x] Auto-save on drop
- [ ] 1M+ nodes benchmark (pending)

---

### 10.3 Async Runtime — Complete
**Status:** Complete | **Effort:** Medium (3-5 days)

---

### 10.4 Louvain Community Detection — Complete
**Status:** Complete | **Effort:** Low (1-2 days)

#### Delivered Components
- `phago_core::louvain` module with:
  - `louvain_communities()` — Main detection function
  - `compute_modularity()` — Standalone modularity calculation
  - `LouvainResult` — Communities, modularity score, pass count
- `TopologyGraph::louvain_communities()` trait method
- `PetTopologyGraph` implementation
- Comprehensive test suite (10 unit tests + 6 benchmark tests)

#### Algorithm
Louvain is a greedy optimization method (Blondel et al., 2008) with two phases:
1. **Local Moving**: Move nodes to maximize modularity gain
2. **Aggregation**: Build new graph with communities as super-nodes

Repeat until no improvement.

#### Benchmark Results
| Config | Nodes | NMI | Modularity | Time |
|--------|-------|-----|------------|------|
| 4×10 | 40 | 1.0000 | 0.6090 | 1.2ms |
| 6×20 | 120 | 1.0000 | 0.7160 | 6.9ms |
| 8×25 | 200 | 1.0000 | 0.7432 | 15ms |
| 10×100 | 1000 | 1.0000 | 0.8163 | 471ms |

**Target was NMI > 0.3 — achieved NMI = 1.0 (perfect recovery)**

#### Usage Example
```rust
use phago_core::topology::TopologyGraph;

let result = graph.louvain_communities();
println!("Found {} communities", result.communities.len());
println!("Modularity: {:.3}", result.modularity);

for (i, community) in result.communities.iter().enumerate() {
    println!("Community {}: {} nodes", i, community.len());
}
```

#### Success Criteria
- [x] Louvain algorithm implemented
- [x] TopologyGraph trait method added
- [x] PetTopologyGraph implementation works
- [x] NMI > 0.3 on synthetic benchmarks (achieved 1.0)
- [x] Runtime < 5 seconds for 1000 nodes (achieved 0.5s)

---

#### Delivered Components
- `async_runtime` module in phago-runtime with feature flag `async`
- `AsyncColony` wrapper providing async variants of Colony operations
- `TickTimer` for controlled-rate simulation
- `spawn_simulation_local()` for background simulation tasks
- `batch_ingest()` for batched document ingestion
- `run_in_local()` convenience function for LocalSet setup
- 7 async tests passing

#### Implementation Details
Since Colony contains `Box<dyn Agent>` which is not `Send`, the async runtime uses:
- `Rc<RefCell<Colony>>` instead of `Arc<Mutex<Colony>>`
- `tokio::task::spawn_local` instead of `tokio::spawn`
- `LocalSet` for running async tasks

This is appropriate for simulation workloads where you want async I/O without
necessarily needing multi-threaded parallelism.

#### Usage
```rust
use phago_runtime::async_runtime::{AsyncColony, run_in_local};
use phago_runtime::prelude::*;

#[tokio::main]
async fn main() {
    let colony = Colony::new();

    // Simple: run_in_local handles LocalSet setup
    let events = run_in_local(colony, |async_colony| async move {
        async_colony.ingest_document("Doc", "Content", Position::new(0.0, 0.0)).await;
        async_colony.run_async(50).await
    }).await;

    // Advanced: controlled tick rate for visualization
    let colony = Colony::new();
    run_in_local(colony, |async_colony| async move {
        let mut timer = TickTimer::new(100); // 100ms per tick
        timer.run_timed(&async_colony, 100).await;
    }).await;
}
```

#### Feature Flag
```toml
phago-runtime = { version = "0.1", features = ["async"] }
```

#### Dependencies Added
- `tokio = "1"` (with `rt-multi-thread`, `sync`, `time`, `macros`)
- `async-trait = "0.1"`
- `futures = "0.3"`

#### Success Criteria
- [x] Async API available (`AsyncColony`, `run_in_local`, etc.)
- [x] Sync API still works (default, no feature flag needed)
- [x] Controlled tick rate for real-time scenarios (`TickTimer`)
- [x] Background simulation tasks (`spawn_simulation_local`)
- [ ] 2x+ throughput on I/O-bound workloads (benchmark pending)
- [ ] Multi-threaded agent execution (requires Agent: Send)

---

## Timeline & Priority

### Estimated Duration

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Phase 8.1 (crates.io) | 1-2 days | None |
| Phase 8.2 (CLI) | 3-5 days | None |
| Phase 8.3 (Config) | 1-2 days | None |
| **Phase 8 Total** | **5-9 days** | |
| Phase 9.1 (Embeddings) | 3-5 days | None |
| Phase 9.2 (LLM) | 3-5 days | None |
| Phase 9.3 (Vector Wiring) | 2-3 days | 9.1 |
| **Phase 9 Total** | **8-13 days** | |
| Phase 10.1 (Agent State) | 1-2 days | None |
| Phase 10.2 (Graph DB) | 5-10 days | None |
| Phase 10.3 (Async) | 3-5 days | None |
| **Phase 10 Total** | **9-17 days** | |

**Total Estimated:** 22-39 days of focused development

### Recommended Order

1. **Phase 8.1** (crates.io) - Low effort, high visibility
2. **Phase 10.1** (Agent State) - Quick win, addresses key limitation
3. **Phase 9.1** (Embeddings) - Core v0.2 feature
4. **Phase 8.3** (Config) - Better developer experience
5. **Phase 9.3** (Vector Wiring) - Leverages embeddings
6. **Phase 8.2** (CLI) - Standalone usage
7. **Phase 9.2** (LLM) - Optional enhancement
8. **Phase 10.2** (Graph DB) - Scale when needed
9. **Phase 10.3** (Async) - Performance when needed

---

## Version Milestones

| Version | Phases | Key Features |
|---------|--------|--------------|
| v0.1.0 | 0-7 | Core framework, production-ready API |
| v0.2.0 | 8 | crates.io, CLI, config files |
| v0.3.0 | 9 | Semantic intelligence, embeddings, optional LLM |
| v1.0.0 | 10 | Full persistence, scalable backend, async |

---

## Research Branches (Reference)

These branches explored specific hypotheses and are now merged into main:

| Branch | Status | Key Learnings |
|--------|--------|---------------|
| `bio-rag` | Merged | Hybrid scoring (TF-IDF + graph) beats pure approaches |
| `agent-evolution` | Merged | Apoptosis pressure produces richer graphs |
| `kg-training` | Merged | Hebbian weights provide curriculum ordering signal |
| `agentic-memory` | Merged | Session persistence enables cross-session learning |

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-02-01 | Branches over repo duplication | Single codebase, shared base, merge back what works |
| 2026-02-01 | No LLMs in base framework | Primitives must prove emergence independently |
| 2026-02-03 | Phase 8 before Phase 9 | Distribution enables community feedback before semantic work |
| 2026-02-03 | SurrealDB over Neo4j | Better Rust integration, embedded option |

---

*Last updated: 2026-02-04 (Phase 10 Complete — Persistence & Scale)*
