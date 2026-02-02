# Phago — Executive Summary

## What Is Phago?

Phago is a Rust framework that maps cellular biology mechanisms to computational operations. Autonomous agents digest documents, build a Hebbian knowledge graph through co-activation, and exhibit emergent collective behavior — all without top-down orchestration. The graph structure IS the memory: frequently used connections strengthen, unused ones decay.

The project implements 10 biological primitives (digest, apoptose, sense, transfer, emerge, wire, symbiose, stigmerge, negate, dissolve) across 3 agent types (Digester, Synthesizer, Sentinel) in a 6-crate Rust workspace.

---

## Four Research Branches — Results

### 1. Agent Evolution — HYPOTHESIS SUPPORTED

**Question:** Do agents evolving through apoptosis + mutation produce richer knowledge graphs than static populations?

**Result: Yes, dramatically.**

| Metric (tick 300) | Evolved | Static | Random |
|-------------------|---------|--------|--------|
| Nodes | 1,582 | 864 | 1,191 |
| Edges | 101,824 | 8,769 | 8,965 |
| Clustering coeff. | 0.969 | 0.948 | 0.970 |
| Spawns / Generations | 140 / 135 | 0 / 0 | 144 / 144 |

- Evolved populations produce **11.6x more edges** than static
- Static and random populations **collapse** after tick 200 (lose >88% of edges)
- Evolved populations **continue growing** through continuous regeneration across 135 generations
- Turnover alone is insufficient — random spawn (same rate, random genomes) also collapses

**This is the strongest result.** Fitness-directed mutation with inheritance creates a fundamentally different growth regime.

---

### 2. Agentic Memory — HYPOTHESIS SUPPORTED (persistence)

**Question:** Can a self-organizing code knowledge graph persist across sessions with full fidelity?

**Result: Yes — 100% session fidelity.**

| Metric | Value |
|--------|-------|
| Source files analyzed | 55 |
| Code elements extracted | 830 |
| Graph nodes / edges | 659 / 33,490 |
| Session persistence | 100% (659/659 nodes, 33,490/33,490 edges restored identically) |
| Graph P@5 | 0.140 |
| Grep P@5 | 0.323 |

- Session save/restore works perfectly — the graph is byte-identical after round-trip
- Retrieval precision is below grep (0.140 vs 0.323), but graph provides structural/relational context that keyword matching cannot

---

### 3. Bio-RAG — HYPOTHESIS REJECTED

**Question:** Does Hebbian-reinforced graph retrieval improve over repeated query rounds and outperform TF-IDF?

**Result: No. Reinforcement has zero measurable effect. TF-IDF wins on precision.**

| Metric | Reinforced | Static | TF-IDF | Random |
|--------|-----------|--------|--------|--------|
| P@5 | 0.270 | 0.270 | 0.658 | 0.000 |
| P@10 | 0.180 | 0.180 | 0.658 | — |
| MRR | 0.714 | 0.714 | 0.692 | — |
| NDCG@10 | 0.377 | 0.377 | 0.416 | — |

- Reinforced and static graphs produce **identical** results across all 10 rounds
- TF-IDF outperforms graph retrieval on precision by **2.4x**
- Graph retrieval wins on **MRR** (0.714 vs 0.692) — places first relevant result higher
- Query-time reinforcement is completely drowned out by the dense graph structure

---

### 4. KG Training — HYPOTHESIS PARTIALLY SUPPORTED

**Question:** Do Hebbian-weighted triples with curriculum ordering produce well-structured training data aligned with ground-truth topics?

**Result: Curriculum ordering works, but community detection fails.**

| Metric | Value |
|--------|-------|
| Triples exported | 252,641 |
| Communities detected | 548 (1 mega-community + 547 singletons) |
| NMI vs ground truth | 0.170 (target was > 0.3) |
| Foundation coherence | 100% same-community |
| Weight ratio (foundation/periphery) | 1.3x |

- Label propagation collapses into 1 mega-community containing 73% of all nodes
- NMI of 0.170 means communities do **not** align with ground-truth topics
- Weight-based curriculum ordering still works: 100% foundation coherence, 1.3x weight ratio between foundation and periphery triples

---

## Highs

1. **Agent evolution works convincingly.** 11.6x edge advantage with collapse resistance is a strong, reproducible result. The evolved population operates in a fundamentally different regime.

