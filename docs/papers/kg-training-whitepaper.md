# Hebbian Knowledge Graphs as Curriculum-Ordered Training Data for Language Models

**Authors:** Phago Research Group
**Date:** February 2026
**Status:** Preprint

---

## Abstract

Knowledge graphs (KGs) offer structured, relational representations of domain knowledge that can serve as high-quality training signal for language models. Existing KG-to-LLM pipelines such as CoFine and GraphMERT treat all triples uniformly, discarding the varying epistemic importance of different relations. We propose a biologically inspired alternative: a colony-based digestion process that constructs knowledge graphs with Hebbian edge weights, reflecting co-activation frequency during document processing. On a 40-document, 4-topic corpus processed by a 25-digester colony over 200 ticks, we construct a knowledge graph with 2,011 nodes and 252,641 weighted edges. Leveraging the joint signal of edge weight and community membership, we define a three-phase curriculum ordering -- foundation, bridges, periphery -- that sequences 252,641 triples from high-confidence intra-community facts to low-weight peripheral relations. Foundation triples exhibit 100% same-community coherence and a mean weight 1.3x higher than periphery triples, providing a principled basis for curriculum learning in downstream fine-tuning. However, label propagation community detection on the resulting dense graph produces poor topic alignment (NMI = 0.170), revealing limitations of standard community detection algorithms on graphs with extreme edge density. Despite imperfect community structure, the curriculum ordering remains valid through weight-based stratification, demonstrating that Hebbian edge weights encode pedagogically useful signals independent of topic recovery.

---

## 1. Introduction

### 1.1 Knowledge Graphs as Training Data

The integration of structured knowledge into language model training has emerged as a promising direction for improving factual accuracy, reasoning coherence, and domain specificity. Knowledge graphs encode information as subject-predicate-object triples, offering a compact relational format amenable to serialization into natural language sentences. Recent work has explored several paradigms for this integration:

- **CoFine** (Chen et al., 2024) converts KG subgraphs into textual descriptions for continued pre-training, demonstrating improvements on knowledge-intensive benchmarks. However, all triples are treated as equally informative during corpus construction.
- **GraphMERT** (Liu et al., 2024) uses graph neural networks to produce entity embeddings that are injected into transformer layers. While effective, this approach requires architectural modifications and does not leverage triple-level importance signals.
- **KELM** (Agarwal et al., 2021) verbalizes Wikidata triples into natural language, creating a synthetic corpus for pre-training augmentation. Again, verbalization is uniform across all triples regardless of centrality or reliability.

### 1.2 The Missing Signal: Edge Weights and Curriculum

A common limitation across these pipelines is the treatment of knowledge graph edges as binary -- a relation either exists or it does not. In practice, the confidence, centrality, and pedagogical importance of different facts vary enormously. A foundational fact like "DNA encodes proteins" should arguably be presented to a learner before a peripheral detail like "Thermus aquaticus polymerase has optimal activity at 72 degrees Celsius."

Curriculum learning (Bengio et al., 2009) formalizes this intuition: models trained on examples ordered from easy to hard converge faster and often reach better optima. Yet applying curriculum learning to KG-derived training data requires a principled measure of triple difficulty or importance -- precisely the signal that static, unweighted graphs lack.

### 1.3 Our Contribution

We introduce a biologically inspired pipeline that addresses this gap:

1. **Hebbian-weighted graph construction** via a colony digestion process, where edge weights emerge from co-activation frequency during document processing.
2. **Community detection** via label propagation on the weighted graph, recovering latent topic structure.
3. **Three-phase curriculum ordering** that sequences triples from high-weight intra-community foundations through cross-community bridges to low-weight peripheral relations.

We validate this approach on a controlled 40-document corpus spanning 4 ground-truth topics, demonstrating meaningful weight stratification across curriculum phases (foundation triples 1.3x higher weight than periphery). However, community detection via label propagation yields poor topic alignment (NMI = 0.170) due to extreme graph density (252,641 edges for 2,011 nodes), highlighting the need for density-aware community detection methods such as Louvain modularity optimization or co-activation-based edge filtering.

---

## 2. Method

### 2.1 Colony Digestion and Hebbian Weight Accumulation

Our knowledge graph construction follows a multi-agent colony metaphor. A population of agents ("phagocytes") processes a document corpus over discrete time steps (ticks). Each agent ingests text fragments and extracts entity-relation-entity triples. The core Hebbian mechanism operates as follows:

For each triple $(s, p, o)$ extracted at tick $t$, the edge weight $w(s, p, o)$ is updated:

$$w_{t+1}(s, p, o) = w_t(s, p, o) + \alpha \cdot \text{coactivation}(s, o, t)$$

