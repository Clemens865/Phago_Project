//! SemanticDigester — embedding-backed digestion.
//!
//! Like the standard Digester, but uses vector embeddings for semantic
//! understanding instead of simple keyword extraction. This enables:
//!
//! - Semantic similarity between concepts (not just word matching)
//! - Better concept extraction from scientific/technical text
//! - Cross-document concept linking based on meaning
//!
//! Biological analog: a more evolved macrophage with pattern recognition
//! receptors that recognize semantic patterns, not just surface keywords.

use phago_core::agent::Agent;
use phago_core::primitives::{Apoptose, Digest, Sense};
use phago_core::primitives::symbiose::AgentProfile;
use phago_core::signal::compute_gradient;
use phago_core::substrate::Substrate;
use phago_core::types::*;
use phago_embeddings::{Embedder, Chunker, ChunkConfig, cosine_similarity};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A semantic concept extracted from text via embeddings.
#[derive(Debug, Clone)]
pub struct SemanticConcept {
    /// The concept label (representative text).
    pub label: String,
    /// Vector embedding for semantic comparison.
    pub embedding: Vec<f32>,
    /// Confidence score (0.0-1.0) based on extraction quality.
    pub confidence: f32,
    /// Source chunk index.
    pub source_chunk: usize,
}

/// Configuration for semantic digestion.
#[derive(Debug, Clone)]
pub struct SemanticConfig {
    /// Chunk configuration for splitting documents.
    pub chunk_config: ChunkConfig,
    /// Minimum similarity threshold for concept clustering.
    pub similarity_threshold: f32,
    /// Maximum concepts to extract per document.
    pub max_concepts: usize,
    /// Whether to extract keyphrases (multi-word concepts).
    pub extract_keyphrases: bool,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            chunk_config: ChunkConfig::medium(),
            similarity_threshold: 0.7,
            max_concepts: 20,
            extract_keyphrases: true,
        }
    }
}

/// Internal state for semantic digester lifecycle.
#[derive(Debug, Clone, PartialEq)]
enum SemanticState {
    /// Searching for work.
    Seeking,
    /// Found a document, requesting to engulf.
    FoundTarget(DocumentId),
    /// Chunking the document.
    Chunking,
    /// Embedding chunks.
    Embedding,
    /// Extracting concepts from embeddings.
    Extracting,
    /// Presenting extracted concepts.
    Presenting,
}

/// A semantic text-digesting agent.
///
/// Uses vector embeddings to understand document semantics and extract
/// meaningful concepts for the knowledge graph.
pub struct SemanticDigester {
    id: AgentId,
    position: Position,
    age_ticks: Tick,
    state: SemanticState,

    // Embedder (shared across digesters for efficiency)
    embedder: Arc<dyn Embedder>,
    chunker: Chunker,
    config: SemanticConfig,

    // Digestion state
    engulfed: Option<String>,
    current_document: Option<DocumentId>,
    chunks: Vec<String>,
    chunk_embeddings: Vec<Vec<f32>>,
    concepts: Vec<SemanticConcept>,
    fragments: Vec<String>,
    all_presentations: Vec<String>,

    // Health tracking
    idle_ticks: u64,
    useful_outputs: u64,

    // Transfer / Symbiose state
    known_concepts: HashMap<String, Vec<f32>>,
    integrated_from: HashSet<AgentId>,
    boundary_permeability: f64,
    symbionts: Vec<SymbiontInfo>,

    // Configuration
    max_idle_ticks: u64,
    sense_radius: f64,
}

impl SemanticDigester {
    /// Create a new semantic digester with an embedder.
    pub fn new(position: Position, embedder: Arc<dyn Embedder>) -> Self {
        Self {
            id: AgentId::new(),
            position,
            age_ticks: 0,
            state: SemanticState::Seeking,
            embedder,
            chunker: Chunker::default(),
            config: SemanticConfig::default(),
            engulfed: None,
            current_document: None,
            chunks: Vec::new(),
            chunk_embeddings: Vec::new(),
            concepts: Vec::new(),
            fragments: Vec::new(),
            all_presentations: Vec::new(),
            idle_ticks: 0,
            useful_outputs: 0,
            known_concepts: HashMap::new(),
            integrated_from: HashSet::new(),
            boundary_permeability: 0.0,
            symbionts: Vec::new(),
            max_idle_ticks: 30,
            sense_radius: 10.0,
        }
    }

