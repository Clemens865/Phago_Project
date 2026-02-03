# phago-viz

Browser-based real-time visualization for Phago colonies.

## Overview

Generates self-contained HTML visualizations with:

- **Force-directed knowledge graph** — D3.js powered interactive graph
- **Agent spatial canvas** — Shows agent positions and movements
- **Event timeline** — Scrollable history of colony events
- **Metrics dashboard** — Real-time stats with tick slider

## Usage

```rust
use phago_viz::HtmlViz;
use phago_runtime::colony::Colony;

let colony = Colony::new();
// ... run simulation ...

// Generate visualization
let html = HtmlViz::generate(&colony);

// Save to file
std::fs::write("colony.html", html).unwrap();
```

Then open `colony.html` in a browser.

## Features

- Self-contained single HTML file (no external dependencies)
- Interactive zoom/pan on graph
- Click nodes to see details
- Playback through simulation ticks
- Export graph as PNG

## Part of Phago

This is a subcrate of [phago](https://crates.io/crates/phago). For most use cases, depend on the main `phago` crate instead.

## License

MIT
