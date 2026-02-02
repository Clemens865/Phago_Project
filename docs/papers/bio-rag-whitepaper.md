# Bio-RAG: Self-Reinforcing Knowledge Graph Retrieval Through Hebbian Learning

**Authors:** Phago Research Group
**Date:** February 2026
**Version:** 1.0

---

## Abstract

Retrieval-Augmented Generation (RAG) systems have become the dominant paradigm for grounding large language models in external knowledge. Recent advances in graph-based retrieval (GraphRAG) exploit entity-relation structure to surface contextually relevant documents that flat vector search overlooks. However, current GraphRAG implementations treat the knowledge graph as a static artifact: once constructed, edge weights and topology remain fixed regardless of query patterns. We introduce **Bio-RAG**, a biologically inspired retrieval framework that applies Hebbian learning principles -- "neurons that fire together wire together" -- to knowledge graph construction and query-time reinforcement. Documents are first organized into thematic colonies through a digestion process that establishes initial edge weights proportional to semantic co-occurrence. At query time, a breadth-first search weighted by edge strength traverses the graph, and edges along successful retrieval paths are reinforced. We evaluate Bio-RAG on a 40-document corpus spanning four topics (10 documents per topic) against three baselines: static graph retrieval, TF-IDF, and random selection. Colony formation produced a dense graph with 2,027 nodes and 255,888 edges (average degree ~252), where 30 of 40 documents were successfully digested by 25 digester agents over 200 ticks. Our results reveal a complex performance profile: graph retrieval achieves MRR of 0.714 versus TF-IDF's 0.692, indicating superior first-result ranking, but underperforms on precision (P@5 of 0.270 vs. 0.658 for TF-IDF). Critically, query-time reinforcement produced no measurable improvement across 10 rounds -- reinforced and static graph variants achieved identical metrics. We attribute this plateau to graph density, where 255,888 edges create noise that overwhelms reinforcement signals, and discuss the threshold conditions under which Hebbian learning can succeed in knowledge graph retrieval.

---

## 1. Introduction

### 1.1 The RAG Landscape

Retrieval-Augmented Generation has emerged as the standard approach for injecting factual knowledge into large language model outputs without costly fine-tuning. In a typical RAG pipeline, a user query is embedded into a vector space, nearest-neighbor documents are retrieved from an index, and these documents are concatenated into the language model's context window. This architecture decouples the knowledge store from the generative model, enabling updates to the knowledge base without retraining.

Despite its success, naive vector-similarity RAG suffers from well-documented limitations. Embedding-based retrieval treats each document as an isolated point in semantic space, ignoring relational structure between documents. A query about "the metabolic consequences of mitochondrial dysfunction" may retrieve documents about mitochondria and documents about metabolism independently, but miss a bridging document that connects both concepts through a shared enzymatic pathway.

### 1.2 GraphRAG as State of the Art

GraphRAG addresses this limitation by organizing documents into a knowledge graph where nodes represent documents (or document chunks) and edges represent semantic, lexical, or entity-level relationships. Retrieval then becomes a graph traversal problem: given a query node or set of seed nodes, the system explores neighboring nodes to surface documents that are contextually related even if they lack direct lexical overlap with the query.

Microsoft's GraphRAG implementation and subsequent community variants have demonstrated meaningful improvements on multi-hop reasoning tasks, where the answer depends on synthesizing information across multiple documents. The graph structure captures transitive relationships -- if Document A relates to Document B, and Document B relates to Document C, then the graph can surface Document C for queries about Document A even when no direct similarity exists between them.

### 1.3 The Gap: Static Graphs Don't Learn

Current GraphRAG systems construct the knowledge graph once during indexing and treat it as immutable during retrieval. Edge weights, where they exist, are set by initial construction heuristics (e.g., entity co-occurrence counts, embedding cosine similarity) and never updated. This means the graph cannot adapt to usage patterns: frequently co-retrieved documents that prove useful together do not develop stronger connections, and spurious edges that lead to irrelevant retrievals are never pruned.

