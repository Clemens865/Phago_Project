# phago-runtime

Colony management, scheduling, and runtime for Phago biological computing.

## Overview

This crate provides the execution environment for Phago agents:

- **Colony**: Main entry point managing agents, documents, and knowledge graph
- **Substrate**: Shared environment with signals, traces, and documents
- **Topology**: Graph implementation with Hebbian wiring and synaptic pruning
- **Session**: Save/restore colony state across sessions
- **Metrics**: Quantitative measurement of colony behavior

## Usage

```rust
use phago_runtime::prelude::*;
use phago_agents::digester::Digester;
use phago_core::types::Position;

// Create a colony
let mut colony = Colony::new();

// Ingest documents
colony.ingest_document("Biology", "Cell membrane transport", Position::new(0.0, 0.0));

// Spawn agents
colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0))));

// Run simulation
colony.run(50);

// Check stats
let stats = colony.stats();
println!("Nodes: {}, Edges: {}", stats.graph_nodes, stats.graph_edges);
```

## Colony Lifecycle (per tick)

1. **Sense** — Agents observe substrate (signals, documents, traces)
2. **Act** — Colony processes agent actions (move, digest, present, wire)
3. **Transfer** — Agents export/integrate vocabulary, attempt symbiosis
4. **Dissolve** — Mature agents modulate boundaries, reinforce graph nodes
5. **Death** — Remove agents that self-assessed for termination
6. **Decay** — Signals, traces, and edge weights decay; weak edges pruned

## Part of Phago

This is a subcrate of [phago](https://crates.io/crates/phago). For most use cases, depend on the main `phago` crate instead.

## License

MIT
