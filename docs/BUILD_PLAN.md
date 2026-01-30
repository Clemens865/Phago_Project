# Phago — Build Plan

**From zero to a living proof of concept.**

---

## Guiding Principles

1. **Each phase produces something that runs.** No phase is "just setup." Every phase ends with a working demo, even if trivial.
2. **Biological fidelity over engineering convenience.** If the biology says ownership transfers, the code uses `move` semantics. If the biology says death is deterministic, we use `Drop`, not GC.
3. **Test emergence, not just correctness.** Unit tests verify trait contracts. Integration tests verify that primitives *interact* to produce behaviors none of them specify individually.
4. **One primitive at a time, then compose.** Each primitive is implemented, tested, and demonstrated in isolation before combining with others.

---

## Phase 0 — Scaffold

**Goal**: Rust workspace compiles, all crate stubs exist, CI runs.

### 0.1 Workspace Setup
```
Action:  Initialize Cargo workspace with all crates
Output:  `cargo build` succeeds across the full workspace
```
- Create `Cargo.toml` workspace root
- Create crate stubs: `phago-core`, `phago-runtime`, `phago-wasm`, `phago-agents`, `phago-viz`
- Create POC crate: `poc/knowledge-ecosystem`
- Each crate has `src/lib.rs` with a module doc comment — nothing else
- `cargo build` and `cargo test` pass (trivially)

### 0.2 Core Types
```
Action:  Define shared types used across all primitives
Output:  Types compile and are importable from any crate
```

In `phago-core/src/types.rs`:
- `AgentId` — unique agent identifier (UUID)
- `NodeId` — knowledge graph node identifier
- `Signal` — type, intensity, position, timestamp
- `Trace` — agent deposit on substrate (type, intensity, timestamp, payload)
- `Gradient` — direction + magnitude
- `CapabilityId` / `CapabilityDescriptor` — for TRANSFER
- `CellHealth` — enum (Healthy, Stressed, Compromised, Redundant, Senescent)

In `phago-core/src/substrate.rs`:
- `Substrate` struct — holds the signal field + knowledge graph + trace storage
- Basic read/write interface (signals, traces, graph nodes/edges)
- No implementation yet — just the API contract

### 0.3 Trait Definitions
```
Action:  Define all ten primitive traits as Rust traits
Output:  All traits compile, are documented, and have associated types
```

In `phago-core/src/primitives/`:
- `digest.rs` — `Digest` trait
- `apoptose.rs` — `Apoptose` trait
- `sense.rs` — `Sense` trait
- `transfer.rs` — `Transfer` trait
- `emerge.rs` — `Emerge` trait
- `wire.rs` — `Wire` trait
- `symbiose.rs` — `Symbiose` trait
- `stigmerge.rs` — `Stigmerge` trait
- `negate.rs` — `Negate` trait
- `dissolve.rs` — `Dissolve` trait

In `phago-core/src/agent.rs`:
- `Agent` trait — requires `Digest + Apoptose + Sense`
- `tick()` method — called each cycle by the runtime

**Phase 0 deliverable**: `cargo doc --open` shows full API documentation for all ten primitives. No implementations yet — just the contracts.

---

## Phase 1 — Cell

**Goal**: A single agent runs as a WASM module, digests input, and can self-terminate.

### 1.1 Substrate Implementation (Minimal)
```
Action:  Implement Substrate with in-memory signal field and graph
Output:  Substrate can store/retrieve signals, traces, and graph nodes
```

In `phago-runtime/src/substrate_impl.rs`:
- `SubstrateImpl` — concrete implementation of `Substrate`
- Signal field: `HashMap<Position, Vec<Signal>>` (simple, upgrade to spatial hash later)
- Knowledge graph: `petgraph::Graph<NodeData, EdgeData>`
- Trace storage: `HashMap<SubstrateLocation, Vec<Trace>>`
- Signal decay function: `decay_signals(rate: f64)` — called each tick
- Substrate serialization to/from disk (JSON or bincode) — for persistence decision

### 1.2 First Agent — The Digester (Native)
```
Action:  Implement a concrete agent that digests text input
Output:  Agent consumes a string, extracts keywords, presents them
```

