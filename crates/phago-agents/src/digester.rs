//! Digester Agent — the first living cell.
//!
//! A Digester consumes text input, breaks it into keyword fragments,
//! and presents them for other agents to read. It senses signals in the
//! substrate to find unprocessed input, and self-terminates (apoptosis)
//! when it has spent too many cycles without producing useful output.
//!
//! Biological analog: a macrophage that patrols tissue, engulfs foreign
//! material, and presents antigen fragments on its surface.

use phago_core::agent::Agent;
use phago_core::primitives::{Apoptose, Digest, Sense};
use phago_core::primitives::symbiose::AgentProfile;
use phago_core::signal::compute_gradient;
use phago_core::substrate::Substrate;
use phago_core::types::*;
use std::collections::{HashMap, HashSet};

/// Internal state machine for the digester's lifecycle.
#[derive(Debug, Clone, PartialEq)]
enum DigesterState {
    /// Searching for work — sensing signals and navigating.
    Seeking,
    /// Found a document, requesting to engulf it next tick.
    FoundTarget(DocumentId),
    /// Currently digesting — will produce fragments next tick.
    Digesting,
    /// Has fragments ready to present to the knowledge graph.
    Presenting,
}

/// A text-digesting agent — the computational macrophage.
pub struct Digester {
    id: AgentId,
    position: Position,
    age_ticks: Tick,
    state: DigesterState,

    // Digestion state
    /// Raw text currently being digested (if any).
    engulfed: Option<String>,
    /// The document currently being digested.
    current_document: Option<DocumentId>,
    /// Fragments extracted from the last digestion.
    fragments: Vec<String>,
    /// Cumulative presentation: all fragments this agent has ever produced.
    all_presentations: Vec<String>,

    // Health tracking for apoptosis
    /// Number of consecutive ticks with no useful output.
    idle_ticks: u64,
    /// Total useful outputs produced in lifetime.
    useful_outputs: u64,

    // Transfer / Symbiose / Dissolve state
    /// Vocabulary learned from integrated capabilities and digestion.
    known_vocabulary: HashSet<String>,
    /// Whether this agent has exported its vocabulary at least once.
    has_exported: bool,
    /// Agent IDs from which we've already integrated vocabulary (avoid re-integration).
    integrated_from: HashSet<AgentId>,
    /// Boundary permeability (0.0 = rigid, 1.0 = fully dissolved).
    boundary_permeability: f64,
    /// Symbionts absorbed by this agent.
    symbionts: Vec<SymbiontInfo>,

    // Configuration
    /// Max consecutive idle ticks before triggering apoptosis.
    max_idle_ticks: u64,
    /// Sensing radius.
    sense_radius: f64,
}

impl Digester {
    pub fn new(position: Position) -> Self {
        Self {
            id: AgentId::new(),
            position,
            age_ticks: 0,
            state: DigesterState::Seeking,
            engulfed: None,
            current_document: None,
            fragments: Vec::new(),
            all_presentations: Vec::new(),
            idle_ticks: 0,
            useful_outputs: 0,
            known_vocabulary: HashSet::new(),
            has_exported: false,
            integrated_from: HashSet::new(),
            boundary_permeability: 0.0,
            symbionts: Vec::new(),
            max_idle_ticks: 30,
            sense_radius: 10.0,
        }
    }

    /// Create a digester with a deterministic ID (for testing).
    pub fn with_seed(position: Position, seed: u64) -> Self {
        Self {
            id: AgentId::from_seed(seed),
            position,
            age_ticks: 0,
            state: DigesterState::Seeking,
            engulfed: None,
            current_document: None,
            fragments: Vec::new(),
            all_presentations: Vec::new(),
            idle_ticks: 0,
            useful_outputs: 0,
            known_vocabulary: HashSet::new(),
            has_exported: false,
            integrated_from: HashSet::new(),
            boundary_permeability: 0.0,
            symbionts: Vec::new(),
            max_idle_ticks: 30,
            sense_radius: 10.0,
        }
    }

