//! Text chunking for embedding.
//!
//! Large documents need to be split into smaller chunks for embedding.
//! This module provides configurable chunking strategies.

/// Chunking configuration.
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum chunk size in characters.
    pub max_size: usize,
    /// Overlap between chunks in characters.
    pub overlap: usize,
    /// Minimum chunk size (don't create tiny chunks).
    pub min_size: usize,
    /// Split on sentence boundaries when possible.
    pub respect_sentences: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_size: 512,
            overlap: 64,
            min_size: 50,
            respect_sentences: true,
        }
    }
}

impl ChunkConfig {
    /// Create config for short chunks (tweets, titles).
    pub fn short() -> Self {
        Self {
            max_size: 128,
            overlap: 16,
            min_size: 10,
            respect_sentences: false,
        }
    }

    /// Create config for medium chunks (paragraphs).
    pub fn medium() -> Self {
        Self {
            max_size: 512,
            overlap: 64,
            min_size: 50,
            respect_sentences: true,
        }
    }

    /// Create config for long chunks (sections).
    pub fn long() -> Self {
        Self {
            max_size: 2048,
            overlap: 256,
            min_size: 100,
            respect_sentences: true,
        }
    }
}

/// Text chunker.
pub struct Chunker {
    config: ChunkConfig,
}

impl Chunker {
    /// Create a new chunker with config.
    pub fn new(config: ChunkConfig) -> Self {
        Self { config }
    }

    /// Create with default config.
    pub fn default_config() -> Self {
        Self::new(ChunkConfig::default())
    }

    /// Split text into chunks.
    pub fn chunk(&self, text: &str) -> Vec<Chunk> {
        if text.len() <= self.config.max_size {
            return vec![Chunk {
                text: text.to_string(),
                start: 0,
                end: text.len(),
                index: 0,
            }];
        }

        let mut chunks = Vec::new();
        let mut start = 0;
        let mut index = 0;

        while start < text.len() {
            let mut end = (start + self.config.max_size).min(text.len());

            // Try to break at sentence boundary
            if self.config.respect_sentences && end < text.len() {
                if let Some(break_point) = self.find_sentence_break(&text[start..end]) {
                    end = start + break_point;
                }
            }

            // Ensure minimum size for non-final chunks
            if end - start < self.config.min_size && end < text.len() {
                end = (start + self.config.min_size).min(text.len());
            }

            chunks.push(Chunk {
                text: text[start..end].to_string(),
                start,
                end,
                index,
            });

            // Move start with overlap
            start = if end >= text.len() {
                end
            } else {
                (end - self.config.overlap).max(start + 1)
            };
            index += 1;
        }

        chunks
    }

    /// Find a good sentence break point within text.
    fn find_sentence_break(&self, text: &str) -> Option<usize> {
        // Look for sentence endings from the end
        let search_start = text.len().saturating_sub(self.config.overlap * 2);

        for (i, c) in text[search_start..].char_indices().rev() {
            let pos = search_start + i;
            if (c == '.' || c == '!' || c == '?') && pos > self.config.min_size {
                // Check if followed by space or end
                let next_pos = pos + c.len_utf8();
                if next_pos >= text.len()
                    || text[next_pos..].starts_with(char::is_whitespace)
                {
                    return Some(next_pos);
                }
            }
        }

        // Fall back to paragraph break
        for (i, c) in text[search_start..].char_indices().rev() {
            if c == '\n' {
                let pos = search_start + i;
                if pos > self.config.min_size {
                    return Some(pos + 1);
                }
            }
        }

        None
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Self::default_config()
    }
}

/// A chunk of text with position info.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The chunk text.
    pub text: String,
    /// Start position in original text.
    pub start: usize,
    /// End position in original text.
    pub end: usize,
    /// Chunk index.
    pub index: usize,
}

impl Chunk {
    /// Get chunk length.
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_text_no_chunking() {
        let chunker = Chunker::default_config();
        let chunks = chunker.chunk("Hello world");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello world");
    }

    #[test]
    fn test_long_text_chunking() {
        let chunker = Chunker::new(ChunkConfig {
            max_size: 50,
            overlap: 10,
            min_size: 10,
            respect_sentences: false,
        });

        let text = "This is a longer text that should be split into multiple chunks for processing.";
        let chunks = chunker.chunk(text);

        assert!(chunks.len() > 1);
        // All chunks should be within size limits
        for chunk in &chunks {
            assert!(chunk.len() <= 50);
        }
    }

    #[test]
    fn test_sentence_boundary_respect() {
        let chunker = Chunker::new(ChunkConfig {
            max_size: 100,
            overlap: 20,
            min_size: 10,
            respect_sentences: true,
        });

        let text = "First sentence here. Second sentence follows. Third sentence ends.";
        let chunks = chunker.chunk(text);

        // Should try to break at sentence boundaries
        for chunk in &chunks {
            // Chunks should generally end with punctuation if they break mid-text
            if chunk.end < text.len() {
                let last_char = chunk.text.trim_end().chars().last();
                // Either ends with punctuation or is the full text
                assert!(
                    last_char == Some('.') || last_char == Some('!') || last_char == Some('?')
                        || chunk.text == text
                );
            }
        }
    }
}
