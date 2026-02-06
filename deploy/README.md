# Phago Distributed Cluster Deployment

## Local Development with Docker Compose

### Prerequisites
- Docker 20.10+
- Docker Compose 2.0+

### Quick Start

```bash
# Build and start the cluster
cd deploy
docker-compose up --build

# Scale shards (e.g., to 5 shards)
docker-compose up --scale shard-0=5

# Stop the cluster
docker-compose down
```

### Architecture

The cluster consists of:
- **Coordinator** (port 9000): Manages shard registry, document routing, tick synchronization
- **Shards** (ports 9001-900x): Store documents and graph nodes, execute tick phases

### Configuration

Environment variables:
- `RUST_LOG`: Log level (trace, debug, info, warn, error)
- `NUM_SHARDS`: Expected number of shards (coordinator only)
- `SHARD_ID`: Unique shard identifier (0, 1, 2, ...)
- `SHARD_PORT`: Port for shard RPC server
- `COORDINATOR_ADDR`: Address of coordinator (host:port)

### Monitoring

Check cluster health:
```bash
curl http://localhost:9000/status
```

View shard stats:
```bash
curl http://localhost:9001/stats
```

### Benchmarking

Run the benchmark suite:
```bash
docker-compose exec shard-0 ./phago-bench --shards 3 --docs 1000
```