    /// Create a digester with custom idle threshold.
    pub fn with_max_idle(mut self, max_idle: u64) -> Self {
        self.max_idle_ticks = max_idle;
        self
    }

    /// Total fragments produced in lifetime.
    pub fn total_fragments(&self) -> usize {
        self.all_presentations.len()
    }

    /// Simulate an idle tick (for testing/demo purposes).
    pub fn increment_idle(&mut self) {
        self.idle_ticks += 1;
    }

    /// Current idle tick count (for testing/inspection).
    pub fn idle_ticks(&self) -> u64 {
        self.idle_ticks
    }

    /// Set idle ticks directly (for testing).
    pub fn set_idle_ticks(&mut self, ticks: u64) {
        self.idle_ticks = ticks;
    }

    /// Direct digestion: feed text and get fragments back immediately.
    /// This is a convenience for testing — in a colony, agents get input
    /// via SENSE + ENGULF from the substrate.
    pub fn digest_text(&mut self, text: String) -> Vec<String> {
        self.engulf(text);
        self.lyse()
    }

    /// Feed document content to this agent (called by colony after EngulfDocument).
    /// Sets internal state so the next tick processes the content.
    pub fn feed_document(&mut self, doc_id: DocumentId, content: String) {
        self.current_document = Some(doc_id);
        self.engulf(content);
    }
}

/// Extract keywords from text using a simple frequency-based approach.
///
/// This is deterministic — no LLMs in v0.1. We extract meaningful words
/// by filtering stopwords, short words, and ranking by frequency.
/// Words in `known_vocabulary` receive a +3 frequency boost (Transfer effect).
fn extract_keywords(text: &str, known_vocabulary: Option<&HashSet<String>>) -> Vec<String> {
    let stopwords: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "shall",
        "should", "may", "might", "must", "can", "could", "of", "in", "to",
        "for", "with", "on", "at", "from", "by", "about", "as", "into",
        "through", "during", "before", "after", "above", "below", "between",
        "out", "off", "over", "under", "again", "further", "then", "once",
        "here", "there", "when", "where", "why", "how", "all", "each",
        "every", "both", "few", "more", "most", "other", "some", "such",
        "no", "nor", "not", "only", "own", "same", "so", "than", "too",
        "very", "just", "because", "but", "and", "or", "if", "while",
        "that", "this", "these", "those", "it", "its", "they", "them",
        "their", "we", "our", "you", "your", "he", "she", "his", "her",
        "which", "what", "who", "whom",
    ]
    .into_iter()
    .collect();

    // Tokenize: lowercase, split on non-alphanumeric, filter short and stopwords
    let mut freq: HashMap<String, usize> = HashMap::new();
    for word in text.split(|c: char| !c.is_alphanumeric()) {
        let word = word.to_lowercase();
        if word.len() >= 3 && !stopwords.contains(word.as_str()) {
            *freq.entry(word).or_insert(0) += 1;
        }
    }

    // Boost words that are in the known vocabulary (Transfer effect)
    if let Some(vocab) = known_vocabulary {
        for (word, count) in freq.iter_mut() {
            if vocab.contains(word) {
                *count += 3;
            }
        }
    }

    // Sort by frequency (descending), take top keywords
    let mut words: Vec<(String, usize)> = freq.into_iter().collect();
    words.sort_by(|a, b| b.1.cmp(&a.1));

    words.into_iter().map(|(word, _)| word).collect()
}

// --- Trait Implementations ---

impl Digest for Digester {
    type Input = String;
    type Fragment = String;
    type Presentation = Vec<String>;

    fn engulf(&mut self, input: String) -> DigestionResult {
        if self.engulfed.is_some() {
            return DigestionResult::Busy;
        }
        if input.trim().is_empty() {
            return DigestionResult::Indigestible;
        }
        self.engulfed = Some(input);
        DigestionResult::Engulfed
    }

