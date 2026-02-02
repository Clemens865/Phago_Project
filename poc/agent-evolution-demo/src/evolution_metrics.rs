//! Evolution-specific metrics for agent-evolution branch.
//!
//! Tracks specialization emergence, genome divergence, and
//! fitness trajectories across generations.

use phago_agents::genome::AgentGenome;
use phago_agents::fitness::AgentFitness;
use serde::Serialize;

/// Metrics for one snapshot in time during evolution.
#[derive(Debug, Clone, Serialize)]
pub struct EvolutionSnapshot {
    pub tick: u64,
    pub population: usize,
    pub mean_fitness: f64,
    pub max_fitness: f64,
    pub max_generation: u32,
    /// Average pairwise genome distance (specialization index).
    pub genome_divergence: f64,
    /// Mean sense_radius across population.
    pub mean_sense_radius: f64,
    /// Mean max_idle across population.
    pub mean_max_idle: f64,
    /// Mean explore_bias across population.
    pub mean_explore_bias: f64,
}

/// Compute genome divergence: average pairwise distance between all genomes.
pub fn genome_divergence(genomes: &[AgentGenome]) -> f64 {
    let n = genomes.len();
    if n < 2 {
        return 0.0;
    }

    let mut total_distance = 0.0;
    let mut pairs = 0;
    for i in 0..n {
        for j in (i + 1)..n {
            total_distance += genomes[i].distance(&genomes[j]);
            pairs += 1;
        }
    }

    if pairs > 0 {
        total_distance / pairs as f64
    } else {
        0.0
    }
}

/// Compute specialization index from genome parameter spread.
pub fn specialization_index(genomes: &[AgentGenome]) -> f64 {
    if genomes.len() < 2 {
        return 0.0;
    }

    // Coefficient of variation for each parameter
    let cvs = [
        cv(genomes.iter().map(|g| g.sense_radius).collect()),
        cv(genomes.iter().map(|g| g.max_idle as f64).collect()),
        cv(genomes.iter().map(|g| g.keyword_boost).collect()),
        cv(genomes.iter().map(|g| g.explore_bias).collect()),
    ];

    // Average CV across all parameters
    cvs.iter().sum::<f64>() / cvs.len() as f64
}

fn cv(values: Vec<f64>) -> f64 {
    let n = values.len() as f64;
    if n < 2.0 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / n;
    if mean.abs() < 1e-10 {
        return 0.0;
    }
    let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    variance.sqrt() / mean.abs()
}

/// Build an evolution snapshot from current state.
pub fn build_snapshot(
    tick: u64,
    genomes: &[AgentGenome],
    fitness_data: &[&AgentFitness],
) -> EvolutionSnapshot {
    let population = genomes.len();
    let mean_fitness = if fitness_data.is_empty() {
        0.0
    } else {
        fitness_data.iter().map(|f| f.fitness).sum::<f64>() / fitness_data.len() as f64
    };
    let max_fitness = fitness_data.iter().map(|f| f.fitness).fold(0.0f64, f64::max);
    let max_generation = fitness_data.iter().map(|f| f.generation).max().unwrap_or(0);

    let divergence = genome_divergence(genomes);

    let mean_sense_radius = if genomes.is_empty() { 0.0 } else {
        genomes.iter().map(|g| g.sense_radius).sum::<f64>() / genomes.len() as f64
    };
    let mean_max_idle = if genomes.is_empty() { 0.0 } else {
        genomes.iter().map(|g| g.max_idle as f64).sum::<f64>() / genomes.len() as f64
    };
    let mean_explore_bias = if genomes.is_empty() { 0.0 } else {
        genomes.iter().map(|g| g.explore_bias).sum::<f64>() / genomes.len() as f64
    };

    EvolutionSnapshot {
        tick,
        population,
        mean_fitness,
        max_fitness,
        max_generation,
        genome_divergence: divergence,
        mean_sense_radius,
        mean_max_idle,
        mean_explore_bias,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divergence_zero_for_identical() {
        let g = AgentGenome::default_genome();
        let genomes = vec![g.clone(), g.clone(), g];
        assert!(genome_divergence(&genomes) < 1e-10);
    }

    #[test]
    fn divergence_positive_for_mutated() {
        let g = AgentGenome::default_genome();
        let genomes: Vec<AgentGenome> = (0..5).map(|i| g.mutate(0.3, i)).collect();
        assert!(genome_divergence(&genomes) > 0.0);
    }

    #[test]
    fn specialization_grows_with_diversity() {
        let g = AgentGenome::default_genome();
        let uniform = vec![g.clone(), g.clone()];
        let diverse: Vec<AgentGenome> = (0..5).map(|i| g.mutate(0.5, i * 100)).collect();
        assert!(specialization_index(&diverse) > specialization_index(&uniform));
    }
}
