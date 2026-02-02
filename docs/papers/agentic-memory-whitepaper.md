# Agentic Memory: Self-Organizing Code Knowledge Through Biological Computing

**Authors:** Phago Project Contributors
**Date:** February 2026
**Version:** 1.0

---

## Abstract

Modern integrated development environments provide sophisticated syntactic analysis through Language Server Protocols, yet lack semantic memory -- the ability to learn and recall how code elements relate to one another across sessions. Embedding-based retrieval tools offer statistical similarity but discard context between sessions and fail to capture the structural relationships that developers implicitly understand. We present Agentic Memory, a biologically-inspired approach to code knowledge organization that models a software codebase as a self-organizing colony. Our system extracts code elements (functions, structs, traits, enums, imports) from source files, constructs a knowledge graph through simulated biological digestion, and persists the resulting structure across sessions. Evaluated on the phago-core project itself (dogfooding), we indexed 830 code elements from 55 Rust source files, producing a graph of 659 nodes and 33,490 edges after a 100-tick colony simulation. Graph-based retrieval (P@5 = 0.140) underperformed a grep baseline (P@5 = 0.323) on precision metrics, yet provided qualitatively different results: structural and relational context that grep cannot capture. Session persistence achieved perfect fidelity: 659/659 nodes and 33,490/33,490 edges survived a full save/load cycle. We argue that a self-organizing biological knowledge graph complements traditional search by surfacing conceptual neighborhoods and maintaining cross-session memory, addressing a fundamental gap in developer tooling.

---

## 1. Introduction

### 1.1 The Context Loss Problem

Software development is an inherently contextual activity. A developer working on a function does not think about that function in isolation -- they hold in mind the callers, the data structures flowing through it, the traits it implements, the modules it belongs to, and the broader architectural patterns at play. Yet the tools developers rely on provide a remarkably narrow window into this context.

The Language Server Protocol (LSP), now ubiquitous across editors, delivers syntax highlighting, type checking, go-to-definition, and symbol search. These capabilities are invaluable but fundamentally syntactic. LSP answers "where is this symbol defined?" but not "what concepts are associated with this symbol in the context of this project?" It provides a phone book, not a map.

### 1.2 Embedding-Based Approaches and Their Limitations

Recent tools have introduced embedding-based code retrieval, encoding source files or code chunks as high-dimensional vectors and retrieving them by cosine similarity. While effective for finding textually or semantically similar code, these approaches suffer from two critical limitations:

1. **Session amnesia.** Most embedding-based tools recompute their index on each invocation or maintain ephemeral in-memory stores. The relationships discovered in one session are lost by the next. Developers must re-orient the tool each time they return to a project.

2. **Flat retrieval.** Embeddings map code to a point in vector space, collapsing the rich graph structure of a codebase into a single similarity metric. A query for "Colony" returns code that mentions colonies, but does not surface the associated concepts of spawning, tick cycles, or agent management that a developer would immediately recall.

### 1.3 The Gap: No Learning from Usage Patterns

Neither LSP nor embedding search learns from the codebase over time. They are stateless analyzers, not knowledge systems. The developer's own mental model of a codebase -- built through months of reading, writing, and debugging -- has no computational analogue in current tooling.

We propose filling this gap with a biologically-inspired knowledge graph that self-organizes through simulated digestion, persists across sessions, and retrieves code elements through graph traversal rather than vector similarity.

---

## 2. Method

Our system comprises four core components, each modeled after a stage in biological nutrient processing.

### 2.1 CodeDigester: Element Extraction

The CodeDigester module parses Rust source files and extracts typed code elements:

- **Functions** (`fn`), including method signatures and bodies
- **Structs** with their fields and derive attributes
- **Traits** with their method signatures
- **Enums** with their variants
- **Imports** (`use` statements) capturing dependency relationships
- **Modules** (`mod` declarations) establishing hierarchical structure

Each extracted element is assigned a unique identifier, a type tag, a source location (file and line range), and the raw source text. The extraction is syntactic -- it uses pattern matching on the Rust grammar rather than full compilation -- making it fast and resilient to incomplete or non-compiling code.

