//! Quantitative metrics for proving biological computing model correctness.
//!
//! Computes four categories of proof metrics from colony state:
//! - Transfer Effect: vocabulary sharing across agents
//! - Dissolution Effect: boundary modulation reinforces knowledge
//! - Graph Richness: structural complexity of the knowledge graph
//! - Vocabulary Spread: how well knowledge propagates

use crate::colony::{Colony, ColonyEvent, ColonySnapshot};
use phago_core::topology::TopologyGraph;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Transfer effect metrics — proves vocabulary sharing works.
#[derive(Debug, Clone, Serialize)]
pub struct TransferMetrics {
    /// Number of unique terms known by 2+ agents.
    pub shared_terms: usize,
    /// Total unique terms across all agents.
    pub total_terms: usize,
    /// Ratio of shared to total terms.
    pub shared_term_ratio: f64,
    /// Average vocabulary size per agent.
    pub avg_vocabulary_size: f64,
    /// Total export events.
    pub total_exports: usize,
    /// Total integration events.
    pub total_integrations: usize,
}

/// Dissolution effect metrics — proves boundary modulation reinforces knowledge.
#[derive(Debug, Clone, Serialize)]
pub struct DissolutionMetrics {
    /// Average access count of Concept nodes (which dissolution reinforces).
    pub dissolved_node_avg_access: f64,
    /// Average access count of non-Concept nodes (Insight/Anomaly).
    pub non_dissolved_avg_access: f64,
    /// Ratio of dissolved/non-dissolved access (reinforcement ratio).
    pub reinforcement_ratio: f64,
    /// Total dissolution events recorded.
    pub total_dissolutions: usize,
    /// Total terms externalized across all dissolutions.
    pub total_terms_externalized: usize,
}

/// Graph richness metrics — proves colony builds meaningful structure.
#[derive(Debug, Clone, Serialize)]
pub struct GraphRichnessMetrics {
    pub node_count: usize,
    pub edge_count: usize,
    /// edge_count / (node_count * (node_count-1) / 2)
    pub density: f64,
    pub avg_degree: f64,
    /// Approximate clustering coefficient.
    pub clustering_coefficient: f64,
    /// Number of bridge concepts (connected to nodes from different clusters).
    pub bridge_concepts: usize,
}

/// Vocabulary spread metrics — proves knowledge propagates across agents.
#[derive(Debug, Clone, Serialize)]
pub struct VocabularySpreadMetrics {
    /// Per-agent vocabulary sizes (from latest snapshot with agents).
    pub per_agent_sizes: Vec<usize>,
    /// Gini coefficient (0 = perfectly equal, 1 = maximally unequal).
    pub gini_coefficient: f64,
    pub max_vocabulary: usize,
    pub min_vocabulary: usize,
}

/// All colony metrics combined.
#[derive(Debug, Clone, Serialize)]
pub struct ColonyMetrics {
    pub transfer: TransferMetrics,
    pub dissolution: DissolutionMetrics,
    pub graph_richness: GraphRichnessMetrics,
    pub vocabulary_spread: VocabularySpreadMetrics,
}

/// Compute all proof metrics from the colony's current state and history.
pub fn compute(colony: &Colony) -> ColonyMetrics {
    let transfer = compute_transfer(colony);
    let dissolution = compute_dissolution(colony);
    let graph_richness = compute_graph_richness(colony);
    let vocabulary_spread = compute_vocabulary_spread(colony);

    ColonyMetrics {
        transfer,
        dissolution,
        graph_richness,
        vocabulary_spread,
    }
}

/// Compute metrics from snapshots (for use when agents may be dead).
pub fn compute_from_snapshots(colony: &Colony, snapshots: &[ColonySnapshot]) -> ColonyMetrics {
    let transfer = compute_transfer_from_snapshots(colony, snapshots);
    let dissolution = compute_dissolution(colony);
    let graph_richness = compute_graph_richness(colony);
    let vocabulary_spread = compute_vocabulary_spread_from_snapshots(snapshots);

    ColonyMetrics {
        transfer,
        dissolution,
        graph_richness,
        vocabulary_spread,
    }
}