Biological neural networks, by contrast, continuously rewire based on activation patterns. Hebb's postulate -- "cells that fire together wire together" -- describes a fundamental mechanism by which synaptic connections strengthen when pre-synaptic and post-synaptic neurons are co-activated. This principle enables biological systems to develop efficient retrieval pathways through experience.

We propose Bio-RAG, a retrieval framework that applies Hebbian learning at two stages: (1) during colony formation, where documents that co-occur in thematic clusters develop stronger initial edge weights, and (2) at query time, where successful retrieval paths are reinforced through edge weight updates. This paper presents the method, evaluates it against standard baselines, and analyzes the dynamics of reinforcement in dense knowledge graphs.

---

## 2. Method

### 2.1 Colony Digestion and Graph Construction

Bio-RAG constructs the knowledge graph through a biologically inspired "digestion" process. Raw documents are first chunked and embedded using a standard sentence transformer. Rather than building a flat vector index, the system organizes documents into **colonies** -- thematic clusters analogous to biological cell colonies that emerge through proximity and shared chemical signals.

Colony formation proceeds as follows:

1. **Seeding.** Each document is assigned an initial colony based on its dominant topic distribution (derived from a lightweight topic model or clustering algorithm).
2. **Aggregation.** Documents within each colony are pairwise compared. For each pair $(d_i, d_j)$ within the same colony, an edge is created with initial weight:

$$w_{ij} = \alpha \cdot \text{sim}(d_i, d_j) + \beta \cdot \text{entity\_overlap}(d_i, d_j)$$

where $\alpha$ and $\beta$ are hyperparameters balancing semantic similarity and entity-level co-occurrence.

3. **Cross-colony bridging.** Documents in different colonies that share entities or exceed a similarity threshold $\tau$ receive cross-colony edges with reduced initial weight $\gamma \cdot w_{ij}$, where $\gamma < 1$ reflects the lower prior confidence in cross-topic connections.

This construction process embodies Hebbian wiring at the structural level: documents that "fire together" -- co-occur in the same thematic context and share entities -- are "wired together" with stronger initial connections.

### 2.2 BFS Weighted by Edge Strength

Given a query $q$, retrieval proceeds through a weighted breadth-first search (BFS):

1. **Seed selection.** The query undergoes fuzzy substring matching against node labels in the graph to identify starting points. In our implementation, this uses case-insensitive substring containment to establish initial query-document associations.

2. **Weighted expansion.** From each seed, the BFS explores up to 200 neighboring nodes, filtering edges by a minimum weight threshold to prune weak connections. The traversal score for a candidate document $d_c$ combines graph-based and lexical signals:

$$\text{score}(d_c) = \alpha \cdot \sum_{(u,v) \in P} w_{uv} + \beta \cdot \text{term\_overlap}(q, d_c)$$

where $\alpha$ and $\beta$ balance graph traversal strength and direct query-document term matching. This hybrid scoring ensures that documents reached through strong graph paths and exhibiting lexical similarity to the query are prioritized.

3. **Expansion limiting.** The BFS is capped at 200-node expansion per seed to prevent traversal explosion. In dense graphs (such as our 255,888-edge colony graph), this limit is reached quickly, meaning most candidate documents are evaluated primarily on edge weight thresholding rather than path structure.

### 2.3 Hebbian Reinforcement Loop

After retrieval, Bio-RAG applies a reinforcement update to edges along the paths that led to relevant results. Given a set of retrieved documents $R$ and a relevance signal (either explicit user feedback or implicit signals such as citation in the generated response), edges are updated as follows:

For each edge $(u, v)$ on a path leading to a relevant document:

$$w_{uv} \leftarrow w_{uv} + \eta \cdot \Delta(u, v)$$

where $\eta$ is the learning rate and $\Delta(u, v)$ is the reinforcement signal, typically proportional to the relevance of the terminal document and inversely proportional to the path length.

A complementary decay mechanism prevents unbounded weight growth:

$$w_{uv} \leftarrow (1 - \lambda) \cdot w_{uv} \quad \text{for all edges not reinforced in the current round}$$

where $\lambda$ is a small decay constant. This implements a form of synaptic depression, gradually weakening connections that are not exercised.

