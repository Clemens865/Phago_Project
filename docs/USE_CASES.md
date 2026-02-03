# Phago Use Cases

## Self-Evolving Knowledge Substrate for Intelligent Systems

Phago is not a search engine. It is a **biological knowledge substrate** where autonomous agents digest information, build Hebbian connections, and exhibit emergent collective behavior. This document outlines concrete use cases where Phago's unique capabilities provide value that traditional approaches cannot match.

---

## Quick Reference: When to Use Phago

| Use Case | Why Phago Wins | Traditional Alternative |
|----------|----------------|------------------------|
| Agent swarm memory | Self-healing, evolves with use | Static RAG (doesn't learn) |
| Codebase intelligence | Identifies hubs, tracks staleness | LSP/ctags (static index) |
| Continuous threat intel | Auto-forgets stale threats | Manual curation |
| Personalized learning | Adapts to learner's graph | Fixed curriculum |
| Auditable AI reasoning | Path traces with weights | Embedding similarity (opaque) |
| Research collaboration | Merge divergent hypotheses | Manual conflict resolution |
| Streaming knowledge | Natural decay + reinforcement | Requires re-indexing |

---

## 1. Agentic Workflow Memory

### The Problem

AI agent swarms (coding assistants, research agents, CI/CD automation) need shared, persistent context that improves with use. Current solutions are:
- **Stateless:** Each agent starts fresh, no learning
- **Static RAG:** Embeddings don't learn from usage patterns
- **Key-value stores:** No structural relationships

### Why Phago Fits

Agents share a knowledge graph where:
- **Successful tool chains strengthen:** Agent A uses `parse_json` after `fetch_url` → edge strengthens
- **Failed workflows decay naturally:** No manual cleanup required
- **New agents inherit strategies:** Fittest predecessors propagate their genomes
- **Path traces explain recommendations:** Why was this tool suggested?
- **Anomaly detection flags divergence:** Sentinel identifies when an agent deviates

### Example Integration

```
┌─────────────────────────────────────────────────────┐
│                  Agent Swarm                        │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐            │
│  │ Coder   │  │ Tester  │  │ Reviewer│            │
│  └────┬────┘  └────┬────┘  └────┬────┘            │
│       │            │            │                  │
│       └────────────┼────────────┘                  │
│                    │                               │
│                    ▼                               │
│            ┌──────────────┐                        │
│            │ Phago Colony │                        │
│            │  ┌────────┐  │                        │
│            │  │ Graph  │  │  Hebbian connections   │
│            │  └────────┘  │  Synaptic pruning      │
│            │  ┌────────┐  │  Multi-objective       │
│            │  │ Agents │  │  evolution             │
│            │  └────────┘  │                        │
│            └──────────────┘                        │
└─────────────────────────────────────────────────────┘
```

### MCP Integration

```json
// Record agent action
phago_remember({
  title: "coder_action_001",
  content: "fetch_url -> parse_json -> validate_schema -> generate_code",
  ticks: 15
})

// Query for optimal next action
phago_recall({
  query: "after parse_json what tool",
  alpha: 0.5,
  max_results: 3
})

// Find bridge concepts between domains
phago_explore({
  type: "bridges",
  top_k: 5
})
```

### Metrics

| Metric | Phago | Static RAG |
|--------|-------|------------|
| Learning from usage | Yes (Hebbian) | No |
| Self-healing | Yes (evolution) | No |
| Explainability | Path traces | Similarity scores |
| Stale knowledge | Auto-decays | Persists forever |

---

## 2. Codebase Intelligence Layer

### The Problem

Static code indices (ctags, LSP, embeddings) go stale when the codebase changes. Developers need to understand:
- Which abstractions matter most?
- How do modules connect?
- What changed recently?
- What code paths are stale?

### Why Phago Fits

The `code_digester.rs` extracts functions, structs, traits, and imports. The Hebbian graph naturally identifies hub abstractions. Evolution keeps the graph current as code changes.

### Unique Queries

```rust
// What are the key abstractions?
// → Hub nodes with highest weighted degree
phago_explore({ type: "centrality", top_k: 10 })

// How does module A depend on module B?
// → Shortest weighted path
phago_explore({ type: "path", from: "auth_module", to: "database_module" })

// What concepts connect these two domains?
// → Bridge nodes with betweenness centrality
phago_explore({ type: "bridges", top_k: 5 })

// What code paths are stale?
// → Low co-activation edges, decaying weight
// (Query by last_activated_tick threshold)
```

### Integration Example

```
┌─────────────────────────────────────────┐
│            IDE / Editor                 │
│  ┌─────────────────────────────────┐   │
│  │    Source Files                 │   │
│  │  ┌──────┐ ┌──────┐ ┌──────┐    │   │
│  │  │ .rs  │ │ .ts  │ │ .py  │    │   │
│  │  └──────┘ └──────┘ └──────┘    │   │
│  └─────────────┬───────────────────┘   │
│                │ File changes          │
│                ▼                       │
│  ┌─────────────────────────────────┐   │
│  │       Phago Colony              │   │
│  │  • Digest on save               │   │
│  │  • Reinforce on navigation      │   │
│  │  • Decay unused paths           │   │
│  │  • Evolve understanding         │   │
│  └─────────────────────────────────┘   │
│                │                       │
│                ▼                       │
│  ┌─────────────────────────────────┐   │
│  │  "Show me the key abstractions" │   │
│  │  "What depends on AuthService?" │   │
│  │  "What's stale in this module?" │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

---

## 3. Continuous Threat Intelligence

### The Problem

Security teams ingest CVEs, vendor advisories, internal logs, and threat intel feeds — thousands of documents per day with rapidly shifting topics. Static indices require re-embedding. Manual KG curation cannot keep pace.

### Why Phago Fits

- **Self-pruning:** Evolutionary agents naturally forget irrelevant old threats (apoptosis of agents specialized in stale patterns)
- **Bridge detection:** Identifies novel attack vectors connecting previously unrelated vulnerability clusters
- **Temporal context:** "This threat cluster strengthened 300% in the last 48 hours"
- **Anomaly detection:** Sentinels flag when a CVE creates unexpected connections

### Unique Capabilities

| Capability | Example |
|------------|---------|
| **Auto-pruning** | 6-month-old CVEs with no recent reinforcement decay away |
| **Bridge detection** | "This new CVE connects your auth system to a network protocol vulnerability" |
| **Temporal tracking** | "Log4j-related edges strengthened 500% this week" |
| **Structural anomaly** | "New document connects 3 previously isolated vulnerability clusters" |

### Example Flow

```
Day 1: Ingest CVE-2024-1234 (auth bypass)
       → Creates tentative edges to "auth", "session", "cookie"

Day 2: Ingest CVE-2024-1235 (same attack pattern)
       → Reinforces edges: weight 0.1 → 0.2

Day 3: Ingest CVE-2024-1236 (unrelated, network layer)
       → Creates new cluster, no reinforcement of auth edges

Day 30: Auth edges reinforced 15 times → weight 0.8
        Network edges reinforced 2 times → weight 0.15
        → Pruning removes weak network edges, keeps strong auth pattern

Query: "What's the biggest threat this month?"
       → Centrality analysis surfaces the auth bypass cluster
```

---

## 4. Personalized Learning / Tutoring Systems

### The Problem

Current learning platforms deliver the same content to every learner. Adaptive learning systems exist but rely on explicit skill models that require manual construction.

### Why Phago Fits

The graph adapts to what the learner knows:
- Frequently queried concepts strengthen
- Path traces explain connections between new and known concepts
- Sentinels identify knowledge gaps

### Unique Capabilities

**Path-based explanations:**
> "You asked about quantum entanglement. Here's how it connects to what you already know: photon (your strongest concept) → polarization (reinforced 8 times) → entanglement"

**Gap analysis via Sentinel:**
> "Your understanding of thermodynamics is missing a connection to entropy — 4 of your 5 thermo concepts connect to it in the reference graph"

**Cross-session adaptation:**
> "Last week you struggled with derivatives. The related concept 'limits' is now reinforced. Try calculus problems again."

### Learning Graph Evolution

```
Session 1: Student queries "photon" 10 times
           → "photon" node weight increases
           → Edges to "light", "energy" strengthen

Session 2: Student queries "wave-particle duality"
           → New connections to existing "photon" cluster
           → Path: photon → wave → particle → duality

Session 3: Student queries "quantum entanglement"
           → System explains via path through known concepts
           → Identifies gap: "superposition" has no incoming edges

Recommendation: "Before entanglement, review superposition —
                 it connects to photon (your strongest concept)"
```

---

## 5. Auditable AI Reasoning (Regulated Industries)

### The Problem

Healthcare, finance, and legal applications require explanation of *why* a recommendation was made. Neural approaches (LLMs, embedding similarity) are opaque.

### Why Phago Fits

Every query result has:
- **Structural trace:** Full path through the graph
- **Edge weights:** Quantified connection strength
- **Co-activation history:** How many documents reinforced this path
- **Temporal history:** When edges were created/reinforced
- **Counterfactual potential:** Remove an edge, re-run, see how results change

### Example: Healthcare Decision Support

```
Query: "treatment options for condition X"

Result: Treatment A (score: 0.87)

Audit Trail:
├── condition_X (seed term, direct match)
│   └── symptom_cluster_Y (Hebbian edge, co-activated 47 times, weight 0.91)
│       └── treatment_A (bridge concept, 3 independent paths, weight 0.78)
│           ├── Path 1: condition_X → symptom_Y → treatment_A (via clinical_trial_123)
│           ├── Path 2: condition_X → biomarker_Z → treatment_A (via research_paper_456)
│           └── Path 3: condition_X → related_condition → treatment_A (via case_study_789)

Counterfactual: If symptom_cluster_Y edge removed, Treatment B ranks #1 instead

Temporal: Edge condition_X → treatment_A reinforced 12 times in last 50 ticks
          Before tick 230, Treatment B ranked higher
```

### Compliance Features

| Requirement | Phago Capability |
|-------------|------------------|
| Explainability | Path traces with weights |
| Auditability | Full edge creation/reinforcement history |
| Reproducibility | Deterministic traversal with saved state |
| Bias detection | Identify over-reinforced paths |
| Counterfactuals | Remove edges and measure impact |

---

## 6. Research Collaboration with Graph Diffing

### The Problem

Research teams explore divergent hypotheses. Merging findings back into a canonical knowledge base is manual and error-prone.

### Why Phago Fits (Planned Feature)

Graph diffing enables:
- **Branching:** Fork session A into session B to explore a hypothesis
- **Observable evolution:** See what changed between sessions
- **Merge semantics:** Edge weights provide natural conflict resolution

### Example Workflow

```
Main Branch (canonical knowledge):
├── hypothesis_A (weight 0.8)
├── hypothesis_B (weight 0.6)
└── hypothesis_C (weight 0.4)

Fork → Researcher 1 explores hypothesis_B deeply:
├── hypothesis_B (weight 0.9, reinforced)
├── new_evidence_1 (added)
└── new_evidence_2 (added)

Fork → Researcher 2 challenges hypothesis_B:
├── hypothesis_B (weight 0.3, decay from counter-evidence)
├── counter_evidence_1 (added)
└── alternative_hypothesis_D (added)

Merge strategy:
├── hypothesis_B: conflict! (0.9 vs 0.3) → flag for human review
├── new_evidence_1: include (no conflict)
├── counter_evidence_1: include (no conflict)
└── alternative_hypothesis_D: include with reduced weight
```

---

## 7. Streaming Knowledge Bases

### The Problem

Traditional indices are built once and queried. When new documents arrive continuously, they require periodic re-indexing or become stale.

### Why Phago Fits

- **Continuous ingestion:** Documents digested as they arrive
- **Natural forgetting:** Unused edges decay without manual cleanup
- **Hebbian reinforcement:** Important concepts strengthen through repeated reference
- **Evolutionary adaptation:** Agent population adapts to data distribution shifts

### Comparison

| Aspect | Traditional RAG | Phago |
|--------|-----------------|-------|
| New document | Re-index batch | Digest immediately |
| Stale document | Persists forever | Edges decay naturally |
| Distribution shift | Full re-embedding | Evolution adapts |
| Index maintenance | Manual | Automatic |

---

## Technical Integration Patterns

### MCP Server Integration

```bash
# Start Phago as MCP server
phago-mcp-server --port 3000

# Available tools:
# - phago_remember: Ingest document
# - phago_recall: Hybrid query
# - phago_explore: Structural queries
```

### Rust Library Integration

```rust
use phago_runtime::colony::Colony;
use phago_rag::{hybrid_query, HybridConfig};

// Create colony
let mut colony = Colony::new();

// Ingest documents
colony.ingest_document("title", "content", Position::new(0.0, 0.0));

// Spawn agents and run
colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0))));
colony.run(100);

