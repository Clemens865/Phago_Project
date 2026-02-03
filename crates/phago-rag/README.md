# phago-rag

Biological RAG — query interface on self-organizing Hebbian knowledge graph.

## Overview

This crate provides retrieval-augmented generation capabilities:

- **Hybrid Query**: Combines TF-IDF with graph-based re-ranking
- **Structural Queries**: Shortest path, centrality, bridges, components
- **MCP Adapter**: Model Context Protocol integration for external LLMs
- **Scoring**: Precision, MRR, NDCG metrics

## Benchmark Results

| Method | P@5 | MRR | NDCG@10 |
|--------|-----|-----|---------|
| Graph-only | 0.280 | 0.650 | 0.357 |
| TF-IDF | 0.742 | 0.775 | 0.404 |
| **Hybrid** | **0.742** | **0.800** | **0.410** |

The graph's value is in re-ranking TF-IDF candidates using structural context.

## Usage

```rust
use phago_rag::prelude::*;

// Hybrid query (TF-IDF + graph re-ranking)
let results = hybrid_query(&colony, "membrane transport", &HybridConfig {
    alpha: 0.5,              // 50% TF-IDF, 50% graph
    max_results: 10,
    candidate_multiplier: 3,
});

for r in results {
    println!("{}: {:.3}", r.label, r.final_score);
}
```

## MCP Integration

```rust
use phago_rag::mcp::*;

// Remember a document
phago_remember(&mut colony, &RememberRequest {
    title: "Doc".into(),
    content: "Content".into(),
    ticks: Some(15),
});

// Recall with hybrid scoring
let results = phago_recall(&colony, &RecallRequest {
    query: "search".into(),
    max_results: 5,
    alpha: 0.5,
});

// Explore graph structure
let stats = phago_explore(&colony, &ExploreRequest::Stats);
```

## Part of Phago

This is a subcrate of [phago](https://crates.io/crates/phago). For most use cases, depend on the main `phago` crate instead.

## License

MIT