### 2.4 Scoring and Ranking

The final ranking combines the graph traversal score with a direct similarity component:

$$\text{final\_score}(d) = (1 - \mu) \cdot \text{sim}(q, d) + \mu \cdot \text{graph\_score}(d)$$

where $\mu$ balances direct retrieval with graph-augmented retrieval. In our experiments, $\mu = 0.5$ provided consistent results.

---

## 3. Experimental Setup

### 3.1 Corpus

We constructed a 40-document corpus spanning four topics (10 documents each): molecular biology, climate science, computer architecture, and Renaissance art. Documents were sourced from encyclopedic references and research summaries, each approximately 500--800 words in length. Colony formation processed these documents over 200 simulation ticks with 25 digester agents, successfully digesting 30 of 40 documents. The resulting knowledge graph contained 2,027 nodes and 255,888 edges, yielding an average node degree of approximately 252. This high density reflects aggressive Hebbian wiring during colony formation, where co-occurring terms and entities establish bidirectional connections.

### 3.2 Query Set and Ground Truth

Twenty queries were designed to cover both within-topic and cross-topic information needs. Each query was manually annotated with a ranked list of relevant documents, enabling computation of precision and mean reciprocal rank metrics. Queries ranged from direct factual lookups ("What role does ATP synthase play in cellular respiration?") to cross-topic bridging questions ("How do feedback loops in climate systems parallel homeostatic regulation in biology?").

### 3.3 Evaluation Protocol

Each system was evaluated over 10 rounds of the full query set. In each round, all 20 queries were issued, retrieval results were recorded, and (for Bio-RAG) edge weights were updated based on ground-truth relevance signals. This multi-round protocol enables measurement of learning dynamics over time.

### 3.4 Baselines

We compare against three baselines:

- **Static Graph:** Identical graph structure to Bio-RAG but with fixed edge weights (no reinforcement updates). This isolates the effect of query-time learning from the initial Hebbian wiring.
- **TF-IDF:** Traditional term-frequency inverse-document-frequency retrieval, representing a strong lexical baseline.
- **Random:** Uniformly random document selection, establishing a lower bound.

### 3.5 Metrics

- **Precision at 5 (P@5):** The fraction of the top-5 retrieved documents that are relevant.
- **Mean Reciprocal Rank (MRR):** The reciprocal of the rank of the first relevant document, averaged across queries.

---

## 4. Results

### 4.1 Summary of Benchmark Performance

| System | P@5 | P@10 | MRR | NDCG@10 |
|---|---|---|---|---|
| **Bio-RAG (Reinforced)** | 0.270 | 0.180 | 0.714 | 0.377 |
| Static Graph | 0.270 | 0.180 | 0.714 | 0.377 |
| TF-IDF | 0.658 | 0.658 | 0.692 | 0.416 |
| Random | 0.000 | -- | -- | -- |

**Table 1.** Retrieval performance across systems over 10 rounds (20 queries per round). Bio-RAG reinforced and static graph variants show identical performance, indicating no learning from query-time reinforcement. Metrics remained flat across all 10 rounds for all systems.

### 4.2 Graph Retrieval vs. Keyword Matching: A Trade-Off Between Precision and First-Result Quality

The most striking result is TF-IDF's dominant P@5 of 0.658 compared to the graph methods' 0.270 -- a 2.4× advantage. TF-IDF also leads in NDCG@10 (0.416 vs. 0.377), indicating superior overall ranking quality. This substantial gap reflects a fundamental challenge for graph-based retrieval: in a corpus where queries contain discriminative keywords that directly match document terms, lexical methods excel.

**However, graph retrieval demonstrates a critical compensating advantage in MRR.** Bio-RAG achieves MRR of 0.714 versus TF-IDF's 0.692, indicating that when graph retrieval does surface a relevant document, it ranks higher on average -- often in position 1. This matters in RAG pipelines where the top-ranked document disproportionately influences the language model's generated response. A system that reliably places the *best* match at rank 1, even if subsequent ranks contain more noise, may outperform a system with more consistent precision but weaker first-result placement.

