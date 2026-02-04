# Phago Next Priorities — Ralph Loop Development Plan

**Created:** 2026-02-04
**Status:** Planning
**Version:** 0.2.0 → 0.3.0+

---

## Overview

This document outlines the next development priorities for Phago, each structured as a Ralph Loop iteration. Each priority includes:
- **Goal** — What we're building
- **Success Criteria** — How we know it's done
- **Ralph Loop Plan** — Iterative steps
- **Completion Promise** — The marker for loop completion

---

## Priority 1: Web Dashboard

### Goal
Real-time browser UI for colony monitoring, graph exploration, and query interface.

### Success Criteria
- [ ] Live colony stats (nodes, edges, agents, tick)
- [ ] Interactive force-directed graph visualization
- [ ] Query interface with hybrid scoring
- [ ] Document ingestion via drag-and-drop
- [ ] Agent inspection panel
- [ ] Works with SQLite persistence

### Tech Stack
- **Backend:** Axum + Tower (Rust)
- **Frontend:** HTMX + Alpine.js (minimal JS) or React
- **WebSocket:** Real-time tick updates
- **Visualization:** D3.js force-directed graph

### Ralph Loop Plan

```
/ralph-loop --max-iterations 10 --completion-promise "DASHBOARD_COMPLETE"

Iteration 1: Backend API scaffold
- Create phago-web crate
- Axum routes: GET /api/stats, GET /api/nodes, GET /api/edges
- Connect to PersistentColony via SQLite
- Test: curl returns JSON

Iteration 2: WebSocket tick stream
- Add WebSocket endpoint /ws/events
- Stream ColonyEvents on each tick
- Test: wscat receives events

Iteration 3: Static HTML shell
- Serve index.html from /
- Stats panel (placeholder)
- Graph container (placeholder)
- Query form (placeholder)

Iteration 4: Live stats
- HTMX polling or WebSocket update
- Display: nodes, edges, agents, tick, clustering
- Auto-refresh every tick

Iteration 5: Graph visualization
- D3.js force-directed layout
- Nodes sized by degree
- Edges weighted by strength
- Click node → show details

Iteration 6: Query interface
- Form: query text, alpha slider, max results
- POST /api/query → hybrid_query()
- Display results with scores
- Click result → highlight in graph

Iteration 7: Document ingestion
- Drag-and-drop zone
- POST /api/ingest with file content
- Run N ticks after ingestion
- Update graph live

Iteration 8: Agent panel
- List all agents with status
- Click agent → show position, health, vocabulary
- Button: force apoptosis

Iteration 9: Polish & responsive
- Mobile-friendly layout
- Dark mode
- Loading states
- Error handling

Iteration 10: Integration test
- Full E2E: ingest → query → visualize
- Performance: 1000 nodes renders smoothly
- Document in README

<promise>DASHBOARD_COMPLETE</promise>
```

### Estimated Effort
**5-8 days**

---

## Priority 2: LangChain/LlamaIndex Integration

### Goal
Python adapters for popular AI frameworks, making Phago a drop-in memory layer.

### Success Criteria
- [ ] `pip install phago-langchain` works
- [ ] LangChain Memory class that uses Phago
- [ ] LlamaIndex KnowledgeStore that uses Phago
- [ ] Examples in documentation
- [ ] Published to PyPI

### Tech Stack
- **Bindings:** PyO3 (Rust → Python)
- **Package:** maturin for builds
- **Targets:** LangChain 0.1+, LlamaIndex 0.10+

### Ralph Loop Plan

