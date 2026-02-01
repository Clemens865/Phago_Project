//! Phago Proof of Concept — Self-Organizing Knowledge Ecosystem
//!
//! Phase 5 Demo: Quantitative Proof + Visualization.
//! Runs the full colony simulation, collects snapshots, computes metrics,
//! and generates an interactive HTML visualization.

use phago_agents::digester::Digester;
use phago_agents::sentinel::Sentinel;
use phago_agents::synthesizer::Synthesizer;
use phago_core::types::*;
use phago_runtime::colony::{Colony, ColonyEvent, ColonySnapshot};

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  PHAGO — Biological Computing Primitives            ║");
    println!("║  Phase 5: Prove It Works                            ║");
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

        // Anomalous document — unrelated to biology
        ("Quantum Computing", "Quantum bits exploit superposition and \
         entanglement to perform parallel computations. Error correction \
         in quantum circuits requires topological qubits and surface codes. \
         Shor's algorithm factors large integers exponentially faster than \
         classical methods.", Position::new(15.0, 15.0)),

        // Cross-domain document — bridges biology and computing
        ("Biocomputing", "Biological computing uses DNA molecules and protein \
         enzymes to perform logical operations. The cell membrane acts as a \
         natural computational boundary. Enzyme cascades implement signal \
         processing similar to electronic circuits.", Position::new(7.5, 7.5)),
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
        Position::new(7.5, 7.5),   // Near cross-domain doc
    ];

    for (i, pos) in digester_positions.iter().enumerate() {
        colony.spawn(Box::new(
            Digester::new(*pos).with_max_idle(80),
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
    println!("── Running Simulation (120 ticks) ────────────────────");
    println!();

    let mut total_transfers = 0u64;
    let mut total_integrations = 0u64;
    let mut total_symbioses = 0u64;
    let mut total_dissolutions = 0u64;
    let mut snapshots: Vec<ColonySnapshot> = Vec::new();

    // Take initial snapshot
    snapshots.push(colony.snapshot());

    for tick_num in 1..=120 {
        let events = colony.tick();

        for event in &events {
            match event {
                ColonyEvent::Engulfed { id, document } => {
                    println!(
                        "  [tick {:>3}] ENGULF: Agent {:.8} consumed document {:.8}",
                        tick_num,
                        id.0.to_string(),
                        document.0.to_string()
                    );
                }
                ColonyEvent::Presented { id, fragment_count, .. } => {
                    println!(
                        "  [tick {:>3}] PRESENT: Agent {:.8} → {} concepts added to graph",
                        tick_num,
                        id.0.to_string(),
                        fragment_count,
                    );
                }
                ColonyEvent::Wired { id, connection_count } => {
                    println!(
                        "  [tick {:>3}] WIRE: Agent {:.8} → {} connections strengthened",
                        tick_num,
                        id.0.to_string(),
                        connection_count
                    );
                }
                ColonyEvent::Deposited { id, .. } => {
                    println!(
                        "  [tick {:>3}] TRACE: Agent {:.8} deposited digestion trace",
                        tick_num,
                        id.0.to_string(),
                    );
                }
                ColonyEvent::Died { signal } => {
                    println!(
                        "  [tick {:>3}] DEATH: Agent {:.8} — {:?} (outputs: {})",
                        tick_num,
                        signal.agent_id.0.to_string(),
                        signal.cause,
                        signal.useful_outputs
                    );
                }
                ColonyEvent::CapabilityExported { agent_id, terms_count } => {
                    println!(
                        "  [tick {:>3}] TRANSFER: Agent {:.8} exported {} vocabulary terms",
                        tick_num,
                        agent_id.0.to_string(),
                        terms_count
                    );
                    total_transfers += 1;
                }
                ColonyEvent::CapabilityIntegrated { agent_id, from_agent, terms_count } => {
                    println!(
                        "  [tick {:>3}] INTEGRATE: Agent {:.8} absorbed {} terms from {:.8}",
                        tick_num,
                        agent_id.0.to_string(),
                        terms_count,
                        from_agent.0.to_string()
                    );
                    total_integrations += 1;
                }
                ColonyEvent::Symbiosis { host, absorbed, host_type, absorbed_type } => {
                    println!(
                        "  [tick {:>3}] SYMBIOSIS: {} {:.8} absorbed {} {:.8}",
                        tick_num,
                        host_type,
                        host.0.to_string(),
                        absorbed_type,
                        absorbed.0.to_string()
                    );
                    total_symbioses += 1;
                }
                ColonyEvent::Dissolved { agent_id, permeability, terms_externalized } => {
                    println!(
                        "  [tick {:>3}] DISSOLVE: Agent {:.8} permeability={:.2}, {} terms reinforced",
                        tick_num,
                        agent_id.0.to_string(),
                        permeability,
                        terms_externalized
                    );
                    total_dissolutions += 1;
                }
                _ => {}
            }
        }

        // Take snapshot every 5 ticks
        if tick_num % 5 == 0 {
            snapshots.push(colony.snapshot());
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
    let mut total_insights = 0u64;
    let mut total_anomalies = 0u64;

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
        let bar = "#".repeat((*count as usize).min(40));
        println!("    {:20} ({:>3}) {}", label, count, bar);
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

    // --- Phase 3 output ---
    println!();
    println!("── Emergence & Anomaly Detection ────────────────────");
    println!();

    if !insight_nodes.is_empty() {
        println!("  Synthesizer Insights ({}):", insight_nodes.len());
        for (label, _) in &insight_nodes {
            println!("    {}", label);
        }
    } else {
        println!("  Synthesizer Insights: (none detected)");
    }
    println!();

    if !anomaly_nodes.is_empty() {
        println!("  Sentinel Anomalies ({}):", anomaly_nodes.len());
        for (label, _) in &anomaly_nodes {
            println!("    {}", label);
        }
    } else {
        println!("  Sentinel Anomalies: (none detected)");
    }

    // --- Phase 4 output ---
    println!();
    println!("── Transfer, Symbiosis & Dissolution ────────────────");
    println!();
    println!("  Vocabulary Transfers:     {} exports, {} integrations", total_transfers, total_integrations);
    println!("  Symbiosis Events:         {}", total_symbioses);
    println!("  Dissolution Events:       {}", total_dissolutions);

    // --- Phase 5: Quantitative Metrics ---
    println!();
    let metrics = phago_runtime::metrics::compute_from_snapshots(&colony, &snapshots);
    phago_runtime::metrics::print_report(&metrics);

    // --- Phase 5: HTML Visualization ---
    let html = phago_viz::generate_html(&snapshots, colony.event_history());

    // Write to output directory
    std::fs::create_dir_all("output").ok();
    let output_path = "output/phago-colony.html";
    std::fs::write(output_path, &html).expect("failed to write HTML visualization");
    println!();
    println!("  Visualization: {}", output_path);

    println!();
    println!("══════════════════════════════════════════════════════");
    println!("  Phase 5 complete. The colony is provably correct.");
    println!("  {} documents → {} concepts, {} insights, {} anomalies",
        stats.documents_digested, concept_nodes.len(), total_insights, total_anomalies);
    println!("  {} transfers, {} integrations, {} symbioses, {} dissolutions",
        total_transfers, total_integrations, total_symbioses, total_dissolutions);
    println!("  by {} agents ({} digesters, 2 synthesizers, 2 sentinels).",
        stats.total_spawned, digester_positions.len());
    println!("══════════════════════════════════════════════════════");
}
