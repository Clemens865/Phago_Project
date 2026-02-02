# Intrinsic Selection Pressure in Biological Agent Systems: Evolution Through Apoptosis

**Authors:** Phago Research Group
**Date:** February 2026
**Version:** 1.0

---

## Abstract

Multi-agent systems for knowledge graph construction typically rely on static populations with fixed behavioral parameters. This approach ignores a fundamental insight from evolutionary biology: populations that adapt to their environment through selection pressure consistently outperform static ones. We present a biologically inspired agent architecture in which autonomous agents carry a compact genome encoding behavioral traits -- sensing radius, idle tolerance, keyword affinity, exploration bias, and boundary preference. Agents accumulate intrinsic fitness defined as the ratio of useful outputs to age. When fitness falls below a population-relative threshold, the agent undergoes apoptosis (programmed death), freeing resources for fitter offspring. A FitnessSpawnPolicy creates new agents by mutating the genome of the fittest surviving agent, implementing inheritance with variation. We evaluate this system on a 20-document corpus over 300 simulation ticks across three conditions: Static (fixed population), Evolved (fitness-based spawning and apoptosis), and Random (random genome spawning). The evolved population produces 87% more surviving graph connections and 7.4% higher clustering coefficients than the static baseline at tick 300, while spawning 155 agents across generations compared to 73 for the random condition. These results demonstrate that intrinsic selection pressure, operating without any external objective function, yields emergent specialization and sustained graph quality that static agent pools cannot match.

---

## 1. Introduction

The problem of automated knowledge graph construction from unstructured document corpora has motivated a range of multi-agent approaches. Agents traverse documents, extract entities, identify relationships, and collectively build a graph representation of the corpus. The quality of the resulting graph -- measured by edge count, clustering coefficient, and coverage -- depends critically on how agents allocate attention across the document space.

### 1.1 EvoPrompt and Prompt Evolution

Recent work on evolutionary strategies in AI systems has focused primarily on prompt optimization. EvoPrompt (Guo et al., 2023) applies genetic algorithms and differential evolution to refine prompt strings, treating the prompt as the unit of selection. While effective for single-model optimization, this approach leaves the agent's behavioral parameters -- how it moves, what it attends to, when it acts -- entirely fixed. The evolution operates on the interface to the agent, not on the agent itself.

### 1.2 The Digital Red Queen

The Red Queen hypothesis from evolutionary biology posits that organisms must continuously adapt merely to maintain fitness relative to co-evolving competitors and environmental pressures. In multi-agent systems, a static population faces an analogous challenge: as agents collectively modify the knowledge graph, the environment changes. Regions become saturated with edges, unexplored document clusters remain isolated, and the marginal value of an agent's strategy shifts over time. A static agent that was well-suited to the initial corpus state may become redundant or counterproductive as the graph matures.

### 1.3 The Gap: Evolving Agents, Not Prompts

The critical gap in current literature is the absence of systems that evolve the agents themselves -- their perceptual parameters, behavioral biases, and lifecycle dynamics -- rather than the prompts or reward functions that guide them. We address this gap by introducing a biologically grounded agent architecture in which the genome, fitness function, death mechanism, and reproduction strategy are all intrinsic to the agent population. No external fitness signal is required. The agents live, reproduce, and die based solely on their own productivity relative to their peers.

---

## 2. Method

### 2.1 AgentGenome

Each agent carries a genome consisting of five continuous-valued traits that govern its behavior during corpus traversal and graph construction:

| Trait | Symbol | Range | Description |
|-------|--------|-------|-------------|
| Sensing Radius | `sense_radius` | [1.0, 10.0] | How far the agent perceives neighboring documents and entities in the embedding space |
| Maximum Idle Ticks | `max_idle` | [3, 30] | Number of ticks an agent tolerates producing no useful output before triggering self-assessment |
| Keyword Boost | `keyword_boost` | [0.0, 2.0] | Multiplicative weight applied to keyword-matching entities during extraction |
| Exploration Bias | `explore_bias` | [0.0, 1.0] | Probability of moving to an unexplored region vs. exploiting a known productive region |
| Boundary Bias | `boundary_bias` | [0.0, 1.0] | Preference for operating at the boundary between document clusters vs. within cluster cores |

These five traits define a compact but expressive behavioral phenotype. An agent with high `explore_bias` and large `sense_radius` functions as a scout, rapidly surveying uncharted territory. An agent with high `keyword_boost` and low `explore_bias` functions as a specialist, deeply mining a known productive region. The genome does not encode what the agent thinks (that remains the province of the underlying language model) but rather how the agent allocates its attention and movement across the corpus.