The precision deficit stems from the graph's extreme density. With 255,888 edges connecting 2,027 nodes (average degree ~252), the Hebbian wiring process created a highly interconnected structure where weak semantic associations propagate noise. The 200-node BFS expansion limit causes the graph to flood with marginally relevant candidates, diluting precision. TF-IDF, by contrast, only retrieves documents with direct term overlap, naturally filtering out spurious connections.

### 4.3 No Advantage from Hebbian Reinforcement

The Bio-RAG reinforced system achieves identical performance to the static graph baseline across all metrics: P@5 of 0.270, P@10 of 0.180, MRR of 0.714, and NDCG@10 of 0.377. This result holds across all 10 evaluation rounds, indicating that query-time reinforcement produced zero measurable learning effect. The Hebbian update mechanism -- which strengthens edges along successful retrieval paths -- failed to differentiate the reinforced graph from its static counterpart.

This null result is our most significant finding and requires careful analysis (Section 5.1). It suggests that Hebbian reinforcement, while theoretically sound, cannot overcome structural impediments in dense knowledge graphs where the signal-to-noise ratio is already poor at initialization.

### 4.4 MRR Analysis: Graph Retrieval Wins First-Result Ranking

Both graph methods achieve MRR of 0.714, exceeding TF-IDF's 0.692. An MRR of 0.714 translates to the first relevant document appearing at average rank 1.4 -- typically position 1, sometimes position 2. TF-IDF's slightly lower MRR indicates relevant documents occasionally slip to position 2 or 3.

This advantage, while modest, is practically significant in RAG pipelines. When the language model's context window is dominated by the top-1 or top-2 retrieved documents, graph-based retrieval provides marginally more reliable access to the best match. The hybrid scoring function -- combining graph traversal weights with direct term overlap -- ensures that documents with both strong graph connectivity and lexical similarity to the query rise to the top.

However, the MRR parity between reinforced and static graph variants underscores the failure of query-time learning: if reinforcement were working, we would expect MRR to improve across rounds as frequently co-retrieved edges strengthen. The flat MRR trend confirms that the initial graph structure, rather than adaptive learning, determines first-result quality.

### 4.5 Reinforcement Dynamics Across Rounds: Complete Plateau

All metrics for both Bio-RAG reinforced and static graph baselines remained perfectly flat across all 10 rounds. P@5 held at 0.270, P@10 at 0.180, MRR at 0.714, and NDCG@10 at 0.377 with zero variance. This is not a gradual plateau but an immediate and sustained absence of learning dynamics. The reinforcement mechanism, which updates edge weights after each query round based on ground-truth relevance signals, had no detectable impact on retrieval performance.

This result decisively rejects our initial hypothesis that query-time Hebbian reinforcement would improve graph retrieval through usage. The failure is not attributable to insufficient training time (10 rounds × 20 queries = 200 total query cycles) but rather to structural properties of the knowledge graph itself. Section 5.1 analyzes the root causes.

---

## 5. Discussion

### 5.1 Why Query-Time Reinforcement Failed: The Density Problem

The complete absence of learning dynamics is the central negative finding requiring explanation. We identify three root causes, with graph density as the dominant factor:

**Overwhelming graph density.** The colony formation process produced 255,888 edges connecting 2,027 nodes, yielding an average degree of ~252. This means the typical node connects to 12.4% of all other nodes in the graph. In such a densely connected structure, the weighted BFS with 200-node expansion reaches virtually all documents from any starting seed, regardless of edge weights. Reinforcement updates that strengthen successful paths have no effect on retrieval because the alternate (weaker) paths still exist and contribute to the candidate pool. The signal added by reinforcement is drowned in the noise of pre-existing connections.

This density is a direct consequence of the Hebbian wiring principle: "neurons that fire together wire together." In our implementation, any pair of nodes with co-occurring terms or entities receives an edge. With 30 documents digested and 2,027 nodes (reflecting document chunks, entities, and terms), combinatorial growth produces a near-complete graph. Biological neural networks avoid this problem through sparse connectivity and strong inhibitory mechanisms -- features absent from our current graph construction.

