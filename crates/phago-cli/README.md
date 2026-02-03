# phago-cli

Command-line interface for the Phago biological computing framework.

## Installation

```bash
cargo install phago-cli
```

Or build from source:
```bash
cargo build --release -p phago-cli
```

## Usage

### Initialize a Project

```bash
phago init
```

Creates a `.phago/` directory and `phago.toml` config file.

### Ingest Documents

```bash
# Ingest a directory of text files
phago ingest ./documents

# Ingest with custom settings
phago ingest ./docs --ticks 50 --extensions "txt,md,rst"
```

### Query the Knowledge Graph

```bash
# Basic query
phago query "cell membrane transport"

# With custom alpha (TF-IDF vs graph weight)
phago query "protein folding" --alpha 0.7 --max-results 20
```

### Explore Graph Structure

```bash
# Find most central concepts
phago explore centrality --top 10

# Find bridge concepts
phago explore bridges --top 5

# Find path between concepts
phago explore path "membrane" "transport"

# Count connected components
phago explore components
```

### Run Simulation

```bash
# Run more ticks on existing session
phago run --ticks 100
```

### Export Graph

```bash
phago export graph.json --format json
```

### Manage Sessions

```bash
# Save current session
phago session save my-project

# Load a session
phago session load my-project

# List saved sessions
phago session list
```

### View Statistics

```bash
phago stats
```

## Configuration

The `phago.toml` file controls colony behavior:

```toml
[colony]
tick_rate = 100
max_agents = 50

[digester]
max_idle = 50
sense_radius = 5.0

[wiring]
edge_decay_rate = 0.01
prune_threshold = 0.05
tentative_weight = 0.1

[query]
default_alpha = 0.5
max_results = 10
```

## License

MIT
