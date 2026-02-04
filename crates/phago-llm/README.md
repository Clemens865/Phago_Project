# phago-llm

LLM integration for Phago semantic intelligence.

## Overview

This crate provides LLM backends for enhanced concept extraction:

- **OllamaBackend** — Local LLM via Ollama (no API key needed)
- **ClaudeBackend** — Anthropic Claude API
- **OpenAiBackend** — OpenAI GPT API
- **MockBackend** — Testing without real LLM calls

## Usage

```rust,ignore
use phago_llm::{OllamaBackend, LlmBackend};

#[tokio::main]
async fn main() {
    // Local Ollama (no API key)
    let ollama = OllamaBackend::localhost().with_model("llama3.2");
    let concepts = ollama.extract_concepts("Cell membrane transport").await.unwrap();

    // Claude API
    let claude = ClaudeBackend::new("sk-ant-...").sonnet();
    let concepts = claude.extract_concepts("Cell membrane transport").await.unwrap();
}
```

## Features

| Feature | Description |
|---------|-------------|
| `local` | Ollama backend |
| `api` | Claude and OpenAI backends |
| `full` | All backends |

## Part of Phago

This is a subcrate of [phago](https://crates.io/crates/phago). For most use cases, depend on the main `phago` crate with the `llm` feature instead.

## License

MIT
