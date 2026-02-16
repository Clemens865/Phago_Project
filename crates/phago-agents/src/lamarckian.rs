//! Lamarckian LLM-Guided Evolution.
//!
//! When an agent dies, optionally feed its death signal + genome to an LLM
//! to suggest targeted genome patches. This runs alongside Darwinian
//! mutation (secondary pathway, not replacement).
//!
//! # Design
//!
//! The `GenomeAdvisor` trait abstracts the LLM call so it can be:
//! - A real LLM backend (wrapped to be sync)
//! - A mock for testing
//! - Disabled (returns empty patches → pure Darwinian fallback)
//!
//! # Example
//!
//! ```rust,ignore
//! use phago_agents::lamarckian::*;
//! use phago_agents::genome::AgentGenome;
//!
//! let advisor = MockAdvisor::new()
//!     .with_suggestion("sense_radius", 15.0);
//!
//! let death = DeathContext {
//!     cause: "SelfAssessed(LowEnergy)".into(),
//!     ticks_alive: 10,
//!     useful_outputs: 2,
//!     fitness: 0.1,
//! };
//!
//! let genome = AgentGenome::default_genome();
//! let patches = advisor.suggest_patches(&death, &genome);
//! let evolved = apply_patches(&genome, &patches);
//! ```

use crate::genome::AgentGenome;
use serde::{Deserialize, Serialize};

/// Context about an agent's death, used to inform LLM suggestions.
#[derive(Debug, Clone, Serialize)]
pub struct DeathContext {
    /// String description of the death cause.
    pub cause: String,
    /// How many ticks the agent lived.
    pub ticks_alive: u64,
    /// How many useful outputs (fragments, edges) it produced.
    pub useful_outputs: u64,
    /// The agent's fitness score at death.
    pub fitness: f64,
}

/// A targeted patch to a single genome parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomePatch {
    /// Which parameter to change.
    pub parameter: String,
    /// New value for the parameter.
    pub value: f64,
    /// Why the LLM suggested this change.
    pub reason: String,
}

/// Trait for genome advisors that suggest patches.
///
/// This abstracts the LLM call so the evolution logic doesn't
/// depend on async runtimes or specific LLM backends.
pub trait GenomeAdvisor {
    /// Suggest genome patches based on how an agent died.
    ///
    /// Returns an empty vec to fall back to pure Darwinian mutation.
    fn suggest_patches(&self, death: &DeathContext, genome: &AgentGenome) -> Vec<GenomePatch>;
}

/// Apply genome patches with clamping to valid parameter ranges.
///
/// Unknown parameter names are silently ignored.
pub fn apply_patches(genome: &AgentGenome, patches: &[GenomePatch]) -> AgentGenome {
    let mut result = genome.clone();

    for patch in patches {
        match patch.parameter.as_str() {
            "sense_radius" => {
                result.sense_radius = patch.value.clamp(2.0, 30.0);
            }
            "max_idle" => {
                result.max_idle = (patch.value.round() as u64).clamp(5, 100);
            }
            "keyword_boost" => {
                result.keyword_boost = patch.value.clamp(0.5, 10.0);
            }
            "explore_bias" => {
                result.explore_bias = patch.value.clamp(0.0, 1.0);
            }
            "boundary_bias" => {
                result.boundary_bias = patch.value.clamp(-1.0, 1.0);
            }
            "tentative_weight" => {
                result.tentative_weight = patch.value.clamp(0.05, 0.5);
            }
            "reinforcement_boost" => {
                result.reinforcement_boost = patch.value.clamp(0.01, 0.3);
            }
            "wiring_selectivity" => {
                result.wiring_selectivity = patch.value.clamp(0.1, 1.0);
            }
            _ => {} // Unknown parameter — ignore
        }
    }

    result
}