    fn lyse(&mut self) -> Vec<String> {
        let Some(text) = self.engulfed.take() else {
            return Vec::new();
        };

        let vocab = if self.known_vocabulary.is_empty() {
            None
        } else {
            Some(&self.known_vocabulary)
        };
        let keywords = extract_keywords(&text, vocab);
        self.fragments = keywords.clone();

        if !self.fragments.is_empty() {
            self.useful_outputs += 1;
            self.idle_ticks = 0;
            self.all_presentations.extend(self.fragments.clone());
        }

        keywords
    }

    fn present(&self) -> Vec<String> {
        self.fragments.clone()
    }
}

impl Apoptose for Digester {
    fn self_assess(&self) -> CellHealth {
        if self.idle_ticks >= self.max_idle_ticks {
            CellHealth::Senescent
        } else if self.idle_ticks >= self.max_idle_ticks / 2 {
            CellHealth::Stressed
        } else {
            CellHealth::Healthy
        }
    }

    fn prepare_death_signal(&self) -> DeathSignal {
        DeathSignal {
            agent_id: self.id,
            total_ticks: self.age_ticks,
            useful_outputs: self.useful_outputs,
            final_fragments: self
                .all_presentations
                .iter()
                .map(|s| s.as_bytes().to_vec())
                .collect(),
            cause: DeathCause::SelfAssessed(self.self_assess()),
        }
    }
}

impl Sense for Digester {
    fn sense_radius(&self) -> f64 {
        self.sense_radius
    }

    fn sense_position(&self) -> Position {
        self.position
    }

    fn gradient(&self, substrate: &dyn Substrate) -> Vec<Gradient> {
        let signals = substrate.signals_near(&self.position, self.sense_radius);

        // Group signals by type and compute gradient for each
        let mut by_type: HashMap<String, Vec<&Signal>> = HashMap::new();
        for signal in &signals {
            let key = format!("{:?}", signal.signal_type);
            by_type.entry(key).or_default().push(signal);
        }

        by_type
            .values()
            .filter_map(|sigs| compute_gradient(sigs, &self.position))
            .collect()
    }

    fn orient(&self, gradients: &[Gradient]) -> Orientation {
        // Move toward the strongest Input signal gradient
        let strongest = gradients
            .iter()
            .filter(|g| matches!(g.signal_type, SignalType::Input))
            .max_by(|a, b| a.magnitude.partial_cmp(&b.magnitude).unwrap_or(std::cmp::Ordering::Equal));

        match strongest {
            Some(g) => Orientation::Toward(Position::new(
                self.position.x + g.direction.x,
                self.position.y + g.direction.y,
            )),
            None => {
                if gradients.is_empty() {
                    Orientation::Explore
                } else {
                    Orientation::Stay
                }
            }
        }
    }
}

impl Agent for Digester {
    fn id(&self) -> AgentId {
        self.id
    }

    fn position(&self) -> Position {
        self.position
    }

    fn set_position(&mut self, position: Position) {
        self.position = position;
    }

    fn agent_type(&self) -> &str {
        "digester"
    }