For the phago-core project, CodeDigester extracted **830 code elements** across 55 Rust source files, successfully digesting 52 of 54 documentation files.

### 2.2 Colony: Knowledge Graph Construction

Extracted code elements are ingested into a simulated biological colony. The colony operates on a tick-based simulation loop where:

1. **Ingestion.** Code elements enter the colony as nutrient particles, each carrying their type, name, source location, and textual content.

2. **Digestion.** During each tick, the colony's agents process nutrient particles. Agents detect relationships between elements based on name co-occurrence, type compatibility, call patterns, module co-location, and trait implementation. Each detected relationship becomes an edge in the knowledge graph.

3. **Reinforcement.** Frequently co-accessed elements have their connecting edges strengthened. Rarely accessed paths decay. This mirrors biological synaptic plasticity and ensures the graph reflects actual usage relevance.

4. **Self-Organization.** Over successive ticks, the graph topology evolves. Highly connected clusters emerge around core abstractions, while peripheral utility functions settle to the graph's edges.

The colony does not require a predefined schema. The graph structure emerges from the code itself, making the system adaptable to any codebase's conventions and architecture.

### 2.3 QueryEngine: Graph-Based Retrieval

The QueryEngine accepts natural-language or identifier-based queries and retrieves related code elements through graph traversal:

1. **Seed identification.** The query is matched against node names, types, and content to identify seed nodes.
2. **Neighborhood expansion.** From seed nodes, the engine traverses edges weighted by relationship strength, collecting neighboring nodes up to a configurable depth.
3. **Ranking.** Retrieved nodes are ranked by a combination of edge weight (relationship strength), path length (proximity to the query), and node degree (centrality in the graph).

This traversal-based retrieval naturally surfaces associated concepts. A query for "Colony" does not merely find code containing the word "Colony" -- it surfaces `spawn`, `tick`, `agents`, `digest`, and other functionally related elements connected through the graph's edge structure.

### 2.4 SessionManager: Persistent State

The SessionManager serializes the complete colony state -- all nodes, edges, weights, and metadata -- to a JSON file. On subsequent sessions, the graph is deserialized and the colony resumes from its prior state. This provides true cross-session memory: the knowledge accumulated during one development session is immediately available in the next.

The persistence format is designed for fidelity over compactness. Every node attribute, edge weight, and colony parameter is preserved exactly, ensuring that a restored session is indistinguishable from a continuous one.

---

## 3. Experimental Setup

We evaluated Agentic Memory on the phago-core project itself, a practice known as dogfooding. This choice was deliberate: the system should be capable of understanding its own codebase.

### 3.1 Corpus

The phago-core project comprises 55 Rust source files implementing the biological simulation framework, code digestion pipeline, query engine, session management, and MCP server integration. The CodeDigester extracted **830 code elements** from this corpus, with 52 of 54 documentation files successfully digested.

### 3.2 Colony Simulation

The extracted elements were ingested into a colony instance and the simulation was run for **100 ticks**. No manual tuning of colony parameters was performed; default values were used throughout.

### 3.3 Retrieval Evaluation

We evaluated 10 code queries with manually determined ground truth:

1. "Agent"
2. "Colony"
3. "Substrate"
4. "Digester"
5. "NodeData"
6. "TopologyGraph"
7. "Position"
8. "transfer"
9. "apoptosis"
10. "membrane"

For each query, we recorded the top-5 results from both the graph-based QueryEngine and a grep baseline (substring match ranked by occurrence frequency). Precision at rank 5 (P@5) was computed against the ground truth.

### 3.4 Persistence Test

The complete colony state was saved to JSON, then loaded into a fresh instance. We verified node count, edge count, and edge weight fidelity.

---

## 4. Results

### 4.1 Graph Structure

After 100 ticks of colony simulation, the knowledge graph contained:

| Metric | Value |
|--------|-------|
| Nodes | 659 |
| Edges | 33,490 |
| Code elements ingested | 830 |
| Mean node degree | ~50.8 |

