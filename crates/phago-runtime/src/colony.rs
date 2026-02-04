//! Colony — agent lifecycle management.
//!
//! The colony is the organism. It manages the birth, life, and death
//! of agents, runs the tick-based simulation loop, and coordinates
//! agent access to the shared substrate.
//!
//! Each tick:
//! 1. All agents sense the substrate and decide an action
//! 2. The colony processes all actions (moves, digestions, signals)
//! 3. Dead agents are removed, death signals collected
//! 4. The substrate decays signals and traces
//! 5. The tick counter advances

use crate::substrate_impl::SubstrateImpl;
use phago_agents::fitness::FitnessTracker;
use phago_core::agent::Agent;
use phago_core::semantic::{compute_semantic_weight, SemanticWiringConfig};
use phago_core::substrate::Substrate;
use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use serde::{Deserialize, Serialize};
use serde_json;

/// Event emitted by the colony during simulation.
#[derive(Debug, Clone, Serialize)]
pub enum ColonyEvent {
    /// An agent was spawned.
    Spawned { id: AgentId, agent_type: String },
    /// An agent moved to a new position.
    Moved { id: AgentId, to: Position },
    /// An agent engulfed a document.
    Engulfed { id: AgentId, document: DocumentId },
    /// An agent presented fragments to the knowledge graph.
    Presented { id: AgentId, fragment_count: usize, node_ids: Vec<NodeId> },
    /// An agent deposited a trace.
    Deposited { id: AgentId, location: SubstrateLocation },
    /// An agent wired connections in the graph.
    Wired { id: AgentId, connection_count: usize },
    /// An agent triggered apoptosis.
    Died { signal: DeathSignal },
    /// A tick completed.
    TickComplete { tick: Tick, alive: usize, dead_this_tick: usize },
    /// An agent exported its vocabulary as a capability deposit.
    CapabilityExported { agent_id: AgentId, terms_count: usize },
    /// An agent integrated vocabulary from a capability deposit.
    CapabilityIntegrated { agent_id: AgentId, from_agent: AgentId, terms_count: usize },
    /// An agent absorbed another through symbiosis.
    Symbiosis { host: AgentId, absorbed: AgentId, host_type: String, absorbed_type: String },
    /// An agent's boundary dissolved, externalizing vocabulary.
    Dissolved { agent_id: AgentId, permeability: f64, terms_externalized: usize },
}

/// Statistics about the colony.
#[derive(Debug, Clone, Serialize)]
pub struct ColonyStats {
    pub tick: Tick,
    pub agents_alive: usize,
    pub agents_died: usize,
    pub total_spawned: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub total_signals: usize,
    pub documents_total: usize,
    pub documents_digested: usize,
}

/// A serializable snapshot of an agent's state.
#[derive(Debug, Clone, Serialize)]
pub struct AgentSnapshot {
    pub id: AgentId,
    pub agent_type: String,
    pub position: Position,
    pub age: Tick,
    pub permeability: f64,
    pub vocabulary_size: usize,
}

/// A serializable snapshot of a graph node.
#[derive(Debug, Clone, Serialize)]
pub struct NodeSnapshot {
    pub id: NodeId,
    pub label: String,
    pub node_type: NodeType,
    pub position: Position,
    pub access_count: u64,
}

/// A serializable snapshot of a graph edge.
#[derive(Debug, Clone, Serialize)]
pub struct EdgeSnapshot {
    pub from_label: String,
    pub to_label: String,
    pub weight: f64,
    pub co_activations: u64,
}

/// A complete serializable snapshot of the colony at a point in time.
#[derive(Debug, Clone, Serialize)]
pub struct ColonySnapshot {
    pub tick: Tick,
    pub agents: Vec<AgentSnapshot>,
    pub nodes: Vec<NodeSnapshot>,
    pub edges: Vec<EdgeSnapshot>,
    pub stats: ColonyStats,
}

/// Configuration for colony simulation parameters.
///
/// This struct contains all the tunable parameters that were previously
/// hardcoded in Colony::new(). Use with Colony::from_config() to create
/// a colony with custom settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColonyConfig {
    /// Rate at which signals decay per tick (default: 0.05).
    pub signal_decay_rate: f64,
    /// Threshold below which signals are removed (default: 0.01).
    pub signal_removal_threshold: f64,
    /// Rate at which traces decay per tick (default: 0.02).
    pub trace_decay_rate: f64,
    /// Threshold below which traces are removed (default: 0.01).
    pub trace_removal_threshold: f64,
    /// Rate at which edges decay per tick (default: 0.005).
    pub edge_decay_rate: f64,
    /// Threshold below which edges are pruned (default: 0.05).
    pub edge_prune_threshold: f64,
    /// Factor for staleness-based decay (default: 1.5).
    pub staleness_factor: f64,
    /// Number of ticks before edges mature and become decay-resistant (default: 50).
    pub maturation_ticks: u64,
    /// Maximum number of edges per node before pruning (default: 30).
    pub max_edge_degree: usize,
    /// Semantic wiring configuration.
    pub semantic_wiring: SemanticWiringConfig,
}