In `phago-agents/src/digester.rs`:
- Implements `Digest` for text: splits into sentences, extracts keywords (TF-IDF or simpler), produces fragments
- Implements `Apoptose`: tracks cycles without useful output, self-terminates after threshold
- Implements `Sense`: reads signals from substrate (initially just "is there input to digest?")
- Test: feed a paragraph → agent produces keyword fragments → fragments are accessible via `present()`
- Test: feed 50 empty inputs → agent triggers `should_die()` → returns `true`

### 1.3 WASM Compilation Pipeline
```
Action:  Compile the Digester agent to a WASM module
Output:  .wasm file loads and runs in wasmtime
```

In `phago-wasm/`:
- Define WIT interfaces for agent↔host communication
  - `agent.wit`: `tick()`, `engulf()`, `present()`, `self-assess()`
  - `substrate.wit`: `read-signals()`, `deposit-trace()`, `read-graph()`
- Use `wit-bindgen` to generate Rust bindings
- Compile Digester agent to `digester.wasm`
- Verify: `wasmtime digester.wasm` loads without error

### 1.4 Host Runtime (Minimal)
```
Action:  Build a host that loads a WASM agent, feeds it input, reads output
Output:  CLI program that digests a text file and prints extracted concepts
```

In `phago-runtime/src/wasm_host.rs`:
- Load a `.wasm` module via `wasmtime`
- Provide substrate access through WASM imports
- Call `tick()` in a loop
- Feed input via `engulf()`
- Read output via `present()`
- Detect `should_die()` and clean up

In `phago-runtime/src/colony.rs` (minimal):
- `Colony` struct — holds a list of agents + the substrate
- `spawn(wasm_bytes) -> AgentHandle`
- `tick_all()` — calls tick on every agent, checks for apoptosis
- `remove_dead()` — removes agents that triggered apoptosis

**Phase 1 deliverable**: Run `cargo run -p phago-poc -- digest sample.txt` and see extracted concepts printed. Kill the agent by feeding it garbage — it self-terminates.

### Phase 1 Demo Script
```bash
# Digest a single document
echo "The mitochondria is the powerhouse of the cell. ATP is produced
through oxidative phosphorylation in the inner membrane." > sample.txt

cargo run -p phago-poc -- digest sample.txt
# Output: Fragments: [mitochondria, powerhouse, cell, ATP, oxidative,
#         phosphorylation, inner membrane]
# Status: Agent healthy, 7 fragments presented

# Feed garbage until apoptosis triggers
cargo run -p phago-poc -- digest /dev/null --repeat 50
# Output: Agent health: Stressed (cycle 30)
# Output: Agent health: Compromised (cycle 45)
# Output: Agent triggered apoptosis. Death signal: {cycles: 50, useful_output: 0}
```

---

## Phase 2 — Colony

**Goal**: Multiple agents share a substrate, navigate via signals, and build a knowledge graph.

### 2.1 Multi-Agent Scheduling
```
Action:  Colony runs N agents in parallel on shared substrate
Output:  Multiple agents digest different documents concurrently
```

In `phago-runtime/src/scheduler.rs`:
- Tick-based scheduler: each tick, all agents execute in parallel (rayon for CPU parallelism)
- Agents read substrate concurrently (shared read access)
- Agents write to substrate through a collected-writes pattern (no write contention)
- Substrate applies all writes at end of tick (discrete time steps — like cellular biology)

In `phago-runtime/src/colony.rs` (expanded):
- `spawn_n(agent_type, count)` — spawn multiple agents
- `tick()` — one full simulation step (all agents act, substrate updates, decay runs)
- `stats()` — alive count, dead count, fragments presented, graph size

### 2.2 SENSE + Chemotaxis
```
Action:  Agents detect signals and navigate toward unprocessed work
Output:  Agents cluster around documents with highest signal intensity
```

- Substrate emits signals at locations where undigested input exists (like inflammation signals)
- Agents implement `Sense::gradient()` — compute direction of increasing signal
- Agents implement `Sense::orient()` — decide to move toward strongest gradient
- Agents that reach a signal source consume it (DIGEST)
- Test: place 3 documents in substrate at different locations → spawn 10 agents at random positions → observe agents migrate to documents

### 2.3 STIGMERGE — Trace Deposits
```
Action:  Agents deposit traces that influence other agents' behavior
Output:  Frequently-visited substrate locations accumulate traces, attracting more agents
```

