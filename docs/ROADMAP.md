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
**Status:** Planned | **Priority:** High

Make Phago accessible to the broader Rust community.

### Phase 9: Semantic Intelligence
**Status:** Planned | **Priority:** Medium

Add embedding-based understanding (v0.2 milestone).

### Phase 10: Persistence & Scale
**Status:** Planned | **Priority:** Low

Enable production deployments at scale.

---

## Phase 8: Distribution & Usability

### Goals
1. Publish to crates.io for easy installation
2. Provide CLI for non-programmatic usage
3. Support configuration without recompilation

---

### 8.1 Publish to crates.io
**Effort:** Low (1-2 days)

#### Prerequisites
- [ ] Verify all crate metadata (authors, license, description, keywords)
- [ ] Ensure no path dependencies in published crates
- [ ] Add repository and documentation links
- [ ] Write crate-level documentation

#### Steps
1. Update `Cargo.toml` for each crate with publish metadata
2. Convert workspace path dependencies to version dependencies
3. Publish in dependency order:
   - `phago-core` (no dependencies)
   - `phago-agents` (depends on core)
   - `phago-runtime` (depends on core, agents)
   - `phago-rag` (depends on runtime)
   - `phago-viz` (depends on runtime)
   - `phago` (facade, depends on all)
4. Verify installation: `cargo add phago`

#### Success Criteria
- [ ] `cargo add phago` works
- [ ] Documentation renders on docs.rs
- [ ] Examples compile from crates.io version

---

### 8.2 CLI Tool
**Effort:** Medium (3-5 days)

#### Commands
```bash
phago init                    # Create .phago/ directory and config
phago ingest <file|dir>       # Ingest documents into colony
phago run [--ticks N]         # Run simulation
phago query <text>            # Query the knowledge graph
phago explore [--centrality|--bridges|--paths]  # Structural queries
phago export [--json|--graphml]  # Export graph
phago session save <name>     # Save session
phago session load <name>     # Load session
phago stats                   # Show colony statistics
```

#### File Structure
```
crates/phago-cli/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── init.rs
│   │   ├── ingest.rs
│   │   ├── run.rs
│   │   ├── query.rs
│   │   ├── explore.rs
│   │   ├── export.rs
│   │   ├── session.rs
│   │   └── stats.rs
│   └── config.rs
```

#### Implementation
1. Create `crates/phago-cli/` crate
2. Add clap dependency with derive feature
3. Implement each subcommand
4. Add to workspace
5. Create binary target

#### Success Criteria
- [ ] `phago --help` shows all commands
- [ ] Can ingest a directory of text files
- [ ] Can query and get results in terminal
- [ ] Session save/load works

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
1. Move from heuristic to semantic concept extraction
2. Enable optional LLM integration
3. Improve edge wiring with vector similarity

---

### 9.1 Embedding-Backed Digester
**Effort:** Medium (3-5 days)

#### Architecture
```rust
pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> Vec<f32>;
    fn embed_batch(&self, texts: &[&str]) -> Vec<Vec<f32>>;
    fn dimension(&self) -> usize;
}

pub struct LocalEmbedder {
    model: ort::Session,  // ONNX runtime
}

pub struct ApiEmbedder {
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
}
```

#### Implementation
1. Create `crates/phago-embeddings/` crate
2. Implement `Embedder` trait
3. Add local ONNX backend (sentence-transformers)
4. Add optional API backend (OpenAI, Voyage, etc.)
5. Create `SemanticDigester` that uses embeddings for:
   - Concept extraction (cluster similar chunks)
   - Edge weight initialization (cosine similarity)

#### Success Criteria
- [ ] Local embeddings work offline
- [ ] API embeddings work with key
- [ ] SemanticDigester produces better concept graphs
- [ ] Benchmark shows improved MRR

---

### 9.2 Optional LLM Integration
**Effort:** Medium (3-5 days)

#### Integration Points
1. **Concept Extraction** - LLM identifies key concepts from text
2. **Relationship Labeling** - LLM suggests edge labels/types
3. **Query Expansion** - LLM rewrites queries for better recall
4. **Synthesis** - LLM generates insights from graph clusters

#### Architecture
```rust
pub trait LlmBackend: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String>;
    async fn extract_concepts(&self, text: &str) -> Result<Vec<Concept>>;
    async fn suggest_relationships(&self, a: &str, b: &str) -> Result<Vec<Relationship>>;
}

pub struct ClaudeBackend { /* ... */ }
pub struct OpenAiBackend { /* ... */ }
pub struct OllamaBackend { /* ... */ }  // Local
```