    fn tick(&mut self, substrate: &dyn Substrate) -> AgentAction {
        self.age_ticks += 1;

        // Check apoptosis first — always
        if self.should_die() {
            return AgentAction::Apoptose;
        }

        match self.state.clone() {
            DigesterState::Seeking => {
                // Check for symbiosis opportunity: if we have 3+ useful outputs
                // and sense a non-self signal emitter nearby, attempt symbiosis
                if self.useful_outputs >= 3 && self.symbionts.is_empty() {
                    let nearby_signals = substrate.signals_near(&self.position, self.sense_radius);
                    // Look for Anomaly or Capability signals from other agents
                    let symbiosis_target = nearby_signals.iter().find(|s| {
                        matches!(s.signal_type, SignalType::Anomaly | SignalType::Insight)
                            && s.emitter != self.id
                    });
                    if let Some(signal) = symbiosis_target {
                        let target_id = signal.emitter;
                        return AgentAction::SymbioseWith(target_id);
                    }
                }

                // Look for nearby undigested documents
                let docs = substrate.undigested_documents();
                let nearby_doc = docs.iter().find(|d| {
                    d.position.distance_to(&self.position) <= self.sense_radius
                });

                if let Some(doc) = nearby_doc {
                    // Found a document — move toward it and request engulf
                    let doc_id = doc.id;
                    let doc_pos = doc.position;

                    if doc_pos.distance_to(&self.position) < 1.0 {
                        // Close enough — engulf next tick
                        self.state = DigesterState::FoundTarget(doc_id);
                        return AgentAction::EngulfDocument(doc_id);
                    } else {
                        // Move toward document
                        self.idle_ticks += 1;
                        return AgentAction::Move(doc_pos);
                    }
                }

                // No documents nearby — follow signal gradients
                let gradients = self.gradient(substrate);
                let orientation = self.orient(&gradients);

                self.idle_ticks += 1;
                match orientation {
                    Orientation::Toward(pos) => AgentAction::Move(pos),
                    Orientation::Stay => AgentAction::Idle,
                    Orientation::Explore => {
                        let angle = (self.age_ticks as f64) * 0.7
                            + (self.id.0.as_u128() % 100) as f64 * 0.1;
                        let dx = angle.cos() * 2.0;
                        let dy = angle.sin() * 2.0;
                        AgentAction::Move(Position::new(
                            self.position.x + dx,
                            self.position.y + dy,
                        ))
                    }
                }
            }

            DigesterState::FoundTarget(_doc_id) => {
                // Colony should have fed us the document content.
                // If we have engulfed content, start digesting.
                if self.engulfed.is_some() {
                    self.state = DigesterState::Digesting;
                    AgentAction::Idle // Digesting takes one tick
                } else {
                    // Colony didn't feed us (maybe doc was already taken)
                    self.state = DigesterState::Seeking;
                    self.idle_ticks += 1;
                    AgentAction::Idle
                }
            }

            DigesterState::Digesting => {
                // Break down the engulfed material
                let fragments = self.lyse();
                if fragments.is_empty() {
                    self.state = DigesterState::Seeking;
                    self.idle_ticks += 1;
                    AgentAction::Idle
                } else {
                    self.state = DigesterState::Presenting;
                    let doc_id = self.current_document.unwrap_or(DocumentId::new());
                    let presentations: Vec<FragmentPresentation> = fragments
                        .iter()
                        .map(|label| FragmentPresentation {
                            label: label.clone(),
                            source_document: doc_id,
                            position: self.position,
                            node_type: NodeType::Concept,
                        })
                        .collect();
                    AgentAction::PresentFragments(presentations)
                }
            }

            DigesterState::Presenting => {
                // After presenting, check if we should export vocabulary
                if self.useful_outputs >= 2 && !self.has_exported {
                    self.has_exported = true;
                    self.state = DigesterState::Seeking;
                    self.current_document = None;
                    return AgentAction::ExportCapability(CapabilityId(
                        format!("vocab-{}", self.id.0),
                    ));
                }

                // Deposit a trace at our location marking successful digestion
                self.state = DigesterState::Seeking;
                self.current_document = None;
                let trace = Trace {
                    agent_id: self.id,
                    trace_type: TraceType::Digestion,
                    intensity: 1.0,
                    tick: self.age_ticks,
                    payload: Vec::new(),
                };
                AgentAction::Deposit(
                    SubstrateLocation::Spatial(self.position),
                    trace,
                )
            }
        }
    }

    fn age(&self) -> Tick {
        self.age_ticks
    }

    // --- Transfer overrides ---

    fn export_vocabulary(&self) -> Option<Vec<u8>> {
        if self.all_presentations.is_empty() {
            return None;
        }
        let cap = VocabularyCapability {
            terms: self.all_presentations.clone(),
            origin: self.id,
            document_count: self.useful_outputs,
        };
        serde_json::to_vec(&cap).ok()
    }