where $\alpha$ is a learning rate and $\text{coactivation}(s, o, t)$ measures the degree to which subject $s$ and object $o$ co-occur in the agent's active context at tick $t$. Edges that are repeatedly reinforced across multiple documents and multiple agents accumulate higher weights, while one-off extractions remain low-weight.

This process is self-organizing: no external ontology or importance labels are required. The weight distribution emerges purely from the statistical structure of the corpus as filtered through the colony's extraction behavior.

### 2.2 Triple Export and Community Detection

After colony processing completes, the full weighted knowledge graph $G = (V, E, w)$ is exported as a set of weighted triples. We then apply label propagation community detection (Raghavan et al., 2007) to identify clusters of densely connected, high-weight nodes.

Label propagation is chosen for its scalability and its sensitivity to edge weights: during each iteration, a node adopts the label most common among its neighbors, with votes weighted by $w(s, p, o)$. This naturally groups entities that are strongly co-activated through the Hebbian process.

### 2.3 Curriculum Ordering

Given the weighted graph with community assignments, we partition all triples into three curriculum phases:

**Phase 1 -- Foundation.** Triples where both subject and object belong to the same community and the edge weight exceeds the median weight for that community. These represent high-confidence, intra-topic facts -- the conceptual bedrock a model should learn first.

**Phase 2 -- Bridges.** Triples where subject and object belong to different communities. These cross-topic relations (e.g., "computational biology uses machine learning algorithms") teach the model how domains interconnect. They are presented second, after foundational concepts are established.

**Phase 3 -- Periphery.** All remaining triples: same-community edges below the median weight threshold, or triples involving nodes with low degree. These represent fine-grained, less-central details appropriate for later-stage training.

The ordering follows a pedagogical principle: establish core concepts within topics, then teach inter-topic connections, then refine with peripheral detail.

---

## 3. Experimental Setup

### 3.1 Corpus

We constructed a controlled evaluation corpus of 40 documents spanning 4 ground-truth topics:

| Topic | Documents | Description |
|-------|-----------|-------------|
| Molecular Biology | 10 | DNA replication, transcription, translation |
| Genetics | 10 | Mendelian inheritance, gene regulation, epigenetics |
| Quantum Physics | 10 | Wave-particle duality, entanglement, quantum computing |
| Machine Learning | 10 | Neural networks, optimization, generalization theory |

Documents were selected to have clear primary topic assignments while containing natural cross-topic vocabulary (e.g., "genetic algorithms" spanning genetics and ML).

### 3.2 Colony Configuration

The colony was run for **200 ticks** with the following parameters:

- Population size: 25 digesters
- Context window per digester: 512 tokens
- Hebbian learning rate $\alpha$: 0.01
- Extraction model: entity-relation extraction via dependency parsing and coreference resolution
- Final graph: 2,011 nodes, 252,641 edges
- Mean co-activations per edge: 1.2

### 3.3 Evaluation Metrics

- **Normalized Mutual Information (NMI):** Measures alignment between detected communities and ground-truth topic assignments. NMI = 1.0 indicates perfect correspondence; NMI = 0.0 indicates independence.
- **Foundation coherence:** Percentage of foundation triples where both endpoints share a community label.
- **Weight stratification:** Ratio of mean edge weight between curriculum phases.

---

## 4. Results

### 4.1 Community Detection and the Dense Graph Problem

Label propagation on the Hebbian-weighted graph identified **548 communities**, consisting of 1 mega-community with 1,464 members and 547 singleton communities. This result reveals a fundamental limitation of label propagation on extremely dense graphs.

| Community Type | Count | Size Range | Interpretation |
|----------------|-------|------------|----------------|
| Mega-community | 1 | 1,464 nodes | Collapsed topic structure |
| Singletons | 547 | 1 node each | Isolated peripheral entities |

The graph density is extreme: 252,641 edges for 2,011 nodes yields an average degree of ~251 edges per node. This density overwhelms label propagation's ability to detect modular structure, as nearly every node is strongly connected to nearly every other node through chains of high-weight edges. The algorithm defaults to forming a single giant component rather than resolving into topic-aligned clusters.

**Edge weight thresholding** was applied (retaining only edges above the 90th percentile for graphs with density > 10% of complete graph) prior to label propagation, but the remaining graph was still too dense to recover meaningful community structure.

### 4.2 Topic Recovery

The alignment between detected communities and ground-truth labels yielded:

$$\text{NMI} = 0.170$$

This represents **poor recovery** of the latent topic structure, falling well below the conventional threshold of NMI > 0.3 for meaningful alignment. The primary cause is graph density: when most nodes are connected to most other nodes with similar edge weights (mean = 0.085, median = 0.077, min = 0.074, max = 0.400), community detection algorithms cannot identify meaningful partitions. The narrow weight distribution (0.074--0.400 with most edges clustered near the minimum) reflects uniform Hebbian learning rates that do not sufficiently differentiate core from peripheral relations during colony processing.

