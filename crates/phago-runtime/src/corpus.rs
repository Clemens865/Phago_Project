//! Corpus loader — replaces hard-coded documents in POC.
//!
//! Provides a standard way to load text documents from a directory
//! or use a built-in embedded test corpus. Every branch prototype
//! uses this to ingest documents into the colony.

use crate::colony::Colony;
use phago_core::types::Position;
use std::path::Path;

/// A corpus of documents to be ingested into a colony.
pub struct Corpus {
    pub documents: Vec<CorpusDocument>,
    pub name: String,
}

/// A single document in a corpus.
#[derive(Debug, Clone)]
pub struct CorpusDocument {
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub position: Position,
}

impl Corpus {
    /// Load all .txt files from a directory.
    ///
    /// Files are assigned positions in a grid layout and categories
    /// are inferred from filename prefixes (e.g., `cell_biology_01.txt`
    /// gets category "cell_biology").
    pub fn from_directory(path: &Path) -> std::io::Result<Self> {
        let mut documents = Vec::new();
        let mut entries: Vec<_> = std::fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext == "txt")
            })
            .collect();

        entries.sort_by_key(|e| e.file_name());

        let cols = 5;
        let spacing = 5.0;

        for (i, entry) in entries.iter().enumerate() {
            let content = std::fs::read_to_string(entry.path())?;
            let filename = entry.file_name().to_string_lossy().to_string();
            let title = filename.trim_end_matches(".txt").to_string();

            // Infer category from filename prefix (everything before last _NN)
            let category = title
                .rfind('_')
                .and_then(|pos| {
                    let suffix = &title[pos + 1..];
                    if suffix.chars().all(|c| c.is_ascii_digit()) {
                        Some(title[..pos].to_string())
                    } else {
                        None
                    }
                });

            let row = i / cols;
            let col = i % cols;
            let position = Position::new(col as f64 * spacing, row as f64 * spacing);

            documents.push(CorpusDocument {
                title,
                content,
                category,
                position,
            });
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "corpus".to_string());

        Ok(Corpus { documents, name })
    }

    /// Built-in 20-document test corpus across 4 topics.
    ///
    /// Topics: cell_biology (5), molecular_transport (5), genetics (5), quantum_computing (5).
    /// Ground-truth clusters enable measuring community detection purity.
    pub fn from_embedded() -> Self {
        let topics: &[(&str, &[&str])] = &[
            ("cell_biology", &[
                "The cell membrane is a phospholipid bilayer that forms the outer boundary of every living cell. Integral membrane proteins span the bilayer and serve as channels receptors and enzymes. The fluid mosaic model describes the dynamic nature of the membrane where lipids and proteins move laterally within the layer.",
                "The cytoskeleton provides structural support and facilitates intracellular transport. Microtubules are hollow polymers of tubulin that serve as tracks for motor proteins like kinesin and dynein. Actin filaments form a dense network beneath the plasma membrane called the cell cortex.",
                "Organelles compartmentalize cellular functions within membrane-bound structures. The endoplasmic reticulum synthesizes proteins and lipids. The Golgi apparatus processes and packages proteins for secretion. Lysosomes contain digestive enzymes that break down cellular waste.",
                "Cell division occurs through mitosis and meiosis. During mitosis the cell duplicates its chromosomes and divides into two identical daughter cells. The mitotic spindle composed of microtubules attaches to kinetochores on chromosomes to ensure proper segregation.",
                "Apoptosis is programmed cell death essential for development and tissue homeostasis. Intrinsic apoptosis is triggered by mitochondrial outer membrane permeabilization releasing cytochrome c. Caspase enzymes execute the dismantling of cellular components.",
            ]),
            ("molecular_transport", &[
                "Active transport moves molecules against their concentration gradient using ATP hydrolysis. The sodium potassium pump exchanges three sodium ions outward for two potassium ions inward maintaining the electrochemical gradient.",
                "Passive transport occurs down the concentration gradient without energy expenditure. Simple diffusion allows small nonpolar molecules like oxygen and carbon dioxide to cross the lipid bilayer. Facilitated diffusion uses channel proteins and carrier proteins.",
                "Vesicular transport moves large molecules between compartments through membrane budding and fusion. Endocytosis internalizes extracellular material by membrane invagination forming vesicles. Exocytosis releases intracellular contents by vesicle fusion with the plasma membrane.",
                "Mitochondria produce ATP through oxidative phosphorylation in the electron transport chain. NADH and FADH2 donate electrons to protein complexes embedded in the inner mitochondrial membrane. The proton gradient drives ATP synthase.",
                "Signal transduction pathways relay extracellular signals to intracellular responses. G-protein coupled receptors activate second messenger cascades involving cyclic AMP and calcium ions. Receptor tyrosine kinases trigger phosphorylation cascades.",
            ]),
            ("genetics", &[
                "DNA replication is semiconservative with each strand serving as a template. DNA helicase unwinds the double helix at the replication fork. DNA polymerase synthesizes new strands in the five prime to three prime direction.",
                "Transcription converts DNA sequence into messenger RNA through RNA polymerase activity. Promoter regions upstream of genes recruit transcription factors. Introns are spliced out by the spliceosome complex leaving exons joined in mature mRNA.",
                "Translation occurs at ribosomes where messenger RNA codons are decoded into amino acid sequences. Transfer RNA molecules carry specific amino acids and recognize codons through anticodon base pairing. The ribosome catalyzes peptide bond formation.",
                "Gene regulation controls when and how much protein is produced from each gene. Transcription factors bind to enhancer and silencer regions to activate or repress gene expression. Epigenetic modifications alter chromatin accessibility.",
                "CRISPR-Cas9 enables precise genome editing by creating targeted double-strand breaks in DNA. Guide RNA directs the Cas9 nuclease to complementary sequences. Homology-directed repair allows insertion of new genetic material at the cut site.",
            ]),
            ("quantum_computing", &[
                "Quantum bits or qubits exploit superposition to exist in multiple states simultaneously. Unlike classical bits a qubit represents a linear combination of both states with complex probability amplitudes. Measurement collapses the superposition.",
                "Quantum entanglement creates correlations between qubits that have no classical analogue. Bell states are maximally entangled two-qubit states used in quantum teleportation and superdense coding. Entanglement is a resource consumed by quantum algorithms.",
                "Quantum gates manipulate qubits through unitary transformations. The Hadamard gate creates superposition from basis states. CNOT gate entangles two qubits and forms a universal gate set when combined with single qubit rotations.",
                "Shor's algorithm factors large integers in polynomial time using quantum Fourier transform. This threatens RSA encryption which relies on the computational difficulty of integer factorization. Grover's algorithm provides quadratic speedup for unstructured search.",
                "Quantum error correction protects quantum information from decoherence and gate errors. The surface code encodes logical qubits in two-dimensional arrays of physical qubits. Topological quantum computing uses anyonic braiding for fault-tolerant operations.",
            ]),
        ];

        let mut documents = Vec::new();
        let spacing = 5.0;

        for (topic_idx, (topic, docs)) in topics.iter().enumerate() {
            for (doc_idx, content) in docs.iter().enumerate() {
                let title = format!("{}_{:02}", topic, doc_idx + 1);
                let x = doc_idx as f64 * spacing;
                let y = topic_idx as f64 * spacing;

                documents.push(CorpusDocument {
                    title,
                    content: content.to_string(),
                    category: Some(topic.to_string()),
                    position: Position::new(x, y),
                });
            }
        }

        Corpus {
            documents,
            name: "embedded-20".to_string(),
        }
    }

    /// Number of documents in the corpus.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Whether the corpus is empty.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Get the ground-truth category labels (for NMI computation).
    /// Returns a map of document title -> category.
    pub fn ground_truth(&self) -> std::collections::HashMap<String, String> {
        self.documents
            .iter()
            .filter_map(|d| {
                d.category
                    .as_ref()
                    .map(|c| (d.title.clone(), c.clone()))
            })
            .collect()
    }

    /// Get unique categories in the corpus.
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self
            .documents
            .iter()
            .filter_map(|d| d.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        cats.sort();
        cats
    }

    /// Ingest all documents into a colony.
    pub fn ingest_into(&self, colony: &mut Colony) {
        for doc in &self.documents {
            colony.ingest_document(&doc.title, &doc.content, doc.position);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_corpus_has_20_documents() {
        let corpus = Corpus::from_embedded();
        assert_eq!(corpus.len(), 20);
    }

    #[test]
    fn embedded_corpus_has_4_categories() {
        let corpus = Corpus::from_embedded();
        let cats = corpus.categories();
        assert_eq!(cats.len(), 4);
        assert!(cats.contains(&"cell_biology".to_string()));
        assert!(cats.contains(&"quantum_computing".to_string()));
    }

    #[test]
    fn ground_truth_maps_all_documents() {
        let corpus = Corpus::from_embedded();
        let gt = corpus.ground_truth();
        assert_eq!(gt.len(), 20);
    }

    #[test]
    fn from_directory_loads_txt_files() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("poc/data/corpus");
        if path.exists() {
            let corpus = Corpus::from_directory(&path).unwrap();
            assert_eq!(corpus.len(), 20);
        }
    }
}