    fn integrate_vocabulary(&mut self, data: &[u8]) -> bool {
        if let Ok(cap) = serde_json::from_slice::<VocabularyCapability>(data) {
            // Only integrate once per source agent
            if self.integrated_from.contains(&cap.origin) {
                return false;
            }
            self.integrated_from.insert(cap.origin);
            for term in cap.terms {
                self.known_vocabulary.insert(term);
            }
            true
        } else {
            false
        }
    }

    // --- Symbiose overrides ---

    fn profile(&self) -> AgentProfile {
        AgentProfile {
            id: self.id,
            agent_type: "digester".to_string(),
            capabilities: Vec::new(),
            health: self.self_assess(),
        }
    }

    fn evaluate_symbiosis(&self, other: &AgentProfile) -> Option<SymbiosisEval> {
        // Only consider symbiosis with non-digester agents that are healthy
        if other.agent_type == "digester" {
            return Some(SymbiosisEval::Coexist);
        }
        if other.health.should_die() {
            return Some(SymbiosisEval::Coexist);
        }
        // Integrate agents of different types — this models endosymbiosis
        Some(SymbiosisEval::Integrate)
    }

    fn absorb_symbiont(&mut self, profile: AgentProfile, data: Vec<u8>) -> bool {
        // Merge target's vocabulary into our known_vocabulary
        if let Ok(cap) = serde_json::from_slice::<VocabularyCapability>(&data) {
            for term in &cap.terms {
                self.known_vocabulary.insert(term.clone());
            }
        }
        self.symbionts.push(SymbiontInfo {
            id: profile.id,
            name: profile.agent_type.clone(),
            capabilities: profile.capabilities,
        });
        true
    }

    // --- Dissolve overrides ---

    fn permeability(&self) -> f64 {
        self.boundary_permeability
    }

    fn modulate_boundary(&mut self, context: &BoundaryContext) {
        // permeability = 0.3*reinforcement + 0.3*age + 0.4*trust, clamped [0,1]
        let reinforcement_factor = (context.reinforcement_count as f64 / 10.0).min(1.0);
        let age_factor = (context.age as f64 / 100.0).min(1.0);
        let trust_factor = context.trust;

        self.boundary_permeability =
            (0.3 * reinforcement_factor + 0.3 * age_factor + 0.4 * trust_factor).clamp(0.0, 1.0);
    }

    fn externalize_vocabulary(&self) -> Vec<String> {
        let mut terms: Vec<String> = self.all_presentations.clone();
        for term in &self.known_vocabulary {
            if !terms.contains(term) {
                terms.push(term.clone());
            }
        }
        terms
    }

    fn internalize_vocabulary(&mut self, terms: &[String]) {
        for term in terms {
            self.known_vocabulary.insert(term.clone());
        }
    }

