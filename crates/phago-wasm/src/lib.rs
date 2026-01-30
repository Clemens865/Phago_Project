//! # Phago WASM
//!
//! WASM compilation targets and host-agent bindings.
//!
//! Each agent compiles to a .wasm module — a sandboxed cell with its
//! own linear memory (cytoplasm) and controlled imports/exports
//! (receptor proteins on the cell membrane).
