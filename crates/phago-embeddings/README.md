# phago-embeddings

Embedding backends for Phago semantic intelligence.

## Overview

This crate provides vector embedding support for Phago:

- **SimpleEmbedder** — Hash-based embeddings (no external dependencies)
- **OnnxEmbedder** — Local ONNX runtime embeddings (optional `local` feature)
- **ApiEmbedder** — API-based embeddings (OpenAI, Voyage, Cohere) (optional `api` feature)
- **Chunker** — Document chunking with configurable overlap
- **Similarity functions** — cosine, euclidean, dot product

## Usage

```rust
use phago_embeddings::{SimpleEmbedder, Embedder, cosine_similarity};

// Create a simple hash-based embedder
let embedder = SimpleEmbedder::new(256);

// Generate embeddings
let v1 = embedder.embed("cell membrane").unwrap();
let v2 = embedder.embed("phospholipid bilayer").unwrap();

// Compute similarity
let similarity = cosine_similarity(&v1, &v2).unwrap();
```

## Features

| Feature | Description |
|---------|-------------|
| `local` | ONNX runtime for local embeddings |
| `api` | HTTP client for API embeddings |
| `full` | All backends |

## Part of Phago

This is a subcrate of [phago](https://crates.io/crates/phago). For most use cases, depend on the main `phago` crate with the `semantic` feature instead.

## License

MIT
