# Phago Project Roadmap

## Base (main branch) — Complete

The core biological computing framework with 10 primitives, 3 agent types, and quantitative proof.

| Phase | Status | Commit |
|-------|--------|--------|
| Phase 0: Scaffold | Done | `9beb9fd` |
| Phase 1: First Cell (Digest, Apoptose) | Done | `338c4ab` |
| Phase 2: Self-Organization (Sense, Wire, Stigmerge) | Done | `4baf6b0` |
| Phase 3: Emergence (Emerge, Negate) | Done | `b3ffc8e` |
| Phase 4: Cooperation (Transfer, Symbiose, Dissolve) | Done | `fe6955a` |
| Phase 5: Prove It Works (Metrics, Viz, Tests) | Done | `fe6955a` |

**Proven metrics**: 51.7% shared vocabulary, 346x dissolution reinforcement, 0.91 clustering coefficient, 51 tests passing.

---

## Branch 1: `bio-rag` — Biological RAG System

**Goal**: Build a query interface on the self-organizing Hebbian knowledge graph that outperforms static GraphRAG on retrieval quality.

**Why this is novel**: Current RAG (including Microsoft's GraphRAG) builds a static knowledge graph at index time. Phago's graph learns from usage — connections strengthen through Hebbian reinforcement, unused ones decay. No existing RAG system does this.

### Milestones

| # | Milestone | Status | Description |
|---|-----------|--------|-------------|
| 1 | Query Engine | Not started | Graph traversal from query terms following strongest connections |
| 2 | Document Loader | Not started | CLI that takes a directory of text files as input |
| 3 | Relevance Scoring | Not started | Rank retrieved concepts by path weight + access count |
| 4 | Feedback Loop | Not started | Query results reinforce traversed paths (the graph learns from queries) |
| 5 | LLM Integration | Not started | Optional: pass retrieved context to an LLM for answer generation |
| 6 | Benchmark | Not started | Compare against baseline RAG and GraphRAG on same corpus |

### Key Question to Answer
Does a self-reinforcing Hebbian knowledge graph produce measurably better retrieval than a static knowledge graph? This is testable and falsifiable.

---

## Branch 2: `agent-evolution` — Evolutionary Agent System

**Goal**: Add agent spawning, mutation, and intrinsic selection pressure so agents evolve to specialize for their document corpus over generations.

**Why this is novel**: Current "self-evolving" agent systems (EvoPrompt, Digital Red Queen) evolve prompts or configurations, not agents themselves. No existing system has apoptosis (graceful self-termination releasing learnings), horizontal gene transfer (runtime capability sharing), or endosymbiosis (failed digestion becoming permanent integration).

### Milestones

| # | Milestone | Status | Description |
|---|-----------|--------|-------------|
| 1 | Agent Spawning | Not started | Colony spawns new agents when demand exceeds capacity |
| 2 | Parameter Variation | Not started | New agents inherit parent parameters with slight mutation |
| 3 | Fitness Tracking | Not started | Track per-agent contribution to graph (terms presented, connections reinforced) |
| 4 | Selection Pressure | Not started | Apoptosis threshold adapts based on colony-wide productivity |
| 5 | Specialization Detection | Not started | Metrics showing agents diverge into specialized roles over generations |
| 6 | Multi-Generation Sim | Not started | Run 1000+ ticks, demonstrate agents improve at processing their corpus |

### Key Question to Answer
Do agents that evolve through intrinsic selection pressure (apoptosis + transfer + symbiosis) produce a better knowledge graph than a static population? Measurable by graph richness metrics.

---

## Branch 3: `kg-training` — Knowledge Graph → SLM Training Data

**Goal**: Export the colony's self-organized knowledge graph as structured training data for small language models.

**Why this is novel**: Current KG-to-LLM pipelines (CoFine, GraphMERT) use static graphs. Phago's graph is organized by usage patterns and Hebbian reinforcement — frequently reinforced connections represent high-confidence knowledge. This weighting could inform training curricula.

### Milestones

| # | Milestone | Status | Description |
|---|-----------|--------|-------------|
| 1 | Triple Export | Not started | Export graph as (subject, predicate, object, weight) triples |
| 2 | Community Detection | Not started | Identify natural clusters in the Hebbian graph |
| 3 | Curriculum Ordering | Not started | Order triples by reinforcement strength for curriculum learning |
| 4 | Training Format | Not started | Output in formats consumable by fine-tuning pipelines (JSONL, etc.) |
| 5 | Quality Metrics | Not started | Compare Phago-generated triples against LLM-extracted baselines |
| 6 | Fine-Tune Experiment | Not started | Fine-tune a small model, compare against baseline on domain QA |

### Key Question to Answer
Does a self-organized Hebbian graph produce better training data than a statically constructed knowledge graph? Measurable by downstream model performance.

---

## Activity Log

| Date | Branch | Activity | Outcome |
|------|--------|----------|---------|
| 2026-02-01 | main | Phase 5 implemented: metrics, visualization, tests | 51 tests, interactive HTML viz |
| 2026-02-01 | main | Pushed to GitHub, README added | https://github.com/Clemens865/Phago_Project |
| 2026-02-01 | all | Created three research branches with plans | bio-rag, agent-evolution, kg-training |

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-02-01 | Branches over repo duplication | Single codebase, shared base, merge back what works |
| 2026-02-01 | bio-rag as primary focus | Highest novelty-to-effort ratio, directly benchmarkable, builds on existing infrastructure |
| 2026-02-01 | No LLMs in base framework | Primitives must prove emergence independently; LLM integration is a branch concern |

---

## Research References

Key papers and systems relevant to these branches:

- **GraphRAG** (Microsoft) — Static KG construction for RAG. Our benchmark target.
- **CoFine** (2026) — KG community-based LLM fine-tuning. Relevant to kg-training.
- **Digital Red Queen** (Sakana AI) — Evolutionary pressure in artificial environments. Relevant to agent-evolution.
- **Nagpal et al. (2004)** — "Catalog of Biologically-Inspired Primitives for Self-Organization." Our intellectual predecessor.
- **Michael Levin** — Multiscale competency architectures. Theoretical alignment with Phago's approach.
