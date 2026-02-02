//! Curriculum ordering for training data.
//!
//! Orders triples in a pedagogically meaningful sequence:
//! 1. Foundation: high-weight, same-community triples (core concepts)
//! 2. Bridges: cross-community triples (connecting knowledge)
//! 3. Periphery: low-weight triples (specialized details)

use crate::community::CommunityResult;
use crate::export::WeightedTriple;
use serde::Serialize;

/// A curriculum-ordered sequence of triples.
#[derive(Debug, Clone, Serialize)]
pub struct Curriculum {
    pub foundation: Vec<WeightedTriple>,
    pub bridges: Vec<WeightedTriple>,
    pub periphery: Vec<WeightedTriple>,
}

impl Curriculum {
    /// Total number of triples.
    pub fn total(&self) -> usize {
        self.foundation.len() + self.bridges.len() + self.periphery.len()
    }

    /// Get all triples in curriculum order.
    pub fn ordered(&self) -> Vec<&WeightedTriple> {
        self.foundation.iter()
            .chain(self.bridges.iter())
            .chain(self.periphery.iter())
            .collect()
    }
}

/// Build a curriculum from triples and community assignments.
///
/// - Foundation: both nodes in the same community, weight > median
/// - Bridge: nodes in different communities
/// - Periphery: same community but weight ≤ median
pub fn build_curriculum(
    triples: &[WeightedTriple],
    communities: &CommunityResult,
) -> Curriculum {
    if triples.is_empty() {
        return Curriculum {
            foundation: Vec::new(),
            bridges: Vec::new(),
            periphery: Vec::new(),
        };
    }

    // Compute median weight
    let mut weights: Vec<f64> = triples.iter().map(|t| t.weight).collect();
    weights.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = weights[weights.len() / 2];

    let mut foundation = Vec::new();
    let mut bridges = Vec::new();
    let mut periphery = Vec::new();

    for triple in triples {
        let subj_community = communities.assignments.get(&triple.subject);
        let obj_community = communities.assignments.get(&triple.object);

        match (subj_community, obj_community) {
            (Some(sc), Some(oc)) if sc != oc => {
                bridges.push(triple.clone());
            }
            _ => {
                if triple.weight > median {
                    foundation.push(triple.clone());
                } else {
                    periphery.push(triple.clone());
                }
            }
        }
    }

    // Sort each section by weight descending
    foundation.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
    bridges.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
    periphery.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));

    Curriculum {
        foundation,
        bridges,
        periphery,
    }
}