- After digesting, agents deposit traces at the substrate location (concept type, importance score)
- Other agents read traces when sensing — high-trace locations indicate important areas
- Trace intensity decays over time (pheromone evaporation)
- Positive feedback loop: important areas get more traces → attract more agents → get processed more deeply
- Test: introduce one important document and two trivial ones → after 100 ticks, important document's location has 10x trace intensity

### 2.4 WIRE — Knowledge Graph Construction
```
Action:  Agents strengthen connections between co-occurring concepts
Output:  After processing a corpus, the graph reflects genuine semantic relationships
```

In `phago-agents/src/connector.rs` — new agent type:
- Connector agents scan recently-presented fragments from nearby Digester agents
- When two fragments co-occur (same document, same digestion cycle), Connector strengthens the edge between them
- `Wire::decay()` runs every N ticks — all edge weights decrease slightly
- `Wire::prune()` removes edges below threshold
- Test: feed 20 biology papers → graph shows strong edges between "cell, membrane, protein" and weak/pruned edges between "cell, economics"

### 2.5 Substrate Persistence
```
Action:  Substrate serializes to disk and restores on restart
Output:  Stop and restart the colony — agents re-orient to existing graph
```

- Serialize `SubstrateImpl` to bincode/JSON on shutdown or periodic checkpoint
- On startup, load existing substrate if file exists
- New agents spawn and SENSE the existing substrate — immediately orient to existing work
- Test: digest 10 documents → stop → restart with fresh agents → verify agents navigate to existing graph nodes and extend (not rebuild) the graph

**Phase 2 deliverable**: Run `cargo run -p phago-poc -- colony ./papers/` — watch 20 agents self-organize around a folder of documents, build a knowledge graph, and persist it.

### Phase 2 Demo Script
```bash
# Start a colony on a folder of documents
cargo run -p phago-poc -- colony ./papers/ --agents 20 --ticks 200

# Output (streaming):
# [tick 001] Spawned 20 agents (15 digesters, 5 connectors)
# [tick 005] Digester-03 sensed signal at papers/biology.txt (intensity: 0.9)
# [tick 008] Digester-03 engulfed biology.txt → 12 fragments
# [tick 010] Connector-01 wired: [cell]--0.8--[membrane]
# [tick 015] Digester-07 sensed trace deposit at papers/biology.txt (intensity: 0.7)
# [tick 040] Digester-12 health: Redundant (area fully digested) → apoptosis
# ...
# [tick 200] Colony stats:
#   Agents alive: 14 (6 died via apoptosis)
#   Graph: 847 nodes, 2,341 edges
#   Strongest cluster: [cell, membrane, protein, transport] (avg weight: 0.83)
#   Substrate saved to .phago/substrate.bin

# Restart with fresh agents — they orient to existing graph
cargo run -p phago-poc -- colony ./papers/ --agents 10 --ticks 50 --resume
# [tick 001] Loaded substrate: 847 nodes, 2,341 edges
# [tick 003] Digester-01 sensed existing graph cluster [cell, membrane...]
# [tick 005] Digester-01 navigating to under-explored region...
```

---

## Phase 3 — Emergence

**Goal**: Collective behaviors that no individual agent possesses. The system becomes more than the sum of its parts.

### 3.1 Quorum Detection Engine
```
Action:  Runtime detects when agent density/signal concentration exceeds threshold
Output:  Phase transitions trigger collective behaviors
```

In `phago-runtime/src/quorum.rs`:
- Monitor signal density across substrate regions
- When density in a region exceeds `QUORUM_THRESHOLD`:
  - Emit a quorum signal (different type — "quorum reached")
  - Agents with `Emerge` trait detect quorum signal and activate emergent behavior
- Quorum threshold is adaptive — adjusts based on colony size

### 3.2 EMERGE — Phase Transitions
```
Action:  Agents that detect quorum produce collective insights
Output:  Cross-document patterns discovered that no single agent found
```

In `phago-agents/src/synthesizer.rs` — new agent type:
- Synthesizer agents are dormant until quorum is detected (biologically: they exist but are inactive)
- At quorum: Synthesizer reads all fragments in the quorum region
- Performs cross-fragment analysis: contradiction detection, pattern matching, gap identification
- Produces emergent insights — new graph nodes typed as "collective insight"
- Deposits high-intensity traces at insight locations → attracts more agents → positive feedback
- Test: 5 papers individually show no contradiction. Collective analysis at quorum detects that Paper A's conclusion contradicts Paper D's methodology. This insight exists in NO individual digestion.

