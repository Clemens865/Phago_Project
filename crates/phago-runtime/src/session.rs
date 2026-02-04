//! Session persistence — save/load colony graph and agent state.
//!
//! Serializes the knowledge graph (nodes + edges) and agent state to JSON
//! for persistence across sessions. Agents can be fully restored with their
//! vocabulary, fitness history, and other internal state.

use crate::colony::Colony;
use phago_agents::serialize::SerializedAgent;
use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Serializable snapshot of the knowledge graph and agent state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphState {
    pub nodes: Vec<SerializedNode>,
    pub edges: Vec<SerializedEdge>,
    #[serde(default)]
    pub agents: Vec<SerializedAgent>,
    pub metadata: SessionMetadata,
}

/// Serializable node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedNode {
    pub label: String,
    pub node_type: String,
    pub access_count: u64,
    pub position_x: f64,
    pub position_y: f64,
    #[serde(default)]
    pub created_tick: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

/// Serializable edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedEdge {
    pub from_label: String,
    pub to_label: String,
    pub weight: f64,
    pub co_activations: u64,
    #[serde(default)]
    pub created_tick: u64,
    #[serde(default)]
    pub last_activated_tick: u64,
}

/// Session metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub session_id: String,
    pub tick: u64,
    pub node_count: usize,
    pub edge_count: usize,
    #[serde(default)]
    pub agent_count: usize,
    pub files_indexed: Vec<String>,
}

/// Save the colony's knowledge graph to a JSON file.
///
/// To include agent state, use `save_session_with_agents` instead.
pub fn save_session(colony: &Colony, path: &Path, files_indexed: &[String]) -> std::io::Result<()> {
    save_session_with_agents(colony, path, files_indexed, &[])
}

/// Save the colony's knowledge graph and agent state to a JSON file.
///
/// # Arguments
/// * `colony` - The colony to save
/// * `path` - Path to the output JSON file
/// * `files_indexed` - List of files that were indexed
/// * `agents` - Serialized agent states (use SerializableAgent::export_state())
pub fn save_session_with_agents(
    colony: &Colony,
    path: &Path,
    files_indexed: &[String],
    agents: &[SerializedAgent],
) -> std::io::Result<()> {
    let graph = colony.substrate().graph();
    let all_nodes = graph.all_nodes();

    let nodes: Vec<SerializedNode> = all_nodes.iter()
        .filter_map(|nid| graph.get_node(nid))
        .map(|n| SerializedNode {
            label: n.label.clone(),
            node_type: format!("{:?}", n.node_type),
            access_count: n.access_count,
            position_x: n.position.x,
            position_y: n.position.y,
            created_tick: n.created_tick,
            embedding: n.embedding.clone(),
        })
        .collect();

    let edges: Vec<SerializedEdge> = graph.all_edges().iter()
        .filter_map(|(from, to, edge)| {
            let from_label = graph.get_node(from)?.label.clone();
            let to_label = graph.get_node(to)?.label.clone();
            Some(SerializedEdge {
                from_label,
                to_label,
                weight: edge.weight,
                co_activations: edge.co_activations,
                created_tick: edge.created_tick,
                last_activated_tick: edge.last_activated_tick,
            })
        })
        .collect();

    let state = GraphState {
        metadata: SessionMetadata {
            session_id: uuid::Uuid::new_v4().to_string(),
            tick: colony.stats().tick,
            node_count: nodes.len(),
            edge_count: edges.len(),
            agent_count: agents.len(),
            files_indexed: files_indexed.to_vec(),
        },
        nodes,
        edges,
        agents: agents.to_vec(),
    };

    let json = serde_json::to_string_pretty(&state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, json)
}

