//! Vector store integration for Colony.
//!
//! Bridges the Colony's knowledge graph with vector similarity search,
//! enabling semantic queries alongside TF-IDF + graph topology scoring.
//!
//! # Architecture
//!
//! `VectorSubstrate` wraps an `Embedder` and maintains an in-memory
//! mapping of node labels to their embeddings. On query, it computes
//! cosine similarity against all indexed embeddings.
//!
//! Feature-gated behind `vectors` in phago-runtime's Cargo.toml.

use phago_core::types::NodeId;
use phago_embeddings::{Embedder, EmbeddingResult};
use std::collections::HashMap;

/// A node entry in the vector index.
struct VectorEntry {
    label: String,
    embedding: Vec<f32>,
}

/// Bridges Colony nodes with vector similarity search.
///
/// Maintains an in-process embedding index. For external vector DBs,
/// use `phago_vectors::VectorStore` directly.
pub struct VectorSubstrate {
    embedder: Box<dyn Embedder>,
    entries: HashMap<NodeId, VectorEntry>,
}

impl VectorSubstrate {
    /// Create a new VectorSubstrate with the given embedder.
    pub fn new(embedder: Box<dyn Embedder>) -> Self {
        Self {
            embedder,
            entries: HashMap::new(),
        }
    }

    /// Index a node's label for vector similarity search.
    ///
    /// If `existing_embedding` is provided (e.g., from `NodeData.embedding`),
    /// it is used directly. Otherwise the embedder generates one.
    pub fn index_node(
        &mut self,
        node_id: NodeId,
        label: &str,
        existing_embedding: Option<&[f32]>,
    ) -> EmbeddingResult<()> {
        let embedding = match existing_embedding {
            Some(emb) => emb.to_vec(),
            None => self.embedder.embed(label)?,
        };
        self.entries.insert(
            node_id,
            VectorEntry {
                label: label.to_string(),
                embedding,
            },
        );
        Ok(())
    }

    /// Remove a node from the vector index.
    pub fn remove_node(&mut self, node_id: &NodeId) {
        self.entries.remove(node_id);
    }

    /// Search for nodes similar to a query text.
    ///
    /// Returns `(label, score)` pairs sorted by descending similarity.
    pub fn search(&self, query: &str, k: usize) -> EmbeddingResult<Vec<(String, f32)>> {
        let query_vec = self.embedder.embed(query)?;

        let mut scores: Vec<(String, f32)> = self
            .entries
            .values()
            .map(|entry| {
                let sim = self
                    .embedder
                    .similarity(&query_vec, &entry.embedding)
                    .unwrap_or(0.0);
                (entry.label.clone(), sim)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        Ok(scores)
    }

    /// Get the embedder's dimension.
    pub fn dimension(&self) -> usize {
        self.embedder.dimension()
    }

    /// Number of indexed nodes.
    pub fn indexed_count(&self) -> usize {
        self.entries.len()
    }

    /// Check if a node is indexed.
    pub fn is_indexed(&self, node_id: &NodeId) -> bool {
        self.entries.contains_key(node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phago_embeddings::SimpleEmbedder;

    #[test]
    fn index_and_search() {
        let embedder = Box::new(SimpleEmbedder::new(64));
        let mut vs = VectorSubstrate::new(embedder);

        // Index several concepts
        let id1 = NodeId::from_seed(1);
        let id2 = NodeId::from_seed(2);
        let id3 = NodeId::from_seed(3);

        vs.index_node(id1, "cell membrane", None).unwrap();
        vs.index_node(id2, "protein transport", None).unwrap();
        vs.index_node(id3, "quantum physics", None).unwrap();

        assert_eq!(vs.indexed_count(), 3);

        let results = vs.search("cell biology", 5).unwrap();
        assert!(!results.is_empty());
        // "cell membrane" should be most similar to "cell biology"
        assert_eq!(results[0].0, "cell membrane");
    }

    #[test]
    fn remove_node() {
        let embedder = Box::new(SimpleEmbedder::new(32));
        let mut vs = VectorSubstrate::new(embedder);

        let id = NodeId::from_seed(1);
        vs.index_node(id, "test", None).unwrap();
        assert!(vs.is_indexed(&id));

        vs.remove_node(&id);
        assert!(!vs.is_indexed(&id));
        assert_eq!(vs.indexed_count(), 0);
    }

    #[test]
    fn existing_embedding() {
        let embedder = Box::new(SimpleEmbedder::new(4));
        let mut vs = VectorSubstrate::new(embedder);

        let id = NodeId::from_seed(1);
        let emb = vec![0.5, 0.5, 0.0, 0.0];
        vs.index_node(id, "custom", Some(&emb)).unwrap();
        assert_eq!(vs.indexed_count(), 1);
    }
}