    /// Create with custom config.
    pub fn with_config(mut self, config: SemanticConfig) -> Self {
        self.chunker = Chunker::new(config.chunk_config.clone());
        self.config = config;
        self
    }

    /// Create with deterministic ID for testing.
    pub fn with_seed(position: Position, embedder: Arc<dyn Embedder>, seed: u64) -> Self {
        Self {
            id: AgentId::from_seed(seed),
            position,
            age_ticks: 0,
            state: SemanticState::Seeking,
            embedder,
            chunker: Chunker::default(),
            config: SemanticConfig::default(),
            engulfed: None,
            current_document: None,
            chunks: Vec::new(),
            chunk_embeddings: Vec::new(),
            concepts: Vec::new(),
            fragments: Vec::new(),
            all_presentations: Vec::new(),
            idle_ticks: 0,
            useful_outputs: 0,
            known_concepts: HashMap::new(),
            integrated_from: HashSet::new(),
            boundary_permeability: 0.0,
            symbionts: Vec::new(),
            max_idle_ticks: 30,
            sense_radius: 10.0,
        }
    }

    /// Set max idle threshold.
    pub fn with_max_idle(mut self, max_idle: u64) -> Self {
        self.max_idle_ticks = max_idle;
        self
    }

    /// Total concepts extracted in lifetime.
    pub fn total_concepts(&self) -> usize {
        self.all_presentations.len()
    }

    /// Get idle tick count.
    pub fn idle_ticks(&self) -> u64 {
        self.idle_ticks
    }

    /// Set idle ticks (for testing).
    pub fn set_idle_ticks(&mut self, ticks: u64) {
        self.idle_ticks = ticks;
    }

    /// Feed document content (called by colony after EngulfDocument).
    pub fn feed_document(&mut self, doc_id: DocumentId, content: String) {
        self.current_document = Some(doc_id);
        self.engulf(content);
    }

    /// Direct digestion for testing.
    pub fn digest_text(&mut self, text: String) -> Vec<String> {
        self.engulf(text);
        self.process_digestion();
        self.present()
    }

    /// Process the digestion pipeline (chunking → embedding → extraction).
    fn process_digestion(&mut self) {
        let Some(text) = self.engulfed.take() else {
            return;
        };

        // Step 1: Chunk the document
        let chunk_data = self.chunker.chunk(&text);
        self.chunks = chunk_data.iter().map(|c| c.text.clone()).collect();

        // Step 2: Embed all chunks
        self.chunk_embeddings = self.chunks
            .iter()
            .filter_map(|chunk| self.embedder.embed(chunk).ok())
            .collect();

        // Step 3: Extract concepts via semantic clustering
        self.extract_concepts();

        // Step 4: Generate labels for presentation
        self.fragments = self.concepts
            .iter()
            .map(|c| c.label.clone())
            .collect();

        if !self.fragments.is_empty() {
            self.useful_outputs += 1;
            self.idle_ticks = 0;
            self.all_presentations.extend(self.fragments.clone());
        }
    }

    /// Extract concepts by finding semantically similar chunks and extracting key terms.
    fn extract_concepts(&mut self) {
        self.concepts.clear();

        if self.chunk_embeddings.is_empty() {
            return;
        }

        // Find representative concepts from each chunk
        for (i, (chunk, embedding)) in self.chunks.iter().zip(self.chunk_embeddings.iter()).enumerate() {
            // Extract key terms from this chunk
            let terms = extract_key_terms(chunk);

            for term in terms {
                // Check if we already have a similar concept
                let is_duplicate = self.concepts.iter().any(|c| {
                    if let Ok(term_emb) = self.embedder.embed(&term) {
                        cosine_similarity(&term_emb, &c.embedding) > self.config.similarity_threshold
                    } else {
                        c.label.to_lowercase() == term.to_lowercase()
                    }
                });

                if !is_duplicate {
                    if let Ok(term_embedding) = self.embedder.embed(&term) {
                        // Calculate confidence based on embedding quality
                        let confidence = self.calculate_confidence(&term_embedding, embedding);

                        self.concepts.push(SemanticConcept {
                            label: term,
                            embedding: term_embedding,
                            confidence,
                            source_chunk: i,
                        });

                        if self.concepts.len() >= self.config.max_concepts {
                            break;
                        }
                    }
                }
            }

            if self.concepts.len() >= self.config.max_concepts {
                break;
            }
        }

        // Sort by confidence and keep top concepts
        self.concepts.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        self.concepts.truncate(self.config.max_concepts);
    }