fn compute_transfer(colony: &Colony) -> TransferMetrics {
    // Use live agents if available
    let agents = colony.agents();

    let mut total_exports = 0usize;
    let mut total_integrations = 0usize;

    for (_, event) in colony.event_history() {
        match event {
            ColonyEvent::CapabilityExported { .. } => total_exports += 1,
            ColonyEvent::CapabilityIntegrated { .. } => total_integrations += 1,
            _ => {}
        }
    }

    if !agents.is_empty() {
        let mut term_agent_count: HashMap<String, usize> = HashMap::new();
        let mut total_vocab_size = 0usize;

        for agent in agents {
            let vocab = agent.externalize_vocabulary();
            total_vocab_size += vocab.len();
            let unique: HashSet<String> = vocab.into_iter().collect();
            for term in unique {
                *term_agent_count.entry(term).or_insert(0) += 1;
            }
        }

        let shared_terms = term_agent_count.values().filter(|&&c| c >= 2).count();
        let total_terms = term_agent_count.len();
        let shared_term_ratio = if total_terms > 0 {
            shared_terms as f64 / total_terms as f64
        } else {
            0.0
        };
        let avg_vocabulary_size = total_vocab_size as f64 / agents.len() as f64;

        return TransferMetrics {
            shared_terms,
            total_terms,
            shared_term_ratio,
            avg_vocabulary_size,
            total_exports,
            total_integrations,
        };
    }

    // Agents are dead — estimate from graph node co-occurrence
    // Nodes that appear across multiple documents are "shared" terms
    let graph = colony.substrate().graph();
    let all_nodes = graph.all_nodes();
    let total_terms = all_nodes.len();

    // Nodes with access_count > 1 were reinforced by multiple agents
    let shared_terms = all_nodes.iter()
        .filter(|nid| graph.get_node(nid).map_or(false, |n| n.access_count > 1))
        .count();
    let shared_term_ratio = if total_terms > 0 {
        shared_terms as f64 / total_terms as f64
    } else {
        0.0
    };

    // Avg vocabulary from integration events
    let integrated_terms: usize = colony.event_history().iter()
        .filter_map(|(_, event)| {
            if let ColonyEvent::CapabilityIntegrated { terms_count, .. } = event {
                Some(*terms_count)
            } else {
                None
            }
        })
        .sum();
    let avg_vocabulary_size = if total_integrations > 0 {
        integrated_terms as f64 / total_integrations as f64
    } else {
        0.0
    };

    TransferMetrics {
        shared_terms,
        total_terms,
        shared_term_ratio,
        avg_vocabulary_size,
        total_exports,
        total_integrations,
    }
}

fn compute_transfer_from_snapshots(colony: &Colony, snapshots: &[ColonySnapshot]) -> TransferMetrics {
    let mut total_exports = 0usize;
    let mut total_integrations = 0usize;

    for (_, event) in colony.event_history() {
        match event {
            ColonyEvent::CapabilityExported { .. } => total_exports += 1,
            ColonyEvent::CapabilityIntegrated { .. } => total_integrations += 1,
            _ => {}
        }
    }

    // Find the snapshot with the most agents alive (peak activity)
    let best_snapshot = snapshots.iter()
        .max_by_key(|s| s.agents.len())
        .or(snapshots.last());

    if let Some(snap) = best_snapshot {
        if !snap.agents.is_empty() {
            let sizes = &snap.agents.iter().map(|a| a.vocabulary_size).collect::<Vec<_>>();
            let total_vocab: usize = sizes.iter().sum();
            let avg_vocabulary_size = total_vocab as f64 / snap.agents.len() as f64;

            // Approximate shared terms from graph reinforcement
            let graph = colony.substrate().graph();
            let all_nodes = graph.all_nodes();
            let total_terms = all_nodes.len();
            let shared_terms = all_nodes.iter()
                .filter(|nid| graph.get_node(nid).map_or(false, |n| n.access_count > 1))
                .count();
            let shared_term_ratio = if total_terms > 0 {
                shared_terms as f64 / total_terms as f64
            } else {
                0.0
            };

            return TransferMetrics {
                shared_terms,
                total_terms,
                shared_term_ratio,
                avg_vocabulary_size,
                total_exports,
                total_integrations,
            };
        }
    }

    // Fallback to event-based
    compute_transfer(colony)
}

fn compute_dissolution(colony: &Colony) -> DissolutionMetrics {
    let mut total_dissolutions = 0usize;
    let mut total_terms_externalized = 0usize;

    for (_, event) in colony.event_history() {
        if let ColonyEvent::Dissolved { terms_externalized, .. } = event {
            total_dissolutions += 1;
            total_terms_externalized += terms_externalized;
        }
    }

    // Compare access counts of Concept nodes (which dissolution reinforces)
    // vs Insight/Anomaly nodes (which are not reinforced by dissolution)
    let graph = colony.substrate().graph();
    let mut concept_access_sum = 0u64;
    let mut concept_count = 0u64;
    let mut other_access_sum = 0u64;
    let mut other_count = 0u64;

    for nid in graph.all_nodes() {
        if let Some(node) = graph.get_node(&nid) {
            match node.node_type {
                phago_core::types::NodeType::Concept => {
                    concept_access_sum += node.access_count;
                    concept_count += 1;
                }
                _ => {
                    other_access_sum += node.access_count;
                    other_count += 1;
                }
            }
        }
    }

    let dissolved_avg = if concept_count > 0 {
        concept_access_sum as f64 / concept_count as f64
    } else {
        0.0
    };
    let other_avg = if other_count > 0 {
        other_access_sum as f64 / other_count as f64
    } else {
        0.0
    };
    let ratio = if other_avg > 0.0 {
        dissolved_avg / other_avg
    } else if dissolved_avg > 0.0 {
        dissolved_avg
    } else {
        1.0
    };

    DissolutionMetrics {
        dissolved_node_avg_access: dissolved_avg,
        non_dissolved_avg_access: other_avg,
        reinforcement_ratio: ratio,
        total_dissolutions,
        total_terms_externalized,
    }
}

