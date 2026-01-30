# Phago — Product Requirements Document

**Biological Computing Primitives for Emergent Agent Systems**

**Version 0.1 — January 2026**

---

## 1. Vision

Build an open-source framework of ten biological computing primitives — implemented in Rust, compiled to WebAssembly — that enables agent systems to self-organize, self-heal, and exhibit emergent intelligence. Prove it works with a self-organizing knowledge ecosystem as the first proof of concept.

---

## 2. Problem Statement

Current agent orchestration systems (LangGraph, CrewAI, AutoGen, etc.) are top-down: roles are defined, communication patterns are fixed, coordination strategies are specified before runtime. This works for predictable tasks but fails in complex, dynamic, or unknown environments.

Biology solves coordination in complex environments through bottom-up mechanisms that have been refined over 3.8 billion years of evolution. These mechanisms have never been faithfully implemented as reusable computational primitives.

---

## 3. Goals

### 3.1 Primary Goals

1. **Implement ten biological primitives** as composable Rust traits
2. **Compile agents to WASM** for isolation, portability, and runtime lifecycle management
3. **Build a proof-of-concept** demonstrating all ten primitives working together
4. **Visualize emergence** in a real-time browser-based interface
5. **Document transfer patterns** showing how primitives apply across domains

### 3.2 Non-Goals (v0.1)

- Production-grade performance optimization
- Integration with specific LLM providers (future milestone)
- Enterprise deployment tooling
- Mobile native targets

---

## 4. The Ten Primitives — Specifications

### 4.1 DIGEST (Phagocytosis)

**Biological basis**: Macrophage engulfs, destroys, and presents antigen fragments.

**Trait definition**:
```rust
pub trait Digest {
    /// The raw input to consume (ownership transferred — the input is destroyed)
    type Input;
    /// Structural fragments extracted during digestion
    type Fragment;
    /// What is presented to other agents after digestion
    type Presentation;

    /// Engulf: take ownership of input. Input ceases to exist independently.
    fn engulf(&mut self, input: Self::Input) -> DigestionResult;

    /// Lyse: break the engulfed material into fragments
    fn lyse(&mut self) -> Vec<Self::Fragment>;

    /// Present: expose fragments on the agent's "surface" for other agents to read
    fn present(&self) -> Self::Presentation;

    /// Full digestion cycle
    fn digest(&mut self, input: Self::Input) -> Self::Presentation {
        self.engulf(input);
        self.lyse();
        self.present()
    }
}
```

**Key behaviors**:
- Input is moved (consumed), not cloned — Rust's ownership enforces real consumption
- Fragments are stored internally until presented
- Presentations are immutable references — readable by others, not modifiable
- Digestion may fail (malformed input) — this is analogous to indigestible material

**Success criteria**: An agent can consume a data payload, extract structured fragments, and expose them for other agents to read without the original payload being accessible anywhere in the system.

---

### 4.2 APOPTOSE (Programmed Cell Death)

**Biological basis**: Cell evaluates own integrity, initiates orderly self-destruction.

**Trait definition**:
```rust
pub trait Apoptose {
    /// Metrics the agent uses to evaluate its own health
    type HealthMetrics;
    /// Final output released upon death (apoptotic bodies)
    type DeathSignal;

    /// Introspect: evaluate own health and usefulness
    fn self_assess(&self) -> CellHealth;

    /// Decide whether to initiate death
    fn should_die(&self) -> bool {
        matches!(self.self_assess(), CellHealth::Compromised | CellHealth::Redundant)
    }

    /// Orderly shutdown: package final state, release resources
    fn trigger_apoptosis(self) -> Self::DeathSignal;
}

pub enum CellHealth {
    Healthy,
    Stressed,       // Under load but functional
    Compromised,    // Integrity lost — should die
    Redundant,      // Others cover this function — can die
    Senescent,      // Too old / too many cycles — should die
}
```

**Key behaviors**:
- `trigger_apoptosis` takes `self` by value — the agent is consumed (moved) by its own death
- The `Drop` implementation handles resource cleanup
- Death signals carry the agent's final learnings — other agents can digest these
- External kill signals (Fas/FasL analog) can also trigger death via the runtime

