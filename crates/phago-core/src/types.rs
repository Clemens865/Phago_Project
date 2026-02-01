//! Shared types used across all Phago primitives and crates.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an agent in the colony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a node in the knowledge graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a document in the substrate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub Uuid);

impl DocumentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

/// A document placed in the substrate for agents to digest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub title: String,
    pub content: String,
    pub position: Position,
    /// Whether this document has been fully digested.
    pub digested: bool,
}

/// A position in the substrate's spatial field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Position) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

/// A location in the substrate — either spatial or graph-based.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SubstrateLocation {
    /// A position in the spatial signal field.
    Spatial(Position),
    /// A node in the knowledge graph.
    GraphNode(NodeId),
}

/// The type of a signal in the substrate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalType {
    /// Unprocessed input available (attracts digesters).
    Input,
    /// Agent presence (for quorum sensing).
    Presence,
    /// Quorum threshold reached in this region.
    Quorum,
    /// Anomaly detected (attracts sentinels/synthesizers).
    Anomaly,
    /// Emergent insight generated.
    Insight,
    /// Capability available for transfer.
    Capability,
    /// Custom signal type for domain-specific use.
    Custom(String),
}

/// A signal emitted into or read from the substrate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub signal_type: SignalType,
    pub intensity: f64,
    pub position: Position,
    pub emitter: AgentId,
    /// Monotonic tick count when this signal was emitted.
    pub tick: u64,
}

/// A directional gradient sensed by an agent.
#[derive(Debug, Clone)]
pub struct Gradient {
    pub signal_type: SignalType,
    pub direction: Position,
    pub magnitude: f64,
}

/// A trace deposited by an agent on the substrate (stigmergy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub agent_id: AgentId,
    pub trace_type: TraceType,
    pub intensity: f64,
    pub tick: u64,
    pub payload: Vec<u8>,
}

/// The type of trace deposited.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraceType {
    /// Agent visited this location.
    Visit,
    /// Agent digested content here.
    Digestion,
    /// Agent found this location important.
    Importance,
    /// Agent deposited a capability here.
    CapabilityDeposit,
    /// Custom trace type.
    Custom(String),
}

/// Health assessment of an agent (used by Apoptose).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellHealth {
    /// Functioning normally.
    Healthy,
    /// Under load but still functional.
    Stressed,
    /// Integrity lost — should self-terminate.
    Compromised,
    /// Others cover this function — can self-terminate.
    Redundant,
    /// Too many cycles without useful output — should self-terminate.
    Senescent,
}

impl CellHealth {
    /// Whether this health state suggests the agent should die.
    pub fn should_die(&self) -> bool {
        matches!(self, CellHealth::Compromised | CellHealth::Redundant | CellHealth::Senescent)
    }
}

/// Signal emitted when an agent dies (apoptotic body).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathSignal {
    pub agent_id: AgentId,
    pub total_ticks: u64,
    pub useful_outputs: u64,
    pub final_fragments: Vec<Vec<u8>>,
    pub cause: DeathCause,
}

/// Why an agent died.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeathCause {
    /// Agent decided to die on its own (intrinsic apoptosis).
    SelfAssessed(CellHealth),
    /// External kill signal (extrinsic apoptosis — Fas/FasL analog).
    ExternalSignal,
    /// Runtime terminated the agent.
    RuntimeTermination,
    /// Agent was absorbed by another through symbiosis.
    SymbioticAbsorption(AgentId),
}

/// A vocabulary-based capability for Transfer in v0.1.
///
/// Since v0.1 has no WASM runtime, capabilities are learned vocabulary sets
/// — serialized lists of concept terms that agents can share.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabularyCapability {
    /// The concept terms comprising this capability.
    pub terms: Vec<String>,
    /// The agent that produced this vocabulary.
    pub origin: AgentId,
    /// How many documents contributed to building this vocabulary.
    pub document_count: u64,
}

/// Identifier for a transferable capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub String);

/// Description of a capability available for transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub name: String,
    pub description: String,
    /// Size of the capability module in bytes.
    pub size_bytes: usize,
    /// The agent that produced this capability.
    pub origin: AgentId,
}

/// Result of evaluating a foreign capability for compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compatibility {
    /// Can integrate directly.
    Compatible,
    /// Possible with modification.
    NeedsAdaptation,
    /// Cannot integrate.
    Incompatible,
}

/// Result of attempting to integrate a foreign capability.
#[derive(Debug, Clone)]
pub enum RejectionReason {
    /// The capability is incompatible with this agent's architecture.
    Incompatible,
    /// The agent already has this capability.
    AlreadyPresent,
    /// Integration failed at runtime.
    IntegrationError(String),
}