// Query with hybrid scoring
let config = HybridConfig { alpha: 0.5, max_results: 10, candidate_multiplier: 3 };
let results = hybrid_query(&colony, "query terms", &config);
```

### Session Persistence

```rust
use phago_runtime::session::{save_session, load_session, restore_into_colony};

// Save session
save_session(&colony, Path::new("session.json"), &files)?;

// Load and restore
let state = load_session(Path::new("session.json"))?;
let mut restored = Colony::new();
restore_into_colony(&mut restored, &state);

// Colony continues from saved tick with full temporal state
```

---

## Summary: The Phago Advantage

| Dimension | What Phago Provides | What Others Lack |
|-----------|---------------------|------------------|
| **Learning** | Hebbian reinforcement from usage | Static embeddings |
| **Self-healing** | Evolutionary agent population | Manual re-indexing |
| **Explainability** | Weighted path traces | Cosine similarity scores |
| **Temporal context** | Edge creation/reinforcement history | Point-in-time snapshots |
| **Anomaly detection** | Sentinel agents (integrated) | Separate pipeline |
| **Structural queries** | Path, centrality, bridges | Not expressible |
| **Session continuity** | Full state restore | Stateless queries |

**Phago is not a better search engine. It is a self-evolving knowledge substrate for systems that need shared, adaptive, explainable memory.**

---

*Based on benchmark results: 98.3% edge reduction, MRR 0.800 (beats TF-IDF), 11.6x evolutionary edge advantage, 100% session fidelity.*