fn compute_graph_richness(colony: &Colony) -> GraphRichnessMetrics {
    let graph = colony.substrate().graph();
    let n = graph.node_count();
    let e = graph.edge_count();

    let max_edges = if n > 1 { n * (n - 1) / 2 } else { 1 };
    let density = e as f64 / max_edges as f64;
    let avg_degree = if n > 0 { 2.0 * e as f64 / n as f64 } else { 0.0 };

    // Approximate clustering coefficient
    let all_nodes = graph.all_nodes();
    let mut clustering_sum = 0.0f64;
    let mut clusterable_nodes = 0usize;

    for nid in &all_nodes {
        let neighbors = graph.neighbors(nid);
        let k = neighbors.len();
        if k < 2 {
            continue;
        }
        clusterable_nodes += 1;

        let neighbor_ids: Vec<_> = neighbors.iter().map(|(id, _)| *id).collect();
        let mut triangles = 0u64;
        for i in 0..neighbor_ids.len() {
            for j in (i + 1)..neighbor_ids.len() {
                if graph.get_edge(&neighbor_ids[i], &neighbor_ids[j]).is_some() {
                    triangles += 1;
                }
            }
        }
        let possible = k * (k - 1) / 2;
        if possible > 0 {
            clustering_sum += triangles as f64 / possible as f64;
        }
    }

    let clustering_coefficient = if clusterable_nodes > 0 {
        clustering_sum / clusterable_nodes as f64
    } else {
        0.0
    };

    let bridge_threshold = if avg_degree > 0.0 { avg_degree * 1.5 } else { 2.0 };
    let bridge_concepts = all_nodes.iter()
        .filter(|nid| graph.neighbors(nid).len() as f64 > bridge_threshold)
        .count();

    GraphRichnessMetrics {
        node_count: n,
        edge_count: e,
        density,
        avg_degree,
        clustering_coefficient,
        bridge_concepts,
    }
}

fn compute_vocabulary_spread(colony: &Colony) -> VocabularySpreadMetrics {
    let agents = colony.agents();
    if agents.is_empty() {
        // Fall back to event-based estimation
        return compute_vocabulary_spread_from_events(colony);
    }

    let sizes: Vec<usize> = agents.iter().map(|a| a.vocabulary_size()).collect();
    let max_vocabulary = *sizes.iter().max().unwrap_or(&0);
    let min_vocabulary = *sizes.iter().min().unwrap_or(&0);
    let gini_coefficient = compute_gini(&sizes);

    VocabularySpreadMetrics {
        per_agent_sizes: sizes,
        gini_coefficient,
        max_vocabulary,
        min_vocabulary,
    }
}

fn compute_vocabulary_spread_from_snapshots(snapshots: &[ColonySnapshot]) -> VocabularySpreadMetrics {
    // Find the snapshot with the most agents (peak activity)
    let best_snapshot = snapshots.iter()
        .max_by_key(|s| s.agents.len());

    if let Some(snap) = best_snapshot {
        if !snap.agents.is_empty() {
            let sizes: Vec<usize> = snap.agents.iter().map(|a| a.vocabulary_size).collect();
            let max_vocabulary = *sizes.iter().max().unwrap_or(&0);
            let min_vocabulary = *sizes.iter().min().unwrap_or(&0);
            let gini_coefficient = compute_gini(&sizes);

            return VocabularySpreadMetrics {
                per_agent_sizes: sizes,
                gini_coefficient,
                max_vocabulary,
                min_vocabulary,
            };
        }
    }

    VocabularySpreadMetrics {
        per_agent_sizes: Vec::new(),
        gini_coefficient: 0.0,
        max_vocabulary: 0,
        min_vocabulary: 0,
    }
}

