//! Spawn policies for evolutionary agent creation.
//!
//! When an agent dies, the colony can spawn a replacement using a policy.
//! The FitnessSpawnPolicy creates a mutated offspring of the fittest
//! living agent, implementing biological selection.

use crate::genome::AgentGenome;
use phago_core::types::{AgentId, Position};

/// Trait for spawn policies.
pub trait SpawnPolicy {
    /// Decide whether to spawn a new agent after a death.
    ///
    /// Returns the genome and position for the new agent, or None if
    /// no spawn should occur (e.g., population cap reached).
    fn on_death(
        &mut self,
        dead_id: AgentId,
        alive_count: usize,
        fittest_genome: Option<&AgentGenome>,
        fittest_position: Option<Position>,
    ) -> Option<(AgentGenome, Position)>;
}

/// Fitness-based spawn: create mutated offspring of the fittest agent.
pub struct FitnessSpawnPolicy {
    /// Maximum population size.
    pub max_population: usize,
    /// Mutation rate for offspring genomes.
    pub mutation_rate: f64,
    /// Counter for seeding mutations.
    spawn_counter: u64,
}

impl FitnessSpawnPolicy {
    pub fn new(max_population: usize, mutation_rate: f64) -> Self {
        Self {
            max_population,
            mutation_rate,
            spawn_counter: 0,
        }
    }
}

impl SpawnPolicy for FitnessSpawnPolicy {
    fn on_death(
        &mut self,
        _dead_id: AgentId,
        alive_count: usize,
        fittest_genome: Option<&AgentGenome>,
        fittest_position: Option<Position>,
    ) -> Option<(AgentGenome, Position)> {
        // Don't spawn if at or above population cap
        if alive_count >= self.max_population {
            return None;
        }

        // Need a fittest agent to inherit from
        let parent_genome = fittest_genome?;
        let parent_pos = fittest_position.unwrap_or(Position::new(0.0, 0.0));

        self.spawn_counter += 1;
        let offspring_genome = parent_genome.mutate(self.mutation_rate, self.spawn_counter);

        // Spawn near parent with slight offset
        let offset_x = ((self.spawn_counter as f64 * 2.7).sin()) * 3.0;
        let offset_y = ((self.spawn_counter as f64 * 1.3).cos()) * 3.0;
        let position = Position::new(parent_pos.x + offset_x, parent_pos.y + offset_y);

        Some((offspring_genome, position))
    }
}

/// No-spawn policy: never create new agents (static population).
pub struct NoSpawnPolicy;

impl SpawnPolicy for NoSpawnPolicy {
    fn on_death(
        &mut self,
        _dead_id: AgentId,
        _alive_count: usize,
        _fittest_genome: Option<&AgentGenome>,
        _fittest_position: Option<Position>,
    ) -> Option<(AgentGenome, Position)> {
        None
    }
}

/// Random spawn policy: create agents with random genomes (control group).
pub struct RandomSpawnPolicy {
    pub max_population: usize,
    spawn_counter: u64,
}

impl RandomSpawnPolicy {
    pub fn new(max_population: usize) -> Self {
        Self {
            max_population,
            spawn_counter: 0,
        }
    }
}

impl SpawnPolicy for RandomSpawnPolicy {
    fn on_death(
        &mut self,
        _dead_id: AgentId,
        alive_count: usize,
        _fittest_genome: Option<&AgentGenome>,
        _fittest_position: Option<Position>,
    ) -> Option<(AgentGenome, Position)> {
        if alive_count >= self.max_population {
            return None;
        }

        self.spawn_counter += 1;
        // Large mutation from default = effectively random
        let genome = AgentGenome::default_genome().mutate(0.8, self.spawn_counter);
        let x = ((self.spawn_counter as f64 * 3.7).sin()) * 10.0;
        let y = ((self.spawn_counter as f64 * 2.1).cos()) * 10.0;

        Some((genome, Position::new(x, y)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitness_spawn_creates_offspring() {
        let mut policy = FitnessSpawnPolicy::new(10, 0.1);
        let genome = AgentGenome::default_genome();
        let pos = Position::new(5.0, 5.0);

        let result = policy.on_death(AgentId::new(), 3, Some(&genome), Some(pos));
        assert!(result.is_some());
    }

    #[test]
    fn fitness_spawn_respects_cap() {
        let mut policy = FitnessSpawnPolicy::new(5, 0.1);
        let genome = AgentGenome::default_genome();

        let result = policy.on_death(AgentId::new(), 5, Some(&genome), Some(Position::new(0.0, 0.0)));
        assert!(result.is_none());
    }

    #[test]
    fn no_spawn_never_spawns() {
        let mut policy = NoSpawnPolicy;
        let genome = AgentGenome::default_genome();
        let result = policy.on_death(AgentId::new(), 1, Some(&genome), Some(Position::new(0.0, 0.0)));
        assert!(result.is_none());
    }
}