    /// Calculate concept confidence based on semantic similarity to chunk.
    fn calculate_confidence(&self, term_embedding: &[f32], chunk_embedding: &[f32]) -> f32 {
        let similarity = cosine_similarity(term_embedding, chunk_embedding);

        // Check if this matches any known concepts (Transfer boost)
        let known_boost = self.known_concepts.values()
            .filter_map(|known_emb| {
                let sim = cosine_similarity(term_embedding, known_emb);
                if sim > 0.8 { Some(sim * 0.2) } else { None }
            })
            .sum::<f32>();

        (similarity + known_boost).min(1.0)
    }

    /// Find semantically similar concepts in this digester's knowledge.
    pub fn find_similar(&self, query: &str, top_k: usize) -> Vec<(String, f32)> {
        let query_embedding = match self.embedder.embed(query) {
            Ok(emb) => emb,
            Err(_) => return Vec::new(),
        };

        let mut results: Vec<(String, f32)> = self.all_presentations
            .iter()
            .filter_map(|concept| {
                let concept_emb = self.embedder.embed(concept).ok()?;
                let sim = cosine_similarity(&query_embedding, &concept_emb);
                Some((concept.clone(), sim))
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    /// Get embedding dimension from the underlying embedder.
    pub fn embedding_dimension(&self) -> usize {
        self.embedder.dimension()
    }
}

/// Extract key terms from a text chunk.
fn extract_key_terms(text: &str) -> Vec<String> {
    let stopwords: HashSet<&str> = [
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

    let mut freq: HashMap<String, usize> = HashMap::new();
    for word in text.split(|c: char| !c.is_alphanumeric()) {
        let word = word.to_lowercase();
        if word.len() >= 3 && !stopwords.contains(word.as_str()) {
            *freq.entry(word).or_insert(0) += 1;
        }
    }

    let mut words: Vec<(String, usize)> = freq.into_iter().collect();
    words.sort_by(|a, b| b.1.cmp(&a.1));
    words.into_iter().take(10).map(|(w, _)| w).collect()
}

// --- Trait Implementations ---

impl Digest for SemanticDigester {
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
        self.process_digestion();
        self.fragments.clone()
    }

    fn present(&self) -> Vec<String> {
        self.fragments.clone()
    }
}

impl Apoptose for SemanticDigester {
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

impl Sense for SemanticDigester {
    fn sense_radius(&self) -> f64 {
        self.sense_radius
    }

    fn sense_position(&self) -> Position {
        self.position
    }

    fn gradient(&self, substrate: &dyn Substrate) -> Vec<Gradient> {
        let signals = substrate.signals_near(&self.position, self.sense_radius);

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

impl Agent for SemanticDigester {
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
        "semantic_digester"
    }

    fn tick(&mut self, substrate: &dyn Substrate) -> AgentAction {
        self.age_ticks += 1;

        if self.should_die() {
            return AgentAction::Apoptose;
        }

        match self.state.clone() {
            SemanticState::Seeking => {
                let docs = substrate.undigested_documents();
                let nearby_doc = docs.iter().find(|d| {
                    d.position.distance_to(&self.position) <= self.sense_radius
                });

                if let Some(doc) = nearby_doc {
                    let doc_id = doc.id;
                    let doc_pos = doc.position;

                    if doc_pos.distance_to(&self.position) < 1.0 {
                        self.state = SemanticState::FoundTarget(doc_id);
                        return AgentAction::EngulfDocument(doc_id);
                    } else {
                        self.idle_ticks += 1;
                        return AgentAction::Move(doc_pos);
                    }
                }

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

            SemanticState::FoundTarget(_doc_id) => {
                if self.engulfed.is_some() {
                    self.state = SemanticState::Chunking;
                    AgentAction::Idle
                } else {
                    self.state = SemanticState::Seeking;
                    self.idle_ticks += 1;
                    AgentAction::Idle
                }
            }

            SemanticState::Chunking => {
                // In a more complex implementation, this could be async
                self.state = SemanticState::Embedding;
                AgentAction::Idle
            }

            SemanticState::Embedding => {
                self.state = SemanticState::Extracting;
                AgentAction::Idle
            }

            SemanticState::Extracting => {
                let fragments = self.lyse();
                if fragments.is_empty() {
                    self.state = SemanticState::Seeking;
                    self.idle_ticks += 1;
                    AgentAction::Idle
                } else {
                    self.state = SemanticState::Presenting;
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

            SemanticState::Presenting => {
                self.state = SemanticState::Seeking;
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

    fn profile(&self) -> AgentProfile {
        AgentProfile {
            id: self.id,
            agent_type: "semantic_digester".to_string(),
            capabilities: Vec::new(),
            health: self.self_assess(),
        }
    }

    fn permeability(&self) -> f64 {
        self.boundary_permeability
    }

    fn modulate_boundary(&mut self, context: &BoundaryContext) {
        let reinforcement_factor = (context.reinforcement_count as f64 / 10.0).min(1.0);
        let age_factor = (context.age as f64 / 100.0).min(1.0);
        let trust_factor = context.trust;

        self.boundary_permeability =
            (0.3 * reinforcement_factor + 0.3 * age_factor + 0.4 * trust_factor).clamp(0.0, 1.0);
    }

    fn externalize_vocabulary(&self) -> Vec<String> {
        self.all_presentations.clone()
    }

    fn internalize_vocabulary(&mut self, terms: &[String]) {
        for term in terms {
            if let Ok(embedding) = self.embedder.embed(term) {
                self.known_concepts.insert(term.clone(), embedding);
            }
        }
    }

    fn vocabulary_size(&self) -> usize {
        self.known_concepts.len() + self.all_presentations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phago_embeddings::SimpleEmbedder;

    fn test_embedder() -> Arc<dyn Embedder> {
        Arc::new(SimpleEmbedder::new(128))
    }

    #[test]
    fn digest_text_extracts_concepts() {
        let embedder = test_embedder();
        let mut digester = SemanticDigester::new(Position::new(0.0, 0.0), embedder);

        let text = "The mitochondria is the powerhouse of the cell. \
                   ATP is produced through oxidative phosphorylation \
                   in the inner membrane.".to_string();

        let fragments = digester.digest_text(text);

        assert!(!fragments.is_empty());
        assert!(fragments.iter().any(|f| f.contains("mitochondria") || f.contains("cell") || f.contains("membrane")));
    }

    #[test]
    fn semantic_similarity_search() {
        let embedder = test_embedder();
        let mut digester = SemanticDigester::new(Position::new(0.0, 0.0), embedder);

        digester.digest_text("cell membrane protein transport biology".to_string());

        let similar = digester.find_similar("cellular biology", 3);
        assert!(!similar.is_empty());
    }

    #[test]
    fn engulf_rejects_empty_input() {
        let embedder = test_embedder();
        let mut digester = SemanticDigester::new(Position::new(0.0, 0.0), embedder);

        assert_eq!(digester.engulf("".to_string()), DigestionResult::Indigestible);
        assert_eq!(digester.engulf("   ".to_string()), DigestionResult::Indigestible);
    }

    #[test]
    fn healthy_when_producing_output() {
        let embedder = test_embedder();
        let mut digester = SemanticDigester::new(Position::new(0.0, 0.0), embedder);

        digester.digest_text("cell membrane protein structure biology".to_string());

        assert_eq!(digester.self_assess(), CellHealth::Healthy);
        assert!(!digester.should_die());
    }

    #[test]
    fn senescent_after_idle_threshold() {
        let embedder = test_embedder();
        let mut digester = SemanticDigester::new(Position::new(0.0, 0.0), embedder)
            .with_max_idle(10);

        digester.set_idle_ticks(10);

        assert_eq!(digester.self_assess(), CellHealth::Senescent);
        assert!(digester.should_die());
    }

    #[test]
    fn embedding_dimension_matches() {
        let embedder = test_embedder();
        let digester = SemanticDigester::new(Position::new(0.0, 0.0), embedder);

        assert_eq!(digester.embedding_dimension(), 128);
    }
}
