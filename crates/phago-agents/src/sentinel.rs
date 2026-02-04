//! Sentinel Agent — anomaly detection through negative selection.
//!
//! During "maturation", the Sentinel observes normal concept distributions
//! in the knowledge graph and builds a self-model. After maturation,
//! it classifies new inputs as Self (normal) or NonSelf (anomalous).
//!
//! Biological analog: T-cell maturation in the thymus. Developing T-cells
//! are shown self-antigens. Those that react to self are destroyed. Only
//! cells that ignore self and react to non-self survive.
//!
//! The Sentinel learns what "normal" looks like (finite, learnable) and
//! flags everything that deviates — without needing to enumerate threats.

use phago_core::agent::Agent;
use phago_core::primitives::{Apoptose, Digest, Negate, Sense};
use phago_core::primitives::symbiose::AgentProfile;
use phago_core::substrate::Substrate;
use phago_core::types::*;
use std::collections::HashMap;

/// How many ticks of observation before the self-model is considered mature.
const MATURATION_TICKS: u64 = 10;
/// Deviation threshold for NonSelf classification (0.0-1.0).
const ANOMALY_THRESHOLD: f64 = 0.5;
/// Maximum anomalies to report per scan cycle.
const MAX_ANOMALIES_PER_SCAN: usize = 10;

/// Statistical self-model: distribution of concept frequencies.
#[derive(Debug, Clone)]
pub struct ConceptSelfModel {
    /// Expected concept frequency distribution.
    concept_freq: HashMap<String, f64>,
    /// Total observations used to build the model.
    observation_count: u64,
    /// Mean edge weight observed.
    mean_edge_weight: f64,
    /// Standard deviation of edge weights.
    edge_weight_std: f64,
}

impl ConceptSelfModel {
    fn new() -> Self {
        Self {
            concept_freq: HashMap::new(),
            observation_count: 0,
            mean_edge_weight: 0.0,
            edge_weight_std: 0.0,
        }
    }

    /// Get all concepts in the self-model.
    fn concepts(&self) -> Vec<&String> {
        self.concept_freq.keys().collect()
    }

    /// Observe a concept with a given frequency.
    fn observe(&mut self, concept: &str, freq: f64) {
        *self.concept_freq.entry(concept.to_string()).or_insert(0.0) += freq;
        self.observation_count += 1;
    }
}

/// State machine for the Sentinel.
#[derive(Debug, Clone, PartialEq)]
enum SentinelState {
    /// Building the self-model from graph observations.
    Maturing(u64), // ticks remaining
    /// Mature — actively scanning for anomalies.
    Scanning,
    /// Detected an anomaly — emitting signal.
    Alerting(String), // anomaly description
}

/// The Sentinel agent — the immune system's anomaly detector.
pub struct Sentinel {
    id: AgentId,
    position: Position,
    age_ticks: Tick,
    state: SentinelState,

    // Self-model
    self_model: ConceptSelfModel,

    // Anomaly tracking
    anomalies_detected: u64,
    last_scan_tick: Tick,

    // Digestion (minimal — Sentinel doesn't digest documents)
    engulfed: Option<String>,
    fragments: Vec<String>,

    // Configuration
    sense_radius: f64,
    max_idle_ticks: u64,
    idle_ticks: u64,
    scan_interval: u64,
}

impl Sentinel {
    pub fn new(position: Position) -> Self {
        Self {
            id: AgentId::new(),
            position,
            age_ticks: 0,
            state: SentinelState::Maturing(MATURATION_TICKS),
            self_model: ConceptSelfModel::new(),
            anomalies_detected: 0,
            last_scan_tick: 0,
            engulfed: None,
            fragments: Vec::new(),
            sense_radius: 50.0,
            max_idle_ticks: 200, // Very patient
            idle_ticks: 0,
            scan_interval: 5,
        }
    }

    /// Create a sentinel with a deterministic ID (for testing).
    pub fn with_seed(position: Position, seed: u64) -> Self {
        Self {
            id: AgentId::from_seed(seed),
            position,
            age_ticks: 0,
            state: SentinelState::Maturing(MATURATION_TICKS),
            self_model: ConceptSelfModel::new(),
            anomalies_detected: 0,
            last_scan_tick: 0,
            engulfed: None,
            fragments: Vec::new(),
            sense_radius: 50.0,
            max_idle_ticks: 200,
            idle_ticks: 0,
            scan_interval: 5,
        }
    }

