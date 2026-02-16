//! KG-Training Demo: Hebbian Knowledge Graph → Curriculum-Ordered Training Data
//!
//! Proves that Hebbian-weighted triples ordered by reinforcement strength
//! produce well-structured training data that aligns with ground-truth topics.
//!
//! Protocol:
//! 1. Run colony on 20-doc corpus (200 ticks)
//! 2. Export all triples with Hebbian weights
//! 3. Detect communities via label propagation
//! 4. Generate curriculum-ordered JSONL
//! 5. Measure NMI vs ground truth

use phago_agents::digester::Digester;
use phago_runtime::bench::{self, BenchmarkConfig};
use phago_runtime::colony::Colony;
use phago_runtime::community;
use phago_runtime::corpus::Corpus;
use phago_runtime::curriculum;
use phago_runtime::export;
use phago_runtime::training_format;

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  KG-Training: Hebbian Graph → Training Data         ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    // --- Phase 1: Build colony ---
    let corpus = Corpus::from_embedded().limit(40);
    let _ground_truth = corpus.ground_truth();
    println!(
        "Corpus: {} documents, {} categories",
        corpus.len(),
        corpus.categories().len()
    );

    let mut colony = Colony::new();
    corpus.ingest_into(&mut colony);

    // Spawn digesters distributed across the corpus (cap at 25 for scalability)
    let max_digesters = 25.min(corpus.documents.len());
    let step = corpus.documents.len().max(1) / max_digesters.max(1);
    for i in 0..max_digesters {
        let doc_idx = (i * step).min(corpus.documents.len() - 1);
        colony.spawn(Box::new(
            Digester::new(corpus.documents[doc_idx].position).with_max_idle(120),
        ));
    }

    println!();
    println!("── Phase 1: Colony Digestion (200 ticks) ──────────────");
    let config = BenchmarkConfig::new("digestion", 200)
        .with_snapshot_interval(10)
        .with_metrics_interval(100);
    let run = bench::run_benchmark(&mut colony, &config);

    let stats = colony.stats();
    println!(
        "  Nodes: {}, Edges: {}, Wall time: {}ms",
        stats.graph_nodes, stats.graph_edges, run.wall_time_ms
    );
    println!();

    // --- Phase 2: Export triples ---
    println!("── Phase 2: Export Weighted Triples ────────────────────");
    let triples = export::export_triples(&colony);
    let triple_stats = export::triple_stats(&triples);
    println!("  Total triples: {}", triple_stats.total);
    println!(
        "  Weight: mean={:.3}, median={:.3}, max={:.3}, min={:.3}",
        triple_stats.mean_weight,
        triple_stats.median_weight,
        triple_stats.max_weight,
        triple_stats.min_weight
    );
    println!(
        "  Mean co-activations: {:.1}",
        triple_stats.mean_co_activations
    );
    println!();

    // Show top-10 and bottom-10 triples
    println!("  Top-10 triples (highest weight):");
    for t in triples.iter().take(10) {
        println!(
            "    {:.3} | {} ─ {} ─ {} (co-act: {})",
            t.weight, t.subject, t.predicate, t.object, t.co_activations
        );
    }
    println!();
    println!("  Bottom-10 triples (lowest weight):");
    for t in triples.iter().rev().take(10) {
        println!(
            "    {:.3} | {} ─ {} ─ {} (co-act: {})",
            t.weight, t.subject, t.predicate, t.object, t.co_activations
        );
    }
    println!();

    // --- Phase 3: Community detection ---
    println!("── Phase 3: Community Detection ─────────────────────");
    let communities = community::detect_communities(&colony, 20);
    println!("  Communities detected: {}", communities.num_communities);
    for c in &communities.communities {
        let sample: Vec<&str> = c.members.iter().take(5).map(|s| s.as_str()).collect();
        println!(
            "    Community {} ({} members): {}{}",
            c.id,
            c.size,
            sample.join(", "),
            if c.size > 5 { ", ..." } else { "" }
        );
    }
    println!();

    // --- Phase 4: NMI against ground truth ---
    println!("── Phase 4: NMI vs Ground Truth ─────────────────────");

    // Map node labels to ground-truth categories by checking which
    // category's documents contain each keyword
    let mut node_gt: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for doc in &corpus.documents {
        if let Some(cat) = &doc.category {
            let words: Vec<String> = doc
                .content
                .to_lowercase()
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| w.len() >= 3)
                .map(|w| w.to_string())
                .collect();
            for word in words {
                node_gt.entry(word).or_insert_with(|| cat.clone());
            }
        }
    }

    let nmi = community::compute_nmi(&communities.assignments, &node_gt);
    println!("  NMI (detected vs ground-truth): {:.3}", nmi);

    if nmi > 0.3 {
        println!("  HYPOTHESIS SUPPORTED: NMI > 0.3, communities align with topics.");
    } else {
        println!("  NMI below threshold (0.3). Community structure differs from topics.");
    }
    println!();

    // --- Phase 5: Curriculum ordering ---
    println!("── Phase 5: Curriculum Ordering ─────────────────────");
    let cur = curriculum::build_curriculum(&triples, &communities);
    let (foundation, bridges, periphery) = training_format::section_counts(&cur);
    println!(
        "  Foundation triples: {} (high-weight, same-community)",
        foundation
    );
    println!("  Bridge triples:    {} (cross-community)", bridges);
    println!("  Periphery triples: {} (low-weight)", periphery);
    println!("  Total:             {}", cur.total());
    println!();

    // Verify coherence: are high-weight triples from same communities?
    let high_weight_same_community = cur
        .foundation
        .iter()
        .filter(|t| {
            let sc = communities.assignments.get(&t.subject);
            let oc = communities.assignments.get(&t.object);
            matches!((sc, oc), (Some(a), Some(b)) if a == b)
        })
        .count();
    let coherence = if !cur.foundation.is_empty() {
        high_weight_same_community as f64 / cur.foundation.len() as f64
    } else {
        0.0
    };
    println!(
        "  Foundation coherence: {:.1}% same-community",
        coherence * 100.0
    );

    // Weight-quality check: mean weight of foundation vs periphery
    let mean_foundation_weight = if cur.foundation.is_empty() {
        0.0
    } else {
        cur.foundation.iter().map(|t| t.weight).sum::<f64>() / cur.foundation.len() as f64
    };
    let mean_periphery_weight = if cur.periphery.is_empty() {
        0.0
    } else {
        cur.periphery.iter().map(|t| t.weight).sum::<f64>() / cur.periphery.len() as f64
    };
    println!("  Foundation mean weight: {:.3}", mean_foundation_weight);
    println!("  Periphery mean weight:  {:.3}", mean_periphery_weight);
    println!(
        "  Weight ratio:           {:.1}x",
        if mean_periphery_weight > 0.0 {
            mean_foundation_weight / mean_periphery_weight
        } else {
            0.0
        }
    );
    println!();

    // --- Write outputs ---
    std::fs::create_dir_all("poc/kg-training-demo/output").ok();

    // Curriculum JSONL
    let jsonl = training_format::to_jsonl(&cur);
    std::fs::write(
        "poc/kg-training-demo/output/curriculum-ordered.jsonl",
        &jsonl,
    )
    .ok();

    // Random-ordered JSONL
    let random_jsonl = training_format::to_jsonl_random(&cur, 42);
    std::fs::write(
        "poc/kg-training-demo/output/random-ordered.jsonl",
        &random_jsonl,
    )
    .ok();

    // CSV benchmark
    let mut csv = String::new();
    csv.push_str("metric,value\n");
    csv.push_str(&format!("nmi,{:.4}\n", nmi));
    csv.push_str(&format!("communities,{}\n", communities.num_communities));
    csv.push_str(&format!("foundation_triples,{}\n", foundation));
    csv.push_str(&format!("bridge_triples,{}\n", bridges));
    csv.push_str(&format!("periphery_triples,{}\n", periphery));
    csv.push_str(&format!("foundation_coherence,{:.4}\n", coherence));
    csv.push_str(&format!(
        "mean_foundation_weight,{:.4}\n",
        mean_foundation_weight
    ));
    csv.push_str(&format!(
        "mean_periphery_weight,{:.4}\n",
        mean_periphery_weight
    ));
    csv.push_str(&format!(
        "triple_coverage,{:.4}\n",
        triple_stats.total as f64 / stats.graph_nodes.max(1) as f64
    ));
    std::fs::write(
        "poc/kg-training-demo/output/kg-training-benchmark.csv",
        &csv,
    )
    .ok();

    // HTML visualization
    let html = phago_viz::generate_html(&run.snapshots, colony.event_history());
    std::fs::write("poc/kg-training-demo/output/kg-training.html", &html).ok();

    println!("  JSONL (curriculum): poc/kg-training-demo/output/curriculum-ordered.jsonl");
    println!("  JSONL (random):     poc/kg-training-demo/output/random-ordered.jsonl");
    println!("  CSV:                poc/kg-training-demo/output/kg-training-benchmark.csv");
    println!("  HTML:               poc/kg-training-demo/output/kg-training.html");
    println!();
    println!("══════════════════════════════════════════════════════");
}
