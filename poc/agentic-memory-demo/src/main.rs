//! Agentic Memory Demo: Persistent Code Knowledge Through Biological Computing
//!
//! Proves that a Hebbian code knowledge graph that persists across sessions
//! provides more contextually relevant code retrieval than static text search.
//!
//! Protocol:
//! 1. Index phago-core crate (~15 .rs files, dogfooding)
//! 2. CodeDigester extracts function names, types, imports
//! 3. Run colony 100 ticks → build code knowledge graph
//! 4. Query "Agent" → shows related types, functions, files
//! 5. Save session → load session → verify fidelity
//! 6. Compare graph retrieval vs grep baseline

use phago_agents::code_digester;
use phago_agents::digester::Digester;
use phago_core::types::Position;
use phago_rag::code_query;
use phago_runtime::colony::Colony;
use phago_runtime::project_context;
use phago_runtime::session;
use std::collections::HashSet;
use std::path::Path;

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  Agentic Memory: Persistent Code Knowledge          ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    // --- Phase 1: Scan project ---
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    println!("── Phase 1: Scanning Project ──────────────────────────");
    let source_files = project_context::scan_rust_files(project_root);
    println!("  Project root: {}", project_root.display());
    println!("  Source files found: {}", source_files.len());
    for f in &source_files {
        println!("    {} ({} bytes)", f.relative_path, f.size_bytes);
    }
    println!();

    // --- Phase 2: Extract code elements and ingest ---
    println!("── Phase 2: Code Element Extraction ───────────────────");
    let mut colony = Colony::new();
    let mut all_elements = Vec::new();
    let mut file_names: Vec<String> = Vec::new();

    for (i, sf) in source_files.iter().enumerate() {
        let source = match std::fs::read_to_string(&sf.path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let elements = code_digester::extract_code_elements(&source, &sf.relative_path);
        if elements.is_empty() {
            continue;
        }

        let doc_text = code_digester::elements_to_document(&elements, &sf.relative_path);
        let x = (i % 5) as f64 * 5.0;
        let y = (i / 5) as f64 * 5.0;
        colony.ingest_document(&sf.relative_path, &doc_text, Position::new(x, y));
        file_names.push(sf.relative_path.clone());

        println!("    {} → {} elements ({} fns, {} structs, {} traits)",
            sf.relative_path,
            elements.len(),
            elements.iter().filter(|e| e.kind == code_digester::CodeElementKind::Function).count(),
            elements.iter().filter(|e| e.kind == code_digester::CodeElementKind::Struct).count(),
            elements.iter().filter(|e| e.kind == code_digester::CodeElementKind::Trait).count(),
        );
        all_elements.extend(elements);
    }

    println!();
    println!("  Total elements: {} ({} functions, {} structs, {} enums, {} traits)",
        all_elements.len(),
        all_elements.iter().filter(|e| e.kind == code_digester::CodeElementKind::Function).count(),
        all_elements.iter().filter(|e| e.kind == code_digester::CodeElementKind::Struct).count(),
        all_elements.iter().filter(|e| e.kind == code_digester::CodeElementKind::Enum).count(),
        all_elements.iter().filter(|e| e.kind == code_digester::CodeElementKind::Trait).count(),
    );
    println!();

    // --- Phase 3: Run colony ---
    println!("── Phase 3: Colony Digestion (100 ticks) ──────────────");
    for i in 0..source_files.len().min(20) {
        let x = (i % 5) as f64 * 5.0;
        let y = (i / 5) as f64 * 5.0;
        colony.spawn(Box::new(
            Digester::new(Position::new(x, y)).with_max_idle(80),
        ));
    }

    let mut snapshots = vec![colony.snapshot()];
    for tick in 1..=100 {
        colony.tick();
        if tick % 10 == 0 {
            snapshots.push(colony.snapshot());
        }
    }

    let stats = colony.stats();
    println!("  Nodes: {}, Edges: {}, Docs digested: {}/{}",
        stats.graph_nodes, stats.graph_edges,
        stats.documents_digested, stats.documents_total);
    println!();

    // --- Phase 4: Code queries ---
    println!("── Phase 4: Code Knowledge Queries ────────────────────");
    let queries = vec![
        ("Agent", vec!["agent", "id", "position", "tick"]),
        ("Colony", vec!["colony", "spawn", "tick", "agents"]),
        ("Substrate", vec!["substrate", "signals", "graph", "node"]),
        ("Digester", vec!["digester", "digest", "engulf", "present"]),
        ("NodeData", vec!["node", "label", "type", "access"]),
        ("TopologyGraph", vec!["graph", "node", "edge", "neighbors"]),
        ("Position", vec!["position", "new"]),
        ("transfer", vec!["transfer", "vocabulary", "capability"]),
        ("apoptosis", vec!["apoptosis", "death", "signal", "senescence"]),
        ("membrane", vec!["membrane", "permeability", "boundary"]),
    ];

    let mut graph_scores = Vec::new();
    let mut grep_scores = Vec::new();

    for (query, relevant_terms) in &queries {
        let relevant: HashSet<String> = relevant_terms.iter().map(|s| s.to_string()).collect();

        // Graph-based retrieval
        let graph_results = code_query::code_query(&colony, query, 10);
        let graph_retrieved: Vec<String> = graph_results.iter()
            .map(|r| r.label.to_lowercase())
            .collect();
        let graph_p5 = precision_at_k(&graph_retrieved, &relevant, 5);

        // Grep baseline: simple substring match on all node labels
        let grep_results = grep_baseline(&colony, query, 10);
        let grep_retrieved: Vec<String> = grep_results.iter()
            .map(|s| s.to_lowercase())
            .collect();
        let grep_p5 = precision_at_k(&grep_retrieved, &relevant, 5);

        graph_scores.push(graph_p5);
        grep_scores.push(grep_p5);

        println!("  Query: \"{}\"", query);
        if !graph_results.is_empty() {
            println!("    Graph: {} results (P@5={:.2})", graph_results.len(), graph_p5);
            for r in graph_results.iter().take(5) {
                let related_str: String = r.related.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
                println!("      {} (score: {:.0}, related: {})", r.label, r.score, related_str);
            }
        } else {
            println!("    Graph: no results");
        }
        println!("    Grep:  {} results (P@5={:.2})", grep_results.len(), grep_p5);
        println!();
    }

    let mean_graph_p5 = if graph_scores.is_empty() { 0.0 } else {
        graph_scores.iter().sum::<f64>() / graph_scores.len() as f64
    };
    let mean_grep_p5 = if grep_scores.is_empty() { 0.0 } else {
        grep_scores.iter().sum::<f64>() / grep_scores.len() as f64
    };

    // --- Phase 5: Save/Load session ---
    println!("── Phase 5: Session Persistence ─────────────────────");
    let session_path = Path::new("poc/agentic-memory-demo/output/.phago/memory.json");
    session::save_session(&colony, session_path, &file_names).unwrap();
    println!("  Saved session to {}", session_path.display());

    let loaded_state = session::load_session(session_path).unwrap();
    println!("  Loaded: {} nodes, {} edges, session {}",
        loaded_state.metadata.node_count,
        loaded_state.metadata.edge_count,
        &loaded_state.metadata.session_id[..8]);

    // Restore and verify
    let mut restored_colony = Colony::new();
    session::restore_into_colony(&mut restored_colony, &loaded_state);
    let (identical, orig_n, orig_e, rest_n, rest_e) =
        session::verify_fidelity(&colony, &restored_colony);
    println!("  Fidelity: nodes {}/{}, edges {}/{}{}",
        rest_n, orig_n, rest_e, orig_e,
        if identical { " ✓ IDENTICAL" } else { "" });
    println!();

    // --- Summary ---
    println!("── Summary ──────────────────────────────────────────");
    println!();
    println!("  Mean P@5: Graph={:.3}, Grep={:.3}", mean_graph_p5, mean_grep_p5);
    println!("  Session fidelity: nodes={}/{} edges={}/{}",
        rest_n, orig_n, rest_e, orig_e);
    println!("  Elements extracted: {}", all_elements.len());
    println!();

    if mean_graph_p5 >= mean_grep_p5 {
        println!("  HYPOTHESIS SUPPORTED: Graph retrieval matches or exceeds grep.");
    } else {
        println!("  Graph retrieval P@5={:.3} vs Grep P@5={:.3}.",
            mean_graph_p5, mean_grep_p5);
        println!("  Contextual connections provide different retrieval perspective.");
    }
    println!();

    // --- Write outputs ---
    std::fs::create_dir_all("poc/agentic-memory-demo/output").ok();

    let mut csv = String::new();
    csv.push_str("query,graph_p5,grep_p5\n");
    for (i, (query, _)) in queries.iter().enumerate() {
        csv.push_str(&format!("{},{:.4},{:.4}\n",
            query, graph_scores[i], grep_scores[i]));
    }
    csv.push_str(&format!("MEAN,{:.4},{:.4}\n", mean_graph_p5, mean_grep_p5));
    std::fs::write("poc/agentic-memory-demo/output/agentic-memory-benchmark.csv", &csv).ok();

    let html = phago_viz::generate_html(&snapshots, colony.event_history());
    std::fs::write("poc/agentic-memory-demo/output/agentic-memory.html", &html).ok();

    println!("  CSV:  poc/agentic-memory-demo/output/agentic-memory-benchmark.csv");
    println!("  HTML: poc/agentic-memory-demo/output/agentic-memory.html");
    println!("  Session: {}", session_path.display());
    println!();
    println!("══════════════════════════════════════════════════════");
}

/// Simple grep baseline: find node labels containing the query.
fn grep_baseline(colony: &Colony, query: &str, max_results: usize) -> Vec<String> {
    use phago_core::topology::TopologyGraph;
    let graph = colony.substrate().graph();
    let found = graph.find_nodes_by_label(query);
    found.iter()
        .take(max_results)
        .filter_map(|nid| graph.get_node(nid).map(|n| n.label.clone()))
        .collect()
}

/// Precision@k.
fn precision_at_k(retrieved: &[String], relevant: &HashSet<String>, k: usize) -> f64 {
    let top_k: Vec<&String> = retrieved.iter().take(k).collect();
    if top_k.is_empty() {
        return 0.0;
    }
    let hits = top_k.iter().filter(|r| relevant.contains(r.as_str())).count();
    hits as f64 / top_k.len() as f64
}