### 3.3 TRANSFER — Capability Sharing
```
Action:  Agent exports a learned capability as WASM; another agent imports it
Output:  A skill learned by one agent spreads to the colony
```

- Digester-A processes markdown files → develops markdown-specific parsing (capability)
- Digester-A exports markdown capability as a WASM module fragment
- Deposits capability in substrate (transformation — like DNA in environment)
- Digester-B encounters a markdown file, lacks capability, picks up exported module
- Digester-B integrates capability → can now parse markdown
- Test: introduce a new file format. Time how long until the first agent learns it, then how long until 50% of agents can handle it. The spread should follow a logistic curve (like real gene transfer).

### 3.4 NEGATE — Anomaly Detection
```
Action:  Sentinel agents build a self-model and flag anomalies
Output:  Novel or contradictory information is automatically surfaced
```

In `phago-agents/src/sentinel.rs` — new agent type:
- During "maturation", Sentinel observes normal digestion cycles and builds a statistical self-model
- Model captures: typical fragment types, concept frequencies, edge weight distributions
- After maturation, Sentinel classifies new inputs as Self (normal) or NonSelf (anomalous)
- Anomalies get flagged with a distinct signal type → attracts Synthesizer agents
- Test: train on 50 biology papers → introduce 1 paper from a different field → Sentinel flags it → system investigates instead of quietly integrating it

**Phase 3 deliverable**: Run the colony on a mixed corpus. Observe quorum-triggered insights, capability spreading, and anomaly detection. No single agent produces these behaviors — they are emergent.

### Phase 3 Demo Script
```bash
cargo run -p phago-poc -- colony ./mixed-corpus/ --agents 30 --ticks 500

# [tick 150] QUORUM reached in region "cellular-biology" (density: 0.87, threshold: 0.75)
# [tick 151] Synthesizer-01 activated — analyzing 47 fragments in quorum region
# [tick 155] EMERGENT INSIGHT: "Paper-07 claims ATP yield of 36, but Paper-12
#            methodology produces 30-32. Contradiction detected across 2 sources."
# [tick 200] Digester-05 exported capability: markdown-parser (234 bytes WASM)
# [tick 210] Digester-11 acquired: markdown-parser via substrate transformation
# [tick 215] Digester-18 acquired: markdown-parser via substrate transformation
# [tick 300] Sentinel-02 flagged: economics-paper.txt (NonSelf, deviation: 0.94)
# [tick 305] Synthesizer-02 investigating anomaly...
# [tick 310] INSIGHT: "economics-paper.txt uses 'cell' in financial context —
#            not biological. Isolated from biology cluster."
```

---

## Phase 4 — Symbiosis

**Goal**: Agents integrate with each other and dissolve boundaries with the substrate. The full organism is alive.

### 4.1 SYMBIOSE — Agent Integration
```
Action:  Two complementary agents merge into a more capable unit
Output:  Merged agent has capabilities of both without reimplementation
```

- During DIGEST, if an agent evaluates another agent's output as highly complementary (SYMBIOSE::evaluate_for_symbiosis returns `Integrate`):
  - The host agent absorbs the other's WASM module as an internal symbiont
  - The symbiont's WASM module runs inside the host's context
  - The host can delegate to the symbiont for specific tasks
- Example: Digester (text) + Digester (structured-data) → Merged agent handles both
- The symbiont retains its own module (like mitochondrial DNA)
- Test: spawn a text digester and a CSV digester. Feed the colony a document with embedded tables. Observe symbiosis forming → merged agent handles both.

### 4.2 DISSOLVE — Boundary Modulation
```
Action:  Agent-substrate boundary becomes permeable
Output:  Mature agents blend with the substrate — their knowledge becomes ambient
```

