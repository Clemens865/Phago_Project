//! Agent Evolution Demo: Evolutionary Agents Through Intrinsic Selection
//!
//! Proves that agents evolving through apoptosis + mutation produce a
//! measurably richer knowledge graph than a static population.
//!
//! Protocol:
//! 1. Run 1000-tick sim with STATIC population: 11 agents, default params, no spawning
//! 2. Run 1000-tick sim with EVOLVING population: start 5, spawn on death, cap 15
//! 3. Run 1000-tick sim with RANDOM spawn: same rate, random genomes (control)
//! 4. Compare graph richness, clustering, vocabulary spread at ticks 200, 500, 1000

mod evolution_metrics;

use phago_agents::digester::Digester;
use phago_core::agent::Agent;
use phago_agents::fitness::FitnessTracker;
use phago_agents::genome::AgentGenome;
use phago_agents::spawn::{FitnessSpawnPolicy, NoSpawnPolicy, RandomSpawnPolicy, SpawnPolicy};
use phago_core::types::*;
use phago_runtime::colony::{Colony, ColonyEvent, ColonySnapshot};
use phago_runtime::corpus::Corpus;
use phago_runtime::metrics;
use std::collections::HashMap;

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  Agent Evolution: Intrinsic Selection Through       ║");
    println!("║  Apoptosis + Mutation                               ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    let corpus = Corpus::from_embedded().limit(40);
    let total_ticks = 300u64;
    let checkpoint_ticks = vec![100, 200, 300];

    // --- Condition 1: Static Population ---
    println!("── Condition 1: Static Population (11 agents) ────────");
    let (_static_snapshots, static_checkpoints, static_evo_snapshots) =
        run_condition("static", &corpus, total_ticks, &checkpoint_ticks,
            11, &mut NoSpawnPolicy, 0.0);

    // --- Condition 2: Evolving Population ---
    println!("── Condition 2: Evolving Population (5→15, mutation) ──");
    let (evolved_snapshots, evolved_checkpoints, evolved_evo_snapshots) =
        run_condition("evolved", &corpus, total_ticks, &checkpoint_ticks,
            5, &mut FitnessSpawnPolicy::new(15, 0.15), 0.15);

    // --- Condition 3: Random Spawn (control) ---
    println!("── Condition 3: Random Spawn (5→15, random genomes) ──");
    let (_random_snapshots, random_checkpoints, random_evo_snapshots) =
        run_condition("random", &corpus, total_ticks, &checkpoint_ticks,
            5, &mut RandomSpawnPolicy::new(15), 0.5);

    // --- Comparison ---
    println!();
    println!("── Comparison at Checkpoints ──────────────────────────");
    println!();

    println!("  {:>12} │ {:>6} {:>6} {:>7} {:>7} {:>8}",
        "Condition", "Nodes", "Edges", "Densty", "Clustr", "AvgDeg");
    println!("  {:─>12}─┼─{:─>6}─{:─>6}─{:─>7}─{:─>7}─{:─>8}",
        "", "", "", "", "", "");

    for (tick_idx, tick) in checkpoint_ticks.iter().enumerate() {
        println!("  Tick {}:", tick);
        for (name, checkpoints) in [
            ("Static", &static_checkpoints),
            ("Evolved", &evolved_checkpoints),
            ("Random", &random_checkpoints),
        ] {
            if let Some(m) = checkpoints.get(tick_idx) {
                println!("    {:>10} │ {:>6} {:>6} {:>7.3} {:>7.3} {:>8.2}",
                    name,
                    m.graph_richness.node_count,
                    m.graph_richness.edge_count,
                    m.graph_richness.density,
                    m.graph_richness.clustering_coefficient,
                    m.graph_richness.avg_degree);
            }
        }
        println!();
    }

    // --- Evolution-specific metrics ---
    println!("── Evolution Metrics ─────────────────────────────────");
    println!();
    for snap in &evolved_evo_snapshots {
        println!("  Tick {:>4}: pop={:>2} gen={:>2} fit={:.3} div={:.3} sense={:.1} idle={:.0} explore={:.2}",
            snap.tick, snap.population, snap.max_generation,
            snap.mean_fitness, snap.genome_divergence,
            snap.mean_sense_radius, snap.mean_max_idle, snap.mean_explore_bias);
    }
    println!();

    // --- Hypothesis test ---
    let final_evolved = evolved_checkpoints.last();
    let final_static = static_checkpoints.last();
    let final_random = random_checkpoints.last();

    if let (Some(ev), Some(st), Some(rn)) = (final_evolved, final_static, final_random) {
        let ev_clustering = ev.graph_richness.clustering_coefficient;
        let st_clustering = st.graph_richness.clustering_coefficient;
        let rn_clustering = rn.graph_richness.clustering_coefficient;

        println!("  Final clustering: Evolved={:.3} Static={:.3} Random={:.3}",
            ev_clustering, st_clustering, rn_clustering);

        let ev_edges = ev.graph_richness.edge_count;
        let st_edges = st.graph_richness.edge_count;

        println!("  Final edges:      Evolved={} Static={} Random={}",
            ev_edges, st_edges, rn.graph_richness.edge_count);
        println!();

        if ev_clustering > st_clustering || ev_edges > st_edges {
            println!("  HYPOTHESIS SUPPORTED: Evolved population produces richer graph.");
        } else {
            println!("  HYPOTHESIS NOT SUPPORTED at tick {}. Results may improve with more ticks.", total_ticks);
        }
    }

    // --- Write outputs ---
    std::fs::create_dir_all("poc/agent-evolution-demo/output").ok();

    // CSV
    let mut csv = String::new();
    csv.push_str("tick,condition,nodes,edges,density,clustering,avg_degree,genome_divergence\n");
    for (tick_idx, tick) in checkpoint_ticks.iter().enumerate() {
        for (name, checkpoints, evo_snaps) in [
            ("static", &static_checkpoints, &static_evo_snapshots),
            ("evolved", &evolved_checkpoints, &evolved_evo_snapshots),
            ("random", &random_checkpoints, &random_evo_snapshots),
        ] {
            if let Some(m) = checkpoints.get(tick_idx) {
                let div = evo_snaps.iter()
                    .find(|s| s.tick == *tick)
                    .map(|s| s.genome_divergence)
                    .unwrap_or(0.0);
                csv.push_str(&format!("{},{},{},{},{:.4},{:.4},{:.2},{:.4}\n",
                    tick, name,
                    m.graph_richness.node_count,
                    m.graph_richness.edge_count,
                    m.graph_richness.density,
                    m.graph_richness.clustering_coefficient,
                    m.graph_richness.avg_degree,
                    div));
            }
        }
    }
    std::fs::write("poc/agent-evolution-demo/output/agent-evolution-benchmark.csv", &csv).ok();
    println!();
    println!("  CSV: poc/agent-evolution-demo/output/agent-evolution-benchmark.csv");

    // HTML visualization (use evolved run)
    let html = phago_viz::generate_html(&evolved_snapshots, &[]);
    std::fs::write("poc/agent-evolution-demo/output/agent-evolution.html", &html).ok();
    println!("  HTML: poc/agent-evolution-demo/output/agent-evolution.html");
    println!();
    println!("══════════════════════════════════════════════════════");
}

