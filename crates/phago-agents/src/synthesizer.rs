//! Synthesizer Agent — collective intelligence through emergence.
//!
//! The Synthesizer is dormant until quorum is reached — enough agents
//! have deposited enough traces and concepts in a region. At quorum,
//! the Synthesizer activates and performs cross-document analysis:
//!
//! - Detects concepts that appear across multiple documents (bridge concepts)
//! - Identifies clusters of highly-connected concepts (topic clusters)
//! - Generates Insight nodes that represent emergent understanding
//!
//! Biological analog: collective bacterial behavior that only activates
//! when autoinducer concentration exceeds the quorum threshold. Individual
//! bacteria cannot perform these behaviors — they are emergent properties
//! of the collective.

use phago_core::agent::Agent;
use phago_core::primitives::{Apoptose, Digest, Emerge, Sense};
use phago_core::substrate::Substrate;
use phago_core::types::*;

/// Configuration for the Synthesizer.
const QUORUM_THRESHOLD: f64 = 3.0;
const MIN_BRIDGE_ACCESS: u64 = 2;
const MIN_CLUSTER_SIZE: usize = 3;
const MIN_CLUSTER_WEIGHT: f64 = 0.15;

/// State machine for the Synthesizer.
#[derive(Debug, Clone, PartialEq)]
enum SynthesizerState {
    /// Dormant — waiting for quorum.
    Dormant,
    /// Quorum reached — analyzing the graph.
    Analyzing,
    /// Presenting insights to the graph.
    Presenting(Vec<InsightData>),
    /// Cooldown after producing insights.
    Cooldown(u64),
}

/// An insight discovered by the Synthesizer.
#[derive(Debug, Clone, PartialEq)]
pub struct InsightData {
    pub label: String,
    pub insight_type: InsightType,
    pub related_concepts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InsightType {
    /// A concept that bridges multiple document clusters.
    BridgeConcept { access_count: u64 },
    /// A tightly connected cluster of concepts.
    TopicCluster { size: usize, avg_weight: f64 },
}

/// The Synthesizer agent — emergent collective intelligence.
pub struct Synthesizer {
    id: AgentId,
    position: Position,
    age_ticks: Tick,
    state: SynthesizerState,

    // Emerge tracking
    insights_produced: u64,

    // Digestion (required by Agent trait but Synthesizer digests insights, not documents)
    engulfed: Option<String>,
    fragments: Vec<String>,

    // Configuration
    sense_radius: f64,
    cooldown_ticks: u64,
    max_idle_ticks: u64,
    idle_ticks: u64,
}

impl Synthesizer {
    pub fn new(position: Position) -> Self {
        Self {
            id: AgentId::new(),
            position,
            age_ticks: 0,
            state: SynthesizerState::Dormant,
            insights_produced: 0,
            engulfed: None,
            fragments: Vec::new(),
            sense_radius: 50.0, // Large radius — synthesizers survey the whole substrate
            cooldown_ticks: 10,
            max_idle_ticks: 100, // Patient — waits longer than digesters
            idle_ticks: 0,
        }
    }

    /// Total insights produced in lifetime.
    pub fn insights_produced(&self) -> u64 {
        self.insights_produced
    }

