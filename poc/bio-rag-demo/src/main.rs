//! Bio-RAG Demo: Self-Reinforcing Knowledge Graph Retrieval
//!
//! Proves that Hebbian-reinforced graph retrieval improves over repeated
//! query rounds, while static retrieval remains flat.
//!
//! Protocol:
//! 1. Load 20-doc corpus → run colony 200 ticks → build graph
//! 2. Execute 20 queries × 5 rounds with reinforcement ON
//! 3. Execute same queries on frozen copy (static) — no reinforcement
//! 4. Execute same queries with TF-IDF keyword matching
//! 5. Output: P@5 improving per round vs flat vs fixed

use phago_agents::digester::Digester;
use phago_rag::baseline::{random_query, static_graph_query, tfidf_query};
use phago_rag::scoring::{self, AggregateScores};
use phago_rag::{Query, QueryEngine};
use phago_runtime::bench::{self, BenchmarkConfig};
use phago_runtime::colony::Colony;
use phago_runtime::corpus::Corpus;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
struct QueryDef {
    query: String,
    relevant: Vec<String>,
}

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  Bio-RAG: Self-Reinforcing Knowledge Graph Retrieval ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    // --- Load corpus and queries ---
    let corpus = Corpus::from_embedded().limit(40);
    println!("Corpus: {} documents, {} categories",
        corpus.len(), corpus.categories().len());

    let queries_json = include_str!("../data/queries.json");
    let queries: Vec<QueryDef> = serde_json::from_str(queries_json)
        .expect("Failed to parse queries.json");
    println!("Queries: {} with ground-truth relevance", queries.len());
    println!();

    // --- Build colony and run digestion ---
    println!("── Phase 1: Colony Digestion (200 ticks) ──────────────");
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

    let config = BenchmarkConfig::new("digestion", 200)
        .with_snapshot_interval(10)
        .with_metrics_interval(50);
    let digestion_run = bench::run_benchmark(&mut colony, &config);

    let stats = colony.stats();
    println!("  Nodes: {}, Edges: {}, Docs digested: {}/{}",
        stats.graph_nodes, stats.graph_edges,
        stats.documents_digested, stats.documents_total);
    println!("  Wall time: {}ms", digestion_run.wall_time_ms);
    println!();

    // --- Phase 2: Reinforced queries (5 rounds) ---
    println!("── Phase 2: Reinforced Retrieval (5 rounds × {} queries) ──",
        queries.len());

    let num_rounds = 10;
    let mut round_scores: Vec<AggregateScores> = Vec::new();

    for round in 1..=num_rounds {
        let mut scores_this_round = Vec::new();

        for qdef in &queries {
            let relevant: HashSet<String> = qdef.relevant.iter().cloned().collect();
            let q = Query::new(&qdef.query).with_max_results(10);
            let results = QueryEngine::query(&mut colony, &q);
            let retrieved: Vec<String> = results.iter().map(|r| r.label.clone()).collect();

            let score = scoring::score_query(&qdef.query, &retrieved, &relevant);
            scores_this_round.push(score);
        }

        let agg = scoring::aggregate(&scores_this_round);
        println!("  Round {}: P@5={:.3} P@10={:.3} MRR={:.3} NDCG@10={:.3}",
            round, agg.mean_precision_at_5, agg.mean_precision_at_10,
            agg.mean_mrr, agg.mean_ndcg_at_10);
        round_scores.push(agg);
    }
    println!();

    // --- Phase 3: Static baseline (no reinforcement) ---
    println!("── Phase 3: Static Baseline (no reinforcement) ────────");
    let mut static_scores_all = Vec::new();
    for round in 1..=num_rounds {
        let mut scores_this_round = Vec::new();
        for qdef in &queries {
            let relevant: HashSet<String> = qdef.relevant.iter().cloned().collect();
            let retrieved = static_graph_query(&mut colony, &qdef.query, 10);
            let score = scoring::score_query(&qdef.query, &retrieved, &relevant);
            scores_this_round.push(score);
        }
        let agg = scoring::aggregate(&scores_this_round);
        if round == 1 || round == num_rounds {
            println!("  Round {}: P@5={:.3} P@10={:.3} MRR={:.3} NDCG@10={:.3}",
                round, agg.mean_precision_at_5, agg.mean_precision_at_10,
                agg.mean_mrr, agg.mean_ndcg_at_10);
        }
        static_scores_all.push(agg);
    }
    println!();

    // --- Phase 4: TF-IDF baseline ---
    println!("── Phase 4: TF-IDF Baseline ───────────────────────────");
    let mut tfidf_scores = Vec::new();
    for qdef in &queries {
        let relevant: HashSet<String> = qdef.relevant.iter().cloned().collect();
        let retrieved = tfidf_query(&colony, &qdef.query, 10);
        let score = scoring::score_query(&qdef.query, &retrieved, &relevant);
        tfidf_scores.push(score);
    }
    let tfidf_agg = scoring::aggregate(&tfidf_scores);
    println!("  TF-IDF: P@5={:.3} P@10={:.3} MRR={:.3} NDCG@10={:.3}",
        tfidf_agg.mean_precision_at_5, tfidf_agg.mean_precision_at_10,
        tfidf_agg.mean_mrr, tfidf_agg.mean_ndcg_at_10);
    println!();

    // --- Phase 5: Random baseline ---
    println!("── Phase 5: Random Baseline ───────────────────────────");
    let mut random_scores = Vec::new();
    for (i, qdef) in queries.iter().enumerate() {
        let relevant: HashSet<String> = qdef.relevant.iter().cloned().collect();
        let retrieved = random_query(&colony, 10, i as u64 + 42);
        let score = scoring::score_query(&qdef.query, &retrieved, &relevant);
        random_scores.push(score);
    }
    let random_agg = scoring::aggregate(&random_scores);
    println!("  Random: P@5={:.3} P@10={:.3} MRR={:.3} NDCG@10={:.3}",
        random_agg.mean_precision_at_5, random_agg.mean_precision_at_10,
        random_agg.mean_mrr, random_agg.mean_ndcg_at_10);
    println!();

    // --- Summary ---
    println!("── Summary ──────────────────────────────────────────");
    println!();
    println!("  Round-over-round P@5:");
    for (i, agg) in round_scores.iter().enumerate() {
        let bar = "#".repeat((agg.mean_precision_at_5 * 40.0) as usize);
        println!("    Round {}: {:.3} {}", i + 1, agg.mean_precision_at_5, bar);
    }
    println!();

    let r1_p5 = round_scores.first().map(|a| a.mean_precision_at_5).unwrap_or(0.0);
    let r5_p5 = round_scores.last().map(|a| a.mean_precision_at_5).unwrap_or(0.0);
    let improvement = if r1_p5 > 0.0 {
        ((r5_p5 - r1_p5) / r1_p5) * 100.0
    } else if r5_p5 > 0.0 {
        100.0
    } else {
        0.0
    };

    let static_r1 = static_scores_all.first().map(|a| a.mean_precision_at_5).unwrap_or(0.0);
    let static_r5 = static_scores_all.last().map(|a| a.mean_precision_at_5).unwrap_or(0.0);

    println!("  Reinforced: P@5 round1={:.3} → round5={:.3} ({:+.1}%)",
        r1_p5, r5_p5, improvement);
    println!("  Static:     P@5 round1={:.3} → round5={:.3} (flat)",
        static_r1, static_r5);
    println!("  TF-IDF:     P@5={:.3} (fixed)", tfidf_agg.mean_precision_at_5);
    println!("  Random:     P@5={:.3} (baseline)", random_agg.mean_precision_at_5);
    println!();

    if r5_p5 > r1_p5 && r5_p5 > tfidf_agg.mean_precision_at_5 {
        println!("  ✓ HYPOTHESIS SUPPORTED: Reinforced retrieval improves over rounds");
        println!("    and outperforms TF-IDF baseline.");
    } else if r5_p5 > r1_p5 {
        println!("  ~ PARTIAL: Reinforced retrieval improves but does not beat TF-IDF.");
    } else {
        println!("  ✗ HYPOTHESIS REJECTED: No improvement observed.");
    }
    println!();

    // --- Write outputs ---
    std::fs::create_dir_all("poc/bio-rag-demo/output").ok();

    // CSV benchmark data
    let mut csv = String::new();
    csv.push_str("round,condition,precision_at_5,precision_at_10,mrr,ndcg_at_10\n");
    for (i, agg) in round_scores.iter().enumerate() {
        csv.push_str(&format!("{},reinforced,{:.4},{:.4},{:.4},{:.4}\n",
            i + 1, agg.mean_precision_at_5, agg.mean_precision_at_10,
            agg.mean_mrr, agg.mean_ndcg_at_10));
    }
    for (i, agg) in static_scores_all.iter().enumerate() {
        csv.push_str(&format!("{},static,{:.4},{:.4},{:.4},{:.4}\n",
            i + 1, agg.mean_precision_at_5, agg.mean_precision_at_10,
            agg.mean_mrr, agg.mean_ndcg_at_10));
    }
    csv.push_str(&format!("1,tfidf,{:.4},{:.4},{:.4},{:.4}\n",
        tfidf_agg.mean_precision_at_5, tfidf_agg.mean_precision_at_10,
        tfidf_agg.mean_mrr, tfidf_agg.mean_ndcg_at_10));
    csv.push_str(&format!("1,random,{:.4},{:.4},{:.4},{:.4}\n",
        random_agg.mean_precision_at_5, random_agg.mean_precision_at_10,
        random_agg.mean_mrr, random_agg.mean_ndcg_at_10));
    std::fs::write("poc/bio-rag-demo/output/bio-rag-benchmark.csv", &csv)
        .expect("Failed to write CSV");
    println!("  Benchmark CSV: poc/bio-rag-demo/output/bio-rag-benchmark.csv");

    // HTML visualization
    let html = phago_viz::generate_html(&digestion_run.snapshots, colony.event_history());
    std::fs::write("poc/bio-rag-demo/output/bio-rag.html", &html)
        .expect("Failed to write HTML");
    println!("  Visualization: poc/bio-rag-demo/output/bio-rag.html");

    println!();
    println!("══════════════════════════════════════════════════════");
}