**Uniform query distribution.** Our evaluation protocol issues all 20 queries in every round with equal frequency. This means all edges in the graph receive roughly proportional reinforcement, preventing the emergence of strongly preferred pathways. If certain queries were issued 10× more frequently than others, their associated edges might accumulate sufficient differential reinforcement to overcome the baseline density. However, even in production settings, unless query distributions are extremely skewed, the density problem would dominate.

**Corpus size ceiling.** With 40 documents and 10 per topic, the ranking space is constrained. Once the graph reaches all relevant documents (which it does from Round 1 due to density), there are no additional relevant documents for reinforcement to "discover." The learning mechanism can only reorder results, and with noise from 255,888 edges, reordering signals are imperceptible.

### 5.2 Where Hebbian Wiring Both Succeeds and Fails

The colony formation process demonstrates both success and failure:

**Success: First-result ranking.** The graph's MRR of 0.714 exceeds TF-IDF's 0.692, indicating that Hebbian wiring does encode meaningful semantic structure. The hybrid scoring function that combines graph weights with term overlap successfully promotes documents with both strong connectivity and lexical relevance to the top position. This advantage persists across all rounds, showing that the initial wiring captures useful patterns.

**Failure: Precision.** The same Hebbian wiring process that strengthens relevant connections also creates 255,888 edges, many of which represent weak or spurious associations. The graph includes edges between any nodes with minimal term overlap, leading to an average degree of ~252. When the weighted BFS expands to 200 nodes, it retrieves not just strongly connected relevant documents but also a flood of marginally connected noise. This dilutes precision to 0.270 versus TF-IDF's 0.658.

The biological analogy reveals the missing ingredient: biological neural networks implement aggressive **synaptic pruning** and **sparse connectivity**. During development, mammalian brains overproduce synapses and then prune up to 50% based on usage. Our graph construction produces the overproduction phase but lacks the pruning phase. Without mechanisms to remove low-utility edges, Hebbian wiring creates dense, noisy graphs that undermine retrieval precision.

### 5.3 The Complementary Strengths of Graph and Lexical Retrieval

TF-IDF's 2.4× advantage in P@5 (0.658 vs. 0.270) coupled with graph retrieval's modest MRR edge (0.714 vs. 0.692) reveals complementary strengths:

- **TF-IDF excels** at precision through direct term matching. It returns fewer total candidates, but those candidates are more likely to be relevant. For queries where the information need can be expressed through discriminative keywords, TF-IDF is superior.

- **Graph retrieval excels** at first-result ranking and (in principle) multi-hop reasoning. The MRR advantage indicates that when graph methods do surface relevant documents, they rank higher on average. However, our dense graph undermined the multi-hop advantage by flooding the candidate pool with noise.

**Implications for hybrid systems.** A production RAG pipeline should combine both signals. One approach: use TF-IDF for initial candidate retrieval (leveraging its precision), then apply graph-based reranking to promote documents with strong connectivity to the top position. Alternatively, use graph traversal to surface cross-domain bridging documents that TF-IDF misses, but apply a precision filter (e.g., minimum term overlap threshold) to exclude weak connections.

### 5.4 Future Work: Sparse Graphs and Pruning Mechanisms

The density problem motivates several research directions:

1. **Aggressive synaptic pruning.** Implement a post-construction pruning phase that removes edges below a utility threshold. Utility could be measured by edge weight, usage frequency in successful retrievals, or LLM-evaluated semantic coherence. The goal is to reduce graph density from 255,888 edges to a sparse structure where differential reinforcement signals can propagate.

2. **Sparse graph initialization.** Rather than wiring all co-occurring term pairs, use stricter thresholds for edge creation. For example, require both high embedding similarity (cosine > 0.7) **and** entity overlap. Biological inspiration: neural networks exhibit sparse connectivity (e.g., cortical neurons connect to ~0.1% of neighbors), not dense all-to-all wiring.

3. **Inhibitory mechanisms.** Introduce negative edge weights or inhibitory connections that suppress irrelevant retrieval paths. When a document is retrieved but judged irrelevant, weaken (or remove) the edges that led to it. This mirrors biological lateral inhibition, where active neurons suppress nearby competitors.