```
/ralph-loop --max-iterations 8 --completion-promise "PYTHON_INTEGRATIONS_COMPLETE"

Iteration 1: PyO3 scaffold
- Create phago-python crate
- Basic PyO3 module with Colony wrapper
- Test: import phago_python in Python

Iteration 2: Core Python API
- PyColony class wrapping Colony
- ingest_document(), run(), query() methods
- stats() returning dict
- Test: basic usage from Python

Iteration 3: SQLite persistence from Python
- Support db_path parameter
- Auto-save behavior
- Test: persist and reload

Iteration 4: LangChain Memory adapter
- Implement BaseChatMemory interface
- load_memory_variables() → hybrid query
- save_context() → ingest + run
- Test: works in ConversationChain

Iteration 5: LlamaIndex KnowledgeStore
- Implement BaseKnowledgeStore interface
- add_documents() → batch ingest
- query() → hybrid query with metadata
- Test: works in VectorStoreIndex

Iteration 6: PyPI packaging
- maturin build for wheels
- Setup pyproject.toml
- README with examples
- Publish to PyPI

Iteration 7: Documentation
- LangChain example notebook
- LlamaIndex example notebook
- API reference

Iteration 8: Integration tests
- Test with real LLM calls
- Memory persists across sessions
- Performance acceptable

<promise>PYTHON_INTEGRATIONS_COMPLETE</promise>
```

### Estimated Effort
**5-7 days**

---

## Priority 3: Louvain Community Detection

### Goal
Replace label propagation with Louvain algorithm for better topic clustering.

### Success Criteria
- [ ] `graph.louvain_communities()` method
- [ ] NMI > 0.3 on test corpus (vs current 0.17)
- [ ] Modularity score exposed
- [ ] Visualization colors by community

### Ralph Loop Plan

```
/ralph-loop --max-iterations 5 --completion-promise "LOUVAIN_COMPLETE"

Iteration 1: Algorithm implementation
- Add louvain.rs to phago-core
- Implement modularity calculation
- Implement community detection loop
- Unit tests for small graphs

Iteration 2: TopologyGraph integration
- Add louvain_communities() to trait
- Implement for PetTopologyGraph
- Implement for SqliteTopologyGraph
- Return Vec<Vec<NodeId>>

Iteration 3: Benchmark against label propagation
- Run on 100-doc corpus
- Measure NMI vs ground truth
- Measure modularity score
- Compare runtime

Iteration 4: Visualization integration
- Color nodes by community
- Add community filter to UI
- Export community assignments

Iteration 5: Documentation
- Update ROADMAP.md
- Add to structural queries docs
- Benchmark results in EXECUTIVE_SUMMARY

<promise>LOUVAIN_COMPLETE</promise>
```

### Estimated Effort
**2-3 days**

---

## Priority 4: Configuration File Support

### Goal
`phago.toml` for colony settings without recompilation.

### Success Criteria
- [ ] Parse phago.toml from current directory
- [ ] All colony parameters configurable
- [ ] CLI respects config file
- [ ] ColonyBuilder loads from config
- [ ] Validation with clear errors

### Configuration Schema

```toml
# phago.toml

[colony]
tick_rate = 100
max_agents = 50
auto_save = true

[persistence]
backend = "sqlite"  # or "memory"
path = "knowledge.db"
cache_size = 5000

[digester]
max_idle = 50
sense_radius = 5.0
keyword_boost = 1.0
tentative_weight = 0.1
reinforcement_boost = 0.1

[wiring]
edge_decay_rate = 0.01
prune_threshold = 0.05
maturation_ticks = 50
max_edges_per_node = 30

[query]
default_alpha = 0.5
max_results = 10
candidate_multiplier = 3

[evolution]
enabled = true
mutation_rate = 0.1
fitness_weights = { productivity = 0.3, novelty = 0.3, quality = 0.2, connectivity = 0.2 }

[semantic]
enabled = false
min_similarity = 0.3
similarity_influence = 0.5
```

### Ralph Loop Plan