/// Result of a digestion attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigestionResult {
    /// Successfully engulfed and ready for lysis.
    Engulfed,
    /// Input was indigestible.
    Indigestible,
    /// Agent is already digesting something.
    Busy,
}

/// Result of evaluating another agent for symbiosis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbiosisEval {
    /// More valuable broken down — proceed with digestion.
    Digest,
    /// More valuable as a permanent symbiont — integrate.
    Integrate,
    /// Leave independent — no action needed.
    Coexist,
}

/// Information about an integrated symbiont.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbiontInfo {
    pub id: AgentId,
    pub name: String,
    pub capabilities: Vec<CapabilityDescriptor>,
}

/// Why symbiosis failed.
#[derive(Debug, Clone)]
pub enum SymbiosisFailure {
    /// The other agent rejected integration.
    Rejected,
    /// Architecturally incompatible.
    Incompatible,
    /// Runtime error during integration.
    IntegrationError(String),
}

/// Classification result from negative selection.
#[derive(Debug, Clone, PartialEq)]
pub enum Classification {
    /// Normal — matches the self-model.
    IsSelf,
    /// Anomalous — deviates from self-model by this degree (0.0-1.0).
    NonSelf(f64),
    /// Cannot classify — insufficient self-model.
    Unknown,
}

/// Context for boundary modulation (Dissolve primitive).
#[derive(Debug, Clone)]
pub struct BoundaryContext {
    /// How many of this agent's contributions have been reinforced by others.
    pub reinforcement_count: u64,
    /// How long the agent has been alive (in ticks).
    pub age: u64,
    /// Trust level from the colony (0.0-1.0).
    pub trust: f64,
}

/// An action returned by an agent's tick.
#[derive(Debug, Clone)]
pub enum AgentAction {
    /// Agent is idle — nothing to do.
    Idle,
    /// Agent wants to move toward a position.
    Move(Position),
    /// Agent wants to engulf a specific document from the substrate.
    EngulfDocument(DocumentId),
    /// Agent is presenting digested fragments (concepts to add to graph).
    PresentFragments(Vec<FragmentPresentation>),
    /// Agent is depositing a trace.
    Deposit(SubstrateLocation, Trace),
    /// Agent is emitting a signal.
    Emit(Signal),
    /// Agent is wiring connections between nodes.
    WireNodes(Vec<(NodeId, NodeId, f64)>),
    /// Agent is triggering apoptosis.
    Apoptose,
    /// Agent is attempting symbiosis with another agent.
    SymbioseWith(AgentId),
    /// Agent is exporting a capability.
    ExportCapability(CapabilityId),
    /// Agent is contributing to collective computation.
    ContributeToCollective,
}

/// A fragment to present to the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentPresentation {
    pub label: String,
    pub source_document: DocumentId,
    pub position: Position,
    /// What type of node to create. Defaults to Concept.
    pub node_type: NodeType,
}

/// Data stored in a knowledge graph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    pub id: NodeId,
    pub label: String,
    pub node_type: NodeType,
    pub position: Position,
    /// Number of times this node has been accessed/reinforced.
    pub access_count: u64,
    pub created_tick: u64,
}

/// Types of nodes in the knowledge graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    /// A concept extracted from a document.
    Concept,
    /// A document that was ingested.
    Document,
    /// An insight generated by collective emergence.
    Insight,
    /// An anomaly flagged by negative selection.
    Anomaly,
}

/// Data stored on a knowledge graph edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    /// Strength of this connection (Hebbian weight).
    pub weight: f64,
    /// Number of times this edge was co-activated.
    pub co_activations: u64,
    pub created_tick: u64,
    pub last_activated_tick: u64,
}

/// A pruned connection, returned by Wire::prune.
#[derive(Debug, Clone)]
pub struct PrunedConnection {
    pub from: NodeId,
    pub to: NodeId,
    pub final_weight: f64,
}

/// Contribution to a collective computation (Emerge).
#[derive(Debug, Clone)]
pub struct Contribution {
    pub agent_id: AgentId,
    pub data: Vec<u8>,
}

/// Response to stigmergic traces.
#[derive(Debug, Clone)]
pub enum StigmergicResponse {
    /// Move toward high-trace area.
    Attract(Position),
    /// Move away from high-trace area.
    Repel(Position),
    /// Deposit own trace here.
    Deposit,
    /// No response to current traces.
    Ignore,
}

/// Orientation decision from sensing.
#[derive(Debug, Clone)]
pub enum Orientation {
    /// Move toward the strongest gradient.
    Toward(Position),
    /// Stay in current position.
    Stay,
    /// Explore randomly (no clear gradient).
    Explore,
}

/// The current tick of the simulation.
pub type Tick = u64;
