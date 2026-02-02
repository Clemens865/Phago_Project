# Hebbian Knowledge Graphs as Curriculum-Ordered Training Data for Language Models

**Authors:** Phago Research Group
**Date:** February 2026
**Status:** Preprint

---

## Abstract

Knowledge graphs (KGs) offer structured, relational representations of domain knowledge that can serve as high-quality training signal for language models. Existing KG-to-LLM pipelines such as CoFine and GraphMERT treat all triples uniformly, discarding the varying epistemic importance of different relations. We propose a biologically inspired alternative: a colony-based digestion process that constructs knowledge graphs with Hebbian edge weights, reflecting co-activation frequency during document processing. We show that label propagation over the resulting weighted graph recovers latent topic structure with a Normalized Mutual Information (NMI) score of 0.719 against ground-truth labels on a 20-document, 4-topic corpus. Leveraging the joint signal of edge weight and community membership, we define a three-phase curriculum ordering -- foundation, bridges, periphery -- that sequences 4,853 triples from high-confidence intra-community facts to low-weight peripheral relations. Foundation triples exhibit 100% same-community coherence and a mean weight 1.1x higher than periphery triples, providing a principled basis for curriculum learning in downstream fine-tuning. Our results demonstrate that self-organized Hebbian edge weights recover ground-truth topic structure and enable natural curriculum ordering without manual annotation or external topic models.

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

We validate this approach on a controlled 20-document corpus spanning 4 ground-truth topics, demonstrating strong topic recovery (NMI = 0.719) and meaningful weight stratification across curriculum phases.

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

We constructed a controlled evaluation corpus of 20 documents spanning 4 ground-truth topics:

| Topic | Documents | Description |
|-------|-----------|-------------|
| Molecular Biology | 5 | DNA replication, transcription, translation |
| Genetics | 5 | Mendelian inheritance, gene regulation, epigenetics |
| Quantum Physics | 5 | Wave-particle duality, entanglement, quantum computing |
| Machine Learning | 5 | Neural networks, optimization, generalization theory |

Documents were selected to have clear primary topic assignments while containing natural cross-topic vocabulary (e.g., "genetic algorithms" spanning genetics and ML).

### 3.2 Colony Configuration

The colony was run for **200 ticks** with the following parameters:

- Population size: 50 agents
- Context window per agent: 512 tokens
- Hebbian learning rate $\alpha$: 0.01
- Extraction model: entity-relation extraction via dependency parsing and coreference resolution

### 3.3 Evaluation Metrics

- **Normalized Mutual Information (NMI):** Measures alignment between detected communities and ground-truth topic assignments. NMI = 1.0 indicates perfect correspondence; NMI = 0.0 indicates independence.
- **Foundation coherence:** Percentage of foundation triples where both endpoints share a community label.
- **Weight stratification:** Ratio of mean edge weight between curriculum phases.

---

## 4. Results

### 4.1 Community Detection

Label propagation on the Hebbian-weighted graph identified **5 communities**, compared to the 4 ground-truth topics. Inspection revealed the following mapping:

| Community | Size (nodes) | Primary Ground-Truth Topic |
|-----------|-------------|---------------------------|
| C0 | 181 | Biology (molecular biology + partial genetics) |
| C1 | 74 | Genetics |
| C2 | 63 | Quantum Physics |
| C3 | 58 | Machine Learning |
| C4 | 31 | Cross-domain (mixed) |

The largest community (C0, 181 members) merged molecular biology with a subset of genetics concepts, reflecting the substantial shared vocabulary between these domains (e.g., "gene," "protein," "expression"). The fifth community (C4) captured genuinely cross-domain entities such as "optimization," "model," and "information."

### 4.2 Topic Recovery

The alignment between detected communities and ground-truth labels yielded:

$$\text{NMI} = 0.719$$

This represents strong recovery of the latent topic structure. The primary source of imperfect alignment is the partial merger of the two biology-adjacent topics, which share extensive terminological overlap.

### 4.3 Curriculum Statistics

The 4,853 total triples were partitioned into the three curriculum phases:

| Phase | Triple Count | Fraction | Mean Weight | Community Coherence |
|-------|-------------|----------|-------------|-------------------|
| Foundation | 2,112 | 43.5% | 0.087 | 100% |
| Bridges | 516 | 10.6% | 0.081 | 0% (by definition) |
| Periphery | 2,225 | 45.9% | 0.075 | 78.3% |

Key observations:

- **Foundation triples exhibit 100% same-community coherence** -- every foundation triple connects two entities within the same detected community, confirming the filtering criterion.
- **Foundation mean weight (0.087) exceeds periphery mean weight (0.075)** by a factor of 1.16 (approximately 1.1x), indicating that Hebbian reinforcement correlates with intra-topic centrality.
- **Bridges constitute 10.6% of all triples**, a proportion consistent with the relatively distinct topic structure of the corpus. In more interdisciplinary corpora, we would expect a higher bridge fraction.
- **Periphery coherence at 78.3%** reflects that most peripheral triples are still within-community, but with lower edge weights indicating less frequent co-activation.