```
/ralph-loop --max-iterations 5 --completion-promise "CONFIG_COMPLETE"

Iteration 1: Config struct and parsing
- Add toml dependency
- Create PhagoConfig struct
- Implement load() and save()
- Test: parse sample config

Iteration 2: ColonyBuilder integration
- from_config() constructor
- Apply all config values
- Validation with helpful errors
- Test: config → colony works

Iteration 3: CLI integration
- --config flag for explicit path
- Auto-discover ./phago.toml
- Fallback to ~/.config/phago/config.toml
- Test: CLI respects config

Iteration 4: Init command
- `phago init` creates default config
- Interactive prompts for key values
- Write phago.toml to current dir
- Test: init creates valid config

Iteration 5: Documentation
- Config reference in docs
- Examples for common scenarios
- Validation error guide

<promise>CONFIG_COMPLETE</promise>
```

### Estimated Effort
**2-3 days**

---

## Priority 5: Streaming Ingestion

### Goal
Real-time document processing as they arrive, not batch.

### Success Criteria
- [ ] `colony.ingest_stream()` accepts async iterator
- [ ] Documents processed incrementally
- [ ] Backpressure handling
- [ ] Works with file watchers
- [ ] Kafka/Redis stream support (optional)

### Ralph Loop Plan

```
/ralph-loop --max-iterations 6 --completion-promise "STREAMING_COMPLETE"

Iteration 1: Async ingest API
- ingest_document_async() method
- Returns immediately, queues for processing
- Background tick loop processes queue
- Test: async ingest works

Iteration 2: Stream trait
- DocumentStream trait
- Adapter for tokio channels
- Adapter for async iterators
- Test: channel-based ingestion

Iteration 3: File watcher integration
- notify crate for filesystem events
- Watch directory for new files
- Auto-ingest on file creation
- Test: drop file → appears in graph

Iteration 4: Backpressure handling
- Queue size limits
- Slow down producers when full
- Metrics for queue depth
- Test: handles burst traffic

Iteration 5: Message queue adapters (optional)
- Kafka consumer adapter
- Redis streams adapter
- Configuration in phago.toml
- Test: consume from Kafka

Iteration 6: Documentation
- Streaming architecture docs
- File watcher example
- Kafka integration guide

<promise>STREAMING_COMPLETE</promise>
```

### Estimated Effort
**4-6 days**

---

## Priority 6: Distributed Colony

### Goal
Multi-node sharding for large-scale deployments.

### Success Criteria
- [ ] Colony can span multiple processes
- [ ] Consistent hashing for node distribution
- [ ] Cross-shard queries work
- [ ] Fault tolerance (node failure recovery)
- [ ] Horizontal scaling demonstrated

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Coordinator                           │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Query Router │ Shard Map │ Health Monitor      │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Shard 0    │  │  Shard 1    │  │  Shard 2    │
│  ┌───────┐  │  │  ┌───────┐  │  │  ┌───────┐  │
│  │Colony │  │  │  │Colony │  │  │  │Colony │  │
│  │SQLite │  │  │  │SQLite │  │  │  │SQLite │  │
│  └───────┘  │  │  └───────┘  │  │  └───────┘  │
└─────────────┘  └─────────────┘  └─────────────┘
```

### Ralph Loop Plan

```
/ralph-loop --max-iterations 10 --completion-promise "DISTRIBUTED_COMPLETE"

Iteration 1: Shard abstraction
- ShardedColony struct
- Hash function for node assignment
- Local shard operations
- Test: single shard works

Iteration 2: Multi-process communication
- gRPC or tarpc for RPC
- Shard-to-shard messaging
- Coordinator service
- Test: two shards communicate

Iteration 3: Distributed ingestion
- Route document to correct shard
- Cross-shard edge handling
- Eventual consistency model
- Test: ingest across shards

Iteration 4: Distributed query
- Scatter query to all shards
- Gather and merge results
- Hybrid scoring across shards
- Test: query returns unified results

Iteration 5: Consistent hashing
- Hash ring for shard assignment
- Virtual nodes for balance
- Rebalancing on shard add/remove
- Test: even distribution

Iteration 6: Fault tolerance
- Health checks between nodes
- Shard failover
- State recovery from SQLite
- Test: survive node failure