    fn vocabulary_size(&self) -> usize {
        self.known_vocabulary.len() + self.all_presentations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_text_extracts_keywords() {
        let mut digester = Digester::new(Position::new(0.0, 0.0));
        let text = "The mitochondria is the powerhouse of the cell. ATP is produced through oxidative phosphorylation in the inner membrane.".to_string();

        let fragments = digester.digest_text(text);

        assert!(!fragments.is_empty());
        assert!(fragments.contains(&"mitochondria".to_string()));
        assert!(fragments.contains(&"cell".to_string()));
        assert!(fragments.contains(&"membrane".to_string()));
        // Stopwords should be excluded
        assert!(!fragments.contains(&"the".to_string()));
        assert!(!fragments.contains(&"is".to_string()));
    }

    #[test]
    fn engulf_rejects_empty_input() {
        let mut digester = Digester::new(Position::new(0.0, 0.0));
        assert_eq!(digester.engulf("".to_string()), DigestionResult::Indigestible);
        assert_eq!(digester.engulf("   ".to_string()), DigestionResult::Indigestible);
    }

    #[test]
    fn engulf_rejects_when_busy() {
        let mut digester = Digester::new(Position::new(0.0, 0.0));
        assert_eq!(digester.engulf("hello world foo".to_string()), DigestionResult::Engulfed);
        assert_eq!(digester.engulf("another input".to_string()), DigestionResult::Busy);
    }

    #[test]
    fn lyse_consumes_engulfed_material() {
        let mut digester = Digester::new(Position::new(0.0, 0.0));
        digester.engulf("cell membrane protein transport".to_string());
        let fragments = digester.lyse();
        assert!(!fragments.is_empty());

        // Second lyse returns nothing — material was consumed
        let fragments2 = digester.lyse();
        assert!(fragments2.is_empty());
    }

    #[test]
    fn present_returns_last_fragments() {
        let mut digester = Digester::new(Position::new(0.0, 0.0));
        digester.engulf("cell membrane protein".to_string());
        digester.lyse();
        let presented = digester.present();
        assert!(!presented.is_empty());
        assert!(presented.contains(&"cell".to_string()));
    }

    #[test]
    fn healthy_when_producing_output() {
        let mut digester = Digester::new(Position::new(0.0, 0.0));
        digester.digest_text("cell membrane protein structure biology".to_string());
        assert_eq!(digester.self_assess(), CellHealth::Healthy);
        assert!(!digester.should_die());
    }

    #[test]
    fn senescent_after_idle_threshold() {
        let mut digester = Digester::new(Position::new(0.0, 0.0)).with_max_idle(10);

        // Simulate idle ticks
        for _ in 0..10 {
            digester.increment_idle();
        }

        assert_eq!(digester.self_assess(), CellHealth::Senescent);
        assert!(digester.should_die());
    }

    #[test]
    fn stressed_at_half_idle_threshold() {
        let mut digester = Digester::new(Position::new(0.0, 0.0)).with_max_idle(10);

        for _ in 0..5 {
            digester.increment_idle();
        }

        assert_eq!(digester.self_assess(), CellHealth::Stressed);
        assert!(!digester.should_die()); // Stressed but not dead yet
    }

    #[test]
    fn apoptosis_produces_death_signal() {
        let mut digester = Digester::new(Position::new(0.0, 0.0));
        let id = digester.id();
        digester.digest_text("biology cell protein".to_string());

        // prepare_death_signal works with &self (trait object compatible)
        let signal = digester.prepare_death_signal();
        assert_eq!(signal.agent_id, id);
        assert_eq!(signal.useful_outputs, 1);
        assert!(!signal.final_fragments.is_empty());

        // trigger_apoptosis consumes self (works with concrete types)
        let signal2 = digester.trigger_apoptosis();
        assert_eq!(signal2.agent_id, id);
    }

    #[test]
    fn useful_output_resets_idle_counter() {
        let mut digester = Digester::new(Position::new(0.0, 0.0)).with_max_idle(10);

        // Build up idle ticks
        digester.set_idle_ticks(8);
        assert_eq!(digester.self_assess(), CellHealth::Stressed);

        // Produce useful output — should reset
        digester.digest_text("cell membrane biology protein structure".to_string());
        assert_eq!(digester.idle_ticks(), 0);
        assert_eq!(digester.self_assess(), CellHealth::Healthy);
    }

    #[test]
    fn extract_keywords_handles_varied_text() {
        let keywords = extract_keywords(
            "Rust programming language provides memory safety \
             without garbage collection. Rust achieves memory safety \
             through its ownership system.",
            None,
        );
        assert!(keywords.contains(&"rust".to_string()));
        assert!(keywords.contains(&"memory".to_string()));
        assert!(keywords.contains(&"safety".to_string()));
        // "rust" and "memory" should rank high (appear twice)
        assert!(keywords.iter().position(|w| w == "rust").unwrap() < 5);
    }

    #[test]
    fn digest_full_cycle() {
        let mut digester = Digester::new(Position::new(0.0, 0.0));

        // Full cycle: engulf → lyse → present
        let presentation = digester.digest("biology cell membrane protein structure".to_string());
        assert!(!presentation.is_empty());
        assert!(presentation.contains(&"cell".to_string()));

        // Agent tracked the output
        assert_eq!(digester.useful_outputs, 1);
        assert_eq!(digester.total_fragments(), presentation.len());
    }
}