#### Implementation
1. Create `crates/phago-llm/` crate
2. Implement backends (Claude, OpenAI, Ollama)
3. Add `LlmDigester` variant
4. Make LLM usage optional (feature flag)

#### Success Criteria
- [ ] Works with local Ollama (no API key needed)
- [ ] Works with Claude/OpenAI APIs
- [ ] Concept extraction quality improves
- [ ] Feature flag disables LLM code entirely

---

### 9.3 Vector Similarity Wiring
**Effort:** Low-Medium (2-3 days)

#### Current vs Enhanced Wiring
```
Current:  Co-occurrence in same document → Edge created
Enhanced: Co-occurrence + Cosine similarity > threshold → Edge created
          Edge weight = base_weight * similarity_score
```

#### Implementation
1. Store embeddings in NodeData
2. Modify `wire_edge` to compute similarity
3. Use similarity as weight multiplier
4. Add similarity threshold to config

#### Success Criteria
- [ ] Semantically similar concepts get stronger edges
- [ ] Unrelated co-occurrences get weaker edges
- [ ] Graph quality metrics improve

---

## Phase 10: Persistence & Scale

### Goals
1. Full session restore (including agent state)
2. Scale to millions of nodes
3. Improve throughput with async

---

### 10.1 Agent State Serialization
**Effort:** Low (1-2 days)

#### Current Limitation
- Graph nodes/edges persist ✓
- Agent structs are recreated on restore ✗
- Agent internal state (vocabulary, fitness history) is lost ✗

#### Data to Persist
```rust
#[derive(Serialize, Deserialize)]
pub struct SerializedAgent {
    pub agent_type: AgentType,
    pub id: AgentId,
    pub position: Position,
    pub genome: AgentGenome,
    pub vocabulary: HashSet<String>,
    pub fitness_history: Vec<f64>,
    pub ticks_alive: u64,
    pub documents_processed: u64,
}
```

#### Implementation
1. Add `Serialize`/`Deserialize` to agent structs
2. Extend `GraphState` to include `Vec<SerializedAgent>`
3. Implement agent reconstruction with state
4. Update session save/load

#### Success Criteria
- [ ] Agent vocabulary survives session restore
- [ ] Fitness history preserved
- [ ] Agents resume from exact state

---

### 10.2 Graph Database Backend
**Effort:** High (5-10 days)

#### Architecture
```rust
pub trait GraphBackend: Send + Sync {
    fn add_node(&mut self, node: NodeData) -> NodeId;
    fn add_edge(&mut self, from: &NodeId, to: &NodeId, data: EdgeData);
    fn get_node(&self, id: &NodeId) -> Option<NodeData>;
    fn get_neighbors(&self, id: &NodeId) -> Vec<(NodeId, EdgeData)>;
    fn query(&self, query: &str) -> Vec<NodeId>;
    fn update_edge_weight(&mut self, from: &NodeId, to: &NodeId, weight: f64);
    fn remove_edge(&mut self, from: &NodeId, to: &NodeId);
    fn all_nodes(&self) -> Vec<NodeId>;
    fn node_count(&self) -> usize;
    fn edge_count(&self) -> usize;
}

pub struct PetgraphBackend { /* current implementation */ }
pub struct SurrealBackend { /* new implementation */ }
pub struct Neo4jBackend { /* alternative */ }
```

#### Implementation
1. Extract current petgraph code behind trait
2. Implement SurrealDB backend
3. Add backend selection to config
4. Benchmark both backends
5. Add migration tool (petgraph → SurrealDB)

#### Success Criteria
- [ ] 1M+ nodes without OOM
- [ ] Query latency < 100ms
- [ ] Seamless backend switching
- [ ] Data migration works

---

### 10.3 Async Runtime
**Effort:** Medium (3-5 days)

#### Blocking Operations to Address
1. File I/O (document ingestion)
2. Graph queries (especially with DB backend)
3. Embedding API calls
4. LLM API calls

#### Implementation
1. Add tokio as optional dependency
2. Create async variants of key functions:
   - `Colony::tick_async()`
   - `Colony::ingest_document_async()`
   - `hybrid_query_async()`
3. Keep sync API for simple use cases
4. Use `tokio::spawn` for parallel agent execution

#### Success Criteria
- [ ] Async API available
- [ ] Sync API still works
- [ ] 2x+ throughput on I/O-bound workloads
- [ ] Agents can run in parallel

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

*Last updated: 2026-02-03 (Post Phase 7 completion)*