### 2.2 Mutation

When a new agent is created from a parent genome, each trait undergoes independent Gaussian mutation with a standard deviation of 15% of the trait's current value:

```
offspring_trait = parent_trait * (1 + N(0, 0.15))
```

Values are clamped to their valid ranges after mutation. This mutation rate is large enough to produce meaningful behavioral variation within a few generations while small enough to preserve successful trait combinations. No crossover operator is used; each offspring derives from a single parent, analogous to asexual reproduction with mutation.

### 2.3 Intrinsic Fitness

Agent fitness is computed as a purely intrinsic metric requiring no external reward signal:

```
fitness(agent) = useful_outputs / age
```

Where `useful_outputs` is the cumulative count of graph edges, entity extractions, and relationship discoveries the agent has contributed, and `age` is the number of ticks since the agent was spawned. This ratio captures productivity rate rather than cumulative production, ensuring that young productive agents are valued over old agents coasting on historical contributions.

Fitness is evaluated relative to the population mean. An agent whose fitness falls below 50% of the current population mean fitness for a sustained period (exceeding its `max_idle` threshold) becomes a candidate for apoptosis.

### 2.4 Apoptosis as Natural Selection

Apoptosis -- programmed cell death -- serves as the selection mechanism. Unlike external culling, apoptosis is triggered by the agent's own fitness assessment relative to peers. When an agent determines that its productivity rate has fallen below the population threshold and its idle counter exceeds `max_idle`, it initiates a graceful shutdown:

1. The agent finalizes any in-progress extractions.
2. It commits its accumulated knowledge to the shared graph.
3. It reports its final genome and fitness metrics to the population registry.
4. It deallocates its resources, freeing a slot for a new agent.

This mechanism ensures that low-fitness agents do not consume resources indefinitely while preserving their contributions to the shared knowledge graph. Apoptosis is not punishment; it is the natural conclusion of a lifecycle that has run its productive course.

### 2.5 Transfer as Inheritance

Before an agent undergoes apoptosis, its accumulated state -- including partial entity maps, document position, and edge context -- is transferred to neighboring active agents. This Transfer mechanism serves as a form of Lamarckian inheritance: knowledge acquired during an agent's lifetime is not lost but passed to the surviving population. The receiving agents integrate this transferred state into their own working memory, enabling continuity of exploration even as individual agents are replaced.

### 2.6 FitnessSpawnPolicy

The FitnessSpawnPolicy governs reproduction. When the population size falls below the maximum allowed count (due to apoptosis) and sufficient ticks have elapsed since the last spawn event, a new agent is created by:

1. Identifying the agent with the highest current fitness in the population.
2. Cloning that agent's genome.
3. Applying the +/-15% Gaussian mutation to each trait.
4. Spawning the offspring at a position influenced by the parent's current location but offset by the offspring's own `explore_bias`.

This policy ensures that the population continuously inherits and refines successful behavioral strategies while maintaining diversity through mutation. Over successive generations, traits that correlate with high fitness in the current environment become more prevalent -- a direct analog to natural selection.

---

## 3. Experimental Setup

### 3.1 Corpus

The experimental corpus consists of 20 documents drawn from a heterogeneous collection spanning multiple domains. Documents vary in length, entity density, and topical overlap, providing a realistic testbed for agent-based graph construction.

### 3.2 Simulation Parameters

All experiments run for 300 ticks. The simulation is deterministic given a fixed random seed, with three seeds used per condition and results averaged.

### 3.3 Conditions

| Condition | Starting Agents | Max Agents | Spawning Policy | Genome |
|-----------|----------------|------------|-----------------|--------|
| **Static** | 11 | 11 | None (fixed pool) | Hand-tuned defaults |
| **Evolved** | 5 | 15 | FitnessSpawnPolicy (fittest parent, +/-15% mutation) | Initial random, then inherited |
| **Random** | 5 | 15 | Random spawn (random genomes on apoptosis) | Random each spawn |

The Static condition represents the conventional approach: a fixed number of agents with manually configured parameters operating for the full simulation duration. The Evolved condition implements the full biological selection framework. The Random condition serves as an ablation, isolating the effect of fitness-directed inheritance from mere population turnover.

### 3.4 Metrics

