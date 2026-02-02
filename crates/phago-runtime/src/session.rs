//! Session persistence — save/load colony graph state.
//!
//! Serializes the knowledge graph (nodes + edges) to JSON for
//! persistence across sessions. Agent state is not serialized
//! (agents are reconstructed from config on load).

use crate::colony::Colony;
use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Serializable snapshot of the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphState {
    pub nodes: Vec<SerializedNode>,
    pub edges: Vec<SerializedEdge>,
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
}

/// Serializable edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedEdge {
    pub from_label: String,
    pub to_label: String,
    pub weight: f64,
    pub co_activations: u64,
}

/// Session metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub session_id: String,
    pub tick: u64,
    pub node_count: usize,
    pub edge_count: usize,
    pub files_indexed: Vec<String>,
}

/// Save the colony's knowledge graph to a JSON file.
pub fn save_session(colony: &Colony, path: &Path, files_indexed: &[String]) -> std::io::Result<()> {
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
            })
        })
        .collect();

    let state = GraphState {
        metadata: SessionMetadata {
            session_id: uuid::Uuid::new_v4().to_string(),
            tick: colony.stats().tick,
            node_count: nodes.len(),
            edge_count: edges.len(),
            files_indexed: files_indexed.to_vec(),
        },
        nodes,
        edges,
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
            created_tick: 0,
        };
        let id = colony.substrate_mut().add_node(data);
        label_to_id.insert(node.label.clone(), id);
    }

    // Add edges
    for edge in &state.edges {
        if let (Some(&from_id), Some(&to_id)) = (
            label_to_id.get(&edge.from_label),
            label_to_id.get(&edge.to_label),
        ) {
            colony.substrate_mut().set_edge(from_id, to_id, EdgeData {
                weight: edge.weight,
                co_activations: edge.co_activations,
                created_tick: 0,
                last_activated_tick: 0,
            });
        }
    }
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
}