impl Default for ColonyConfig {
    fn default() -> Self {
        Self {
            signal_decay_rate: 0.05,
            signal_removal_threshold: 0.01,
            trace_decay_rate: 0.02,
            trace_removal_threshold: 0.01,
            edge_decay_rate: 0.005,
            edge_prune_threshold: 0.05,
            staleness_factor: 1.5,
            maturation_ticks: 50,
            max_edge_degree: 30,
            semantic_wiring: SemanticWiringConfig::default(),
        }
    }
}

/// The colony — manages agent lifecycle and simulation.
pub struct Colony {
    substrate: SubstrateImpl,
    agents: Vec<Box<dyn Agent<Input = String, Fragment = String, Presentation = Vec<String>>>>,
    death_signals: Vec<DeathSignal>,
    event_history: Vec<(Tick, ColonyEvent)>,
    total_spawned: usize,
    total_died: usize,
    fitness_tracker: FitnessTracker,

    // Configuration
    signal_decay_rate: f64,
    signal_removal_threshold: f64,
    trace_decay_rate: f64,
    trace_removal_threshold: f64,
    edge_decay_rate: f64,
    edge_prune_threshold: f64,
    staleness_factor: f64,
    maturation_ticks: u64,
    max_edge_degree: usize,
    semantic_wiring: SemanticWiringConfig,
}

impl Colony {
    /// Create a new colony with default configuration.
    pub fn new() -> Self {
        Self::from_config(ColonyConfig::default())
    }