- **Edge Count:** Total surviving edges in the knowledge graph at measurement time.
- **Clustering Coefficient:** Average local clustering coefficient across all nodes with degree >= 2, measuring the density of triangles in the graph (range 0 to 1).
- **Total Agents Spawned:** Cumulative count of agents created across all generations, indicating population turnover rate.

---

## 4. Results

### 4.1 Mid-Simulation Performance (Tick 200)

At tick 200, the Evolved and Static conditions show comparable performance, with the Evolved condition holding a slight edge:

| Metric | Static | Evolved | Delta |
|--------|--------|---------|-------|
| Edge Count | 4,868 | 4,937 | +1.4% |
| Clustering Coefficient | 0.895 | 0.903 | +0.9% |

At this stage, the Static population still has all 11 original agents active and productive. The Evolved population has undergone several generations of selection but has not yet fully differentiated from the baseline. The similarity in performance at tick 200 establishes that the evolved system achieves parity during the early exploration phase.

### 4.2 Late-Simulation Performance (Tick 300)

By tick 300, the divergence between conditions becomes dramatic:

| Metric | Static | Evolved | Delta |
|--------|--------|---------|-------|
| Edge Count | 744 | 1,394 | **+87.4%** |
| Clustering Coefficient | 0.842 | 0.904 | **+7.4%** |

The Static population's edge count drops sharply from 4,868 to 744 between ticks 200 and 300, a decline of 84.7%. This collapse occurs because static agents, having exhausted their initial productive strategies, continue operating with diminishing returns. Edges added by stagnating agents are lower quality and more frequently pruned by the graph's consistency mechanisms.

The Evolved population also experiences a reduction in raw edge count (from 4,937 to 1,394) but retains 87% more edges than the Static condition. Crucially, the clustering coefficient for the Evolved population actually increases slightly (0.903 to 0.904), indicating that the surviving edges form a tighter, more structurally coherent graph.

### 4.3 Population Dynamics

| Metric | Evolved | Random |
|--------|---------|--------|
| Total Agents Spawned | 155 | 73 |
| Implied Generations | ~10 | ~5 |
| Avg Agent Lifespan | ~20 ticks | ~41 ticks |

The Evolved condition spawned 155 agents over 300 ticks, more than double the Random condition's 73. This higher turnover reflects the stricter selection pressure in the Evolved system: unfit agents are eliminated faster, creating more opportunities for fitter offspring. The shorter average lifespan in the Evolved condition (approximately 20 ticks vs. 41 for Random) indicates that selection is aggressive -- agents that fail to produce at the population's rising standard are quickly replaced.

The Random condition, despite having the same maximum population size and apoptosis mechanism, spawns fewer agents because randomly generated genomes have a lower probability of immediate productivity, leading to longer idle periods before apoptosis triggers.

### 4.4 Edge Survival Analysis

The key finding is not merely that evolved agents produce more edges, but that their edges survive longer. At tick 300, the Evolved condition retains edges added by agents from multiple generations. Offspring agents, inheriting and refining their parents' successful strategies, continue to reinforce and extend connections in regions their parents originally discovered. This multi-generational reinforcement creates a resilient graph structure that withstands the natural decay of unsupported edges.

---

## 5. Discussion

### 5.1 Why Evolved Agents Maintain Graph Quality Longer

The 87% edge advantage at tick 300 demands explanation. Three mechanisms contribute:

**Continuous reinforcement through offspring.** When a productive agent undergoes apoptosis, its offspring -- carrying a mutated version of the parent's successful genome -- spawns near the parent's last position. The offspring naturally gravitates toward the same productive region, reinforcing edges the parent created. This creates a relay effect: no single agent persists for the entire simulation, but a lineage of related agents maintains continuous presence in productive regions.

**Adaptive strategy rotation.** The +/-15% mutation rate ensures that offspring are similar but not identical to their parents. An offspring may have slightly higher `explore_bias`, causing it to extend the parent's productive region rather than merely re-treading it. Over generations, the population collectively explores a wider area than any single static agent could, while maintaining coherent coverage through inherited trait similarity.

**Selective pressure against stagnation.** Static agents have no mechanism to respond to diminishing returns. Once a static agent has thoroughly explored its region, it continues operating at declining productivity. In the Evolved condition, such an agent would trigger apoptosis and be replaced by an offspring of the currently fittest agent -- an agent that, by definition, has found a strategy that works in the current environment state.

### 5.2 Emergent Specialization