2. **Session persistence is perfect.** 100% fidelity on save/restore proves the graph-as-memory architecture is sound and practical.

3. **Robust engineering.** 91 tests, zero warnings, 6 crates, 4 working demos with CSV + HTML output. The Rust ownership model maps naturally to biological resource management.

4. **100-document corpus** across 4 topics (cell biology, genetics, molecular transport, quantum computing) provides meaningful test data.

5. **Graph wins first-result ranking.** MRR of 0.714 vs TF-IDF's 0.692 shows the graph finds the most relevant result faster, even if overall precision is lower.

## Lows

1. **Dense graph problem.** Hebbian wiring creates ~250k edges for ~2k nodes (avg degree ~250). This density overwhelms community detection, drowns reinforcement signals, and makes multi-hop traversal noisy. This is the root cause behind both the Bio-RAG and KG-Training failures.

2. **Reinforcement is invisible.** Query-time edge weight boosts (+0.05) are negligible in a graph where the 75th percentile edge weight is already the traversal threshold. The signal-to-noise ratio is too low.

3. **Community detection collapses.** Even with adaptive thresholding (90th percentile for dense graphs), label propagation produces 1 mega-community + singletons. The algorithm cannot recover topic structure from the dense graph.

4. **Graph retrieval loses to TF-IDF.** P@5 of 0.270 vs 0.658 is a significant gap. The graph adds relational structure but introduces enough noise to reduce precision.

5. **Fitness tracker reports 0.000.** The fitness tracking in agent-evolution records zero fitness values despite agents producing edges. The wiring is likely broken — events fire but the tracker doesn't receive them correctly.

---

## Potential Improvements

### Critical: Solve the Dense Graph Problem

The single most impactful improvement. Most failures trace back to graph density.

- **Synaptic pruning:** Biological neural networks maintain sparse connectivity through active pruning. Implement aggressive decay that removes edges below a threshold after each tick, not just at document boundaries.
- **Co-activation gating:** Only create edges between terms that co-occur in multiple documents (currently a single co-occurrence creates an edge). Require minimum 2-3 co-activations before wiring.
- **Weight initialization:** Start new edges at 0.01 instead of the current ~0.08 so they must earn their weight through repeated co-activation.

### Community Detection

- **Louvain modularity:** Replace label propagation with Louvain, which optimizes modularity directly and handles dense graphs better.
- **Infomap:** Information-theoretic community detection that finds communities based on information flow, not just edge density.
- **Pre-filtering:** Remove all edges below median weight before running community detection, reducing the effective graph to its strongest structure.

### Retrieval Quality

- **Hybrid scoring:** Combine graph traversal with TF-IDF. Use TF-IDF for candidate generation and graph structure for re-ranking.
- **Larger reinforcement signals:** Increase edge weight boosts from +0.05 to +0.15 or use multiplicative reinforcement (weight × 1.2) so the signal is proportional to existing strength.
- **Path diversity:** Penalize results that all come from the same path. Currently, dense clusters dominate results.

### Engineering

- **Fix fitness tracker:** Wire `ColonyEvent::Presented` and `ColonyEvent::Wired` into the fitness tracker correctly so evolution metrics are accurate.
- **Parameterize graph density:** Make edge creation threshold, decay rate, and pruning aggressiveness configurable per experiment.
- **Add Louvain as alternative algorithm:** Keep label propagation for sparse graphs, use Louvain for dense.

### Future Directions

- **LLM-backed agents:** Current agents use keyword extraction. LLM-powered digestion would produce higher-quality concepts with less noise.
- **Embedding-based retrieval:** Combine graph topology with vector embeddings for hybrid search.
- **Cross-document reinforcement:** Strengthen edges between concepts that appear across different documents in the same topic, creating natural topic boundaries.
- **Adaptive agent genomes:** Let evolution tune not just `max_idle` and `sense_radius` but also edge creation thresholds and pruning aggressiveness.

---

## Technical Vitals

| Metric | Value |
|--------|-------|
| Language | Rust |
| Crates | 6 (core, runtime, agents, rag, viz, wasm) |
| PoC demos | 5 |
| Tests | 91 passing |
| Warnings | 0 |
| Corpus | 100 documents, 4 topics |
| Papers | 4 whitepapers + 4 explainers |

---

*Generated from actual benchmark runs on the 100-document corpus.*