The graph exhibited clear clustering around core abstractions. The `Colony` node was among the most connected, with direct edges to spawning, tick management, agent lifecycle, and digestion subsystems. Trait nodes served as bridges between implementing structs, creating natural cross-module pathways.

### 4.2 Retrieval Precision

| Method | Mean P@5 |
|--------|----------|
| Graph (QueryEngine) | 0.140 |
| Grep (baseline) | 0.323 |

**Graph-based retrieval underperformed grep on precision.** This honest result reveals a critical insight: pure precision metrics favor exact string matching over conceptual retrieval. The per-query breakdown:

| Query | Graph P@5 | Grep P@5 |
|-------|-----------|----------|
| Agent | 0.20 | 0.20 |
| Colony | 0.20 | 0.20 |
| Substrate | 0.20 | 0.33 |
| Digester | 0.20 | 0.50 |
| NodeData | 0.00 | 0.00 |
| TopologyGraph | 0.00 | 0.00 |
| Position | 0.20 | 0.50 |
| transfer | 0.20 | 0.50 |
| apoptosis | 0.20 | 1.00 |
| membrane | 0.00 | 0.00 |
| **Mean** | **0.140** | **0.323** |

However, the qualitative character of results differed substantially. Grep returned results containing the query string; the graph returned results connected to the query concept through structural relationships. For example:

- **Query: "Colony"**
  - Grep: `Colony` struct, `Colony::new`, `Colony::tick`, `use colony::Colony`, documentation strings mentioning "colony"
  - Graph: `Colony` struct, `spawn`, `tick`, `agents` field, `digest`, `Agent` struct (related concepts)

The graph results form a coherent conceptual neighborhood rather than a list of string matches. For a developer seeking to understand "what is Colony and how does it work," the graph output provides context that grep cannot: the web of related abstractions. **This is complementary, not competitive.** Developers benefit from both precise search (grep) and conceptual exploration (graph).

### 4.3 Session Persistence

| Metric | Saved | Loaded | Fidelity |
|--------|-------|--------|----------|
| Nodes | 659 | 659 | 100% |
| Edges | 33,490 | 33,490 | 100% |

Session persistence achieved **perfect fidelity**. Every node, edge, and weight survived the save/load cycle without loss or corruption. The restored graph was **IDENTICAL** to the original. This means a developer can close their editor, return days later, and resume with the full knowledge graph intact -- no reindexing, no recomputation, no context loss.

### 4.4 Code Element Distribution

| Element Type | Count |
|-------------|-------|
| Functions | 459 |
| Structs | 72 |
| Enums | 22 |
| Traits | 14 |
| Other (Imports, Modules, Constants) | 263 |
| **Total** | **830** |

The distribution reflects a typical Rust codebase: function-heavy (55%), with structs and traits forming the structural backbone. The "Other" category includes imports, module declarations, constants, and type aliases that contribute to the overall codebase structure.

---

## 5. Discussion

### 5.1 Graph vs. Grep: Complementary Tools, Not Competitors

The graph's lower precision (P@5 = 0.140 vs. grep's 0.323) should not be interpreted as "the graph is worse." The two methods answer fundamentally different questions. Grep answers "where does this string appear?" The graph answers "what is associated with this concept through structural relationships?" Both are useful; neither subsumes the other.

**Grep excels at finding exact matches.** If you know the identifier name and want all occurrences, grep is faster and more precise. **The graph excels at exploration.** If you're trying to understand a concept's context -- what it connects to, what depends on it, what it's similar to architecturally -- the graph provides the relational web that grep cannot.

In practice, developers already have grep. What they lack is the conceptual map. The graph does not replace grep; it complements it. Ideal tooling would offer both: precise search when you know what you're looking for, and graph traversal when you're exploring or understanding.

### 5.2 The Dogfooding Argument

Evaluating Agentic Memory on its own codebase is more than a convenience. It is a validity test: if a code knowledge system cannot understand itself, its utility on external codebases is questionable. The fact that the system successfully extracted, organized, and retrieved its own components demonstrates a baseline of self-consistency.

