# Phago Python Bindings

Python bindings for the Phago biological computing framework.

## Installation

```bash
pip install phago
```

For framework integrations:
```bash
pip install 'phago[langchain]'  # LangChain support
pip install 'phago[llamaindex]'  # LlamaIndex support
```

## Quick Start

```python
from phago import Colony, Position

# Create a colony
colony = Colony()

# Ingest documents
colony.ingest_document("Biology 101", "The cell membrane controls transport.")
colony.ingest_document("Proteins", "Proteins are molecular machines.")

# Run simulation
colony.run(50)

# Query the knowledge graph
results = colony.query("cell membrane")
for r in results:
    print(f"{r.label}: {r.score:.3f}")

# Get statistics
stats = colony.stats()
print(f"Nodes: {stats.graph_nodes}, Edges: {stats.graph_edges}")
```

## LangChain Integration

```python
from langchain.chains import ConversationChain
from langchain_openai import ChatOpenAI
from phago.langchain import PhagoMemory

# Create memory backed by Phago
memory = PhagoMemory()

# Use with any LangChain chain
llm = ChatOpenAI()
chain = ConversationChain(llm=llm, memory=memory)

# Memory grows organically with each conversation
response = chain.run("Tell me about cells")
```

## LlamaIndex Integration

```python
from phago.llamaindex import PhagoKnowledgeStore

# Create knowledge store
store = PhagoKnowledgeStore()

# Add documents
from llama_index.core.schema import Document
docs = [
    Document(text="Cells are the basic unit of life"),
    Document(text="DNA encodes genetic information"),
]
store.add_documents(docs)

# Query
results = store.query("cells")
```

## API Reference

### Colony

The main class for biological knowledge graph operations.

- `Colony(config=None)` - Create a new colony
- `ingest_document(title, content, position=None)` - Add a document
- `run(ticks)` - Run simulation for N ticks
- `query(query, alpha=0.5, max_results=10)` - Query the graph
- `stats()` - Get colony statistics
- `snapshot_json()` - Get full snapshot as JSON

### ColonyConfig

Configuration for colony parameters.

- `signal_decay_rate` - Rate of signal decay
- `trace_decay_rate` - Rate of trace decay
- `edge_decay_rate` - Rate of edge weight decay
- `edge_prune_threshold` - Threshold for edge pruning
- `staleness_factor` - Factor for staleness-based decay
- `maturation_ticks` - Ticks before edges mature
- `max_edge_degree` - Maximum edges per node

## Building from Source

Requires Rust and maturin:

```bash
cd crates/phago-python
pip install maturin
maturin develop
```

## License

MIT
