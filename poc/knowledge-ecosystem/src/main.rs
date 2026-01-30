//! Phago Proof of Concept — Self-Organizing Knowledge Ecosystem
//!
//! Phase 2 Demo: Multiple agents self-organize around documents,
//! build a knowledge graph through digestion and Hebbian wiring,
//! deposit traces (stigmergy), and die when idle (apoptosis).

use phago_agents::digester::Digester;
use phago_core::types::*;
use phago_runtime::colony::{Colony, ColonyEvent};

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  PHAGO — Biological Computing Primitives            ║");
    println!("║  Phase 2: The Colony Self-Organizes                 ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    let mut colony = Colony::new();

    // --- Ingest documents into the substrate ---
    println!("── Ingesting Documents ──────────────────────────────");
    println!();

    let docs = vec![
        ("Cell Biology", "The cell membrane is a lipid bilayer that controls \
         the transport of molecules. Proteins embedded in the membrane serve \
         as channels and receptors. The cytoskeleton provides structural \
         support within the cell.", Position::new(0.0, 0.0)),

        ("Molecular Transport", "Active transport across the cell membrane \
         requires ATP energy produced by mitochondria. Channel proteins \
         facilitate passive transport of ions and small molecules across \
         the lipid bilayer.", Position::new(5.0, 0.0)),

        ("Cell Signaling", "Signal transduction begins when a ligand binds \
         to a receptor protein on the cell membrane. This triggers a cascade \
         of intracellular events involving kinase enzymes and secondary \
         messengers.", Position::new(0.0, 5.0)),

        ("Energy Metabolism", "Mitochondria produce ATP through oxidative \
         phosphorylation. The electron transport chain in the inner membrane \
         creates a proton gradient that drives ATP synthase. Glucose is first \
         broken down through glycolysis in the cytoplasm.", Position::new(5.0, 5.0)),

        ("Genetics", "DNA replication occurs in the nucleus before cell \
         division. RNA polymerase transcribes DNA into messenger RNA. \
         Ribosomes translate mRNA into proteins using transfer RNA and \
         amino acids.", Position::new(10.0, 0.0)),
    ];

    for (title, content, pos) in &docs {
        colony.ingest_document(title, content, *pos);
        println!("  [doc] \"{}\" at ({:.0}, {:.0})", title, pos.x, pos.y);
    }
    println!();

    // --- Spawn agents ---
    println!("── Spawning Agents ─────────────────────────────────");
    println!();

    let agent_positions = vec![
        Position::new(0.0, 0.0),
        Position::new(5.0, 0.0),
        Position::new(0.0, 5.0),
        Position::new(5.0, 5.0),
        Position::new(10.0, 0.0),
        Position::new(2.5, 2.5), // Center — will explore
        Position::new(7.5, 2.5), // Between docs
    ];

    for (i, pos) in agent_positions.iter().enumerate() {
        colony.spawn(Box::new(
            Digester::new(*pos).with_max_idle(40),
        ));
        println!("  [agent {}] Digester at ({:.1}, {:.1})", i + 1, pos.x, pos.y);
    }
    println!();

    // --- Run simulation ---
    println!("── Running Simulation (50 ticks) ────────────────────");
    println!();

    for tick_num in 1..=50 {
        let events = colony.tick();

        for event in &events {
            match event {
                ColonyEvent::Engulfed { id, document } => {
                    println!(
                        "  [tick {:>2}] ENGULF: Agent {:.8} consumed document {:.8}",
                        tick_num,
                        id.0.to_string(),
                        document.0.to_string()
                    );
                }
                ColonyEvent::Presented { id, fragment_count, .. } => {
                    println!(
                        "  [tick {:>2}] PRESENT: Agent {:.8} → {} concepts added to graph",
                        tick_num,
                        id.0.to_string(),
                        fragment_count
                    );
                }
                ColonyEvent::Wired { id, connection_count } => {
                    println!(
                        "  [tick {:>2}] WIRE: Agent {:.8} → {} connections strengthened",
                        tick_num,
                        id.0.to_string(),
                        connection_count
                    );
                }
                ColonyEvent::Deposited { id, .. } => {
                    println!(
                        "  [tick {:>2}] TRACE: Agent {:.8} deposited digestion trace",
                        tick_num,
                        id.0.to_string(),
                    );
                }
                ColonyEvent::Died { signal } => {
                    println!(
                        "  [tick {:>2}] DEATH: Agent {:.8} — {:?} (outputs: {})",
                        tick_num,
                        signal.agent_id.0.to_string(),
                        signal.cause,
                        signal.useful_outputs
                    );
                }
                _ => {}
            }
        }
    }

    // --- Results ---
    println!();
    println!("── Results ─────────────────────────────────────────");
    println!();

    let stats = colony.stats();
    println!("  Colony:");
    println!("    Ticks elapsed:      {}", stats.tick);
    println!("    Agents alive:       {} / {} spawned ({} died)",
        stats.agents_alive, stats.total_spawned, stats.agents_died);
    println!();
    println!("  Documents:");
    println!("    Total:              {}", stats.documents_total);
    println!("    Digested:           {}", stats.documents_digested);
    println!();
    println!("  Knowledge Graph:");
    println!("    Concept nodes:      {}", stats.graph_nodes);
    println!("    Connections:        {}", stats.graph_edges);
    println!();

    // Show top concepts (nodes with highest access count)
    use phago_core::topology::TopologyGraph;
    let graph = colony.substrate().graph();
    let mut nodes: Vec<_> = graph.all_nodes().iter().filter_map(|id| {
        graph.get_node(id).map(|n| (n.label.clone(), n.access_count))
    }).collect();
    nodes.sort_by(|a, b| b.1.cmp(&a.1));

    println!("  Top Concepts (by reinforcement):");
    for (label, count) in nodes.iter().take(10) {
        let bar = "#".repeat(*count as usize);
        println!("    {:20} ({}) {}", label, count, bar);
    }
    println!();

    // Show strongest connections
    let mut edges: Vec<_> = graph.all_edges().iter().map(|(from, to, data)| {
        let from_label = graph.get_node(from).map(|n| n.label.as_str()).unwrap_or("?");
        let to_label = graph.get_node(to).map(|n| n.label.as_str()).unwrap_or("?");
        (from_label.to_string(), to_label.to_string(), data.weight, data.co_activations)
    }).collect();
    edges.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    println!("  Strongest Connections:");
    for (from, to, weight, co_act) in edges.iter().take(10) {
        println!("    {} ←→ {} (weight: {:.3}, co-activations: {})", from, to, weight, co_act);
    }

    println!();
    println!("══════════════════════════════════════════════════════");
    println!("  Phase 2 complete. The colony self-organizes.");
    println!("  Knowledge graph built from {} documents by {} agents.",
        stats.documents_digested, stats.total_spawned);
    println!("══════════════════════════════════════════════════════");
}
