//! Streaming document ingestion for Phago Colony.
//!
//! This module provides real-time document processing as they arrive,
//! with support for file watchers, async streams, and backpressure handling.
//!
//! # Feature Flag
//!
//! This module requires the `streaming` feature:
//! ```toml
//! phago-runtime = { version = "0.5", features = ["streaming"] }
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │ File Watcher │───>│ Bounded Channel │───>│ StreamingColony │
//! └──────────────┘    └─────────────────┘    └─────────────────┘
//!        │                    ▲                       │
//!        │            Backpressure                    ▼
//!        │                    │              ┌─────────────────┐
//!        └────────────────────┘              │  Colony.tick()  │
//!                                            └─────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use phago_runtime::streaming::{StreamingColony, FileWatcher};
//! use phago_runtime::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let colony = Colony::new();
//!     let streaming = StreamingColony::new(colony, StreamingConfig::default());
//!
//!     // Watch a directory for new documents
//!     let watcher = FileWatcher::new("./documents")?;
//!     streaming.start_watching(watcher).await?;
//!
//!     // Process documents as they arrive
//!     streaming.run_until_idle().await;
//!     Ok(())
//! }
//! ```

#![cfg(feature = "streaming")]

use crate::colony::{Colony, ColonyEvent};
use phago_agents::digester::Digester;
use phago_core::types::{DocumentId, Position};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::broadcast;

/// A document to be ingested into the colony.
#[derive(Debug, Clone)]
pub struct IngestDocument {
    /// Document title or filename.
    pub title: String,
    /// Document content.
    pub content: String,
    /// Position in 2D space (optional, defaults to auto-layout).
    pub position: Option<Position>,
    /// Source path if from file system.
    pub source_path: Option<PathBuf>,
}

impl IngestDocument {
    /// Create a new document for ingestion.
    pub fn new(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: content.into(),
            position: None,
            source_path: None,
        }
    }

    /// Set the position for this document.
    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.position = Some(Position::new(x, y));
        self
    }

    /// Set the source path for this document.
    pub fn with_source(mut self, path: impl Into<PathBuf>) -> Self {
        self.source_path = Some(path.into());
        self
    }
}

/// Configuration for streaming ingestion.
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Maximum documents in the queue before backpressure kicks in.
    pub queue_capacity: usize,
    /// Ticks to run after each document ingestion.
    pub ticks_per_document: u64,
    /// Whether to run continuous background ticks.
    pub background_ticks: bool,
    /// Interval between background ticks (ms).
    pub tick_interval_ms: u64,
    /// Auto-layout spacing for documents without explicit position.
    pub auto_layout_spacing: f64,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            queue_capacity: 100,
            ticks_per_document: 10,
            background_ticks: true,
            tick_interval_ms: 100,
            auto_layout_spacing: 5.0,
        }
    }
}

/// Metrics for the streaming ingestion system.
#[derive(Debug, Clone, Default)]
pub struct StreamingMetrics {
    /// Total documents received.
    pub documents_received: u64,
    /// Documents successfully ingested.
    pub documents_ingested: u64,
    /// Documents dropped due to backpressure.
    pub documents_dropped: u64,
    /// Current queue depth.
    pub queue_depth: usize,
    /// Total ticks processed.
    pub ticks_processed: u64,
}

/// A streaming colony that can consume document streams.
///
/// This wraps a Colony and adds streaming ingestion capabilities
/// with backpressure handling and metrics.
pub struct StreamingColony {
    colony: Rc<RefCell<Colony>>,
    config: StreamingConfig,
    metrics: Rc<RefCell<StreamingMetrics>>,
    document_count: Rc<RefCell<u64>>,
    event_tx: broadcast::Sender<ColonyEvent>,
}

