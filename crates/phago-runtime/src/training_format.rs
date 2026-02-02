//! Training data format generators.
//!
//! Converts curriculum-ordered triples into JSONL and Alpaca
//! instruction format for language model fine-tuning.

use crate::curriculum::Curriculum;
use crate::export::WeightedTriple;
use serde::Serialize;

/// A single training example in JSONL format.
#[derive(Debug, Clone, Serialize)]
pub struct TrainingExample {
    pub instruction: String,
    pub input: String,
    pub output: String,
    pub weight: f64,
    pub section: String,
}

/// Generate JSONL training data from a curriculum.
pub fn to_jsonl(curriculum: &Curriculum) -> String {
    let mut lines = Vec::new();

    for triple in &curriculum.foundation {
        lines.push(triple_to_example(triple, "foundation"));
    }
    for triple in &curriculum.bridges {
        lines.push(triple_to_example(triple, "bridge"));
    }
    for triple in &curriculum.periphery {
        lines.push(triple_to_example(triple, "periphery"));
    }

    lines.iter()
        .filter_map(|ex| serde_json::to_string(ex).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate randomly-ordered JSONL from the same triples (baseline).
pub fn to_jsonl_random(curriculum: &Curriculum, seed: u64) -> String {
    let mut all_triples: Vec<(&WeightedTriple, &str)> = Vec::new();
    for t in &curriculum.foundation { all_triples.push((t, "foundation")); }
    for t in &curriculum.bridges { all_triples.push((t, "bridge")); }
    for t in &curriculum.periphery { all_triples.push((t, "periphery")); }

    // Deterministic shuffle
    let mut indices: Vec<usize> = (0..all_triples.len()).collect();
    let mut rng = seed;
    for i in (1..indices.len()).rev() {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (rng >> 33) as usize % (i + 1);
        indices.swap(i, j);
    }

    let lines: Vec<String> = indices.iter()
        .filter_map(|&i| {
            let (triple, section) = all_triples.get(i)?;
            let ex = triple_to_example(triple, section);
            serde_json::to_string(&ex).ok()
        })
        .collect();

    lines.join("\n")
}

fn triple_to_example(triple: &WeightedTriple, section: &str) -> TrainingExample {
    TrainingExample {
        instruction: format!(
            "What is the relationship between '{}' and '{}'?",
            triple.subject, triple.object
        ),
        input: String::new(),
        output: format!(
            "'{}' is {} '{}'. This is a {} concept with connection strength {:.2}.",
            triple.subject,
            triple.predicate,
            triple.object,
            section,
            triple.weight,
        ),
        weight: triple.weight,
        section: section.to_string(),
    }
}

/// Count examples per section.
pub fn section_counts(curriculum: &Curriculum) -> (usize, usize, usize) {
    (
        curriculum.foundation.len(),
        curriculum.bridges.len(),
        curriculum.periphery.len(),
    )
}