    pub fn anomalies_detected(&self) -> u64 {
        self.anomalies_detected
    }

    /// Build the self-model by observing the current graph state.
    fn observe_graph(&mut self, substrate: &dyn Substrate) {
        let all_nodes = substrate.all_nodes();
        let mut concept_counts: HashMap<String, u64> = HashMap::new();

        for node_id in &all_nodes {
            if let Some(node) = substrate.get_node(node_id) {
                if node.node_type == NodeType::Concept {
                    *concept_counts.entry(node.label.clone()).or_insert(0) += node.access_count;
                }
            }
        }

        // Update frequency distribution
        let total: u64 = concept_counts.values().sum();
        if total > 0 {
            for (label, count) in &concept_counts {
                let freq = *count as f64 / total as f64;
                let existing = self.self_model.concept_freq.entry(label.clone()).or_insert(0.0);
                // Running average
                *existing = (*existing * self.self_model.observation_count as f64 + freq)
                    / (self.self_model.observation_count + 1) as f64;
            }
        }

        // Compute edge weight statistics
        let all_edges = substrate.all_edges();
        if !all_edges.is_empty() {
            let weights: Vec<f64> = all_edges.iter().map(|(_, _, e)| e.weight).collect();
            let mean = weights.iter().sum::<f64>() / weights.len() as f64;
            let variance = weights.iter().map(|w| (w - mean).powi(2)).sum::<f64>() / weights.len() as f64;
            let std = variance.sqrt();

            // Running average
            let n = self.self_model.observation_count as f64;
            self.self_model.mean_edge_weight =
                (self.self_model.mean_edge_weight * n + mean) / (n + 1.0);
            self.self_model.edge_weight_std =
                (self.self_model.edge_weight_std * n + std) / (n + 1.0);
        }

        self.self_model.observation_count += 1;
    }

    /// Scan the graph for anomalies by comparing current state to self-model.
    fn scan_for_anomalies(&self, substrate: &dyn Substrate) -> Vec<String> {
        let mut anomalies = Vec::new();

        if self.self_model.observation_count == 0 {
            return anomalies;
        }

        // Check for concept nodes that deviate from expected distribution
        let all_nodes = substrate.all_nodes();
        let mut current_counts: HashMap<String, u64> = HashMap::new();
        let mut total_count: u64 = 0;

        for node_id in &all_nodes {
            if let Some(node) = substrate.get_node(node_id) {
                if node.node_type == NodeType::Concept {
                    *current_counts.entry(node.label.clone()).or_insert(0) += node.access_count;
                    total_count += node.access_count;
                }
            }
        }

        if total_count == 0 {
            return anomalies;
        }

        // Find concepts that are new (not in self-model) or have unusual frequency
        for (label, count) in &current_counts {
            let current_freq = *count as f64 / total_count as f64;

            match self.self_model.concept_freq.get(label) {
                None => {
                    // Concept not in self-model — it's novel
                    if current_freq > 0.01 {
                        anomalies.push(format!(
                            "Novel concept '{}' not in self-model (freq: {:.3})",
                            label, current_freq
                        ));
                    }
                }
                Some(&expected_freq) => {
                    // Check deviation from expected frequency
                    if expected_freq > 0.0 {
                        let deviation = (current_freq - expected_freq).abs() / expected_freq;
                        if deviation > ANOMALY_THRESHOLD {
                            anomalies.push(format!(
                                "Concept '{}' deviates from self-model: expected {:.3}, got {:.3} (deviation: {:.1}%)",
                                label, expected_freq, current_freq, deviation * 100.0
                            ));
                        }
                    }
                }
            }
        }

        // Check for edge weight anomalies (only if we have enough observations)
        if self.self_model.observation_count >= 5 {
            let all_edges = substrate.all_edges();
            let mean = self.self_model.mean_edge_weight;
            let std = self.self_model.edge_weight_std.max(0.05);

            for (from_id, to_id, edge) in &all_edges {
                let z_score = (edge.weight - mean).abs() / std;
                if z_score > 3.0 {
                    let from_label = substrate.get_node(from_id).map(|n| n.label.as_str()).unwrap_or("?");
                    let to_label = substrate.get_node(to_id).map(|n| n.label.as_str()).unwrap_or("?");
                    anomalies.push(format!(
                        "Edge '{}'-'{}' has anomalous weight {:.3} (z-score: {:.1})",
                        from_label, to_label, edge.weight, z_score
                    ));
                }
            }
        }

        anomalies
    }
}