/// Build the prompt to send to an LLM for genome advice.
pub fn build_advice_prompt(death: &DeathContext, genome: &AgentGenome) -> String {
    format!(
        r#"An agent in a biological computing system has died. Analyze its death and suggest genome parameter changes to improve the next generation.

## Death Context
- Cause: {}
- Ticks alive: {}
- Useful outputs: {}
- Fitness score: {:.4}

## Current Genome
- sense_radius: {:.2} (range: 2.0-30.0) — how far the agent can detect signals
- max_idle: {} (range: 5-100) — maximum idle ticks before apoptosis
- keyword_boost: {:.2} (range: 0.5-10.0) — boost for known vocabulary during digestion
- explore_bias: {:.2} (range: 0.0-1.0) — random exploration vs gradient following
- boundary_bias: {:.2} (range: -1.0-1.0) — preference for boundary vs center
- tentative_weight: {:.3} (range: 0.05-0.5) — initial weight for new edges
- reinforcement_boost: {:.3} (range: 0.01-0.3) — weight boost per co-activation
- wiring_selectivity: {:.2} (range: 0.1-1.0) — fraction of concept pairs to wire

## Instructions
Suggest 1-3 parameter changes that would help the next generation agent survive longer and be more productive. Return a JSON array of patches:

```json
[
  {{"parameter": "param_name", "value": 12.5, "reason": "explanation"}}
]
```

Only include parameters you want to change. Values must be within the specified ranges."#,
        death.cause,
        death.ticks_alive,
        death.useful_outputs,
        death.fitness,
        genome.sense_radius,
        genome.max_idle,
        genome.keyword_boost,
        genome.explore_bias,
        genome.boundary_bias,
        genome.tentative_weight,
        genome.reinforcement_boost,
        genome.wiring_selectivity,
    )
}

/// Parse an LLM response into genome patches.
///
/// Tries to extract a JSON array from the response text.
/// Returns an empty vec if parsing fails (graceful fallback).
pub fn parse_llm_response(response: &str) -> Vec<GenomePatch> {
    // Try to find JSON array in the response
    let start = response.find('[');
    let end = response.rfind(']');

    match (start, end) {
        (Some(s), Some(e)) if e > s => {
            let json_str = &response[s..=e];
            serde_json::from_str(json_str).unwrap_or_default()
        }
        _ => vec![],
    }
}

/// A mock advisor for testing that returns preconfigured suggestions.
pub struct MockAdvisor {
    patches: Vec<GenomePatch>,
}

impl MockAdvisor {
    /// Create a new mock advisor with no suggestions.
    pub fn new() -> Self {
        Self { patches: vec![] }
    }

    /// Add a suggestion to the mock advisor.
    pub fn with_suggestion(mut self, parameter: &str, value: f64) -> Self {
        self.patches.push(GenomePatch {
            parameter: parameter.to_string(),
            value,
            reason: format!("Mock suggestion for {}", parameter),
        });
        self
    }
}

impl Default for MockAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

impl GenomeAdvisor for MockAdvisor {
    fn suggest_patches(&self, _death: &DeathContext, _genome: &AgentGenome) -> Vec<GenomePatch> {
        self.patches.clone()
    }
}

/// A no-op advisor that always returns empty patches (pure Darwinian fallback).
pub struct DarwinianFallback;

impl GenomeAdvisor for DarwinianFallback {
    fn suggest_patches(&self, _death: &DeathContext, _genome: &AgentGenome) -> Vec<GenomePatch> {
        vec![]
    }
}