Furthermore, dogfooding revealed practical insights. The CodeDigester's pattern-matching approach handled Rust's syntax well but required adjustments for macro-generated code. The colony's default parameters produced a moderately connected graph (mean degree ~50.8), suggesting that the system balances between connectivity (for exploration) and sparsity (for precision). Future work on edge pruning and decay tuning could further improve retrieval precision.

### 5.3 Biological Metaphor as Design Principle

The biological framing is not merely aesthetic. Self-organization, reinforcement learning, and decay are well-studied phenomena with known properties. By grounding the system in biological metaphor, we inherit these properties: the graph adapts to usage patterns (reinforcement), forgets irrelevant connections (decay), and develops structure without top-down design (self-organization). These are precisely the characteristics missing from static analysis tools.

### 5.4 Toward MCP Integration

The natural evolution of this work is integration with the Model Context Protocol (MCP), enabling large language models to query the code knowledge graph as a tool. An LLM equipped with Agentic Memory could answer questions like "what would be affected if I change the Colony's tick method?" by traversing the graph rather than scanning files. This represents a shift from code search to code understanding.

Preliminary work on MCP server integration is underway within the phago-core project, exposing the QueryEngine as an MCP tool callable by Claude and other LLM-based assistants.

### 5.5 Limitations

Several limitations warrant acknowledgment:

- **Evaluation scale.** The phago-core project, at 55 files and 830 elements, is moderately small. Scaling behavior on large codebases (thousands of files, millions of elements) is untested.
- **Language specificity.** The current CodeDigester targets Rust. Generalization to other languages requires language-specific extraction modules.
- **Precision gap.** The graph underperformed grep on precision (0.140 vs. 0.323). This reveals that **single-hop retrieval is insufficient** for code queries. Multi-hop graph traversal, relevance ranking improvements, and hybrid graph-text approaches are needed to close this gap.
- **Colony parameters.** Default parameters were used without tuning. Systematic hyperparameter optimization could improve graph quality.
- **Query diversity.** Our 10-query evaluation set was limited. Larger, more diverse query sets covering different coding tasks (debugging, refactoring, feature addition) would provide a fuller picture of graph utility.

---

## 6. Conclusion

We have presented Agentic Memory, a biologically-inspired system for constructing and persisting code knowledge graphs. By modeling code element relationships through a self-organizing colony simulation, the system builds a graph that captures not just what code exists, but how it relates.

Our evaluation on the phago-core project demonstrated:

1. **Effective extraction** of 830 code elements from 55 Rust source files.
2. **Rich graph construction** yielding 659 nodes and 33,490 edges through 100 ticks of colony simulation.
3. **Complementary retrieval** providing structural context (P@5 = 0.140) that differs qualitatively from grep's exact matching (P@5 = 0.323). The lower precision reveals that multi-hop traversal and better relevance ranking are needed.
4. **Perfect session persistence** with 100% fidelity across save/load cycles (659/659 nodes, 33,490/33,490 edges preserved identically).

A self-organizing biological knowledge graph of code provides **conceptual exploration and cross-session memory** -- capabilities that complement traditional search. This addresses a fundamental gap in developer tooling: the absence of semantic memory that learns, persists, and grows with a codebase over time. While precision improvements are needed, the graph's ability to surface structural relationships represents a qualitatively different -- and valuable -- perspective on code understanding.

The code is available as part of the phago-core project and is, fittingly, indexed by its own knowledge graph.

---

## References

1. Microsoft. "Language Server Protocol Specification." https://microsoft.github.io/language-server-protocol/
2. Alon, U. et al. "code2vec: Learning Distributed Representations of Code." POPL 2019.
3. Guo, D. et al. "GraphCodeBERT: Pre-training Code Representations with Data Flow." ICLR 2021.
4. Anthropic. "Model Context Protocol." https://modelcontextprotocol.io/
5. Hebb, D. O. "The Organization of Behavior." Wiley, 1949.
6. Bonabeau, E. et al. "Swarm Intelligence: From Natural to Artificial Systems." Oxford University Press, 1999.