**Success criteria**: An agent detects it is stuck or producing no value, self-terminates, releases a death signal containing its final state, and the runtime reclaims its resources without leaks or orphaned state.

---

### 4.3 SENSE (Chemotaxis)

**Biological basis**: Cell detects chemical gradients and navigates toward/away from signals.

**Trait definition**:
```rust
pub trait Sense {
    /// Types of signals this agent can detect
    type SignalFilter;

    /// Read the local signal environment from the substrate
    fn sense(&self, substrate: &Substrate) -> Vec<Signal>;

    /// Compute the gradient — which direction has increasing signal strength
    fn gradient(&self, substrate: &Substrate) -> Vec<Gradient>;

    /// Emit signals into the substrate (for quorum sensing)
    fn emit(&self, signal: Signal, substrate: &mut Substrate);

    /// Determine next action based on sensed environment
    fn orient(&self, gradients: &[Gradient]) -> Orientation;
}
```

**Key behaviors**:
- Agents sense only their local neighborhood, not global state
- Signal strength decays with distance (diffusion model)
- Agents can emit signals (autoinducers for quorum sensing)
- Orientation produces a direction/priority, not a destination

**Success criteria**: Agents navigate toward areas of high signal concentration without any central routing. When a new problem appears in the substrate, nearby agents detect it and converge.

---

### 4.4 TRANSFER (Horizontal Gene Transfer)

**Biological basis**: Bacteria share genetic material across species boundaries.

**Trait definition**:
```rust
pub trait Transfer {
    /// A portable capability (compiled WASM module)
    type Capability;

    /// List capabilities available for export
    fn available_capabilities(&self) -> Vec<CapabilityDescriptor>;

    /// Export a capability as a portable WASM module
    fn export_capability(&self, id: &CapabilityId) -> Option<Self::Capability>;

    /// Evaluate whether to accept a foreign capability
    fn evaluate_foreign(&self, cap: &CapabilityDescriptor) -> Compatibility;

    /// Integrate a foreign capability into this agent
    fn integrate(&mut self, capability: Self::Capability) -> Result<(), RejectionReason>;
}

pub enum Compatibility {
    Compatible,           // Can integrate directly
    NeedsAdaptation,      // Possible with modification
    Incompatible,         // Reject
}
```

**Key behaviors**:
- Capabilities are WASM modules — truly portable across agent types
- Not all transfers succeed — incompatibility is expected (like immune rejection)
- Transfer can happen through direct exchange (conjugation), substrate pickup (transformation), or third-party relay (transduction)
- Integrated capabilities persist across the agent's lifetime

**Success criteria**: Agent A learns to parse a specific data format. Agent A exports this as a WASM module. Agent B imports and integrates it. Agent B can now parse that format without retraining.

---

### 4.5 EMERGE (Quorum Sensing + Phase Transitions)

**Biological basis**: Bacteria detect population density via signaling molecules and undergo collective behavioral shifts at threshold.

**Trait definition**:
```rust
pub trait Emerge {
    /// The collective behavior that emerges at quorum
    type EmergentBehavior;

    /// Current signal density detected
    fn signal_density(&self, substrate: &Substrate) -> f64;

    /// Threshold for phase transition
    fn quorum_threshold(&self) -> f64;

    /// Whether quorum has been reached
    fn quorum_reached(&self, substrate: &Substrate) -> bool {
        self.signal_density(substrate) >= self.quorum_threshold()
    }

    /// Behavior that activates only when quorum is reached
    fn emergent_behavior(&self) -> Option<Self::EmergentBehavior>;

    /// Contribute to collective computation
    fn contribute(&self, collective: &mut Collective) -> Contribution;
}
```

**Key behaviors**:
- Phase transition is discrete, not gradual — behavior qualitatively changes
- Individual agents cannot perform emergent behaviors alone
- The collective computation produces outputs none of the individuals could
- Threshold is adaptive — it can shift based on environment

**Success criteria**: Five agents individually process documents and find nothing notable. When a sixth agent joins and quorum is reached, the collective detects a cross-document pattern that no individual agent identified.

---