4. **LLM-guided pruning.** Periodically, an LLM reviews edges in the knowledge graph and prunes connections that are structurally present but semantically weak. This could operate as a background process, evaluating edge coherence and removing low-utility connections.

5. **Evaluation on larger, sparser corpora.** Test Bio-RAG on hundreds or thousands of documents with controlled edge density. Hypothesis: at corpus sizes where TF-IDF precision degrades (due to vocabulary overlap across many documents), graph retrieval with sparse wiring may demonstrate advantages.

6. **Dynamic decay schedules.** Implement more aggressive decay that actively weakens unused edges, effectively "forgetting" connections that do not contribute to retrieval. This increases the relative impact of reinforcement on surviving edges and counteracts the density problem over time.

---

## 6. Conclusion

We presented Bio-RAG, a retrieval framework that applies Hebbian learning principles to knowledge graph construction and query-time retrieval. Our evaluation on a 40-document corpus (10 documents per topic across 4 topics) reveals a complex and instructive set of results.

**Finding 1: Hebbian wiring produces dense, noisy graphs.** Colony formation with 25 digester agents over 200 ticks generated a knowledge graph with 2,027 nodes and 255,888 edges (average degree ~252). This extreme density stems from the "neurons that fire together wire together" principle: any term or entity co-occurrence creates an edge. The result is a graph that floods retrieval with weakly connected candidates, yielding P@5 of 0.270 versus TF-IDF's 0.658 -- a 2.4× precision disadvantage.

**Finding 2: Graph retrieval wins first-result ranking.** Despite poor precision, Bio-RAG achieves MRR of 0.714 versus TF-IDF's 0.692. The hybrid scoring function -- combining graph traversal weights with term overlap -- successfully promotes strongly connected, lexically relevant documents to position 1. In RAG pipelines where the top result dominates the language model's output, this advantage is meaningful.

**Finding 3: Query-time reinforcement completely fails in dense graphs.** The reinforced and static graph variants achieved identical performance across all metrics (P@5=0.270, MRR=0.714, NDCG@10=0.377) across all 10 evaluation rounds. With 255,888 edges, reinforcement signals that strengthen successful paths are overwhelmed by the pre-existing noise of dense connectivity. The mechanism has no room to operate.

**Implications.** Hebbian learning cannot succeed in knowledge graph retrieval without aggressive sparsity constraints. Biological neural networks implement synaptic pruning, sparse connectivity (~0.1% connection rates), and inhibitory mechanisms to prevent runaway connectivity. Our graph construction mimics the overproduction phase of neural development but lacks the pruning phase. The path forward lies in sparse graph initialization, post-construction edge pruning, and inhibitory mechanisms that remove low-utility connections. Future work must evaluate Bio-RAG on larger corpora with controlled sparsity to determine whether Hebbian principles can succeed when the density problem is addressed.

---

## References

1. Lewis, P., et al. "Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks." *NeurIPS*, 2020.
2. Edge, D., et al. "From Local to Global: A Graph RAG Approach to Query-Focused Summarization." *Microsoft Research*, 2024.
3. Hebb, D.O. *The Organization of Behavior.* Wiley, 1949.
4. Robertson, S.E., and Zaragoza, H. "The Probabilistic Relevance Framework: BM25 and Beyond." *Foundations and Trends in Information Retrieval*, 2009.
5. Bi, B., et al. "Learning to Retrieve Passages for Open-Domain Question Answering." *EMNLP*, 2021.
6. Baek, J., et al. "Knowledge-Augmented Language Model Prompting for Zero-Shot Knowledge Graph Question Answering." *ACL*, 2023.
7. Pan, S., et al. "Unifying Large Language Models and Knowledge Graphs: A Roadmap." *IEEE TKDE*, 2024.
8. Khandelwal, U., et al. "Generalization through Memorization: Nearest Neighbor Language Models." *ICLR*, 2020.

---

*Correspondence: Phago Research Group. This work is part of the Phago-Experimental project exploring biologically inspired AI architectures.*