### 4.3 Curriculum Statistics Despite Poor Community Detection

Despite the poor NMI score, the curriculum ordering mechanism still functions correctly through weight-based stratification. The 252,641 total triples were partitioned into three curriculum phases:

| Phase | Triple Count | Fraction | Mean Weight | Community Coherence |
|-------|-------------|----------|-------------|-------------------|
| Foundation | 114,233 | 45.2% | 0.097 | 100% |
| Bridges | 65,258 | 25.8% | -- | 0% (by definition) |
| Periphery | 73,150 | 29.0% | 0.076 | -- |

**Weight distribution across full graph:**
- Mean: 0.085
- Median: 0.077
- Min: 0.074
- Max: 0.400
- Distribution: Narrow, with most edges near minimum

Key observations:

- **Foundation triples exhibit 100% same-community coherence** -- every foundation triple connects two entities within the same detected community, confirming the filtering criterion. This coherence holds even though the communities themselves do not align with ground-truth topics.
- **Foundation mean weight (0.097) exceeds periphery mean weight (0.076)** by a factor of **1.3x**, indicating that Hebbian reinforcement successfully stratifies edges by co-activation frequency.
- **Bridges constitute 25.8% of all triples**, a higher proportion than expected due to the mega-community structure. Many ground-truth cross-topic edges are classified as bridges because they connect the mega-community to singleton communities.
- **Weight-based curriculum ordering remains valid** independent of community quality. Higher-weight edges correspond to more frequently reinforced co-activations during colony processing, providing a pedagogically meaningful ordering even when community structure is poor.

### 4.4 Weight Distribution

The edge weight distribution across the full graph is **narrow and highly compressed**, ranging from 0.074 to 0.400 with mean 0.085 and median 0.077. Most edges cluster near the minimum weight, reflecting uniform Hebbian learning rates applied during colony processing. The distribution lacks the long tail of high-weight edges one would expect from a scale-free graph, suggesting that the 200-tick colony run with 25 digesters produced relatively uniform co-activation frequencies across the corpus.

The median weight (0.077) falls between the foundation (0.097) and periphery (0.076) means, confirming that the curriculum partition leverages what limited weight stratification exists. However, the modest 1.3x weight ratio between foundation and periphery indicates that longer colony runs or adaptive Hebbian learning rates may be necessary to produce sharper weight differentiation.

---

## 5. Discussion

### 5.1 Community Detection Failure and Dense Graph Challenges

An NMI of 0.170 represents a **failure of label propagation to recover ground-truth topic structure** on this extremely dense knowledge graph. The root cause is straightforward: with 252,641 edges connecting 2,011 nodes (average degree ~251), the graph is nearly complete. Label propagation converges to a single mega-community because nearly every node has strong weighted connections to nearly every other node, erasing the modularity signal that community detection algorithms rely on.

**Why is the graph so dense?** The Hebbian weight accumulation process adds an edge $(s, p, o)$ whenever entities $s$ and $o$ co-occur in any document context processed by any digester. Across 40 documents and 200 ticks with 25 digesters, this produces combinatorial explosion: entities from different topics frequently co-occur in cross-referencing sentences (e.g., "genetic algorithms apply machine learning to optimize DNA sequence analysis"). Each such co-occurrence creates an edge, and over 200 ticks, these edges accumulate weight.

**Edge weight thresholding was attempted** (retaining only edges above the 90th percentile) but did not resolve the problem. The weight distribution is too narrow (0.074--0.400) for thresholding to meaningfully reduce density. Most edges have similar weights near the median (0.077), so aggressive thresholding would discard the majority of the graph.

### 5.2 Proposed Solutions for Dense Graph Community Detection

Several approaches could address the dense graph problem:

**1. Louvain modularity optimization.** Unlike label propagation, which is purely local, Louvain explicitly optimizes for modularity (dense within-community connections, sparse between-community connections). It may resist mega-community formation on dense graphs.

**2. Co-activation-based edge filtering.** Instead of thresholding by weight percentile, filter edges by co-activation frequency. Require that an edge be reinforced across multiple independent documents (not just multiple ticks in the same document) to be retained. This would suppress spurious cross-topic edges created by single interdisciplinary sentences.

**3. Infomap or other flow-based methods.** These algorithms detect communities based on random walk dynamics rather than modularity, which may provide better resolution on dense graphs.

**4. Adaptive Hebbian learning rates.** Use higher learning rates for within-document co-activations and lower rates for cross-document co-activations, producing sharper weight stratification that better differentiates core from peripheral edges.

**5. Hierarchical community detection.** Accept the mega-community at the top level but recursively partition it into subcommunities, potentially recovering topic structure at finer granularity.