### 4.6 WIRE (Hebbian Learning + Synaptic Pruning)

**Biological basis**: Neural connections that fire together strengthen; unused connections are pruned.

**Trait definition**:
```rust
pub trait Wire {
    /// Strengthen the connection between two nodes in the graph
    fn strengthen(&self, from: NodeId, to: NodeId, weight: f64, graph: &mut TopologyGraph);

    /// Record a co-activation event
    fn co_activate(&self, nodes: &[NodeId], graph: &mut TopologyGraph);

    /// Prune connections below threshold
    fn prune(&self, threshold: f64, graph: &mut TopologyGraph) -> Vec<PrunedConnection>;

    /// Decay all connection weights (simulating time-based weakening)
    fn decay(&self, rate: f64, graph: &mut TopologyGraph);
}
```

**Key behaviors**:
- The topology graph is stored in the substrate (shared environment)
- All agents read and modify the same graph — collective wiring
- Pruning is automatic on a decay schedule — not manually triggered
- Connection weights encode collective knowledge — topology IS memory

**Success criteria**: After 100 digestion cycles, the topology graph shows strong connections between genuinely related concepts and has pruned spurious connections — without any explicit knowledge engineering.

---

### 4.7 SYMBIOSE (Endosymbiosis)

**Biological basis**: One cell engulfs another but integrates it instead of digesting it.

**Trait definition**:
```rust
pub trait Symbiose {
    /// Evaluate whether integration is more valuable than digestion
    fn evaluate_for_symbiosis(&self, other: &dyn Agent) -> SymbiosisEval;

    /// Merge another agent into this one as a permanent sub-component
    fn integrate_symbiont(&mut self, other: Box<dyn Agent>) -> Result<(), SymbiosisFailure>;

    /// List current symbionts
    fn symbionts(&self) -> &[SymbiontInfo];

    /// Access a symbiont's capability
    fn delegate_to_symbiont(&self, symbiont_id: &str, input: &[u8]) -> Option<Vec<u8>>;
}

pub enum SymbiosisEval {
    Digest,           // More valuable broken down
    Integrate,        // More valuable as permanent symbiont
    Coexist,          // Leave independent — no action
}
```

**Key behaviors**:
- Symbiosis is a DIGEST that detects high complementary value and pivots
- The symbiont retains its own WASM module (like mitochondria retain their own DNA)
- The host gains the symbiont's capabilities without reimplementing them
- Symbiosis is permanent within the agent's lifetime

**Success criteria**: Agent A (text parser) digests Agent B's output (image analyzer). During digestion, it detects that image analysis is highly complementary. Instead of digesting, Agent A integrates Agent B as a symbiont and can now process both text and images.

---

### 4.8 STIGMERGE (Stigmergy)

**Biological basis**: Termites coordinate construction through environmental modification.

**Trait definition**:
```rust
pub trait Stigmerge {
    /// Deposit a trace in the substrate
    fn deposit(&self, location: SubstrateLocation, trace: Trace, substrate: &mut Substrate);

    /// Read traces at a location
    fn read_traces(&self, location: SubstrateLocation, substrate: &Substrate) -> Vec<Trace>;

    /// Modify behavior based on trace density
    fn respond_to_traces(&self, traces: &[Trace]) -> StigmergicResponse;
}

pub struct Trace {
    pub agent_id: AgentId,
    pub trace_type: TraceType,
    pub intensity: f64,
    pub timestamp: Instant,
    pub payload: Vec<u8>,
}
```

**Key behaviors**:
- Traces evaporate over time (like pheromones) — intensity decays
- High trace density at a location attracts more agents (positive feedback)
- No direct agent-to-agent communication needed — substrate mediates
- The substrate (knowledge graph, shared state) IS the coordination mechanism

**Success criteria**: Agents depositing traces on knowledge graph nodes naturally cluster around important topics. Areas with high trace density attract more agents, producing deeper analysis in important areas without any centralized prioritization.

---

### 4.9 NEGATE (Negative Selection)

**Biological basis**: T-cells that react to self-antigens are destroyed; only non-self-reactive cells survive.

