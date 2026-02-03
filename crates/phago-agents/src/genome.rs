//! Agent genome — evolvable parameters for biological agents.
//!
//! Each agent carries a genome that encodes its behavioral parameters.
//! When agents reproduce (via Transfer/Spawn), the genome is inherited
//! with random mutations. Natural selection occurs through apoptosis:
//! agents with poor fitness die faster, removing their genomes.

use serde::{Deserialize, Serialize};

/// Evolvable parameters for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentGenome {
    /// Sensing radius — how far the agent can detect signals.
    pub sense_radius: f64,
    /// Maximum idle ticks before apoptosis triggers.
    pub max_idle: u64,
    /// Boost factor for known vocabulary terms during digestion.
    pub keyword_boost: f64,
    /// Probability of exploring randomly vs following gradients.
    pub explore_bias: f64,
    /// Tendency to move toward substrate boundary vs center.
    pub boundary_bias: f64,

    // Wiring strategy parameters
    /// Initial weight for tentative edges (first co-occurrence). Range: [0.05, 0.5].
    pub tentative_weight: f64,
    /// Weight boost per subsequent co-activation. Range: [0.01, 0.3].
    pub reinforcement_boost: f64,
    /// Fraction of concept pairs to wire per document. Range: [0.1, 1.0].
    /// 1.0 = wire all pairs, 0.5 = wire ~half (probabilistic), etc.
    pub wiring_selectivity: f64,
}

impl AgentGenome {
    /// Default genome with standard parameters.
    pub fn default_genome() -> Self {
        Self {
            sense_radius: 10.0,
            max_idle: 30,
            keyword_boost: 3.0,
            explore_bias: 0.2,
            boundary_bias: 0.0,
            tentative_weight: 0.1,
            reinforcement_boost: 0.1,
            wiring_selectivity: 1.0,
        }
    }

    /// Create a mutated copy of this genome.
    ///
    /// Each parameter is perturbed by ±mutation_rate (as a fraction).
    /// Uses a simple deterministic PRNG seeded by the given value.
    pub fn mutate(&self, mutation_rate: f64, seed: u64) -> Self {
        let mut rng = seed;
        let mut next = || -> f64 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            // Map to [-1.0, 1.0]
            ((rng >> 33) as f64 / (u32::MAX as f64)) * 2.0 - 1.0
        };

        Self {
            sense_radius: (self.sense_radius * (1.0 + next() * mutation_rate)).clamp(2.0, 30.0),
            max_idle: ((self.max_idle as f64 * (1.0 + next() * mutation_rate)).round() as u64).clamp(5, 100),
            keyword_boost: (self.keyword_boost * (1.0 + next() * mutation_rate)).clamp(0.5, 10.0),
            explore_bias: (self.explore_bias + next() * mutation_rate * 0.5).clamp(0.0, 1.0),
            boundary_bias: (self.boundary_bias + next() * mutation_rate * 0.5).clamp(-1.0, 1.0),
            tentative_weight: (self.tentative_weight * (1.0 + next() * mutation_rate)).clamp(0.05, 0.5),
            reinforcement_boost: (self.reinforcement_boost * (1.0 + next() * mutation_rate)).clamp(0.01, 0.3),
            wiring_selectivity: (self.wiring_selectivity + next() * mutation_rate * 0.3).clamp(0.1, 1.0),
        }
    }

    /// Compute the Euclidean distance between two genomes in parameter space.
    /// Parameters are normalized to [0,1] range before computing distance.
    pub fn distance(&self, other: &AgentGenome) -> f64 {
        let dims = [
            ((self.sense_radius - 2.0) / 28.0, (other.sense_radius - 2.0) / 28.0),
            (self.max_idle as f64 / 100.0, other.max_idle as f64 / 100.0),
            ((self.keyword_boost - 0.5) / 9.5, (other.keyword_boost - 0.5) / 9.5),
            (self.explore_bias, other.explore_bias),
            ((self.boundary_bias + 1.0) / 2.0, (other.boundary_bias + 1.0) / 2.0),
            ((self.tentative_weight - 0.05) / 0.45, (other.tentative_weight - 0.05) / 0.45),
            ((self.reinforcement_boost - 0.01) / 0.29, (other.reinforcement_boost - 0.01) / 0.29),
            ((self.wiring_selectivity - 0.1) / 0.9, (other.wiring_selectivity - 0.1) / 0.9),
        ];

        let sum_sq: f64 = dims.iter().map(|(a, b)| (a - b).powi(2)).sum();
        sum_sq.sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_genome_is_valid() {
        let g = AgentGenome::default_genome();
        assert!(g.sense_radius > 0.0);
        assert!(g.max_idle > 0);
    }

    #[test]
    fn mutation_changes_genome() {
        let g = AgentGenome::default_genome();
        let mutated = g.mutate(0.2, 42);
        // At least one parameter should differ
        let same = (g.sense_radius - mutated.sense_radius).abs() < 1e-10
            && g.max_idle == mutated.max_idle
            && (g.keyword_boost - mutated.keyword_boost).abs() < 1e-10;
        assert!(!same, "Mutation should change at least one parameter");
    }

    #[test]
    fn mutation_stays_in_bounds() {
        let g = AgentGenome::default_genome();
        for seed in 0..100 {
            let m = g.mutate(0.5, seed);
            assert!(m.sense_radius >= 2.0 && m.sense_radius <= 30.0);
            assert!(m.max_idle >= 5 && m.max_idle <= 100);
            assert!(m.explore_bias >= 0.0 && m.explore_bias <= 1.0);
        }
    }

    #[test]
    fn distance_is_zero_for_same_genome() {
        let g = AgentGenome::default_genome();
        assert!(g.distance(&g) < 1e-10);
    }

    #[test]
    fn distance_increases_with_mutation() {
        let g = AgentGenome::default_genome();
        let m1 = g.mutate(0.1, 42);
        let m2 = g.mutate(0.5, 42);
        assert!(g.distance(&m2) >= g.distance(&m1) * 0.5,
            "Larger mutation should generally produce larger distance");
    }
}
