# phago-agents

Reference agent implementations using Phago biological primitives.

## Agent Types

### Digester
Consumes documents, extracts keywords, presents concepts to the knowledge graph.
- Implements: DIGEST, SENSE, APOPTOSE, TRANSFER, SYMBIOSE, DISSOLVE

### Synthesizer
Dormant until quorum reached, then identifies bridge concepts and topic clusters.
- Implements: EMERGE, SENSE, APOPTOSE

### Sentinel
Learns what "normal" looks like, flags anomalies by deviation from self-model.
- Implements: NEGATE, SENSE, APOPTOSE

## Usage

```rust
use phago_agents::prelude::*;
use phago_core::types::Position;

// Create a digester agent
let digester = Digester::new(Position::new(0.0, 0.0))
    .with_max_idle(50);

// Create with deterministic seed for testing
let seeded = Digester::with_seed(Position::new(0.0, 0.0), 42);
```

## Evolution

Agents include genome-based evolution with multi-objective fitness:
- 30% Productivity (concepts + edges per tick)
- 30% Novelty (novel concepts / total)
- 20% Quality (strong edges / total)
- 20% Connectivity (bridge edges / total)

## Part of Phago

This is a subcrate of [phago](https://crates.io/crates/phago). For most use cases, depend on the main `phago` crate instead.

## License

MIT