**Trait definition**:
```rust
pub trait Negate {
    /// The self-model (what is normal/expected)
    type SelfModel;

    /// Build or update the self-model from observations
    fn learn_self(&mut self, observations: &[Observation]) -> &Self::SelfModel;

    /// Test whether an input matches self (normal) or non-self (anomalous)
    fn classify(&self, input: &Observation) -> Classification;

    /// Maturation: test candidate detectors against self-model, destroy those that react to self
    fn mature_detectors(&mut self) -> MaturatedSet;
}

pub enum Classification {
    Self_,            // Normal — matches self-model
    NonSelf(f64),     // Anomalous — degree of deviation
    Unknown,          // Cannot classify — insufficient self-model
}
```

**Key behaviors**:
- The system learns what "normal" looks like — a finite, learnable space
- Anything not matching normal is flagged as potentially anomalous
- This is the inverse of pattern matching: detect by exclusion, not inclusion
- The self-model evolves as the system's baseline shifts

**Success criteria**: After observing 1000 normal data points, the system flags genuinely anomalous data without ever being trained on examples of anomalies.

---

### 4.10 DISSOLVE (Holobiont)

**Biological basis**: The organism-environment boundary is a gradient, not a wall.

**Trait definition**:
```rust
pub trait Dissolve {
    /// Current boundary permeability (0.0 = rigid wall, 1.0 = no boundary)
    fn permeability(&self) -> f64;

    /// Adjust boundary based on trust/context
    fn modulate_boundary(&mut self, context: &BoundaryContext);

    /// Expose internal state to substrate (partial dissolution)
    fn externalize(&self, aspect: &str, substrate: &mut Substrate);

    /// Internalize substrate state (partial absorption)
    fn internalize(&mut self, aspect: &str, substrate: &Substrate);
}
```

**Key behaviors**:
- Agents can selectively expose internal state to the substrate
- Agents can absorb substrate state into their internal processing
- The degree of dissolution is dynamic and context-dependent
- At maximum dissolution, the agent IS the substrate — no meaningful boundary

**Success criteria**: In a mature system, the knowledge graph contains agent-deposited insights that other agents have internalized and extended — making it impossible to determine where "agent knowledge" ends and "substrate knowledge" begins.

---

## 5. System Architecture

### 5.1 Crate Structure

```
phago/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── phago-core/               # Primitive traits + shared types
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── primitives/       # The ten traits
│   │   │   │   ├── mod.rs
│   │   │   │   ├── digest.rs
│   │   │   │   ├── apoptose.rs
│   │   │   │   ├── sense.rs
│   │   │   │   ├── transfer.rs
│   │   │   │   ├── emerge.rs
│   │   │   │   ├── wire.rs
│   │   │   │   ├── symbiose.rs
│   │   │   │   ├── stigmerge.rs
│   │   │   │   ├── negate.rs
│   │   │   │   └── dissolve.rs
│   │   │   ├── agent.rs          # Base Agent trait
│   │   │   ├── substrate.rs      # Shared environment
│   │   │   ├── signal.rs         # Signal types
│   │   │   ├── topology.rs       # Graph structures
│   │   │   └── types.rs          # Common types
│   │   └── Cargo.toml
│   │
│   ├── phago-runtime/            # Colony management + lifecycle
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── colony.rs         # Agent lifecycle (birth, death, management)
│   │   │   ├── scheduler.rs      # Tick-based execution scheduler
│   │   │   ├── quorum.rs         # Quorum detection engine
│   │   │   ├── substrate_impl.rs # Substrate implementation
│   │   │   ├── wasm_host.rs      # WASM module loading/management
│   │   │   └── metrics.rs        # Observability
│   │   └── Cargo.toml
│   │
│   ├── phago-wasm/               # WASM agent compilation + bindings
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── bindings.rs       # Host↔Agent interface (wit-bindgen)
│   │   │   └── membrane.rs       # Import/export management
│   │   ├── wit/                  # WIT interface definitions
│   │   │   ├── agent.wit
│   │   │   ├── substrate.wit
│   │   │   └── signal.wit
│   │   └── Cargo.toml
│   │
│   ├── phago-agents/             # Reference agent implementations
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── digester.rs       # Document/data digestion agent
│   │   │   ├── sentinel.rs       # Anomaly detection agent (NEGATE)
│   │   │   ├── connector.rs      # Relationship discovery agent (WIRE)
│   │   │   ├── scout.rs          # Exploration agent (SENSE + STIGMERGE)
│   │   │   └── synthesizer.rs    # Insight generation agent (EMERGE)
│   │   └── Cargo.toml
│   │
│   └── phago-viz/                # Browser visualization
│       ├── src/
│       │   ├── lib.rs
│       │   ├── renderer.rs       # WebGL/Canvas rendering
│       │   ├── layout.rs         # Force-directed graph layout
│       │   └── ui.rs             # Controls and overlays
│       ├── www/                  # Static web assets
│       │   ├── index.html
│       │   └── style.css
│       └── Cargo.toml
│
├── poc/                          # Proof of concept
│   ├── knowledge-ecosystem/      # Primary POC
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── ingest.rs         # Document ingestion
│   │   │   └── domain.rs         # Domain-specific types
│   │   ├── data/                 # Sample datasets
│   │   └── Cargo.toml
│   └── scenarios/                # Demonstration scenarios
│       ├── emergence.rs          # Quorum + phase transition demo
│       ├── self_healing.rs       # Apoptosis + recovery demo
│       ├── capability_spread.rs  # Horizontal transfer demo
│       └── stigmergy_nav.rs      # Environmental navigation demo
│
└── docs/
    ├── WHITEPAPER.md
    ├── PRD.md
    ├── ARCHITECTURE.md
    └── PRIMITIVES.md
```

