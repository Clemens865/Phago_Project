# phago-core

Core traits and types for Phago biological computing primitives.

## Overview

This crate provides the foundational abstractions for the Phago framework:

- **10 Biological Primitive Traits**: Digest, Apoptose, Sense, Transfer, Emerge, Wire, Symbiose, Stigmerge, Negate, Dissolve
- **Core Types**: AgentId, NodeId, DocumentId, Position, Signal, Trace, NodeData, EdgeData
- **Substrate Trait**: Environment abstraction for agent interactions
- **TopologyGraph Trait**: Structural query interface for knowledge graphs
- **Error Types**: Structured error handling with `PhagoError`

## Usage

```rust
use phago_core::prelude::*;

// Access all core types and traits
let pos = Position::new(1.0, 2.0);
let agent_id = AgentId::new();
```

## Part of Phago

This is a subcrate of [phago](https://crates.io/crates/phago). For most use cases, depend on the main `phago` crate instead:

```toml
[dependencies]
phago = "0.1"
```

## License

MIT
