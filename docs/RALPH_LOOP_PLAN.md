# Ralph Loop Iteration Plan -- Phago Revolutionary Build

## Mission

Transform Phago from a research prototype (91 tests, 2 failing) into a revolutionary biological memory system across multiple product surfaces, through iterative build-test-improve cycles.

## Parallel Product Tracks

Based on the Competitive Analysis and Strategic Synthesis, we pursue **three product tracks simultaneously**, because each track validates a different winning dimension and shares the same core improvements:

### Track A: Agentic Swarm Memory (PhagoFS)
**Winning dimension:** Evolutionary self-healing + session persistence
**Target:** MCP adapter for AI agent swarms (`phago_remember` / `phago_recall`)
**Validation metric:** Agent swarm performance over 100+ sessions improves vs degrades

### Track B: Codebase Intelligence Layer
**Winning dimension:** Structural queries + anomaly detection
**Target:** Code knowledge graph with relational queries (path, bridge, fragility)
**Validation metric:** Hub detection accuracy, dead code identification vs manual analysis

### Track C: Living RAG Engine
**Winning dimension:** MRR advantage + Hebbian reinforcement (after sparse graph fix)
**Target:** Self-reinforcing retrieval that demonstrably improves with use
**Validation metric:** MRR improvement over 10 query rounds, P@5 gap closure vs TF-IDF

## Iteration Phases

### Phase 0: Fix Foundations (CRITICAL)
All three tracks depend on these fixes.

#### 0.1 Fix Failing Tests
- `colony::tests::agent_finds_and_digests_document` fails because aggressive decay (0.05/tick) kills edges at 0.15 init weight before test assertion
- `session::tests::save_load_roundtrip` fails for same root cause
- **Fix:** Increase maturation grace period so young edges survive initial ticks, OR adjust test to run more ticks / use multi-doc setup

#### 0.2 Co-activation Gating
- Currently: single co-occurrence within one document creates an edge
- **Fix:** Track co-occurrence count. Only create edge after N co-occurrences (default 2, evolvable)
- **Impact:** Massive edge count reduction. 100 docs with 20 terms each: from ~190 edges/doc to ~30 edges/doc (only terms appearing together in 2+ docs get wired)

#### 0.3 Fix Fitness Tracker Wiring
- `FitnessTracker.record_concepts()` and `record_edges()` never called with nonzero values
- Wire `ColonyEvent::Presented` and `ColonyEvent::Wired` events to the tracker
- **Impact:** Evolution operates on real contribution data instead of blind `max_idle` timeouts

#### 0.4 Verify Synaptic Pruning
- Run `synaptic_pruning.rs` integration test
- Verify edge count reduction targets (expect ~50% reduction)
- Re-run Bio-RAG benchmark, record new MRR / P@5 numbers

**Exit criteria:** All tests pass. Fitness tracker reports nonzero values. Edge count for 100-doc corpus drops to <50k (from 255k).

### Phase 1: Amplify Evolution (Track A + C)

#### 1.1 Genome Expansion
Add to `AgentGenome`:
- `edge_creation_threshold: u32` (range 1-5) -- co-occurrences required
- `decay_aggressiveness: f64` (range 0.5-3.0) -- personal decay multiplier
- `reinforcement_strength: f64` (range 0.01-0.3) -- weight boost per co-activation

#### 1.2 Multi-Objective Fitness
New fitness function components:
- Edge survival rate (fraction surviving pruning)
- Bridge creation bonus (connecting disconnected clusters)
- Redundancy penalty (duplicating existing strong edges)

#### 1.3 Sexual Recombination
Crossover between two fit parents' genomes. Add `CrossoverSpawnPolicy` alongside existing `FitnessSpawnPolicy`.

