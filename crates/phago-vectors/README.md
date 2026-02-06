# phago-vectors

External vector database adapters for Phago biological computing framework.

Provides trait-based integration with popular vector databases for embedding storage and similarity search alongside Phago's Hebbian knowledge graph.

## Supported Backends

- **Qdrant** (feature: `qdrant`) - Local or cloud, high-performance similarity search
- **Pinecone** (feature: `pinecone`) - Serverless, fully managed, global scale
- **Weaviate** (feature: `weaviate`) - Graph-native vector search

## Usage

```toml
[dependencies]
phago-vectors = { version = "1.0", features = ["qdrant"] }
```

```rust
use phago_vectors::VectorStore;
```

See the [main project](https://github.com/Clemens865/Phago_Project) for full documentation.