- Agents implement `Dissolve::permeability()` — starts at 0.0 (rigid boundary)
- As an agent's contributions prove valuable (traces get reinforced by other agents), permeability increases
- At high permeability, agent's internal state becomes readable by the substrate
- At maximum dissolution: agent's knowledge graph contributions are indistinguishable from substrate-native data
- This is the HOLOBIONT endpoint — the system and its components co-constitute each other
- Test: after 500 ticks, attempt to remove a dissolved agent. The knowledge it contributed remains in the substrate. The agent was temporary; its contribution is permanent.

### 4.3 Full Integration Test
```
Action:  All ten primitives running simultaneously
Output:  The colony exhibits self-organization, self-healing, adaptation, and emergence
```

Integration test scenarios:
1. **Self-healing**: Kill 30% of agents mid-run → colony detects gaps (SENSE) → spawns replacements → new agents orient via STIGMERGE → recovery within 50 ticks
2. **Adaptive specialization**: Feed mixed corpus → agents naturally specialize by content type (WIRE + STIGMERGE) → specialization visible in agent behavior patterns
3. **Innovation cascade**: One agent discovers new parsing approach → TRANSFER spreads it → EMERGE produces collective insight using new capability → insight attracts more agents (STIGMERGE) → cascade
4. **Graceful degradation**: Starve the colony (no new input) → agents with no work APOPTOSE → colony shrinks to minimal viable size → substrate persists → new input triggers regrowth

**Phase 4 deliverable**: All ten primitives operational. Integration tests pass. The colony behaves as a living system.

---

## Phase 5 — Visualization

**Goal**: Watch the colony live in a browser. See agents move, eat, die, merge, wire, emerge.

### 5.1 Rendering Engine
```
Action:  Browser-based real-time visualization of colony state
Output:  Interactive canvas showing agents, substrate, graph, signals
```

In `phago-viz/`:
- Compile visualization code to WASM (runs in browser)
- Canvas/WebGL renderer
- Force-directed graph layout for knowledge graph
- Visual elements:
  - **Agents**: Circles with color = type, size = health, border = permeability
  - **Signals**: Gradient overlays on substrate (heatmap)
  - **Traces**: Fading dots at deposit locations
  - **Graph edges**: Lines with thickness = weight, opacity = recency
  - **Quorum zones**: Highlighted regions when density exceeds threshold
  - **Apoptosis**: Fade-out animation when agent dies
  - **Symbiosis**: Two circles merging animation
  - **Emergence**: Pulse effect when collective insight is generated

### 5.2 Runtime Bridge
```
Action:  Connect running colony to browser visualization
Output:  Live-updating display of colony state via WebSocket
```

- Colony runtime exposes state via WebSocket (or shared memory if same WASM context)
- State snapshots sent each tick: agent positions, health, graph delta, signals, events
- Browser receives and renders
- Controls: pause, speed up/slow down, inspect agent, inject input, kill agent

### 5.3 Interactive Features
```
Action:  User can interact with the colony
Output:  Drop documents, kill agents, create signals, query the graph
```

- Drag-and-drop document onto substrate → triggers SENSE cascade
- Click agent → inspect panel (health, fragments, symbionts, capabilities)
- Click graph node → trace provenance (which agents contributed, from which documents)
- Right-click agent → force apoptosis (external kill signal)
- Search box → query knowledge graph → highlight paths

**Phase 5 deliverable**: Open `http://localhost:3000` — see a living colony processing documents, building knowledge, self-organizing.

---

## Phase 6 — POC: Knowledge Ecosystem

**Goal**: End-to-end demonstration on a real dataset. The thesis is proven or falsified.

### 6.1 Dataset Preparation
```
Action:  Curate a corpus that will test emergence
Output:  50-100 documents with known cross-document patterns
```

- Select a domain (e.g., biology + adjacent fields)
- Include documents with:
  - Obvious internal patterns (should be easy for individual agents)
  - Cross-document patterns (require EMERGE to detect)
  - Contradictions (require NEGATE to flag)
  - Mixed formats (require TRANSFER to handle)
  - Anomalies (out-of-domain documents to test NEGATE)
- Create a ground-truth concept map for comparison

### 6.2 End-to-End Run
```
Action:  Run the full colony on the curated corpus
Output:  Measure all success metrics from the PRD
```

- Spawn colony with 30-50 agents (mixed types)
- Run for 1000+ ticks
- Measure:
  - Self-organization: do clusters match ground truth?
  - Emergence: do collective insights exist that no individual found?
  - Self-healing: kill agents → measure recovery
  - Adaptive wiring: compare graph to human concept map
  - Capability transfer: introduce new format → measure adoption curve
  - Graceful death: count self-terminations vs. external kills