### 5.2 Technology Stack

| Layer | Technology | Rationale |
|---|---|---|
| **Core primitives** | Rust (stable) | Ownership = biology, zero-cost abstractions, no GC |
| **Agent isolation** | WASM (wasmtime) | Sandboxing, portability, runtime lifecycle |
| **Agent interfaces** | WIT (Component Model) | Standardized cross-module interfaces |
| **Substrate graph** | petgraph + custom | In-memory graph with persistence hooks |
| **Signal field** | Custom spatial hash | Efficient neighbor queries for gradient computation |
| **Visualization** | Rust → WASM + Canvas/WebGL | Native browser rendering from Rust code |
| **Build system** | Cargo workspace | Native Rust tooling |
| **WASM tooling** | wasm-pack, wit-bindgen | Standard WASM ecosystem tools |
| **Testing** | cargo test + property testing (proptest) | Correctness and emergence verification |

### 5.3 Host Environments

The same WASM agents run in multiple host environments:

| Environment | Runtime | Use Case |
|---|---|---|
| **Server** | wasmtime | Production workloads, large colonies |
| **Browser** | Native WASM | Visualization, demos, education |
| **Edge** | Cloudflare Workers / Deno | Distributed colony across edge nodes |
| **CLI** | wasmtime (embedded) | Development, testing, scripting |

---

## 6. Proof of Concept: Self-Organizing Knowledge Ecosystem

### 6.1 Description

The POC demonstrates all ten primitives operating together on a real problem: organizing and surfacing insights from a corpus of documents (research papers, articles, reports).

### 6.2 User Experience

1. **Ingest**: User provides a set of documents (text files, PDFs, markdown)
2. **Observe**: Browser visualization shows agents spawning, sensing, digesting, wiring
3. **Discover**: Knowledge graph builds itself — concepts connect, clusters form, insights emerge
4. **Interact**: User can query the graph, explore connections, trace how insights formed
5. **Evolve**: Add more documents — watch the system adapt, strengthen relevant paths, prune irrelevant ones

### 6.3 Agent Types for POC

| Agent | Primitives Used | Role |
|---|---|---|
| **Digester** | DIGEST, SENSE, STIGMERGE | Consume documents, extract concepts, deposit traces |
| **Connector** | WIRE, SENSE, STIGMERGE | Discover and strengthen concept relationships |
| **Sentinel** | NEGATE, SENSE, APOPTOSE | Detect anomalous or contradictory information |
| **Scout** | SENSE, STIGMERGE, TRANSFER | Explore under-visited areas of the graph |
| **Synthesizer** | EMERGE, DIGEST, DISSOLVE | Generate cross-document insights at quorum |