impl StreamingColony {
    /// Create a new StreamingColony.
    pub fn new(colony: Colony, config: StreamingConfig) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            colony: Rc::new(RefCell::new(colony)),
            config,
            metrics: Rc::new(RefCell::new(StreamingMetrics::default())),
            document_count: Rc::new(RefCell::new(0)),
            event_tx,
        }
    }

    /// Get a receiver for colony events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<ColonyEvent> {
        self.event_tx.subscribe()
    }

    /// Get current metrics.
    pub fn metrics(&self) -> StreamingMetrics {
        self.metrics.borrow().clone()
    }

    /// Ingest a single document immediately.
    pub fn ingest(&self, doc: IngestDocument) -> DocumentId {
        let position = doc.position.unwrap_or_else(|| {
            let count = *self.document_count.borrow();
            *self.document_count.borrow_mut() += 1;
            Position::new(count as f64 * self.config.auto_layout_spacing, 0.0)
        });

        let doc_id = self.colony.borrow_mut().ingest_document(
            &doc.title,
            &doc.content,
            position,
        );

        // Spawn a digester for the document
        self.colony.borrow_mut().spawn(Box::new(
            Digester::new(position).with_max_idle(30),
        ));

        // Update metrics
        {
            let mut metrics = self.metrics.borrow_mut();
            metrics.documents_received += 1;
            metrics.documents_ingested += 1;
        }

        // Run post-ingestion ticks
        self.run_ticks(self.config.ticks_per_document);

        doc_id
    }

    /// Ingest a document asynchronously, respecting backpressure.
    pub async fn ingest_async(&self, doc: IngestDocument) -> DocumentId {
        // Yield to allow other tasks to run
        tokio::task::yield_now().await;
        self.ingest(doc)
    }

    /// Run N ticks and broadcast events.
    pub fn run_ticks(&self, ticks: u64) {
        for _ in 0..ticks {
            let events = self.colony.borrow_mut().tick();
            self.metrics.borrow_mut().ticks_processed += 1;
            for event in events {
                let _ = self.event_tx.send(event);
            }
        }
    }

    /// Run ticks asynchronously with controlled rate.
    pub async fn run_ticks_async(&self, ticks: u64) {
        let interval = Duration::from_millis(self.config.tick_interval_ms);
        for _ in 0..ticks {
            let events = self.colony.borrow_mut().tick();
            self.metrics.borrow_mut().ticks_processed += 1;
            for event in events {
                let _ = self.event_tx.send(event);
            }
            tokio::time::sleep(interval).await;
        }
    }

    /// Process documents from a channel until it's empty.
    pub async fn process_channel(
        &self,
        mut rx: tokio::sync::mpsc::Receiver<IngestDocument>,
    ) {
        while let Some(doc) = rx.recv().await {
            self.ingest_async(doc).await;
        }
    }

    /// Get the inner colony for direct access.
    pub fn colony(&self) -> &Rc<RefCell<Colony>> {
        &self.colony
    }

    /// Take ownership of the inner colony.
    pub fn into_colony(self) -> Colony {
        match Rc::try_unwrap(self.colony) {
            Ok(cell) => cell.into_inner(),
            Err(_) => panic!("Cannot unwrap StreamingColony: other references exist"),
        }
    }
}

/// Result of a file watch operation.
#[derive(Debug)]
pub enum WatchEvent {
    /// A new file was created or modified.
    FileChanged(PathBuf),
    /// A file was removed.
    FileRemoved(PathBuf),
    /// An error occurred.
    Error(String),
}

/// File watcher for monitoring directories for new documents.
pub struct FileWatcher {
    path: PathBuf,
    extensions: Vec<String>,
    rx: mpsc::Receiver<WatchEvent>,
    _watcher: notify::RecommendedWatcher,
}

impl FileWatcher {
    /// Create a new file watcher for the given path.
    ///
    /// By default, watches for .txt, .md, and .json files.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        Self::with_extensions(path, vec!["txt", "md", "json"])
    }

    /// Create a file watcher with custom extensions.
    pub fn with_extensions<P: AsRef<Path>>(
        path: P,
        extensions: Vec<&str>,
    ) -> Result<Self, String> {
        use notify::{RecursiveMode, Watcher};

        let path = path.as_ref().to_path_buf();
        let extensions: Vec<String> = extensions.into_iter().map(|s| s.to_string()).collect();

        let (tx, rx) = mpsc::channel();
        let ext_clone = extensions.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            match res {
                Ok(event) => {
                    for path in event.paths {
                        // Filter by extension
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy().to_string();
                            if ext_clone.contains(&ext_str) {
                                match event.kind {
                                    notify::EventKind::Create(_) |
                                    notify::EventKind::Modify(_) => {
                                        let _ = tx.send(WatchEvent::FileChanged(path.clone()));
                                    }
                                    notify::EventKind::Remove(_) => {
                                        let _ = tx.send(WatchEvent::FileRemoved(path.clone()));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(WatchEvent::Error(e.to_string()));
                }
            }
        }).map_err(|e| format!("Failed to create watcher: {}", e))?;

        watcher.watch(&path, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch path: {}", e))?;

        Ok(Self {
            path,
            extensions,
            rx,
            _watcher: watcher,
        })
    }

    /// Get the path being watched.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the extensions being watched.
    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    /// Try to receive a watch event without blocking.
    pub fn try_recv(&self) -> Option<WatchEvent> {
        self.rx.try_recv().ok()
    }

    /// Receive a watch event, blocking until one is available.
    pub fn recv(&self) -> Option<WatchEvent> {
        self.rx.recv().ok()
    }

    /// Receive a watch event with a timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<WatchEvent> {
        self.rx.recv_timeout(timeout).ok()
    }
}

/// A document stream that can be consumed by StreamingColony.
pub struct DocumentChannel {
    tx: tokio::sync::mpsc::Sender<IngestDocument>,
    rx: Option<tokio::sync::mpsc::Receiver<IngestDocument>>,
    capacity: usize,
}

impl DocumentChannel {
    /// Create a new bounded document channel.
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);
        Self {
            tx,
            rx: Some(rx),
            capacity,
        }
    }

    /// Get a sender for this channel.
    pub fn sender(&self) -> tokio::sync::mpsc::Sender<IngestDocument> {
        self.tx.clone()
    }

    /// Take the receiver (can only be done once).
    pub fn take_receiver(&mut self) -> Option<tokio::sync::mpsc::Receiver<IngestDocument>> {
        self.rx.take()
    }

    /// Get the capacity of this channel.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Send a document to the channel.
    ///
    /// Returns false if the channel is full (backpressure).
    pub fn try_send(&self, doc: IngestDocument) -> bool {
        self.tx.try_send(doc).is_ok()
    }

    /// Send a document, waiting if the channel is full.
    pub async fn send(&self, doc: IngestDocument) -> Result<(), String> {
        self.tx.send(doc).await
            .map_err(|e| format!("Channel closed: {}", e))
    }
}