/// Run one experimental condition and collect metrics.
fn run_condition(
    name: &str,
    corpus: &Corpus,
    total_ticks: u64,
    checkpoint_ticks: &[u64],
    initial_agents: usize,
    spawn_policy: &mut dyn SpawnPolicy,
    mutation_rate: f64,
) -> (Vec<ColonySnapshot>, Vec<metrics::ColonyMetrics>, Vec<evolution_metrics::EvolutionSnapshot>) {
    let mut colony = Colony::new();
    corpus.ingest_into(&mut colony);

    // Track genomes per agent
    let mut agent_genomes: HashMap<AgentId, AgentGenome> = HashMap::new();
    let mut fitness_tracker = FitnessTracker::new();

    // Spawn initial agents with default or slightly mutated genomes
    for i in 0..initial_agents {
        let genome = if mutation_rate > 0.0 {
            AgentGenome::default_genome().mutate(mutation_rate * 0.5, i as u64)
        } else {
            AgentGenome::default_genome()
        };

        let pos = Position::new(
            (i % 5) as f64 * 5.0,
            (i / 5) as f64 * 5.0,
        );
        let digester = Digester::new(pos)
            .with_max_idle(genome.max_idle);
        let id = digester.id();
        agent_genomes.insert(id, genome);
        fitness_tracker.register(id, 0);
        colony.spawn(Box::new(digester));
    }

    let mut snapshots = Vec::new();
    let mut checkpoint_metrics = Vec::new();
    let mut evo_snapshots = Vec::new();
    let mut _total_spawned = 0u64;

    snapshots.push(colony.snapshot());

    for tick in 1..=total_ticks {
        let events = colony.tick();

        // Track fitness from events
        let alive_ids: Vec<AgentId> = colony.agents().iter().map(|a| a.id()).collect();
        fitness_tracker.tick_all(&alive_ids);

        for event in &events {
            match event {
                ColonyEvent::Presented { id, fragment_count, .. } => {
                    fitness_tracker.record_concepts(id, *fragment_count as u64);
                }
                ColonyEvent::Wired { id, connection_count } => {
                    fitness_tracker.record_edges(id, *connection_count as u64);
                }
                ColonyEvent::Died { signal } => {
                    // On death, try to spawn replacement
                    let fittest = fitness_tracker.fittest(&alive_ids);
                    let fittest_genome = fittest.and_then(|f| agent_genomes.get(&f.agent_id));
                    let fittest_pos = fittest.and_then(|f| {
                        colony.agents().iter()
                            .find(|a| a.id() == f.agent_id)
                            .map(|a| a.position())
                    });

                    if let Some((genome, pos)) = spawn_policy.on_death(
                        signal.agent_id,
                        colony.alive_count(),
                        fittest_genome,
                        fittest_pos,
                    ) {
                        let generation = fitness_tracker.next_generation();
                        let digester = Digester::new(pos)
                            .with_max_idle(genome.max_idle);
                        let id = digester.id();
                        agent_genomes.insert(id, genome);
                        fitness_tracker.register(id, generation);
                        colony.spawn(Box::new(digester));
                        _total_spawned += 1;
                    }
                }
                _ => {}
            }
        }

        // Collect snapshots every 50 ticks
        if tick % 50 == 0 {
            snapshots.push(colony.snapshot());
        }

        // Checkpoints
        if checkpoint_ticks.contains(&tick) {
            let m = metrics::compute_from_snapshots(&colony, &snapshots);
            checkpoint_metrics.push(m);

            // Evolution snapshot
            let alive_ids: Vec<AgentId> = colony.agents().iter().map(|a| a.id()).collect();
            let genomes: Vec<AgentGenome> = alive_ids.iter()
                .filter_map(|id| agent_genomes.get(id).cloned())
                .collect();
            let fitness_data: Vec<&phago_agents::fitness::AgentFitness> = alive_ids.iter()
                .filter_map(|id| fitness_tracker.get(id))
                .collect();
            let evo_snap = evolution_metrics::build_snapshot(tick, &genomes, &fitness_data);
            evo_snapshots.push(evo_snap);
        }
    }

    let stats = colony.stats();
    println!("  {} complete: {} nodes, {} edges, {} alive, {} total spawned",
        name, stats.graph_nodes, stats.graph_edges,
        stats.agents_alive, stats.total_spawned);

    (snapshots, checkpoint_metrics, evo_snapshots)
}