// --- Trait Implementations ---

impl Digest for Sentinel {
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
        self.engulfed.take().map(|s| vec![s]).unwrap_or_default()
    }

    fn present(&self) -> Vec<String> {
        self.fragments.clone()
    }
}

impl Apoptose for Sentinel {
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
            useful_outputs: self.anomalies_detected,
            final_fragments: Vec::new(),
            cause: DeathCause::SelfAssessed(self.self_assess()),
        }
    }
}

impl Sense for Sentinel {
    fn sense_radius(&self) -> f64 {
        self.sense_radius
    }

    fn sense_position(&self) -> Position {
        self.position
    }

    fn gradient(&self, _substrate: &dyn Substrate) -> Vec<Gradient> {
        Vec::new() // Sentinels don't chase signals
    }

    fn orient(&self, _gradients: &[Gradient]) -> Orientation {
        Orientation::Stay // Sentinels are stationary
    }
}

impl Negate for Sentinel {
    type Observation = Vec<(String, u64)>; // (concept_label, access_count)
    type SelfModel = ConceptSelfModel;

    fn learn_self(&mut self, observations: &[Self::Observation]) {
        for obs in observations {
            let total: u64 = obs.iter().map(|(_, c)| c).sum();
            if total == 0 {
                continue;
            }
            for (label, count) in obs {
                let freq = *count as f64 / total as f64;
                let existing = self.self_model.concept_freq.entry(label.clone()).or_insert(0.0);
                let n = self.self_model.observation_count as f64;
                *existing = (*existing * n + freq) / (n + 1.0);
            }
            self.self_model.observation_count += 1;
        }
    }

    fn self_model(&self) -> &ConceptSelfModel {
        &self.self_model
    }

    fn is_mature(&self) -> bool {
        !matches!(self.state, SentinelState::Maturing(_))
    }

    fn classify(&self, observation: &Self::Observation) -> Classification {
        if !self.is_mature() {
            return Classification::Unknown;
        }

        let total: u64 = observation.iter().map(|(_, c)| c).sum();
        if total == 0 {
            return Classification::Unknown;
        }

        let mut max_deviation = 0.0f64;
        for (label, count) in observation {
            let freq = *count as f64 / total as f64;
            if let Some(&expected) = self.self_model.concept_freq.get(label) {
                if expected > 0.0 {
                    let deviation = (freq - expected).abs() / expected;
                    max_deviation = max_deviation.max(deviation);
                }
            } else {
                // Novel concept — high deviation
                max_deviation = max_deviation.max(1.0);
            }
        }

        if max_deviation > ANOMALY_THRESHOLD {
            Classification::NonSelf(max_deviation.min(1.0))
        } else {
            Classification::IsSelf
        }
    }
}