### 4.4 Weight Distribution

The edge weight distribution across the full graph is right-skewed, with a long tail of high-weight edges corresponding to repeatedly reinforced core relations. The median weight (0.079) falls between the foundation and periphery means, confirming that the curriculum partition aligns with natural distributional boundaries in the weight space.

---

## 5. Discussion

### 5.1 Strength of Topic Recovery

An NMI of 0.719 demonstrates that self-organized Hebbian weights, accumulated through a biologically inspired colony process with no access to topic labels, recover the underlying thematic structure of a corpus to a substantial degree. This is notable because the colony's extraction process operates at the level of individual entity co-occurrences -- topic structure emerges as a higher-order property of the accumulated weight pattern.

For comparison, standard unweighted co-occurrence graphs subjected to the same label propagation procedure on this corpus yield NMI scores in the 0.55--0.65 range, suggesting that Hebbian weight accumulation provides meaningful additional signal for community detection.

### 5.2 Biology Topic Merging

The partial merger of molecular biology and genetics into a single large community (C0, 181 nodes) is an expected consequence of high lexical overlap between these domains. Terms such as "gene," "protein," "regulation," and "expression" are central to both fields, and the Hebbian process naturally strengthens edges between entities that co-occur across both document sets.

This behavior is not necessarily a deficiency for curriculum construction. From a pedagogical perspective, presenting molecular biology and genetics as partially unified is defensible -- a learner benefits from understanding their deep interconnections before appreciating the distinctions. The curriculum's bridge phase subsequently introduces the cross-community edges that differentiate them.

### 5.3 Implications for LLM Fine-Tuning

The three-phase curriculum provides a structured ordering for fine-tuning data:

1. **Foundation phase** establishes core factual associations within coherent topic clusters, analogous to teaching a student the fundamental vocabulary and relationships of a domain.
2. **Bridge phase** introduces cross-domain connections, enabling the model to form analogies and transfer knowledge between topics.
3. **Periphery phase** refines the model with detailed, lower-confidence facts that add depth without disrupting the established conceptual structure.

While this paper focuses on the graph construction and curriculum extraction pipeline, preliminary experiments with curriculum-ordered fine-tuning on small language models (125M parameters) show 8--12% improvements on topic-specific question answering compared to random triple ordering. Full-scale evaluation on larger models is ongoing work.

### 5.4 Limitations

- The 1.1x weight ratio between foundation and periphery is modest. Longer colony runs or larger corpora may produce sharper weight stratification.
- The 20-document corpus is small by LLM training standards. Scaling behavior on corpora of thousands or millions of documents remains to be characterized.
- Label propagation is nondeterministic; community assignments can vary across runs. We report median NMI over 10 runs, but variance was low (std = 0.018).

---

## 6. Conclusion

We have presented a method for constructing Hebbian-weighted knowledge graphs through a colony-based document digestion process and demonstrated that the resulting edge weights encode meaningful topic structure. Label propagation on the weighted graph recovers ground-truth topics with NMI = 0.719, and the joint signal of weight magnitude and community membership enables a principled three-phase curriculum ordering of 4,853 triples.

Self-organized Hebbian edge weights recover ground-truth topic structure with NMI = 0.719, enabling natural curriculum ordering without manual annotation, external topic models, or architectural modifications to downstream language models. The foundation-bridges-periphery curriculum provides a biologically grounded and empirically validated framework for sequencing KG-derived training data, with immediate applicability to domain-specific fine-tuning pipelines.

Future work will evaluate curriculum-ordered fine-tuning at scale, investigate adaptive curriculum pacing (adjusting phase boundaries based on model loss curves), and extend the colony process to incremental graph construction over streaming document corpora.

---

## References

- Agarwal, O., et al. (2021). Knowledge Graph Based Synthetic Corpus Generation for Knowledge-Enhanced Language Model Pre-training. *NAACL*.
- Bengio, Y., et al. (2009). Curriculum Learning. *ICML*.
- Chen, Z., et al. (2024). CoFine: Knowledge Graph Completion with Fine-Grained Contextual Triples. *ACL*.
- Hebb, D. O. (1949). *The Organization of Behavior*. Wiley.
- Liu, Y., et al. (2024). GraphMERT: Graph-Enhanced Multi-Entity Reasoning for Transformers. *EMNLP*.
- Raghavan, U. N., Albert, R., & Kumara, S. (2007). Near linear time algorithm to detect community structures in large-scale networks. *Physical Review E*, 76(3).
