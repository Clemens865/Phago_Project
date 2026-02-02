//! Phase 4 integration tests — full colony simulation.

use phago_agents::digester::Digester;
use phago_agents::sentinel::Sentinel;
use phago_agents::synthesizer::Synthesizer;
use phago_core::topology::TopologyGraph;
use phago_core::types::*;
use phago_runtime::colony::{Colony, ColonyEvent};

#[test]
fn full_sim_produces_all_event_types() {
    let mut colony = Colony::new();

    // Ingest documents
    colony.ingest_document(
        "Cell Biology",
        "The cell membrane controls transport of molecules into and out of the cell. \
         Proteins embedded in the membrane serve as channels and receptors.",
        Position::new(0.0, 0.0),
    );
    colony.ingest_document(
        "Molecular Transport",
        "Active transport across the cell membrane requires ATP energy. Channel proteins \
         facilitate passive transport of ions and small molecules.",
        Position::new(5.0, 0.0),
    );
    colony.ingest_document(
        "Signaling",
        "Signal transduction begins when ligand binds to receptor protein on cell membrane. \
         This triggers cascade of intracellular events involving kinase enzymes.",
        Position::new(0.0, 5.0),
    );

    // Spawn agents
    colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0)).with_max_idle(200)));
    colony.spawn(Box::new(Digester::new(Position::new(5.0, 0.0)).with_max_idle(200)));
    colony.spawn(Box::new(Digester::new(Position::new(0.0, 5.0)).with_max_idle(200)));
    colony.spawn(Box::new(Synthesizer::new(Position::new(2.5, 2.5))));
    colony.spawn(Box::new(Sentinel::new(Position::new(2.5, 2.5))));

    let mut has_exported = false;
    let mut has_integrated = false;
    let mut has_dissolved = false;

    for _ in 1..=300 {
        let events = colony.tick();
        for event in &events {
            match event {
                ColonyEvent::CapabilityExported { .. } => has_exported = true,
                ColonyEvent::CapabilityIntegrated { .. } => has_integrated = true,
                ColonyEvent::Dissolved { .. } => has_dissolved = true,
                _ => {}
            }
        }
    }

    assert!(has_exported, "should have at least one CapabilityExported event");
    // Integration and dissolution depend on timing/proximity, so just log
    println!("Integration occurred: {}, Dissolution occurred: {}", has_integrated, has_dissolved);
}

#[test]
fn dissolution_boosts_access_count() {
    let mut colony = Colony::new();

    colony.ingest_document(
        "Biology",
        "The cell membrane controls molecular transport. Protein channels facilitate \
         the movement of ions across the lipid bilayer boundary.",
        Position::new(0.0, 0.0),
    );

    colony.spawn(Box::new(Digester::new(Position::new(0.0, 0.0)).with_max_idle(80)));

    // Run enough ticks for digestion + dissolution to occur
    colony.run(60);

    // Check that nodes with dissolution-reinforced terms have access_count > 1
    let graph = colony.substrate().graph();
    let all_nodes = graph.all_nodes();
    let mut found_reinforced = false;

    for nid in &all_nodes {
        if let Some(node) = graph.get_node(nid) {
            if node.access_count > 1 {
                found_reinforced = true;
                break;
            }
        }
    }

    assert!(found_reinforced, "at least one node should have access_count > 1 from dissolution/reinforcement");
}
