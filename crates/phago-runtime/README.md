# phago-runtime

Colony management, scheduling, and runtime for Phago biological computing.

## Overview

This crate provides the execution environment for Phago agents:

- **Colony**: Main entry point managing agents, documents, and knowledge graph
- **Substrate**: Shared environment with signals, traces, and documents
- **Topology**: Graph implementation with Hebbian wiring and synaptic pruning
- **Session**: Save/restore colony state across sessions
- **Metrics**: Quantitative measurement of colony behavior
- **ColonyBuilder**: Builder pattern for colonies with optional SQLite persistence
- **AsyncColony**: Async runtime for concurrent and timed simulation

## Feature Flags

| Feature | Description |
|---------|-------------|
| `sqlite` | SQLite-backed persistence via `ColonyBuilder` |
| `async` | Async runtime with `AsyncColony`, `TickTimer`, `run_in_local` |

Enable features:
```toml
phago-runtime = { version = "0.1", features = ["sqlite", "async"] }
```

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

### With SQLite Persistence

```rust
use phago_runtime::prelude::*;
use phago_agents::digester::Digester;

// Create colony with SQLite persistence
let mut colony = ColonyBuilder::new()
    .with_persistence("knowledge.db")
    .auto_save(true)  // Save on drop
    .build()?;

// Use as normal
colony.ingest_document("Bio", "Cell transport", Position::new(0.0, 0.0));
colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0))));
colony.run(100);
colony.save()?;  // Explicit save

// Later: reload from database
let colony2 = ColonyBuilder::new()
    .with_persistence("knowledge.db")
    .build()?;
```

### With Async Runtime

```rust
use phago_runtime::prelude::*;
use phago_runtime::async_runtime::{run_in_local, TickTimer};

#[tokio::main]
async fn main() {
    let colony = Colony::new();

    // Run async simulation
    let events = run_in_local(colony, |ac| async move {
        ac.run_async(50).await
    }).await;

    // Or with controlled tick rate
    let colony2 = Colony::new();
    run_in_local(colony2, |ac| async move {
        let mut timer = TickTimer::new(100);  // 100ms per tick
        timer.run_timed(&ac, 50).await;
    }).await;
}
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
