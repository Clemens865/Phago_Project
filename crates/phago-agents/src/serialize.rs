//! Agent state serialization for session persistence.
//!
//! Enables saving and restoring agent state across sessions.
//! Each agent type has a corresponding serializable state struct.

use phago_core::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Enumeration of all agent types for deserialization dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    Digester,
    Synthesizer,
    Sentinel,
    #[cfg(feature = "semantic")]
    SemanticDigester,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Digester => write!(f, "digester"),
            AgentType::Synthesizer => write!(f, "synthesizer"),
            AgentType::Sentinel => write!(f, "sentinel"),
            #[cfg(feature = "semantic")]
            AgentType::SemanticDigester => write!(f, "semantic_digester"),
        }
    }
}

/// Serializable state for a Digester agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigesterState {
    pub id: AgentId,
    pub position: Position,
    pub age_ticks: u64,
    pub idle_ticks: u64,
    pub useful_outputs: u64,
    pub all_presentations: Vec<String>,
    pub known_vocabulary: Vec<String>,
    pub has_exported: bool,
    pub boundary_permeability: f64,
    pub max_idle_ticks: u64,
    pub sense_radius: f64,
}

/// Serializable state for a Synthesizer agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizerState {
    pub id: AgentId,
    pub position: Position,
    pub age_ticks: u64,
    pub idle_ticks: u64,
    pub insights_produced: u64,
    pub sense_radius: f64,
    pub cooldown_ticks: u64,
    pub max_idle_ticks: u64,
}

/// Serializable state for a Sentinel agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelState {
    pub id: AgentId,
    pub position: Position,
    pub age_ticks: u64,
    pub idle_ticks: u64,
    pub anomalies_detected: u64,
    pub last_scan_tick: u64,
    pub self_model_concepts: Vec<String>,
    pub sense_radius: f64,
    pub max_idle_ticks: u64,
    pub scan_interval: u64,
}

/// Union of all serializable agent states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializedAgent {
    Digester(DigesterState),
    Synthesizer(SynthesizerState),
    Sentinel(SentinelState),
}

impl SerializedAgent {
    /// Get the agent type.
    pub fn agent_type(&self) -> AgentType {
        match self {
            SerializedAgent::Digester(_) => AgentType::Digester,
            SerializedAgent::Synthesizer(_) => AgentType::Synthesizer,
            SerializedAgent::Sentinel(_) => AgentType::Sentinel,
        }
    }

    /// Get the agent ID.
    pub fn id(&self) -> AgentId {
        match self {
            SerializedAgent::Digester(s) => s.id,
            SerializedAgent::Synthesizer(s) => s.id,
            SerializedAgent::Sentinel(s) => s.id,
        }
    }

    /// Get the agent position.
    pub fn position(&self) -> Position {
        match self {
            SerializedAgent::Digester(s) => s.position,
            SerializedAgent::Synthesizer(s) => s.position,
            SerializedAgent::Sentinel(s) => s.position,
        }
    }
}

/// Trait for agents that can be serialized.
pub trait SerializableAgent {
    /// Export the agent's state for serialization.
    fn export_state(&self) -> SerializedAgent;

    /// Create an agent from serialized state.
    fn from_state(state: &SerializedAgent) -> Option<Self>
    where
        Self: Sized;
}

// Helper function to convert HashSet to Vec for serialization
pub(crate) fn hashset_to_vec(set: &HashSet<String>) -> Vec<String> {
    set.iter().cloned().collect()
}

// Helper function to convert Vec to HashSet for deserialization
pub(crate) fn vec_to_hashset(vec: &[String]) -> HashSet<String> {
    vec.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_type_display() {
        assert_eq!(AgentType::Digester.to_string(), "digester");
        assert_eq!(AgentType::Synthesizer.to_string(), "synthesizer");
        assert_eq!(AgentType::Sentinel.to_string(), "sentinel");
    }

    #[test]
    fn digester_state_serializes() {
        let state = DigesterState {
            id: AgentId::from_seed(42),
            position: Position::new(1.0, 2.0),
            age_ticks: 100,
            idle_ticks: 5,
            useful_outputs: 10,
            all_presentations: vec!["cell".to_string(), "membrane".to_string()],
            known_vocabulary: vec!["protein".to_string()],
            has_exported: true,
            boundary_permeability: 0.5,
            max_idle_ticks: 30,
            sense_radius: 10.0,
        };

        let json = serde_json::to_string(&state).unwrap();
        let restored: DigesterState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, state.id);
        assert_eq!(restored.age_ticks, 100);
        assert_eq!(restored.all_presentations.len(), 2);
    }

    #[test]
    fn serialized_agent_enum_works() {
        let state = DigesterState {
            id: AgentId::from_seed(1),
            position: Position::new(0.0, 0.0),
            age_ticks: 50,
            idle_ticks: 0,
            useful_outputs: 5,
            all_presentations: vec![],
            known_vocabulary: vec![],
            has_exported: false,
            boundary_permeability: 0.0,
            max_idle_ticks: 30,
            sense_radius: 10.0,
        };

        let agent = SerializedAgent::Digester(state);
        assert_eq!(agent.agent_type(), AgentType::Digester);

        let json = serde_json::to_string(&agent).unwrap();
        let restored: SerializedAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.agent_type(), AgentType::Digester);
    }
}