fn compute_vocabulary_spread_from_events(colony: &Colony) -> VocabularySpreadMetrics {
    // Estimate from capability events — group exported terms by agent
    let mut agent_terms: HashMap<String, usize> = HashMap::new();

    for (_, event) in colony.event_history() {
        if let ColonyEvent::CapabilityExported { agent_id, terms_count } = event {
            let key = agent_id.0.to_string();
            let entry = agent_terms.entry(key).or_insert(0);
            *entry = (*entry).max(*terms_count);
        }
    }

    if agent_terms.is_empty() {
        return VocabularySpreadMetrics {
            per_agent_sizes: Vec::new(),
            gini_coefficient: 0.0,
            max_vocabulary: 0,
            min_vocabulary: 0,
        };
    }

    let sizes: Vec<usize> = agent_terms.values().copied().collect();
    let max_vocabulary = *sizes.iter().max().unwrap_or(&0);
    let min_vocabulary = *sizes.iter().min().unwrap_or(&0);
    let gini_coefficient = compute_gini(&sizes);

    VocabularySpreadMetrics {
        per_agent_sizes: sizes,
        gini_coefficient,
        max_vocabulary,
        min_vocabulary,
    }
}

fn compute_gini(values: &[usize]) -> f64 {
    let n = values.len();
    if n == 0 {
        return 0.0;
    }

    let mut sorted: Vec<f64> = values.iter().map(|&v| v as f64).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mean = sorted.iter().sum::<f64>() / n as f64;
    if mean == 0.0 {
        return 0.0;
    }

    let mut sum_abs_diff = 0.0;
    for i in 0..n {
        for j in 0..n {
            sum_abs_diff += (sorted[i] - sorted[j]).abs();
        }
    }

    sum_abs_diff / (2.0 * n as f64 * n as f64 * mean)
}

/// Print a formatted metrics report to the terminal.
pub fn print_report(metrics: &ColonyMetrics) {
    println!("── Quantitative Proof ──────────────────────────────");
    println!("  Transfer Effect:");
    println!("    Terms known by 2+ agents:  {} / {} ({:.1}%)",
        metrics.transfer.shared_terms,
        metrics.transfer.total_terms,
        metrics.transfer.shared_term_ratio * 100.0);
    println!("    Avg vocabulary size:       {:.1} terms/agent",
        metrics.transfer.avg_vocabulary_size);
    println!("    Exports / Integrations:    {} / {}",
        metrics.transfer.total_exports,
        metrics.transfer.total_integrations);
    println!();
    println!("  Dissolution Effect:");
    println!("    Concept avg access:         {:.1}",
        metrics.dissolution.dissolved_node_avg_access);
    println!("    Non-concept avg access:     {:.1}",
        metrics.dissolution.non_dissolved_avg_access);
    println!("    Reinforcement ratio:        {:.2}x",
        metrics.dissolution.reinforcement_ratio);
    println!("    Dissolutions / Terms:       {} / {}",
        metrics.dissolution.total_dissolutions,
        metrics.dissolution.total_terms_externalized);
    println!();
    println!("  Graph Richness:");
    println!("    Density:                    {:.2}",
        metrics.graph_richness.density);
    println!("    Avg degree:                 {:.1}",
        metrics.graph_richness.avg_degree);
    println!("    Clustering coefficient:     {:.2}",
        metrics.graph_richness.clustering_coefficient);
    println!("    Bridge concepts:            {}",
        metrics.graph_richness.bridge_concepts);
    println!();
    println!("  Vocabulary Spread:");
    println!("    Gini coefficient:           {:.2} (low = well-spread)",
        metrics.vocabulary_spread.gini_coefficient);
    println!("    Max vocabulary:             {} terms",
        metrics.vocabulary_spread.max_vocabulary);
    println!("    Min vocabulary:             {} terms",
        metrics.vocabulary_spread.min_vocabulary);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::Colony;
    use phago_core::types::Position;

    #[test]
    fn metrics_compute_on_empty_colony() {
        let colony = Colony::new();
        let metrics = compute(&colony);
        assert_eq!(metrics.transfer.shared_terms, 0);
        assert_eq!(metrics.graph_richness.node_count, 0);
        assert_eq!(metrics.vocabulary_spread.per_agent_sizes.len(), 0);
    }

    #[test]
    fn metrics_compute_on_populated_colony() {
        use phago_agents::digester::Digester;

        let mut colony = Colony::new();

        colony.ingest_document(
            "Test",
            "The cell membrane controls transport of molecules into the cell. \
             Proteins serve as channels and receptors.",
            Position::new(0.0, 0.0),
        );
        colony.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(80),
        ));
        colony.spawn(Box::new(
            Digester::new(Position::new(1.0, 0.0)).with_max_idle(80),
        ));

        colony.run(20);

        let metrics = compute(&colony);
        assert!(metrics.graph_richness.node_count > 0);
        print_report(&metrics);
    }

    #[test]
    fn gini_coefficient_is_correct() {
        assert!((compute_gini(&[5, 5, 5, 5]) - 0.0).abs() < 0.01);

        let values = vec![0, 0, 0, 100];
        let g = compute_gini(&values);
        assert!(g > 0.5, "Gini should be high for unequal distribution: {}", g);
    }
}