/// Evolve a genome using Lamarckian advice + Darwinian mutation.
///
/// 1. Ask the advisor for targeted patches
/// 2. Apply patches (if any)
/// 3. Apply random Darwinian mutation on top
///
/// If the advisor returns no patches, this is equivalent to pure Darwinian mutation.
pub fn evolve_genome(
    genome: &AgentGenome,
    death: &DeathContext,
    advisor: &dyn GenomeAdvisor,
    mutation_rate: f64,
    seed: u64,
) -> AgentGenome {
    let patches = advisor.suggest_patches(death, genome);

    // Apply Lamarckian patches first
    let patched = if patches.is_empty() {
        genome.clone()
    } else {
        apply_patches(genome, &patches)
    };

    // Then apply Darwinian mutation on top (smaller rate if we already have patches)
    let effective_rate = if patches.is_empty() {
        mutation_rate
    } else {
        mutation_rate * 0.5 // Reduce random noise when we have targeted advice
    };

    patched.mutate(effective_rate, seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_death(cause: &str, fitness: f64) -> DeathContext {
        DeathContext {
            cause: cause.to_string(),
            ticks_alive: 20,
            useful_outputs: 5,
            fitness,
        }
    }

    #[test]
    fn mock_advisor_suggests_patches() {
        let advisor = MockAdvisor::new()
            .with_suggestion("sense_radius", 15.0)
            .with_suggestion("max_idle", 50.0);

        let genome = AgentGenome::default_genome();
        let death = make_death("SelfAssessed(LowEnergy)", 0.1);

        let patches = advisor.suggest_patches(&death, &genome);
        assert_eq!(patches.len(), 2);
        assert_eq!(patches[0].parameter, "sense_radius");
    }

    #[test]
    fn apply_patches_changes_genome() {
        let genome = AgentGenome::default_genome();
        let patches = vec![
            GenomePatch {
                parameter: "sense_radius".into(),
                value: 20.0,
                reason: "test".into(),
            },
            GenomePatch {
                parameter: "max_idle".into(),
                value: 60.0,
                reason: "test".into(),
            },
        ];

        let patched = apply_patches(&genome, &patches);
        assert!((patched.sense_radius - 20.0).abs() < 1e-6);
        assert_eq!(patched.max_idle, 60);
        // Unchanged parameters should remain the same
        assert!((patched.keyword_boost - genome.keyword_boost).abs() < 1e-6);
    }

    #[test]
    fn apply_patches_clamps_to_valid_ranges() {
        let genome = AgentGenome::default_genome();
        let patches = vec![
            GenomePatch {
                parameter: "sense_radius".into(),
                value: 999.0, // Way above max (30.0)
                reason: "test".into(),
            },
            GenomePatch {
                parameter: "explore_bias".into(),
                value: -5.0, // Below min (0.0)
                reason: "test".into(),
            },
        ];

        let patched = apply_patches(&genome, &patches);
        assert!(
            (patched.sense_radius - 30.0).abs() < 1e-6,
            "Should clamp to 30.0"
        );
        assert!(
            (patched.explore_bias - 0.0).abs() < 1e-6,
            "Should clamp to 0.0"
        );
    }

    #[test]
    fn unknown_parameters_are_ignored() {
        let genome = AgentGenome::default_genome();
        let patches = vec![GenomePatch {
            parameter: "nonexistent_param".into(),
            value: 42.0,
            reason: "test".into(),
        }];

        let patched = apply_patches(&genome, &patches);
        assert!((patched.sense_radius - genome.sense_radius).abs() < 1e-6);
    }

    #[test]
    fn parse_llm_response_extracts_json() {
        let response = r#"
Based on the death cause, I suggest:

```json
[
  {"parameter": "sense_radius", "value": 15.0, "reason": "Agent died too quickly, needs wider sensing"},
  {"parameter": "max_idle", "value": 45, "reason": "Allow more idle time before apoptosis"}
]
```

These changes should help.
"#;
        let patches = parse_llm_response(response);
        assert_eq!(patches.len(), 2);
        assert_eq!(patches[0].parameter, "sense_radius");
        assert!((patches[0].value - 15.0).abs() < 1e-6);
        assert_eq!(patches[1].parameter, "max_idle");
    }

    #[test]
    fn parse_llm_response_handles_invalid_json() {
        let patches = parse_llm_response("No JSON here, just text.");
        assert!(patches.is_empty(), "Should return empty on invalid JSON");
    }

    #[test]
    fn darwinian_fallback_returns_empty() {
        let genome = AgentGenome::default_genome();
        let death = make_death("RuntimeTermination", 0.5);
        let patches = DarwinianFallback.suggest_patches(&death, &genome);
        assert!(patches.is_empty());
    }

    #[test]
    fn evolve_with_lamarckian_advice() {
        let advisor = MockAdvisor::new().with_suggestion("sense_radius", 20.0);

        let genome = AgentGenome::default_genome();
        let death = make_death("SelfAssessed(LowEnergy)", 0.1);

        let evolved = evolve_genome(&genome, &death, &advisor, 0.1, 42);

        // sense_radius should be closer to 20.0 than the default (10.0),
        // even after Darwinian mutation on top
        let distance_to_target = (evolved.sense_radius - 20.0).abs();
        let distance_from_default = (evolved.sense_radius - genome.sense_radius).abs();
        assert!(
            distance_to_target < distance_from_default + 5.0,
            "Lamarckian advice should move sense_radius toward 20.0: got {}",
            evolved.sense_radius
        );
    }

    #[test]
    fn evolve_with_darwinian_fallback() {
        let genome = AgentGenome::default_genome();
        let death = make_death("RuntimeTermination", 0.5);

        let evolved = evolve_genome(&genome, &death, &DarwinianFallback, 0.2, 42);

        // Should still mutate (pure Darwinian)
        let same = (genome.sense_radius - evolved.sense_radius).abs() < 1e-10
            && genome.max_idle == evolved.max_idle;
        assert!(!same, "Darwinian mutation should change the genome");
    }

    #[test]
    fn build_prompt_includes_all_parameters() {
        let death = make_death("SelfAssessed(LowEnergy)", 0.15);
        let genome = AgentGenome::default_genome();

        let prompt = build_advice_prompt(&death, &genome);

        assert!(prompt.contains("SelfAssessed(LowEnergy)"));
        assert!(prompt.contains("sense_radius"));
        assert!(prompt.contains("max_idle"));
        assert!(prompt.contains("keyword_boost"));
        assert!(prompt.contains("explore_bias"));
        assert!(prompt.contains("wiring_selectivity"));
        assert!(prompt.contains("JSON"));
    }
}
