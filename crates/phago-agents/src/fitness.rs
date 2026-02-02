//! Fitness tracking for evolutionary agent selection.
//!
//! Tracks per-agent graph contributions to compute fitness scores.
//! Fitness determines which genomes propagate: fitter agents live longer
//! (higher max_idle effectively) and their genomes seed new agents.

use phago_core::types::AgentId;
use serde::Serialize;
use std::collections::HashMap;

/// Per-agent fitness data.
#[derive(Debug, Clone, Serialize)]
pub struct AgentFitness {
    pub agent_id: AgentId,
    /// Total concepts added to the knowledge graph.
    pub concepts_added: u64,
    /// Total edges created or strengthened.
    pub edges_contributed: u64,
    /// Total ticks alive.
    pub ticks_alive: u64,
    /// Fitness score = (concepts_added + edges_contributed) / ticks_alive.
    pub fitness: f64,
    /// Generation number (0 = original, 1 = first offspring, etc.)
    pub generation: u32,
}

/// Tracks fitness across all agents in a colony.
pub struct FitnessTracker {
    data: HashMap<AgentId, AgentFitness>,
    generation_counter: u32,
}

impl FitnessTracker {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            generation_counter: 0,
        }
    }

    /// Register a new agent with its generation.
    pub fn register(&mut self, agent_id: AgentId, generation: u32) {
        self.data.insert(agent_id, AgentFitness {
            agent_id,
            concepts_added: 0,
            edges_contributed: 0,
            ticks_alive: 0,
            fitness: 0.0,
            generation,
        });
    }

    /// Record that an agent added concepts to the graph.
    pub fn record_concepts(&mut self, agent_id: &AgentId, count: u64) {
        if let Some(f) = self.data.get_mut(agent_id) {
            f.concepts_added += count;
            Self::recompute_fitness(f);
        }
    }

    /// Record that an agent contributed edges.
    pub fn record_edges(&mut self, agent_id: &AgentId, count: u64) {
        if let Some(f) = self.data.get_mut(agent_id) {
            f.edges_contributed += count;
            Self::recompute_fitness(f);
        }
    }

    /// Record a tick for all registered agents.
    pub fn tick_all(&mut self, alive_ids: &[AgentId]) {
        for id in alive_ids {
            if let Some(f) = self.data.get_mut(id) {
                f.ticks_alive += 1;
                Self::recompute_fitness(f);
            }
        }
    }

    fn recompute_fitness(f: &mut AgentFitness) {
        if f.ticks_alive > 0 {
            f.fitness = (f.concepts_added as f64 + f.edges_contributed as f64) / f.ticks_alive as f64;
        }
    }

    /// Get the fittest living agent.
    pub fn fittest(&self, alive_ids: &[AgentId]) -> Option<&AgentFitness> {
        alive_ids.iter()
            .filter_map(|id| self.data.get(id))
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Get fitness data for an agent.
    pub fn get(&self, agent_id: &AgentId) -> Option<&AgentFitness> {
        self.data.get(agent_id)
    }

    /// Get all fitness data.
    pub fn all(&self) -> Vec<&AgentFitness> {
        self.data.values().collect()
    }

    /// Mean fitness of living agents.
    pub fn mean_fitness(&self, alive_ids: &[AgentId]) -> f64 {
        let fitnesses: Vec<f64> = alive_ids.iter()
            .filter_map(|id| self.data.get(id))
            .map(|f| f.fitness)
            .collect();
        if fitnesses.is_empty() {
            0.0
        } else {
            fitnesses.iter().sum::<f64>() / fitnesses.len() as f64
        }
    }

    /// Next generation number.
    pub fn next_generation(&mut self) -> u32 {
        self.generation_counter += 1;
        self.generation_counter
    }

    /// Current max generation.
    pub fn max_generation(&self) -> u32 {
        self.data.values().map(|f| f.generation).max().unwrap_or(0)
    }
}

impl Default for FitnessTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitness_tracks_contributions() {
        let mut tracker = FitnessTracker::new();
        let id = AgentId::new();
        tracker.register(id, 0);
        tracker.record_concepts(&id, 5);
        tracker.tick_all(&[id]);

        let f = tracker.get(&id).unwrap();
        assert_eq!(f.concepts_added, 5);
        assert_eq!(f.ticks_alive, 1);
        assert!(f.fitness > 0.0);
    }

    #[test]
    fn fittest_returns_best_agent() {
        let mut tracker = FitnessTracker::new();
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        tracker.register(id1, 0);
        tracker.register(id2, 0);
        tracker.record_concepts(&id1, 10);
        tracker.record_concepts(&id2, 2);
        tracker.tick_all(&[id1, id2]);

        let best = tracker.fittest(&[id1, id2]).unwrap();
        assert_eq!(best.agent_id, id1);
    }
}