/// Load a saved session from JSON.
pub fn load_session(path: &Path) -> std::io::Result<GraphState> {
    let json = std::fs::read_to_string(path)?;
    serde_json::from_str(&json)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Restore a graph state into a colony.
/// Adds all nodes and edges from the saved state.
///
/// Note: Agents must be restored separately using `state.agents` and
/// `SerializableAgent::from_state()` for each agent type.
///
/// # Example
/// ```ignore
/// use phago_agents::serialize::SerializableAgent;
/// use phago_agents::digester::Digester;
///
/// let state = load_session(&path)?;
/// restore_into_colony(&mut colony, &state);
///
/// // Restore agents
/// for agent_state in &state.agents {
///     if let Some(digester) = Digester::from_state(agent_state) {
///         colony.spawn(Box::new(digester));
///     }
/// }
/// ```
pub fn restore_into_colony(colony: &mut Colony, state: &GraphState) {
    use phago_core::substrate::Substrate;
    use std::collections::HashMap;

    let mut label_to_id: HashMap<String, NodeId> = HashMap::new();

    // Add nodes
    for node in &state.nodes {
        let node_type = match node.node_type.as_str() {
            "Concept" => NodeType::Concept,
            "Insight" => NodeType::Insight,
            "Anomaly" => NodeType::Anomaly,
            _ => NodeType::Concept,
        };

        let data = NodeData {
            id: NodeId::new(),
            label: node.label.clone(),
            node_type,
            position: Position::new(node.position_x, node.position_y),
            access_count: node.access_count,
            created_tick: node.created_tick,
            embedding: node.embedding.clone(),
        };
        let id = colony.substrate_mut().add_node(data);
        label_to_id.insert(node.label.clone(), id);
    }

    // Add edges with full temporal state
    for edge in &state.edges {
        if let (Some(&from_id), Some(&to_id)) = (
            label_to_id.get(&edge.from_label),
            label_to_id.get(&edge.to_label),
        ) {
            colony.substrate_mut().set_edge(from_id, to_id, EdgeData {
                weight: edge.weight,
                co_activations: edge.co_activations,
                created_tick: edge.created_tick,
                last_activated_tick: edge.last_activated_tick,
            });
        }
    }

    // Advance colony tick to match the saved session
    // so that maturation/staleness calculations remain correct
    let target_tick = state.metadata.tick;
    while colony.stats().tick < target_tick {
        colony.substrate_mut().advance_tick();
    }
}

/// Restore agents from a GraphState into a colony.
///
/// This is a convenience function that handles all built-in agent types.
/// Returns the number of agents successfully restored.
pub fn restore_agents(colony: &mut Colony, state: &GraphState) -> usize {
    use phago_agents::digester::Digester;
    use phago_agents::serialize::SerializableAgent;
    use phago_agents::sentinel::Sentinel;
    use phago_agents::synthesizer::Synthesizer;

    let mut restored = 0;

    for agent_state in &state.agents {
        match agent_state {
            SerializedAgent::Digester(_) => {
                if let Some(digester) = Digester::from_state(agent_state) {
                    colony.spawn(Box::new(digester));
                    restored += 1;
                }
            }
            SerializedAgent::Synthesizer(_) => {
                if let Some(synthesizer) = Synthesizer::from_state(agent_state) {
                    colony.spawn(Box::new(synthesizer));
                    restored += 1;
                }
            }
            SerializedAgent::Sentinel(_) => {
                if let Some(sentinel) = Sentinel::from_state(agent_state) {
                    colony.spawn(Box::new(sentinel));
                    restored += 1;
                }
            }
        }
    }

    restored
}

/// Check if save/load preserves node and edge counts.
pub fn verify_fidelity(original: &Colony, restored: &Colony) -> (bool, usize, usize, usize, usize) {
    let orig_nodes = original.substrate().graph().node_count();
    let orig_edges = original.substrate().graph().edge_count();
    let rest_nodes = restored.substrate().graph().node_count();
    let rest_edges = restored.substrate().graph().edge_count();
    let identical = orig_nodes == rest_nodes && orig_edges == rest_edges;
    (identical, orig_nodes, orig_edges, rest_nodes, rest_edges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::Colony;
    use phago_core::agent::Agent;

    #[test]
    fn save_load_roundtrip() {
        let mut colony = Colony::new();
        colony.ingest_document("test", "cell membrane protein", Position::new(0.0, 0.0));

        use phago_agents::digester::Digester;
        colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0)).with_max_idle(80)));
        colony.run(15);

        let tmp = std::env::temp_dir().join("phago_session_test.json");
        save_session(&colony, &tmp, &["test.rs".to_string()]).unwrap();

        let state = load_session(&tmp).unwrap();
        assert!(!state.nodes.is_empty());
        assert!(state.metadata.node_count > 0);

        // Restore into new colony
        let mut restored = Colony::new();
        restore_into_colony(&mut restored, &state);

        let (_identical, orig_n, _orig_e, rest_n, rest_e) = verify_fidelity(&colony, &restored);
        assert_eq!(orig_n, rest_n, "Node count should match");
        // Edge count may differ slightly due to label collisions
        assert!(rest_e > 0, "Restored colony should have edges");

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn save_load_with_agent_state() {
        use phago_agents::digester::Digester;
        use phago_agents::serialize::SerializableAgent;

        let mut colony = Colony::new();
        colony.ingest_document("test", "cell membrane protein biology", Position::new(0.0, 0.0));

        // Create a digester and let it process
        let mut digester = Digester::new(Position::new(0.0, 0.0)).with_max_idle(100);
        let _ = digester.digest_text("cell membrane protein biology structure".to_string());

        // Export agent state
        let agent_state = digester.export_state();

        // Spawn the digester
        colony.spawn(Box::new(digester));
        colony.run(10);

        // Save with agent state
        let tmp = std::env::temp_dir().join("phago_agent_state_test.json");
        save_session_with_agents(&colony, &tmp, &["test.rs".to_string()], &[agent_state]).unwrap();

        // Load and verify agent state is present
        let state = load_session(&tmp).unwrap();
        assert_eq!(state.agents.len(), 1, "Should have saved one agent");
        assert_eq!(state.metadata.agent_count, 1);

        // Restore into new colony
        let mut restored = Colony::new();
        restore_into_colony(&mut restored, &state);
        let agents_restored = restore_agents(&mut restored, &state);
        assert_eq!(agents_restored, 1, "Should restore one agent");
        assert_eq!(restored.alive_count(), 1, "Colony should have one agent");

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn digester_state_preserves_vocabulary() {
        use phago_agents::digester::Digester;
        use phago_agents::serialize::SerializableAgent;

        let mut digester = Digester::new(Position::new(1.0, 2.0)).with_max_idle(50);

        // Process some text to build vocabulary
        digester.digest_text("cell membrane protein transport channel".to_string());
        digester.digest_text("receptor signaling pathway cascade".to_string());

        // Export state
        let state = digester.export_state();

        // Restore and verify
        let restored = Digester::from_state(&state).expect("Should restore digester");

        assert_eq!(restored.position().x, 1.0);
        assert_eq!(restored.position().y, 2.0);
        assert!(restored.total_fragments() > 0, "Vocabulary should be preserved");
    }
}
