//! Phago Proof of Concept — Self-Organizing Knowledge Ecosystem
//!
//! Phase 3 Demo: Multiple agent types self-organize around documents.
//! Digesters break down text. Synthesizers detect cross-document patterns
//! once quorum is reached. Sentinels mature and detect anomalies.

use phago_agents::digester::Digester;
use phago_agents::sentinel::Sentinel;
use phago_agents::synthesizer::Synthesizer;
use phago_core::types::*;
use phago_runtime::colony::{Colony, ColonyEvent};

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  PHAGO — Biological Computing Primitives            ║");
    println!("║  Phase 3: Emergence — Collective Intelligence       ║");
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

        // Anomalous document — unrelated to biology, should trigger Sentinel
        ("Quantum Computing", "Quantum bits exploit superposition and \
         entanglement to perform parallel computations. Error correction \
         in quantum circuits requires topological qubits and surface codes. \
         Shor's algorithm factors large integers exponentially faster than \
         classical methods.", Position::new(15.0, 15.0)),
    ];

    for (title, content, pos) in &docs {
        colony.ingest_document(title, content, *pos);
        println!("  [doc] \"{}\" at ({:.0}, {:.0})", title, pos.x, pos.y);
    }
    println!();

    // --- Spawn agents ---
    println!("── Spawning Agents ─────────────────────────────────");
    println!();

    // Digesters — one near each document cluster
    let digester_positions = vec![
        Position::new(0.0, 0.0),
        Position::new(5.0, 0.0),
        Position::new(0.0, 5.0),
        Position::new(5.0, 5.0),
        Position::new(10.0, 0.0),
        Position::new(15.0, 15.0), // Near anomalous doc
        Position::new(2.5, 2.5),   // Explorer
    ];

    for (i, pos) in digester_positions.iter().enumerate() {
        colony.spawn(Box::new(
            Digester::new(*pos).with_max_idle(60),
        ));
        println!("  [digester  {}] at ({:.1}, {:.1})", i + 1, pos.x, pos.y);
    }

    // Synthesizers — positioned centrally to survey the whole substrate
    for (i, pos) in [Position::new(5.0, 2.5), Position::new(7.5, 5.0)].iter().enumerate() {
        colony.spawn(Box::new(Synthesizer::new(*pos)));
        println!("  [synthesizer {}] at ({:.1}, {:.1})", i + 1, pos.x, pos.y);
    }

    // Sentinels — positioned to observe different regions
    for (i, pos) in [Position::new(2.5, 2.5), Position::new(12.0, 10.0)].iter().enumerate() {
        colony.spawn(Box::new(Sentinel::new(*pos)));
        println!("  [sentinel  {}] at ({:.1}, {:.1})", i + 1, pos.x, pos.y);
    }

    println!();
    println!("  Total agents: {} digesters, 2 synthesizers, 2 sentinels",
        digester_positions.len());
    println!();

    // --- Run simulation ---
    println!("── Running Simulation (80 ticks) ────────────────────");
    println!();

    let mut total_insights = 0u64;
    let mut total_anomalies = 0u64;

    for tick_num in 1..=80 {
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
                    // Check if this is an insight or anomaly presentation
                    let agent_type = if fragment_count > &0 {
                        // We can infer from agent type string, but since we
                        // don't have that here, just report generically
                        "concepts"
                    } else {
                        "fragments"
                    };
                    println!(
                        "  [tick {:>2}] PRESENT: Agent {:.8} → {} {} added to graph",
                        tick_num,
                        id.0.to_string(),
                        fragment_count,
                        agent_type,
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
    println!("    Total nodes:        {}", stats.graph_nodes);
    println!("    Connections:        {}", stats.graph_edges);
    println!();

    // Show top concepts (nodes with highest access count)
    use phago_core::topology::TopologyGraph;
    let graph = colony.substrate().graph();
    let mut concept_nodes = Vec::new();
    let mut insight_nodes = Vec::new();
    let mut anomaly_nodes = Vec::new();

    for id in graph.all_nodes() {
        if let Some(n) = graph.get_node(&id) {
            if n.label.starts_with("[BRIDGE") || n.label.starts_with("[CLUSTER") {
                insight_nodes.push((n.label.clone(), n.access_count));
                total_insights += 1;
            } else if n.label.starts_with("[ANOMALY") {
                anomaly_nodes.push((n.label.clone(), n.access_count));
                total_anomalies += 1;
            } else {
                concept_nodes.push((n.label.clone(), n.access_count));
            }
        }
    }
    concept_nodes.sort_by(|a, b| b.1.cmp(&a.1));

    println!("  Top Concepts (by reinforcement):");
    for (label, count) in concept_nodes.iter().take(10) {
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
        println!("    {} <-> {} (weight: {:.3}, co-activations: {})", from, to, weight, co_act);
    }

    // --- Phase 3 specific output ---
    println!();
    println!("── Emergence & Anomaly Detection ────────────────────");
    println!();

    if !insight_nodes.is_empty() {
        println!("  Synthesizer Insights ({}):", insight_nodes.len());
        for (label, _) in &insight_nodes {
            println!("    {}", label);
        }
    } else {
        println!("  Synthesizer Insights: (none detected — quorum may not have been reached)");
    }
    println!();

    if !anomaly_nodes.is_empty() {
        println!("  Sentinel Anomalies ({}):", anomaly_nodes.len());
        for (label, _) in &anomaly_nodes {
            println!("    {}", label);
        }
    } else {
        println!("  Sentinel Anomalies: (none detected — maturation/scanning may need more ticks)");
    }

    println!();
    println!("══════════════════════════════════════════════════════");
    println!("  Phase 3 complete. The colony demonstrates emergence.");
    println!("  {} documents → {} concepts, {} insights, {} anomalies",
        stats.documents_digested, concept_nodes.len(), total_insights, total_anomalies);
    println!("  by {} agents ({} digesters, 2 synthesizers, 2 sentinels).",
        stats.total_spawned, digester_positions.len());
    println!("══════════════════════════════════════════════════════");
}