### 6.3 Documentation and Release
```
Action:  Document results, write tutorials, open-source
Output:  Public repository with documentation, demos, and tutorials
```

- Results document: what emerged, what didn't, what surprised us
- Architecture guide: how to build agents with Phago primitives
- Tutorial: build your first Phago agent
- Transfer guide: how to apply primitives to other domains (from the whitepaper table)
- README with visualization screenshots/GIFs
- MIT license
- GitHub release

**Phase 6 deliverable**: Public repository. Working demo. Thesis tested against reality.

---

## Dependency Graph

```
Phase 0 ─── Scaffold
  │
  ├── 0.1 Workspace ──┐
  ├── 0.2 Core Types ──┼── all required for Phase 1
  └── 0.3 Traits ──────┘
                        │
Phase 1 ─── Cell ───────┘
  │
  ├── 1.1 Substrate (minimal) ──┐
  ├── 1.2 Digester Agent ───────┤
  ├── 1.3 WASM Pipeline ────────┼── all required for Phase 2
  └── 1.4 Host Runtime ─────────┘
                                 │
Phase 2 ─── Colony ──────────────┘
  │
  ├── 2.1 Multi-Agent Scheduler ──┐
  ├── 2.2 SENSE + Chemotaxis ─────┤
  ├── 2.3 STIGMERGE ──────────────┼── all required for Phase 3
  ├── 2.4 WIRE ───────────────────┤
  └── 2.5 Persistence ────────────┘
                                   │
Phase 3 ─── Emergence ────────────┘
  │
  ├── 3.1 Quorum Engine ───┐
  ├── 3.2 EMERGE ───────────┤
  ├── 3.3 TRANSFER ─────────┼── all required for Phase 4
  └── 3.4 NEGATE ───────────┘
                             │
Phase 4 ─── Symbiosis ──────┘
  │
  ├── 4.1 SYMBIOSE ──────────┐
  ├── 4.2 DISSOLVE ──────────┼── required for Phase 5/6
  └── 4.3 Integration Tests ─┘
                               │
Phase 5 ─── Visualization ────┘ (can start in parallel with Phase 3)
  │
  ├── 5.1 Renderer ─────┐
  ├── 5.2 Runtime Bridge ┼── required for Phase 6
  └── 5.3 Interactivity ─┘
                           │
Phase 6 ─── POC ──────────┘
  │
  ├── 6.1 Dataset
  ├── 6.2 End-to-End Run
  └── 6.3 Release
```

**Note**: Phase 5 (Visualization) can begin in parallel with Phase 3 since it only needs the colony runtime from Phase 2. This is the one parallelism opportunity in the build.

---

## Definition of Done — Per Phase

| Phase | It's done when... |
|---|---|
| **0 — Scaffold** | `cargo build` passes, `cargo doc` shows all ten traits, CI green |
| **1 — Cell** | Single WASM agent digests text and self-terminates on failure |
| **2 — Colony** | 20 agents self-organize around documents, build a persisted knowledge graph |
| **3 — Emergence** | Colony produces a cross-document insight that no individual agent found |
| **4 — Symbiosis** | Two agents merge, colony self-heals after 30% agent loss |
| **5 — Visualization** | Browser shows live colony with interactive inspection |
| **6 — POC** | End-to-end run on real corpus, results documented, repo public |

---

## Risk Checkpoints

After each phase, evaluate:

1. **Is the biology holding up?** Are the primitives producing biologically plausible behavior, or are we forcing it?
2. **Is emergence real?** Is the collective doing something the individuals can't, or is it just parallel processing?
3. **Is WASM the right call?** Is the sandbox boundary adding value, or is it just overhead? (Escape hatch: native agents with process isolation instead)
4. **Is the scope right?** Are we building what the PRD says, or drifting?

If any answer is "no" at a checkpoint, stop and reassess before proceeding.

---

*This plan is sequential by necessity — each phase depends on the previous. The one exception is Phase 5 (Visualization), which can begin after Phase 2. Everything else is strictly ordered: you can't have a colony without a cell, and you can't have emergence without a colony.*