Iteration 7: Cross-shard edges
- Edge routing table
- Lazy edge synchronization
- Conflict resolution
- Test: edges span shards correctly

Iteration 8: Coordinator HA
- Multiple coordinator replicas
- Leader election
- State synchronization
- Test: coordinator failover

Iteration 9: Deployment tooling
- Docker Compose for local cluster
- Kubernetes manifests
- Helm chart
- Test: deploy 3-node cluster

Iteration 10: Benchmarks and docs
- Throughput scaling test
- Latency under load
- Operations guide
- Architecture documentation

<promise>DISTRIBUTED_COMPLETE</promise>
```

### Estimated Effort
**15-25 days**

---

## Priority 7: Vector Database Integration

### Goal
Use dedicated vector databases for embedding storage and similarity search.

### Success Criteria
- [ ] Qdrant backend adapter
- [ ] Pinecone backend adapter
- [ ] Weaviate backend adapter
- [ ] Hybrid: Phago graph + external vectors
- [ ] Configurable in phago.toml

### Ralph Loop Plan

```
/ralph-loop --max-iterations 6 --completion-promise "VECTORDB_COMPLETE"

Iteration 1: VectorStore trait
- Abstract interface for vector operations
- store_embedding(), search_similar()
- Batch operations
- Test: trait compiles

Iteration 2: Qdrant adapter
- qdrant-client crate
- Implement VectorStore
- Collection management
- Test: store and search works

Iteration 3: Pinecone adapter
- pinecone-sdk crate
- Implement VectorStore
- Index management
- Test: store and search works

Iteration 4: Weaviate adapter
- weaviate-client crate
- Implement VectorStore
- Schema management
- Test: store and search works

Iteration 5: Colony integration
- VectorStore in Substrate
- Embeddings stored externally
- Graph references external IDs
- Test: hybrid query with external vectors

Iteration 6: Configuration and docs
- phago.toml vector backend config
- Migration guide from local embeddings
- Performance comparison

<promise>VECTORDB_COMPLETE</promise>
```

### Estimated Effort
**5-7 days**

---

## Execution Order

### Phase A: Foundation (Weeks 1-2)
1. **Priority 3: Louvain** (2-3 days) — Quick win, improves quality
2. **Priority 4: Config** (2-3 days) — Developer experience

### Phase B: User Interface (Weeks 3-4)
3. **Priority 1: Web Dashboard** (5-8 days) — Major visibility

### Phase C: Ecosystem (Weeks 5-6)
4. **Priority 2: Python Integrations** (5-7 days) — Adoption driver
5. **Priority 5: Streaming** (4-6 days) — Production feature

### Phase D: Scale (Weeks 7-10)
6. **Priority 6: Distributed** (15-25 days) — Enterprise feature
7. **Priority 7: Vector DBs** (5-7 days) — Performance at scale

---

## How to Start a Ralph Loop

```bash
# Start with Priority 3 (Louvain) as the first quick win
cd /Users/clemenshoenig/Documents/My-Coding-Programs/Phago-Experimental

# Invoke Ralph Loop
/ralph-loop --max-iterations 5 --completion-promise "LOUVAIN_COMPLETE"

# The loop will:
# 1. Create .claude/ralph-loop.local.md to track state
# 2. Work through iterations
# 3. Run tests to verify
# 4. Output <promise>LOUVAIN_COMPLETE</promise> when done
```

---

## Version Milestones

| Version | Priorities Included | Target |
|---------|---------------------|--------|
| **0.3.0** | Louvain, Config | Week 2 |
| **0.4.0** | Web Dashboard | Week 4 |
| **0.5.0** | Python Integrations | Week 6 |
| **0.6.0** | Streaming Ingestion | Week 7 |
| **1.0.0** | Distributed Colony | Week 10 |

---

*This plan is iterative. Priorities may shift based on user feedback and discovered requirements.*