    /// Analyze the knowledge graph for cross-document patterns.
    ///
    /// This is the core emergence logic. It finds patterns that no
    /// individual digester could detect because they require seeing
    /// the full graph structure.
    fn analyze_graph(&self, substrate: &dyn Substrate) -> Vec<InsightData> {
        let mut insights = Vec::new();

        // --- Bridge Concepts ---
        // Concepts accessed by multiple documents (access_count > 1)
        // are "bridges" — they connect different knowledge domains.
        let all_nodes = substrate.all_nodes();
        for node_id in &all_nodes {
            if let Some(node) = substrate.get_node(node_id) {
                if node.access_count >= MIN_BRIDGE_ACCESS && node.node_type == NodeType::Concept {
                    // Check if this bridge hasn't been reported yet
                    let existing_insights = substrate.all_nodes().iter().any(|nid| {
                        substrate.get_node(nid).map_or(false, |n| {
                            n.node_type == NodeType::Insight
                                && n.label.contains(&node.label)
                        })
                    });

                    if !existing_insights {
                        // Find what this concept is connected to
                        let neighbors = substrate.neighbors(node_id);
                        let connected: Vec<String> = neighbors
                            .iter()
                            .filter_map(|(nid, _)| {
                                substrate.get_node(nid).map(|n| n.label.clone())
                            })
                            .take(5)
                            .collect();

                        insights.push(InsightData {
                            label: format!(
                                "Bridge: '{}' connects {} document contexts",
                                node.label, node.access_count
                            ),
                            insight_type: InsightType::BridgeConcept {
                                access_count: node.access_count,
                            },
                            related_concepts: connected,
                        });
                    }
                }
            }
        }

        // --- Topic Clusters ---
        // Find groups of tightly connected concepts (avg edge weight above threshold).
        // Use a simple greedy approach: for each high-access node, collect its
        // strongly-connected neighbors.
        let mut reported_clusters: Vec<Vec<String>> = Vec::new();

        for node_id in &all_nodes {
            if let Some(node) = substrate.get_node(node_id) {
                if node.node_type != NodeType::Concept {
                    continue;
                }

                let neighbors = substrate.neighbors(node_id);
                let strong_neighbors: Vec<(String, f64)> = neighbors
                    .iter()
                    .filter_map(|(nid, edge)| {
                        if edge.weight >= MIN_CLUSTER_WEIGHT {
                            substrate.get_node(nid).map(|n| (n.label.clone(), edge.weight))
                        } else {
                            None
                        }
                    })
                    .collect();

                if strong_neighbors.len() >= MIN_CLUSTER_SIZE {
                    let mut cluster_labels: Vec<String> = strong_neighbors
                        .iter()
                        .map(|(label, _)| label.clone())
                        .collect();
                    cluster_labels.sort();

                    // Check if we already reported a similar cluster
                    let already_reported = reported_clusters.iter().any(|existing| {
                        let overlap = cluster_labels
                            .iter()
                            .filter(|l| existing.contains(l))
                            .count();
                        overlap > existing.len() / 2
                    });

                    if !already_reported {
                        let avg_weight: f64 = strong_neighbors.iter().map(|(_, w)| w).sum::<f64>()
                            / strong_neighbors.len() as f64;

                        // Check no existing insight for this cluster
                        let cluster_key = format!("Cluster: {}", node.label);
                        let exists = substrate.all_nodes().iter().any(|nid| {
                            substrate.get_node(nid).map_or(false, |n| {
                                n.node_type == NodeType::Insight && n.label == cluster_key
                            })
                        });

                        if !exists {
                            insights.push(InsightData {
                                label: cluster_key,
                                insight_type: InsightType::TopicCluster {
                                    size: strong_neighbors.len(),
                                    avg_weight,
                                },
                                related_concepts: cluster_labels.clone(),
                            });
                            reported_clusters.push(cluster_labels);
                        }
                    }
                }
            }
        }

        insights
    }
}

// --- Trait Implementations ---

impl Digest for Synthesizer {
    type Input = String;
    type Fragment = String;
    type Presentation = Vec<String>;

    fn engulf(&mut self, input: String) -> DigestionResult {
        if input.trim().is_empty() {
            return DigestionResult::Indigestible;
        }
        self.engulfed = Some(input);
        DigestionResult::Engulfed
    }

    fn lyse(&mut self) -> Vec<String> {
        self.engulfed
            .take()
            .map(|s| vec![s])
            .unwrap_or_default()
    }

    fn present(&self) -> Vec<String> {
        self.fragments.clone()
    }
}

impl Apoptose for Synthesizer {
    fn self_assess(&self) -> CellHealth {
        if self.idle_ticks >= self.max_idle_ticks {
            CellHealth::Senescent
        } else if self.idle_ticks >= self.max_idle_ticks / 2 {
            CellHealth::Stressed
        } else {
            CellHealth::Healthy
        }
    }

    fn prepare_death_signal(&self) -> DeathSignal {
        DeathSignal {
            agent_id: self.id,
            total_ticks: self.age_ticks,
            useful_outputs: self.insights_produced,
            final_fragments: Vec::new(),
            cause: DeathCause::SelfAssessed(self.self_assess()),
        }
    }
}

impl Sense for Synthesizer {
    fn sense_radius(&self) -> f64 {
        self.sense_radius
    }

    fn sense_position(&self) -> Position {
        self.position
    }

    fn gradient(&self, substrate: &dyn Substrate) -> Vec<Gradient> {
        // Synthesizer doesn't chase gradients — it surveys the whole area
        let _ = substrate;
        Vec::new()
    }

    fn orient(&self, _gradients: &[Gradient]) -> Orientation {
        Orientation::Stay // Synthesizers don't move
    }
}

impl Emerge for Synthesizer {
    type EmergentBehavior = Vec<InsightData>;

    fn signal_density(&self, substrate: &dyn Substrate) -> f64 {
        // Count digestion traces in sensing radius — this is our quorum signal
        let nearby_signals = substrate.signals_near(&self.position, self.sense_radius);
        let trace_count = nearby_signals.len();

        // Also count concept nodes — more concepts = more material to synthesize
        let node_count = substrate.node_count();

        // Quorum is based on both agent activity (traces) and knowledge density (nodes)
        (trace_count as f64) * 0.3 + (node_count as f64) * 0.1
    }