impl Agent for Sentinel {
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
        "sentinel"
    }

    fn tick(&mut self, substrate: &dyn Substrate) -> AgentAction {
        self.age_ticks += 1;

        if self.should_die() {
            return AgentAction::Apoptose;
        }

        // Clone state to avoid borrow conflict with observe_graph
        let current_state = self.state.clone();

        match &current_state {
            SentinelState::Maturing(remaining) => {
                // Observe the graph to build self-model
                self.observe_graph(substrate);

                if *remaining <= 1 {
                    self.state = SentinelState::Scanning;
                } else {
                    self.state = SentinelState::Maturing(remaining - 1);
                }
                self.idle_ticks = 0; // Maturation counts as activity
                AgentAction::Idle
            }

            SentinelState::Scanning => {
                // Periodically scan for anomalies
                if self.age_ticks - self.last_scan_tick >= self.scan_interval {
                    self.last_scan_tick = self.age_ticks;

                    let mut anomalies = self.scan_for_anomalies(substrate);
                    // Cap to most significant anomalies per cycle
                    anomalies.truncate(MAX_ANOMALIES_PER_SCAN);

                    if !anomalies.is_empty() {
                        let description = anomalies.join("; ");
                        self.anomalies_detected += anomalies.len() as u64;
                        self.state = SentinelState::Alerting(description.clone());
                        self.idle_ticks = 0;

                        // Present anomalies as insight fragments
                        let presentations: Vec<FragmentPresentation> = anomalies
                            .iter()
                            .map(|a| FragmentPresentation {
                                label: format!("[ANOMALY] {}", a),
                                source_document: DocumentId::new(),
                                position: self.position,
                                node_type: NodeType::Anomaly,
                            })
                            .collect();

                        return AgentAction::PresentFragments(presentations);
                    }
                }

                self.idle_ticks += 1;
                AgentAction::Idle
            }

            SentinelState::Alerting(_description) => {
                // Emit anomaly signal to attract synthesizers
                self.state = SentinelState::Scanning;
                AgentAction::Emit(Signal::new(
                    SignalType::Anomaly,
                    1.0,
                    self.position,
                    self.id,
                    self.age_ticks,
                ))
            }
        }
    }

    fn age(&self) -> Tick {
        self.age_ticks
    }

    fn export_vocabulary(&self) -> Option<Vec<u8>> {
        if self.self_model.concept_freq.is_empty() {
            return None;
        }
        let terms: Vec<String> = self.self_model.concept_freq.keys().cloned().collect();
        let cap = VocabularyCapability {
            terms,
            origin: self.id,
            document_count: self.self_model.observation_count,
        };
        serde_json::to_vec(&cap).ok()
    }

    fn profile(&self) -> AgentProfile {
        AgentProfile {
            id: self.id,
            agent_type: "sentinel".to_string(),
            capabilities: Vec::new(),
            health: self.self_assess(),
        }
    }
}

// --- Serialization ---

use crate::serialize::{
    SerializableAgent, SerializedAgent,
    SentinelState as SerializedSentinelState,
};

impl SerializableAgent for Sentinel {
    fn export_state(&self) -> SerializedAgent {
        SerializedAgent::Sentinel(SerializedSentinelState {
            id: self.id,
            position: self.position,
            age_ticks: self.age_ticks,
            idle_ticks: self.idle_ticks,
            anomalies_detected: self.anomalies_detected,
            last_scan_tick: self.last_scan_tick,
            self_model_concepts: self.self_model.concepts().into_iter().cloned().collect(),
            sense_radius: self.sense_radius,
            max_idle_ticks: self.max_idle_ticks,
            scan_interval: self.scan_interval,
        })
    }

    fn from_state(state: &SerializedAgent) -> Option<Self> {
        match state {
            SerializedAgent::Sentinel(s) => {
                let mut sentinel = Sentinel {
                    id: s.id,
                    position: s.position,
                    age_ticks: s.age_ticks,
                    state: SentinelState::Scanning,
                    self_model: ConceptSelfModel::new(),
                    anomalies_detected: s.anomalies_detected,
                    last_scan_tick: s.last_scan_tick,
                    engulfed: None,
                    fragments: Vec::new(),
                    sense_radius: s.sense_radius,
                    max_idle_ticks: s.max_idle_ticks,
                    idle_ticks: s.idle_ticks,
                    scan_interval: s.scan_interval,
                };
                // Restore self-model concepts with default frequency
                for concept in &s.self_model_concepts {
                    sentinel.self_model.observe(concept, 1.0);
                }
                Some(sentinel)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_starts_maturing() {
        let sentinel = Sentinel::new(Position::new(0.0, 0.0));
        assert!(!sentinel.is_mature());
        assert_eq!(sentinel.agent_type(), "sentinel");
    }

    #[test]
    fn classify_unknown_when_immature() {
        let sentinel = Sentinel::new(Position::new(0.0, 0.0));
        let obs = vec![("cell".to_string(), 5)];
        assert_eq!(sentinel.classify(&obs), Classification::Unknown);
    }

    #[test]
    fn self_model_learns_from_observations() {
        let mut sentinel = Sentinel::new(Position::new(0.0, 0.0));
        let obs = vec![
            vec![("cell".to_string(), 10u64), ("membrane".to_string(), 8)],
            vec![("cell".to_string(), 12), ("membrane".to_string(), 7)],
        ];
        sentinel.learn_self(&obs);

        assert!(sentinel.self_model().concept_freq.contains_key("cell"));
        assert!(sentinel.self_model().concept_freq.contains_key("membrane"));
        assert_eq!(sentinel.self_model().observation_count, 2);
    }
}