### 6.4 Success Metrics

| Metric | Target | Measurement |
|---|---|---|
| **Self-organization** | Agents cluster around important topics without direction | Observe cluster formation in visualization |
| **Emergence** | Collective produces insights no individual found | Compare individual vs. collective outputs |
| **Self-healing** | System recovers when agents are killed | Kill agents, measure recovery time |
| **Adaptive wiring** | Graph structure reflects genuine relationships | Compare to human-curated concept map |
| **Capability transfer** | Learned parsing spreads across agents | Introduce new format, measure adoption time |
| **Graceful death** | Stuck agents self-terminate | Monitor APOPTOSE triggers and resource reclamation |

---

## 7. Build Phases

### Phase 0 — Foundation
- Rust workspace setup
- Core trait definitions (all ten primitives)
- Basic types and substrate structure
- Unit tests for trait contracts

### Phase 1 — Cell
- Single agent implementation (all traits)
- WASM compilation pipeline
- Basic host runtime (load, run, terminate agents)
- DIGEST + APOPTOSE working end-to-end

### Phase 2 — Colony
- Multi-agent runtime (colony management)
- Substrate implementation (signal field + graph)
- SENSE + STIGMERGE working (agents navigate substrate)
- WIRE working (graph strengthening/pruning)

### Phase 3 — Emergence
- Quorum detection engine
- EMERGE implementation (phase transitions)
- TRANSFER implementation (capability sharing via WASM modules)
- NEGATE implementation (self-model + anomaly detection)

### Phase 4 — Symbiosis
- SYMBIOSE implementation (agent integration)
- DISSOLVE implementation (boundary modulation)
- All ten primitives operational simultaneously
- Integration tests for emergent behaviors

### Phase 5 — Proof of Concept
- Knowledge ecosystem POC
- Document ingestion pipeline
- All five agent types (Digester, Connector, Sentinel, Scout, Synthesizer)
- Browser visualization

### Phase 6 — Polish and Release
- Documentation
- Performance profiling and optimization
- Example scenarios and tutorials
- Open-source release

---

## 8. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| WASM Component Model immaturity | Medium | High | Fall back to WASI Preview 1 + custom ABI |
| Emergence doesn't actually emerge | Medium | Critical | Start with simple, well-studied emergent systems (ant colony optimization) before attempting novel emergence |
| Performance insufficient for real-time viz | Low | Medium | Reduce agent count, optimize hot paths, use WebGL |
| Complexity spiral | Medium | High | Strict phase gates — each phase must work before proceeding |
| Scope creep | High | Medium | PRD is the contract — no features outside this document for v0.1 |

---

## 9. Design Decisions (Resolved)

1. **LLM Integration**: **No — deterministic algorithms only in v0.1.** The primitives must be the variable under test. If agents use LLMs, we cannot distinguish whether emergence comes from biological primitives or from LLM intelligence. LLM integration becomes a v0.2 amplifier once primitives are proven.

2. **Persistence**: **Substrate only — not individual agents.** The knowledge graph and signal field persist across restarts (serialize to disk). Individual agent state does not persist — agents are ephemeral, like cells. New agents re-orient by SENSING the existing substrate. This is biologically accurate: cells die, the organism's structure persists.

3. **Distributed colonies**: **No — single-host only for v0.1.** Distribution adds network failure modes and consensus overhead unrelated to biological primitives. All ten primitives can be demonstrated with hundreds of WASM agents on a single machine. Distribution becomes meaningful in v0.2+ for true cross-host TRANSFER.

4. **Agent evolution**: **No — post-v0.1.** The ten primitives already cover adaptation (WIRE reshapes structure, TRANSFER shares capabilities, APOPTOSE removes the unfit, EMERGE produces novelty). Genetic algorithm-style mutation requires generational timescales that don't apply to a POC. Evolution deserves its own research phase.

---

## 10. License

Phago will be released under the **MIT License** — maximizing accessibility and adoption for public good.

---

*This PRD is a living document. It will evolve as the project develops.*