#### 1.4 Benchmark Evolution Improvements
Re-run agent-evolution-demo with expanded genome. Compare:
- Edge growth rate (baseline: 11.6x)
- Graph density trajectory
- Genome diversity (population shouldn't converge to single phenotype)
- Fitness distribution (should see specialists emerge)

**Exit criteria:** Evolved populations maintain or exceed 11.6x advantage. Genome diversity > 0.3 (normalized distance between agents). At least 2 distinct fitness strategies emerge.

### Phase 2: Structural Query Engine (Track B)

#### 2.1 Path Queries
Shortest weighted path between two concepts. Dijkstra on edge weights (invert weight: strongest = shortest).

#### 2.2 Bridge Detection
Betweenness centrality for identifying cross-cluster connector concepts.

#### 2.3 Hub / Centrality Queries
Weighted PageRank on subgraph. Identify the most important concepts.

#### 2.4 Fragility Analysis
Remove node, measure connectivity change. Quantifies structural importance.

#### 2.5 Code Knowledge Graph Enhancement
Extend `code_digester.rs` to extract:
- Call relationships (fn A calls fn B -> directed edge)
- Type usage (struct A used in fn B -> edge)
- Module hierarchy (mod A contains fn B -> hierarchical edge)

#### 2.6 Benchmark Structural Queries
Run on the Phago codebase itself:
- Do hub detection results match the actual key abstractions?
- Does fragility analysis identify the real load-bearing types?
- Does bridge detection find cross-crate connector types?

**Exit criteria:** Path queries return correct shortest paths. Hub detection identifies Colony, TopologyGraph, and AgentGenome as top-3 hubs. Bridge detection finds cross-crate types.

### Phase 3: Product Surfaces (All Tracks)

#### 3.1 MCP Adapter (Track A: PhagoFS)
MCP server exposing:
- `phago_remember(context, content)` -- Digester spawns, processes, wires
- `phago_recall(query)` -- graph query with path traces
- `phago_explore(concept)` -- neighborhood / structural context
- `phago_anomaly(content)` -- Sentinel classification
- `phago_status()` -- colony metrics
- `phago_evolve()` -- trigger evolution cycle
- `phago_save()` / `phago_load()` -- session persistence

#### 3.2 Full Session Persistence (Track A + C)
Extend `session.rs`:
- Serialize `FitnessTracker`, `AgentGenome`, generation counter
- Graph diffing between saves
- Session metadata (corpus stats, evolution stats)

#### 3.3 Hybrid Scoring (Track C: Living RAG)
Combine graph traversal with TF-IDF:
- TF-IDF for candidate generation (fast, high precision)
- Graph re-ranking (structural signal, path diversity, co-activation count)
- Measure: P@5 should approach 0.5+ (from current 0.270)

#### 3.4 Enriched Explanations (All Tracks)
Query results include:
- Weighted path traces with co-activation counts
- Confidence score from path diversity
- Temporal context (when edge was created, last reinforced)

**Exit criteria:** MCP adapter responds to all 7 tool calls. Hybrid scoring P@5 > 0.45. Session persistence round-trips with full evolutionary state.

### Phase 4: Temporal Intelligence (Track A + B)

#### 4.1 Directed Edge Type
Add `EdgeKind` enum: `CoOccurrence` (undirected) | `Temporal` (directed).
Modify `PetTopologyGraph` to support mixed directed/undirected edges.

#### 4.2 STDP for Agent Actions
When agent performs action A then action B within N ticks:
- Create directed edge A -> B with weight proportional to temporal proximity
- Enables "what usually comes next?" queries

#### 4.3 Predictive Context
Given current context, follow directed edges forward to pre-fetch likely next steps.
In MCP adapter: `phago_predict(current_context)` returns anticipated next needs.

**Exit criteria:** Directed temporal edges created from agent action sequences. Predict query returns valid next-step suggestions.

## Success Metrics Per Track

### Track A: Agentic Swarm Memory
| Metric | Baseline | Target |
|--------|----------|--------|
| MCP tool response time | N/A | <100ms |
| Session persistence fidelity | 100% (graph only) | 100% (graph + evolution) |
| Knowledge improvement over sessions | unmeasured | measurable MRR increase |
| Agent swarm task success rate | unmeasured | +15% vs no-Phago baseline |

### Track B: Codebase Intelligence
| Metric | Baseline | Target |
|--------|----------|--------|
| Hub detection accuracy | N/A | Top-3 match manual assessment |
| Dead code detection | N/A | Flag functions with 0-weight edges |
| Path query correctness | N/A | 100% shortest-path verified |
| Bridge detection | N/A | Cross-crate types identified |

### Track C: Living RAG
| Metric | Baseline | Target |
|--------|----------|--------|
| P@5 | 0.270 | >0.45 (hybrid scoring) |
| MRR | 0.714 | >0.80 |
| Reinforcement effect | 0.000 (identical to static) | measurable improvement per round |
| Edge count (100 docs) | 255,888 | <50,000 |

## Ralph Loop Configuration

Each iteration cycle:
1. **Pick the next incomplete item** from the current phase
2. **Implement** the change (code)
3. **Test** -- run `cargo test --workspace`, verify no regressions
4. **Benchmark** -- run relevant POC demo, record metrics
5. **Compare** -- metrics vs baseline and vs targets
6. **Decide** -- if target met, move to next item. If not, iterate on current item.
7. **Persist** -- commit passing state, update metrics log

Phase 0 items are blocking. Phases 1-4 can interleave across tracks.