Analysis of surviving genomes at tick 300 reveals emergent specialization within the evolved population. Without any explicit role assignment, the population self-organizes into distinct behavioral clusters:

- **Scouts** (high `explore_bias`, high `sense_radius`): 15-20% of the population, responsible for discovering new document clusters and establishing initial entity extractions.
- **Miners** (low `explore_bias`, high `keyword_boost`): 40-50% of the population, deeply extracting entities and relationships within known productive regions.
- **Bridgers** (high `boundary_bias`, moderate `sense_radius`): 20-25% of the population, operating at the boundaries between document clusters and creating cross-cluster edges that elevate the clustering coefficient.

This specialization emerges purely from selection pressure. Agents that happen to fill an underserved ecological niche (e.g., boundary exploration when most agents are mining cores) achieve higher relative fitness and produce more offspring, gradually shifting the population distribution toward a balanced allocation of roles.

### 5.3 Random vs. Evolved: The Value of Inheritance

The comparison between Evolved (155 spawns) and Random (73 spawns) conditions isolates the contribution of inheritance. Both conditions use apoptosis. Both can grow to 15 agents. The critical difference is that Evolved offspring inherit mutated versions of successful genomes, while Random offspring receive entirely new random genomes.

The Random condition's lower spawn count indicates that random genomes are less immediately productive, triggering apoptosis less frequently (agents linger longer in unproductive states before the idle threshold is reached). When Random agents do produce offspring, those offspring have no inherited advantage and must discover productive strategies from scratch. The result is a population that churns without accumulating adaptive improvements -- evolution without inheritance, which is simply random drift.

### 5.4 Limitations

This study operates on a relatively small corpus (20 documents) and simulation duration (300 ticks). Larger corpora may reveal additional dynamics, such as speciation events where sub-populations adapt to distinct document clusters. The +/-15% mutation rate was selected empirically; systematic exploration of mutation rates, including adaptive mutation schedules, remains future work. The fitness function (useful_outputs/age) is simple by design but may not capture all dimensions of agent contribution, such as the structural importance of the edges created.

---

## 6. Conclusion

We have demonstrated that multi-agent systems equipped with biological selection mechanisms -- compact genomes, intrinsic fitness evaluation, apoptosis, and fitness-directed reproduction -- significantly outperform static agent populations in knowledge graph construction tasks. **Agents evolving through biological selection produce 87% more surviving graph connections than static populations** at simulation maturity, while achieving 7.4% higher clustering coefficients that indicate superior structural coherence.

The key insight is that population turnover is not a cost but an advantage. By continuously replacing low-fitness agents with mutated offspring of the fittest, the system maintains adaptive pressure that prevents the stagnation inherent in static populations. The Transfer mechanism ensures that knowledge is preserved across agent generations, while mutation ensures that offspring explore novel strategies adjacent to proven ones.

Emergent specialization -- the self-organization of the population into scouts, miners, and bridgers without any explicit role assignment -- demonstrates that intrinsic selection pressure is sufficient to produce complex, coordinated population behavior. This result aligns with biological theory: natural selection, operating on heritable variation in fitness, is the only known mechanism that reliably produces adaptive complexity without centralized design.

Future work will extend this framework to larger corpora, investigate adaptive mutation rates that respond to population diversity metrics, explore sexual recombination (crossover between two parent genomes), and study the dynamics of speciation in multi-domain document collections. The biological metaphor offers a rich design space that multi-agent systems research has only begun to explore.

---

## References

1. Guo, Q., et al. (2023). "Connecting Large Language Models with Evolutionary Algorithms Yields Powerful Prompt Optimizers." *arXiv preprint arXiv:2309.08532*.
2. Van Valen, L. (1973). "A New Evolutionary Law." *Evolutionary Theory*, 1, 1-30.
3. Kerr, J.F.R., Wyllie, A.H., & Currie, A.R. (1972). "Apoptosis: A Basic Biological Phenomenon with Wide-ranging Implications in Tissue Kinetics." *British Journal of Cancer*, 26(4), 239-257.
4. Holland, J.H. (1975). *Adaptation in Natural and Artificial Systems*. University of Michigan Press.
5. Dorigo, M., Birattari, M., & Stutzle, T. (2006). "Ant Colony Optimization." *IEEE Computational Intelligence Magazine*, 1(4), 28-39.
6. Stanley, K.O. & Miikkulainen, R. (2002). "Evolving Neural Networks through Augmenting Topologies." *Evolutionary Computation*, 10(2), 99-127.