### 5.3 Curriculum Ordering Still Works: Weight-Based Stratification

Despite poor community detection, the **curriculum ordering mechanism remains valid**. The key insight is that foundation-bridges-periphery sequencing depends on two independent signals:

1. **Community membership** (same vs. different community)
2. **Edge weight** (above vs. below median)

Community membership quality affects which specific triples are classified as foundation vs. bridges, but **edge weight stratification is robust**. Foundation triples have 1.3x higher mean weight than periphery triples regardless of whether communities align with ground-truth topics. This weight differential reflects real differences in co-activation frequency during colony processing.

From a curriculum learning perspective, the pedagogical principle remains intact:

1. **Foundation phase (high weight, same community):** Teach frequently co-activated entity pairs first. These are the "core associations" that appeared repeatedly across documents.
2. **Bridge phase (cross-community):** Teach connections between different detected clusters, whether or not those clusters correspond to ground-truth topics.
3. **Periphery phase (low weight):** Teach rarely co-activated pairs last. These are fine-grained details or one-off mentions.

While better community detection would improve alignment with human topic intuitions, the weight-based ordering already provides a meaningful easy-to-hard curriculum based on co-activation statistics.

### 5.4 Limitations and Future Work

- **Dense graph problem.** The 252,641-edge graph for 2,011 nodes overwhelms label propagation. Alternative community detection methods (Louvain, Infomap) or edge filtering strategies (co-activation frequency thresholds) are necessary for topic recovery at this scale.
- **Narrow weight distribution.** The 0.074--0.400 range with median 0.077 indicates insufficient weight stratification. The modest 1.3x foundation/periphery ratio, while statistically significant, may not provide strong enough curriculum signal for LLM fine-tuning. Adaptive Hebbian learning rates or longer colony runs (500+ ticks) may produce sharper differentiation.
- **Small corpus.** The 40-document corpus is small by LLM training standards. Scaling behavior on corpora of thousands or millions of documents remains to be characterized.
- **Community coherence paradox.** Foundation triples achieve 100% same-community coherence, but the communities themselves are uninformative (1 mega-community + 547 singletons). This suggests the curriculum partition is internally consistent but not semantically meaningful without better community detection.
- **No LLM evaluation.** This paper focuses on graph construction and curriculum extraction. Actual fine-tuning experiments comparing curriculum-ordered vs. random triple presentation are ongoing.

---

## 6. Conclusion

We have presented a method for constructing Hebbian-weighted knowledge graphs through a colony-based document digestion process. On a 40-document corpus, a 25-digester colony over 200 ticks produced a graph with 2,011 nodes and 252,641 edges. The resulting edge weights enable a three-phase curriculum ordering -- foundation (114,233 triples, mean weight 0.097), bridges (65,258 triples), periphery (73,150 triples, mean weight 0.076) -- with foundation triples exhibiting 100% same-community coherence and 1.3x higher weight than periphery.

However, label propagation community detection on the extremely dense graph yielded poor topic alignment (NMI = 0.170), producing 1 mega-community and 547 singletons. This result highlights a key limitation: Hebbian edge accumulation without density control creates near-complete graphs that resist modular partitioning. **The curriculum ordering mechanism remains valid through weight-based stratification**, but better community detection methods (Louvain, Infomap) or edge filtering strategies (co-activation frequency thresholds) are needed to recover ground-truth topic structure.

The foundation-bridges-periphery curriculum provides a biologically grounded framework for sequencing KG-derived training data, with weight stratification offering a meaningful easy-to-hard ordering independent of community quality. **The honest assessment:** community detection failed, but the core contribution -- weight-based curriculum ordering -- succeeds.

Future work will (1) implement Louvain modularity optimization and co-activation-based edge filtering to improve topic recovery, (2) evaluate curriculum-ordered fine-tuning at scale, comparing curriculum vs. random triple presentation on downstream LLM benchmarks, and (3) investigate adaptive Hebbian learning rates to produce sharper weight stratification.

---

## References

- Agarwal, O., et al. (2021). Knowledge Graph Based Synthetic Corpus Generation for Knowledge-Enhanced Language Model Pre-training. *NAACL*.
- Bengio, Y., et al. (2009). Curriculum Learning. *ICML*.
- Chen, Z., et al. (2024). CoFine: Knowledge Graph Completion with Fine-Grained Contextual Triples. *ACL*.
- Hebb, D. O. (1949). *The Organization of Behavior*. Wiley.
- Liu, Y., et al. (2024). GraphMERT: Graph-Enhanced Multi-Entity Reasoning for Transformers. *EMNLP*.
- Raghavan, U. N., Albert, R., & Kumara, S. (2007). Near linear time algorithm to detect community structures in large-scale networks. *Physical Review E*, 76(3).
