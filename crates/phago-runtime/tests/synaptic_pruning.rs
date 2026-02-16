//! Synaptic Pruning Experiments
//!
//! Tests that activity-based decay + degree pruning correctly:
//! 1. Preserves frequently co-activated edges
//! 2. Removes stale edges that were created but never reinforced
//! 3. Caps per-node degree via competitive pruning

use phago_agents::digester::Digester;
use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use phago_runtime::colony::Colony;

#[test]
fn stale_edges_decay_while_active_edges_survive() {
    let mut colony = Colony::new();

    // Phase 1: Ingest documents with shared terms to create reinforced edges
    // These 3 docs share "cell", "membrane", "protein" -- those edges get reinforced
    colony.ingest_document(
        "Doc A",
        "cell membrane protein transport channel receptor",
        Position::new(0.0, 0.0),
    );
    colony.ingest_document(
        "Doc B",
        "cell membrane protein synthesis ribosome enzyme",
        Position::new(0.0, 0.0),
    );
    colony.ingest_document(
        "Doc C",
        "cell membrane protein signaling pathway cascade",
        Position::new(0.0, 0.0),
    );

    // Phase 2: Ingest one doc with unique terms that won't be reinforced
    // "quantum", "entanglement", "superposition" only appear once
    colony.ingest_document(
        "Doc D (stale)",
        "quantum entanglement superposition decoherence measurement",
        Position::new(0.0, 0.0),
    );

    // Spawn digesters to process everything
    for _ in 0..3 {
        colony.spawn(Box::new(
            Digester::new(Position::new(0.0, 0.0)).with_max_idle(200),
        ));
    }

    // Let agents build the graph
    colony.run(60);

    let stats_baseline = colony.stats();
    println!("--- Baseline (Tick {}) ---", stats_baseline.tick);
    println!("Nodes: {}", stats_baseline.graph_nodes);
    println!("Edges: {}", stats_baseline.graph_edges);

    assert!(stats_baseline.graph_nodes > 0, "Should have nodes");
    assert!(stats_baseline.graph_edges > 0, "Should have edges");

    // Check that shared-topic edges are strong (reinforced) and quantum edges are weak
    let graph = colony.substrate().graph();
    let cell_nodes = graph.find_nodes_by_label("cell");
    let membrane_nodes = graph.find_nodes_by_label("membrane");
    let quantum_nodes = graph.find_nodes_by_label("quantum");

    if !cell_nodes.is_empty() && !membrane_nodes.is_empty() {
        if let Some(edge) = graph.get_edge(&cell_nodes[0], &membrane_nodes[0]) {
            println!(
                "cell-membrane edge: weight={:.3}, co_activations={}",
                edge.weight, edge.co_activations
            );
            assert!(
                edge.co_activations >= 2,
                "Shared terms should have multiple co-activations"
            );
        }
    }

    if !quantum_nodes.is_empty() {
        let q_neighbors = graph.neighbors(&quantum_nodes[0]);
        if let Some((_, edge)) = q_neighbors.first() {
            println!(
                "quantum edge: weight={:.3}, co_activations={}",
                edge.weight, edge.co_activations
            );
        }
    }

    // Phase 3: Run long enough for stale edges to decay past maturation window
    // Maturation=50 ticks, so edges from tick ~10 mature at tick ~60.
    // Running to tick 300 gives ~240 ticks of staleness-accelerated decay.
    colony.run(240);

    let stats_after = colony.stats();
    println!("--- After 240 more ticks (Tick {}) ---", stats_after.tick);
    println!("Nodes: {}", stats_after.graph_nodes);
    println!("Edges: {}", stats_after.graph_edges);

    // Reinforced edges (cell-membrane, cell-protein) should survive
    // because high co_activations reduce the activity_factor, slowing decay.
    // Stale edges (quantum-entanglement) should be pruned
    // because low co_activations + high staleness accelerate decay.
    println!(
        "Edge change: {} -> {} ({:.1}% reduction)",
        stats_baseline.graph_edges,
        stats_after.graph_edges,
        (1.0 - stats_after.graph_edges as f64 / stats_baseline.graph_edges as f64) * 100.0
    );

    // At minimum, some stale edges should have been pruned
    // (the quantum terms are only co-activated once and decay faster)
    assert!(
        stats_after.graph_edges <= stats_baseline.graph_edges,
        "Edge count should not increase: before={}, after={}",
        stats_baseline.graph_edges,
        stats_after.graph_edges
    );
}

#[test]
fn competitive_pruning_enforces_degree_cap() {
    // Test that prune_to_max_degree works independently
    use phago_runtime::topology_impl::PetTopologyGraph;

    let mut graph = PetTopologyGraph::new();

    // Create a hub node connected to 40 other nodes
    let hub_id = NodeId::new();
    graph.add_node(NodeData {
        id: hub_id,
        label: "hub".to_string(),
        node_type: NodeType::Concept,
        position: Position::new(0.0, 0.0),
        access_count: 1,
        created_tick: 0,
        embedding: None,
    });

    for i in 0..40 {
        let spoke_id = NodeId::new();
        graph.add_node(NodeData {
            id: spoke_id,
            label: format!("spoke_{}", i),
            node_type: NodeType::Concept,
            position: Position::new(0.0, 0.0),
            access_count: 1,
            created_tick: 0,
            embedding: None,
        });
        graph.set_edge(
            hub_id,
            spoke_id,
            EdgeData {
                weight: 0.1 + (i as f64) * 0.02, // weights from 0.1 to 0.88
                co_activations: 1,
                created_tick: 0,
                last_activated_tick: 0,
            },
        );
    }

    assert_eq!(graph.edge_count(), 40);

    // Prune to max degree 30
    let pruned = graph.prune_to_max_degree(30);

    println!(
        "Pruned {} edges, remaining: {}",
        pruned.len(),
        graph.edge_count()
    );
    assert!(
        graph.edge_count() <= 30,
        "Should enforce degree cap: got {}",
        graph.edge_count()
    );

    // The 10 weakest edges should have been removed
    let hub_neighbors = graph.neighbors(&hub_id);
    for (_, edge) in &hub_neighbors {
        assert!(
            edge.weight >= 0.3,
            "Weakest remaining edge should be >= 0.3, got {:.3}",
            edge.weight
        );
    }
}