    fn quorum_threshold(&self) -> f64 {
        QUORUM_THRESHOLD
    }

    fn emergent_behavior(&self) -> Option<Vec<InsightData>> {
        // This is called by tick() when quorum is reached
        None // We compute insights in tick() directly
    }

    fn contribute(&self) -> Contribution {
        Contribution {
            agent_id: self.id,
            data: format!("insights:{}", self.insights_produced).into_bytes(),
        }
    }
}

impl Agent for Synthesizer {
    fn id(&self) -> AgentId {
        self.id
    }

    fn position(&self) -> Position {
        self.position
    }

    fn set_position(&mut self, position: Position) {
        self.position = position;
    }

    fn agent_type(&self) -> &str {
        "synthesizer"
    }

    fn tick(&mut self, substrate: &dyn Substrate) -> AgentAction {
        self.age_ticks += 1;

        if self.should_die() {
            return AgentAction::Apoptose;
        }

        match &self.state {
            SynthesizerState::Dormant => {
                // Check quorum
                let density = self.signal_density(substrate);
                if density >= self.quorum_threshold() {
                    self.state = SynthesizerState::Analyzing;
                    self.idle_ticks = 0;
                    // Emit quorum signal to alert other agents
                    AgentAction::Emit(Signal::new(
                        SignalType::Quorum,
                        1.0,
                        self.position,
                        self.id,
                        self.age_ticks,
                    ))
                } else {
                    self.idle_ticks += 1;
                    AgentAction::Idle
                }
            }

            SynthesizerState::Analyzing => {
                // Perform collective analysis
                let mut insights = self.analyze_graph(substrate);

                // Cap to top 10 most significant insights per cycle
                // Sort bridges by access_count (desc), clusters by size (desc)
                insights.sort_by(|a, b| {
                    let score_a = match &a.insight_type {
                        InsightType::BridgeConcept { access_count } => *access_count as f64,
                        InsightType::TopicCluster { size, avg_weight } => *size as f64 * avg_weight,
                    };
                    let score_b = match &b.insight_type {
                        InsightType::BridgeConcept { access_count } => *access_count as f64,
                        InsightType::TopicCluster { size, avg_weight } => *size as f64 * avg_weight,
                    };
                    score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
                });
                insights.truncate(10);

                if insights.is_empty() {
                    // Nothing new to report — go back to dormant
                    self.state = SynthesizerState::Dormant;
                    self.idle_ticks += 1;
                    AgentAction::Idle
                } else {
                    self.state = SynthesizerState::Presenting(insights.clone());
                    self.insights_produced += insights.len() as u64;

                    // Present insights as fragment presentations
                    let presentations: Vec<FragmentPresentation> = insights
                        .iter()
                        .map(|insight| {
                            let label = match &insight.insight_type {
                                InsightType::BridgeConcept { access_count } => {
                                    format!("[BRIDGE:{}] {}", access_count, insight.label)
                                }
                                InsightType::TopicCluster { size, avg_weight } => {
                                    format!(
                                        "[CLUSTER:{}/w{:.2}] {}",
                                        size, avg_weight, insight.label
                                    )
                                }
                            };
                            FragmentPresentation {
                                label,
                                source_document: DocumentId::new(),
                                position: self.position,
                                node_type: NodeType::Insight,
                            }
                        })
                        .collect();

                    AgentAction::PresentFragments(presentations)
                }
            }

            SynthesizerState::Presenting(_insights) => {
                // Emit insight signal and enter cooldown
                self.state = SynthesizerState::Cooldown(self.cooldown_ticks);
                AgentAction::Emit(Signal::new(
                    SignalType::Insight,
                    1.0,
                    self.position,
                    self.id,
                    self.age_ticks,
                ))
            }

            SynthesizerState::Cooldown(remaining) => {
                if *remaining == 0 {
                    self.state = SynthesizerState::Dormant;
                    AgentAction::Idle
                } else {
                    self.state = SynthesizerState::Cooldown(remaining - 1);
                    AgentAction::Idle
                }
            }
        }
    }

    fn age(&self) -> Tick {
        self.age_ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesizer_starts_dormant() {
        let synth = Synthesizer::new(Position::new(0.0, 0.0));
        assert_eq!(synth.state, SynthesizerState::Dormant);
        assert_eq!(synth.insights_produced(), 0);
    }

    #[test]
    fn synthesizer_type_name() {
        let synth = Synthesizer::new(Position::new(0.0, 0.0));
        assert_eq!(synth.agent_type(), "synthesizer");
    }
}