    /// Create a new colony with the specified configuration.
    pub fn from_config(config: ColonyConfig) -> Self {
        Self {
            substrate: SubstrateImpl::new(),
            agents: Vec::new(),
            death_signals: Vec::new(),
            event_history: Vec::new(),
            total_spawned: 0,
            total_died: 0,
            fitness_tracker: FitnessTracker::new(),
            signal_decay_rate: config.signal_decay_rate,
            signal_removal_threshold: config.signal_removal_threshold,
            trace_decay_rate: config.trace_decay_rate,
            trace_removal_threshold: config.trace_removal_threshold,
            edge_decay_rate: config.edge_decay_rate,
            edge_prune_threshold: config.edge_prune_threshold,
            staleness_factor: config.staleness_factor,
            maturation_ticks: config.maturation_ticks,
            max_edge_degree: config.max_edge_degree,
            semantic_wiring: config.semantic_wiring,
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> ColonyConfig {
        ColonyConfig {
            signal_decay_rate: self.signal_decay_rate,
            signal_removal_threshold: self.signal_removal_threshold,
            trace_decay_rate: self.trace_decay_rate,
            trace_removal_threshold: self.trace_removal_threshold,
            edge_decay_rate: self.edge_decay_rate,
            edge_prune_threshold: self.edge_prune_threshold,
            staleness_factor: self.staleness_factor,
            maturation_ticks: self.maturation_ticks,
            max_edge_degree: self.max_edge_degree,
            semantic_wiring: self.semantic_wiring.clone(),
        }
    }

    /// Configure semantic wiring for embedding-based edge weights.
    pub fn with_semantic_wiring(mut self, config: SemanticWiringConfig) -> Self {
        self.semantic_wiring = config;
        self
    }

    /// Get the current semantic wiring configuration.
    pub fn semantic_wiring_config(&self) -> &SemanticWiringConfig {
        &self.semantic_wiring
    }

    /// Set the semantic wiring configuration.
    pub fn set_semantic_wiring(&mut self, config: SemanticWiringConfig) {
        self.semantic_wiring = config;
    }

    /// Spawn an agent into the colony.
    pub fn spawn(
        &mut self,
        agent: Box<dyn Agent<Input = String, Fragment = String, Presentation = Vec<String>>>,
    ) -> AgentId {
        let id = agent.id();
        self.total_spawned += 1;
        self.fitness_tracker.register(id, 0);
        self.agents.push(agent);
        id
    }

    /// Ingest a document into the substrate.
    ///
    /// Places the document at the given position and emits an Input signal
    /// to attract nearby agents via chemotaxis.
    pub fn ingest_document(&mut self, title: &str, content: &str, position: Position) -> DocumentId {
        let doc = Document {
            id: DocumentId::new(),
            title: title.to_string(),
            content: content.to_string(),
            position,
            digested: false,
        };
        let doc_id = doc.id;
        let doc_pos = doc.position;

        self.substrate.add_document(doc);

        // Emit input signal to attract agents
        self.substrate.emit_signal(Signal::new(
            SignalType::Input,
            1.0,
            doc_pos,
            AgentId::new(), // System-emitted
            self.substrate.current_tick(),
        ));

        doc_id
    }

    /// Run a single simulation tick.
    pub fn tick(&mut self) -> Vec<ColonyEvent> {
        let mut events = Vec::new();
        let mut actions: Vec<(usize, AgentAction)> = Vec::new();

        // Phase 1: All agents sense and decide
        for (idx, agent) in self.agents.iter_mut().enumerate() {
            let action = agent.tick(&self.substrate);
            actions.push((idx, action));
        }

        // Phase 2: Process actions
        let mut to_die = Vec::new();
        let mut symbiotic_deaths: Vec<(usize, AgentId)> = Vec::new(); // (idx, absorber_id)

        for (idx, action) in actions {
            match action {
                AgentAction::Move(pos) => {
                    self.agents[idx].set_position(pos);
                    events.push(ColonyEvent::Moved {
                        id: self.agents[idx].id(),
                        to: pos,
                    });
                }

                AgentAction::EngulfDocument(doc_id) => {
                    // Try to consume the document from substrate
                    if let Some(content) = self.substrate.consume_document(&doc_id) {
                        self.agents[idx].engulf(content);
                        // Also set the document context via downcast
                        // (The agent's state machine will handle digestion next tick)
                        events.push(ColonyEvent::Engulfed {
                            id: self.agents[idx].id(),
                            document: doc_id,
                        });
                    }
                }

                AgentAction::PresentFragments(fragments) => {
                    let agent_id = self.agents[idx].id();
                    let tick = self.substrate.current_tick();
                    let mut node_ids = Vec::new();

                    for frag in &fragments {
                        // Check if this concept already exists in the graph
                        let existing = self.substrate.graph().find_nodes_by_label(&frag.label);
                        let node_id = if let Some(&existing_id) = existing.first() {
                            // Reinforce existing node
                            if let Some(node) = self.substrate.graph_mut().get_node_mut(&existing_id) {
                                node.access_count += 1;
                            }
                            existing_id
                        } else {
                            // Create new node with the type specified by the agent
                            let node = NodeData {
                                id: NodeId::new(),
                                label: frag.label.clone(),
                                node_type: frag.node_type.clone(),
                                position: frag.position,
                                access_count: 1,
                                created_tick: tick,
                                embedding: None,
                            };
                            self.substrate.add_node(node)
                        };
                        node_ids.push(node_id);
                    }

                    // Wire co-occurring concepts (from same document)
                    // Only wire Concept nodes — Insight/Anomaly nodes don't co-occur
                    //
                    // Co-activation gating (Hebbian LTP model):
                    // - First co-occurrence: create a TENTATIVE edge with low weight (0.1)
                    // - Subsequent co-occurrences: reinforce to full weight (+0.1 per hit)
                    // - Only edges reinforced by multiple documents survive synaptic pruning
                    // This reduces the dense graph problem: single-doc edges are weak
                    // and decay quickly unless reinforced by cross-document co-occurrence.
                    //
                    // Semantic wiring (Phase 9.3):
                    // - If nodes have embeddings, modulate edge weight by similarity
                    // - weight = base_weight * (1 + similarity_influence * similarity)
                    // - Below min_similarity threshold: skip or use base weight
                    let concept_node_ids: Vec<NodeId> = node_ids.iter().filter(|id| {
                        self.substrate.graph().get_node(id)
                            .map_or(false, |n| n.node_type == NodeType::Concept)
                    }).copied().collect();
                    let mut wire_events = Vec::new();
                    for i in 0..concept_node_ids.len() {
                        for j in (i + 1)..concept_node_ids.len() {
                            let from = concept_node_ids[i];
                            let to = concept_node_ids[j];

                            // Get embeddings for semantic wiring (clone to avoid borrow issues)
                            let embedding_from = self.substrate.graph().get_node(&from)
                                .and_then(|n| n.embedding.clone());
                            let embedding_to = self.substrate.graph().get_node(&to)
                                .and_then(|n| n.embedding.clone());

                            // Compute semantic weight before mutating graph
                            let base_weight = 0.1;
                            let semantic_weight = compute_semantic_weight(
                                base_weight,
                                embedding_from.as_deref(),
                                embedding_to.as_deref(),
                                &self.semantic_wiring,
                            );

                            if let Some(edge) = self.substrate.graph_mut().get_edge_mut(&from, &to) {
                                // Edge already exists: strengthen it (Hebbian reinforcement)
                                // Use semantic similarity to modulate reinforcement
                                let reinforcement = semantic_weight.unwrap_or(base_weight);
                                edge.weight = (edge.weight + reinforcement).min(1.0);
                                edge.co_activations += 1;
                                edge.last_activated_tick = tick;
                                wire_events.push((from, to));
                            } else {
                                // First co-occurrence: create tentative edge with low weight.
                                // Use semantic similarity to compute initial weight.
                                let weight = semantic_weight;

                                // Only create edge if semantic check passes
                                if let Some(w) = weight {
                                    self.substrate.set_edge(from, to, EdgeData {
                                        weight: w,
                                        co_activations: 1,
                                        created_tick: tick,
                                        last_activated_tick: tick,
                                    });
                                    wire_events.push((from, to));
                                }
                            }
                        }
                    }

                    events.push(ColonyEvent::Presented {
                        id: agent_id,
                        fragment_count: fragments.len(),
                        node_ids,
                    });

                    if !wire_events.is_empty() {
                        events.push(ColonyEvent::Wired {
                            id: agent_id,
                            connection_count: wire_events.len(),
                        });
                    }
                }

                AgentAction::Deposit(location, trace) => {
                    let agent_id = self.agents[idx].id();
                    self.substrate.deposit_trace(&location, trace);
                    events.push(ColonyEvent::Deposited {
                        id: agent_id,
                        location,
                    });
                }

                AgentAction::Emit(signal) => {
                    self.substrate.emit_signal(signal);
                }

                AgentAction::WireNodes(connections) => {
                    let agent_id = self.agents[idx].id();
                    let tick = self.substrate.current_tick();
                    let mut wired_count = 0;
                    for (from, to, base_weight) in &connections {
                        // Get embeddings for semantic wiring (clone to avoid borrow issues)
                        let embedding_from = self.substrate.graph().get_node(from)
                            .and_then(|n| n.embedding.clone());
                        let embedding_to = self.substrate.graph().get_node(to)
                            .and_then(|n| n.embedding.clone());

                        // Compute semantic weight before mutating graph
                        let weight = compute_semantic_weight(
                            *base_weight,
                            embedding_from.as_deref(),
                            embedding_to.as_deref(),
                            &self.semantic_wiring,
                        );

                        if let Some(w) = weight {
                            if let Some(edge) = self.substrate.graph_mut().get_edge_mut(from, to) {
                                edge.weight = (edge.weight + w).min(1.0);
                                edge.co_activations += 1;
                                edge.last_activated_tick = tick;
                            } else {
                                self.substrate.set_edge(*from, *to, EdgeData {
                                    weight: w,
                                    co_activations: 1,
                                    created_tick: tick,
                                    last_activated_tick: tick,
                                });
                            }
                            wired_count += 1;
                        }
                    }
                    if wired_count > 0 {
                        events.push(ColonyEvent::Wired {
                            id: agent_id,
                            connection_count: wired_count,
                        });
                    }
                }

                AgentAction::ExportCapability(_cap_id) => {
                    let agent_id = self.agents[idx].id();
                    let agent_pos = self.agents[idx].position();
                    if let Some(vocab_bytes) = self.agents[idx].export_vocabulary() {
                        // Count terms for event
                        let terms_count = serde_json::from_slice::<VocabularyCapability>(&vocab_bytes)
                            .map(|v| v.terms.len())
                            .unwrap_or(0);

                        // Deposit as CapabilityDeposit trace at agent position
                        let trace = Trace {
                            agent_id,
                            trace_type: TraceType::CapabilityDeposit,
                            intensity: 1.0,
                            tick: self.substrate.current_tick(),
                            payload: vocab_bytes,
                        };
                        self.substrate.deposit_trace(
                            &SubstrateLocation::Spatial(agent_pos),
                            trace,
                        );

                        // Emit Capability signal to attract other agents
                        self.substrate.emit_signal(Signal::new(
                            SignalType::Capability,
                            0.8,
                            agent_pos,
                            agent_id,
                            self.substrate.current_tick(),
                        ));

                        events.push(ColonyEvent::CapabilityExported {
                            agent_id,
                            terms_count,
                        });
                    }
                }

                AgentAction::SymbioseWith(target_id) => {
                    let host_idx = idx;
                    let host_id = self.agents[host_idx].id();

                    // Find target agent
                    if let Some(target_idx) = self.agents.iter().position(|a| a.id() == target_id) {
                        // Build target's profile and extract vocabulary
                        let target_profile = self.agents[target_idx].profile();
                        let target_vocab = self.agents[target_idx].export_vocabulary()
                            .unwrap_or_default();

                        // Evaluate symbiosis
                        if let Some(SymbiosisEval::Integrate) =
                            self.agents[host_idx].evaluate_symbiosis(&target_profile)
                        {
                            let host_type = self.agents[host_idx].agent_type().to_string();
                            let absorbed_type = self.agents[target_idx].agent_type().to_string();

                            // Host absorbs the symbiont
                            self.agents[host_idx].absorb_symbiont(target_profile, target_vocab);

                            // Mark target for removal via symbiotic absorption
                            symbiotic_deaths.push((target_idx, host_id));

                            events.push(ColonyEvent::Symbiosis {
                                host: host_id,
                                absorbed: target_id,
                                host_type,
                                absorbed_type,
                            });
                        }
                    }
                }

                AgentAction::Apoptose => {
                    to_die.push(idx);
                }

                AgentAction::Idle => {}

                _ => {}
            }
        }

        // Phase 2.5: Dissolution + Capability Integration
        // For each agent: compute BoundaryContext, modulate boundary,
        // externalize/internalize vocabulary, integrate nearby capabilities
        {
            let _tick = self.substrate.current_tick();
            let agent_count = self.agents.len();

            for i in 0..agent_count {
                let agent_id = self.agents[i].id();
                let agent_pos = self.agents[i].position();
                let agent_age = self.agents[i].age();

                // Compute BoundaryContext — cache externalized vocab for reuse
                let vocab_terms = self.agents[i].externalize_vocabulary();
                let mut reinforcement_count = 0u64;
                let graph = self.substrate.graph();
                for term in &vocab_terms {
                    let matching = graph.find_nodes_by_exact_label(term);
                    for nid in matching {
                        if let Some(node) = graph.get_node(nid) {
                            reinforcement_count += node.access_count;
                        }
                    }
                }

                let useful_outputs_estimate = reinforcement_count.min(100);
                let trust = if agent_age > 0 {
                    (useful_outputs_estimate as f64 / agent_age as f64).min(1.0)
                } else {
                    0.0
                };

                let context = BoundaryContext {
                    reinforcement_count,
                    age: agent_age,
                    trust,
                };

                self.agents[i].modulate_boundary(&context);
                let permeability = self.agents[i].permeability();

                // High permeability: boost matching graph nodes' access_count
                if permeability > 0.5 {
                    // Reuse cached vocab_terms instead of calling externalize_vocabulary again
                    let mut terms_externalized = 0usize;
                    for term in &vocab_terms {
                        let matching: Vec<NodeId> = self.substrate.graph().find_nodes_by_exact_label(term).to_vec();
                        for nid in &matching {
                            if let Some(node) = self.substrate.graph_mut().get_node_mut(nid) {
                                node.access_count += 1;
                                terms_externalized += 1;
                            }
                        }
                    }
                    if terms_externalized > 0 {
                        events.push(ColonyEvent::Dissolved {
                            agent_id,
                            permeability,
                            terms_externalized,
                        });
                    }
                }

                // Any permeability > 0: internalize nearby concept labels
                if permeability > 0.0 {
                    let all_nodes = self.substrate.graph().all_nodes();
                    let nearby_labels: Vec<String> = all_nodes.iter()
                        .filter_map(|nid| {
                            let node = self.substrate.graph().get_node(nid)?;
                            if node.position.distance_to(&agent_pos) <= 15.0
                                && node.node_type == NodeType::Concept
                            {
                                Some(node.label.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    if !nearby_labels.is_empty() {
                        self.agents[i].internalize_vocabulary(&nearby_labels);
                    }
                }

                // Capability integration: check for CapabilityDeposit traces near agent
                let traces = self.substrate.traces_near(
                    &agent_pos,
                    10.0,
                    &TraceType::CapabilityDeposit,
                );
                for trace in &traces {
                    if trace.agent_id != agent_id
                        && !trace.payload.is_empty()
                    {
                        let payload = trace.payload.clone();
                        let from_agent = trace.agent_id;
                        let terms_count = serde_json::from_slice::<VocabularyCapability>(&payload)
                            .map(|v| v.terms.len())
                            .unwrap_or(0);
                        if self.agents[i].integrate_vocabulary(&payload) {
                            events.push(ColonyEvent::CapabilityIntegrated {
                                agent_id,
                                from_agent,
                                terms_count,
                            });
                        }
                    }
                }
            }
        }

        // Add symbiotic deaths to the death list
        for (idx, _absorber_id) in &symbiotic_deaths {
            if !to_die.contains(idx) {
                to_die.push(*idx);
            }
        }

        // Phase 3: Remove dead agents
        to_die.sort();
        to_die.dedup();
        let dead_count = to_die.len();
        for idx in to_die.into_iter().rev() {
            let agent = self.agents.remove(idx);
            let mut death_signal = agent.prepare_death_signal();

            // Override cause if this was a symbiotic absorption
            if let Some((_, absorber_id)) = symbiotic_deaths.iter().find(|(i, _)| *i == idx) {
                death_signal.cause = DeathCause::SymbioticAbsorption(*absorber_id);
            }

            events.push(ColonyEvent::Died {
                signal: death_signal.clone(),
            });
            self.death_signals.push(death_signal);
            self.total_died += 1;
        }

        // Phase 4: Substrate decay
        self.substrate
            .decay_signals(self.signal_decay_rate, self.signal_removal_threshold);
        self.substrate
            .decay_traces(self.trace_decay_rate, self.trace_removal_threshold);
        // Synaptic pruning: activity-based decay with maturation protection
        let current_tick = self.substrate.current_tick();
        self.substrate.graph_mut().decay_edges_activity(
            self.edge_decay_rate,
            self.edge_prune_threshold,
            current_tick,
            self.staleness_factor,
            self.maturation_ticks,
        );
        // Competitive pruning: cap per-node degree
        self.substrate
            .graph_mut()
            .prune_to_max_degree(self.max_edge_degree);

        // Phase 4b: Fitness tracking — wire colony events to the tracker
        for event in &events {
            match event {
                ColonyEvent::Presented { id, fragment_count, .. } => {
                    self.fitness_tracker.record_concepts(id, *fragment_count as u64);
                }
                ColonyEvent::Wired { id, connection_count } => {
                    self.fitness_tracker.record_edges(id, *connection_count as u64);
                }
                _ => {}
            }
        }
        let alive_ids: Vec<AgentId> = self.agents.iter().map(|a| a.id()).collect();
        self.fitness_tracker.tick_all(&alive_ids);

        // Phase 5: Advance tick
        self.substrate.advance_tick();

        events.push(ColonyEvent::TickComplete {
            tick: self.substrate.current_tick(),
            alive: self.agents.len(),
            dead_this_tick: dead_count,
        });

        // Record events in history
        let current_tick = self.substrate.current_tick();
        for event in &events {
            self.event_history.push((current_tick, event.clone()));
        }

        events
    }

    /// Run the simulation for N ticks.
    pub fn run(&mut self, ticks: u64) -> Vec<Vec<ColonyEvent>> {
        let mut all_events = Vec::new();
        for _ in 0..ticks {
            all_events.push(self.tick());
        }
        all_events
    }

    /// Get colony statistics.
    pub fn stats(&self) -> ColonyStats {
        let docs = self.substrate.all_documents();
        let digested = docs.iter().filter(|d| d.digested).count();
        ColonyStats {
            tick: self.substrate.current_tick(),
            agents_alive: self.agents.len(),
            agents_died: self.total_died,
            total_spawned: self.total_spawned,
            graph_nodes: self.substrate.node_count(),
            graph_edges: self.substrate.edge_count(),
            total_signals: self.substrate.all_signals().len(),
            documents_total: docs.len(),
            documents_digested: digested,
        }
    }

    /// Get a reference to the substrate.
    pub fn substrate(&self) -> &SubstrateImpl {
        &self.substrate
    }

    /// Get a mutable reference to the substrate.
    pub fn substrate_mut(&mut self) -> &mut SubstrateImpl {
        &mut self.substrate
    }

    /// Number of agents currently alive.
    pub fn alive_count(&self) -> usize {
        self.agents.len()
    }

    /// All death signals collected during the simulation.
    pub fn death_signals(&self) -> &[DeathSignal] {
        &self.death_signals
    }

    /// Feed text input to a specific agent by index.
    pub fn feed_agent(&mut self, agent_idx: usize, input: String) -> Option<DigestionResult> {
        self.agents
            .get_mut(agent_idx)
            .map(|agent| agent.engulf(input))
    }

    /// Take a serializable snapshot of the colony's current state.
    pub fn snapshot(&self) -> ColonySnapshot {
        let graph = self.substrate.graph();

        let agents: Vec<AgentSnapshot> = self.agents.iter().map(|a| {
            AgentSnapshot {
                id: a.id(),
                agent_type: a.agent_type().to_string(),
                position: a.position(),
                age: a.age(),
                permeability: a.permeability(),
                vocabulary_size: a.vocabulary_size(),
            }
        }).collect();

        let nodes: Vec<NodeSnapshot> = graph.all_nodes().iter().filter_map(|nid| {
            let n = graph.get_node(nid)?;
            Some(NodeSnapshot {
                id: n.id,
                label: n.label.clone(),
                node_type: n.node_type.clone(),
                position: n.position,
                access_count: n.access_count,
            })
        }).collect();

        let edges: Vec<EdgeSnapshot> = graph.all_edges().iter().map(|(from, to, data)| {
            let from_label = graph.get_node(from).map(|n| n.label.clone()).unwrap_or_default();
            let to_label = graph.get_node(to).map(|n| n.label.clone()).unwrap_or_default();
            EdgeSnapshot {
                from_label,
                to_label,
                weight: data.weight,
                co_activations: data.co_activations,
            }
        }).collect();

        ColonySnapshot {
            tick: self.substrate.current_tick(),
            agents,
            nodes,
            edges,
            stats: self.stats(),
        }
    }

    /// Get the full event history with tick numbers.
    pub fn event_history(&self) -> &[(Tick, ColonyEvent)] {
        &self.event_history
    }

    /// Get a reference to the agents.
    pub fn agents(&self) -> &[Box<dyn Agent<Input = String, Fragment = String, Presentation = Vec<String>>>] {
        &self.agents
    }

    /// Get a reference to the fitness tracker.
    pub fn fitness_tracker(&self) -> &FitnessTracker {
        &self.fitness_tracker
    }

    /// Get a mutable reference to the fitness tracker.
    pub fn fitness_tracker_mut(&mut self) -> &mut FitnessTracker {
        &mut self.fitness_tracker
    }

    /// Emit an input signal at a position (to attract agents).
    pub fn emit_input_signal(&mut self, position: Position, intensity: f64) {
        let signal = Signal::new(
            SignalType::Input,
            intensity,
            position,
            AgentId::new(),
            self.substrate.current_tick(),
        );
        self.substrate.emit_signal(signal);
    }
}

impl Default for Colony {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phago_agents::digester::Digester;

    #[test]
    fn spawn_and_count_agents() {
        let mut colony = Colony::new();
        colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0))));
        colony.spawn(Box::new(Digester::new(Position::new(5.0, 5.0))));
        assert_eq!(colony.alive_count(), 2);
        assert_eq!(colony.stats().total_spawned, 2);
    }

    #[test]
    fn tick_advances_simulation() {
        let mut colony = Colony::new();
        colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0))));
        colony.tick();
        assert_eq!(colony.stats().tick, 1);
    }

    #[test]
    fn agent_apoptosis_in_colony() {
        let mut colony = Colony::new();
        colony.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(3),
        ));

        assert_eq!(colony.alive_count(), 1);

        for _ in 0..5 {
            colony.tick();
        }

        assert_eq!(colony.alive_count(), 0);
        assert_eq!(colony.stats().agents_died, 1);
        assert_eq!(colony.death_signals().len(), 1);
    }

    #[test]
    fn ingest_document_creates_signal() {
        let mut colony = Colony::new();
        let pos = Position::new(5.0, 5.0);
        let doc_id = colony.ingest_document("Test Doc", "cell membrane protein", pos);

        let stats = colony.stats();
        assert_eq!(stats.documents_total, 1);
        assert_eq!(stats.documents_digested, 0);
        assert_eq!(stats.total_signals, 1); // Input signal emitted

        // Document is in the substrate
        let doc = colony.substrate().get_document(&doc_id);
        assert!(doc.is_some());
        assert!(!doc.unwrap().digested);
    }

    #[test]
    fn agent_finds_and_digests_document() {
        let mut colony = Colony::new();

        // Place document at origin
        colony.ingest_document(
            "Biology 101",
            "The cell membrane controls transport of molecules into and out of the cell. \
             Proteins embedded in the membrane serve as channels and receptors.",
            Position::new(0.0, 0.0),
        );

        // Spawn agent at origin (right on top of the document)
        colony.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(50),
        ));

        // Run enough ticks for the full cycle:
        // tick 1: Seeking → finds doc → EngulfDocument
        // tick 2: FoundTarget → engulfed → Digesting
        // tick 3: Digesting → lyse → PresentFragments
        // tick 4: Presenting → Deposit trace
        colony.run(10);

        let stats = colony.stats();
        assert_eq!(stats.documents_digested, 1, "Document should be digested");
        assert!(stats.graph_nodes > 0, "Should have concept nodes: got {}", stats.graph_nodes);
        assert!(stats.graph_edges > 0, "Should have edges: got {}", stats.graph_edges);
    }

    #[test]
    fn multiple_documents_build_graph() {
        let mut colony = Colony::new();

        // Two documents about related topics
        colony.ingest_document(
            "Cell Biology",
            "The cell membrane is a lipid bilayer that controls transport. \
             Proteins in the membrane act as channels and receptors for signaling.",
            Position::new(0.0, 0.0),
        );
        colony.ingest_document(
            "Molecular Transport",
            "Active transport across the cell membrane requires ATP energy. \
             Channel proteins facilitate passive transport of ions and molecules.",
            Position::new(2.0, 0.0),
        );

        // Spawn agents near each document
        colony.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(50),
        ));
        colony.spawn(Box::new(
            Digester::new(Position::new(2.0, 0.0)).with_max_idle(50),
        ));

        colony.run(20);

        let stats = colony.stats();
        assert_eq!(stats.documents_digested, 2, "Both documents should be digested");
        // Shared concepts (cell, membrane, transport, proteins) should create
        // overlapping graph nodes and strengthen edges
        assert!(stats.graph_nodes >= 5, "Expected at least 5 concept nodes, got {}", stats.graph_nodes);
    }

    #[test]
    fn colony_stats_are_accurate() {
        let mut colony = Colony::new();
        colony.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(2),
        ));
        colony.spawn(Box::new(
            Digester::new(Position::new(5.0, 5.0)).with_max_idle(100),
        ));

        colony.run(5);

        let stats = colony.stats();
        assert_eq!(stats.total_spawned, 2);
        assert_eq!(stats.agents_died, 1);
        assert_eq!(stats.agents_alive, 1);
    }

    #[test]
    fn semantic_wiring_config_is_accessible() {
        let colony = Colony::new();
        let config = colony.semantic_wiring_config();
        // Default config should not require embeddings
        assert!(!config.require_embeddings);
        assert!(config.min_similarity >= 0.0);
        assert!(config.similarity_influence >= 0.0);
    }

    #[test]
    fn with_semantic_wiring_configures_colony() {
        use phago_core::semantic::SemanticWiringConfig;

        let colony = Colony::new()
            .with_semantic_wiring(SemanticWiringConfig::strict());

        let config = colony.semantic_wiring_config();
        assert!(config.require_embeddings);
        assert!(config.min_similarity > 0.0);
    }

    #[test]
    fn semantic_wiring_boosts_similar_concept_edges() {
        use phago_core::semantic::SemanticWiringConfig;

        let mut colony = Colony::new()
            .with_semantic_wiring(SemanticWiringConfig::default());

        // Manually add two nodes with similar embeddings
        let emb_a = vec![1.0, 0.0, 0.0];  // Unit vector along x
        let emb_b = vec![0.95, 0.31, 0.0]; // ~18° from emb_a (high similarity)

        let node_a = colony.substrate_mut().add_node(NodeData {
            id: NodeId::new(),
            label: "concept_a".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(0.0, 0.0),
            access_count: 1,
            created_tick: 0,
            embedding: Some(emb_a),
        });

        let node_b = colony.substrate_mut().add_node(NodeData {
            id: NodeId::new(),
            label: "concept_b".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(1.0, 0.0),
            access_count: 1,
            created_tick: 0,
            embedding: Some(emb_b),
        });

        // Wire them manually using WireNodes action
        colony.substrate_mut().set_edge(node_a, node_b, EdgeData {
            weight: 0.1,
            co_activations: 1,
            created_tick: 0,
            last_activated_tick: 0,
        });

        let edge = colony.substrate().graph().get_edge(&node_a, &node_b).unwrap();
        assert!(edge.weight >= 0.1, "Edge should have at least base weight");
    }

    #[test]
    fn semantic_wiring_with_no_embeddings_uses_base_weight() {
        use phago_core::semantic::SemanticWiringConfig;

        let mut colony = Colony::new()
            .with_semantic_wiring(SemanticWiringConfig::default());

        // Add two nodes WITHOUT embeddings
        let node_a = colony.substrate_mut().add_node(NodeData {
            id: NodeId::new(),
            label: "plain_a".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(0.0, 0.0),
            access_count: 1,
            created_tick: 0,
            embedding: None,
        });

        let node_b = colony.substrate_mut().add_node(NodeData {
            id: NodeId::new(),
            label: "plain_b".to_string(),
            node_type: NodeType::Concept,
            position: Position::new(1.0, 0.0),
            access_count: 1,
            created_tick: 0,
            embedding: None,
        });

        // Wire them
        colony.substrate_mut().set_edge(node_a, node_b, EdgeData {
            weight: 0.1,
            co_activations: 1,
            created_tick: 0,
            last_activated_tick: 0,
        });

        let edge = colony.substrate().graph().get_edge(&node_a, &node_b).unwrap();
        // With default config, no embeddings means base weight is used
        assert!((edge.weight - 0.1).abs() < 0.01, "Edge should use base weight: got {}", edge.weight);
    }
}