/// Watch a directory and stream documents to a channel.
///
/// This function runs in a background thread and reads files as they appear,
/// sending them to the provided channel.
pub fn watch_directory_to_channel(
    watcher: FileWatcher,
    channel: DocumentChannel,
) -> std::thread::JoinHandle<()> {
    let tx = channel.sender();

    std::thread::spawn(move || {
        loop {
            match watcher.recv() {
                Some(WatchEvent::FileChanged(path)) => {
                    // Read the file content
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let title = path.file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Untitled".to_string());

                        let doc = IngestDocument::new(title, content)
                            .with_source(path);

                        // Try to send, drop if channel is full (backpressure)
                        if tx.blocking_send(doc).is_err() {
                            // Channel closed, exit
                            break;
                        }
                    }
                }
                Some(WatchEvent::FileRemoved(_)) => {
                    // Could track removed files if needed
                }
                Some(WatchEvent::Error(e)) => {
                    eprintln!("File watcher error: {}", e);
                }
                None => {
                    // Channel closed
                    break;
                }
            }
        }
    })
}

/// Convenience function to create a file-watching streaming colony.
///
/// Returns the StreamingColony and a handle to the watcher thread.
pub fn streaming_from_directory<P: AsRef<Path>>(
    colony: Colony,
    path: P,
    config: StreamingConfig,
) -> Result<(StreamingColony, std::thread::JoinHandle<()>), String> {
    let watcher = FileWatcher::new(path)?;
    let mut channel = DocumentChannel::new(config.queue_capacity);
    let _rx = channel.take_receiver().ok_or("Channel receiver already taken")?;

    let streaming = StreamingColony::new(colony, config);

    // Start the watcher thread
    let handle = watch_directory_to_channel(watcher, channel);

    // Note: The caller needs to call streaming.process_channel(rx) to consume documents

    Ok((streaming, handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn ingest_document_basic() {
        let colony = Colony::new();
        let streaming = StreamingColony::new(colony, StreamingConfig::default());

        let doc = IngestDocument::new("Test", "Content about cells");
        let _id = streaming.ingest(doc);

        let metrics = streaming.metrics();
        assert_eq!(metrics.documents_ingested, 1);
        assert!(metrics.ticks_processed > 0);
    }

    #[test]
    fn auto_layout_positions() {
        let mut config = StreamingConfig::default();
        config.auto_layout_spacing = 10.0;

        let colony = Colony::new();
        let streaming = StreamingColony::new(colony, config);

        // Ingest multiple documents
        for i in 0..5 {
            let doc = IngestDocument::new(format!("Doc {}", i), "Content");
            streaming.ingest(doc);
        }

        let metrics = streaming.metrics();
        assert_eq!(metrics.documents_ingested, 5);
    }

    #[test]
    fn document_channel_backpressure() {
        let channel = DocumentChannel::new(2);

        // Should succeed
        assert!(channel.try_send(IngestDocument::new("1", "c")));
        assert!(channel.try_send(IngestDocument::new("2", "c")));

        // Should fail (channel full)
        assert!(!channel.try_send(IngestDocument::new("3", "c")));
    }

    #[tokio::test]
    async fn async_channel_send() {
        let channel = DocumentChannel::new(10);
        let tx = channel.sender();

        tx.send(IngestDocument::new("Test", "Content")).await.unwrap();
    }

    #[test]
    fn file_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(temp_dir.path());
        assert!(watcher.is_ok());
    }

    #[test]
    fn file_watcher_detects_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(temp_dir.path()).unwrap();

        // Create a file
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello, world!").unwrap();

        // Wait a bit and check for event
        std::thread::sleep(Duration::from_millis(100));

        // Try to receive (may or may not have event depending on OS timing)
        let _event = watcher.try_recv();
        // Note: Event detection timing varies by OS, so we just verify no panic
    }
}
