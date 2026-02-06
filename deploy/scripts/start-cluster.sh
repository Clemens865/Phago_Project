#!/bin/bash
set -e

NUM_SHARDS=${1:-3}

echo "Starting Phago distributed cluster with $NUM_SHARDS shards..."

cd "$(dirname "$0")/.."

# Build images
docker-compose build

# Start coordinator
docker-compose up -d coordinator
sleep 2

# Start shards
for i in $(seq 0 $((NUM_SHARDS-1))); do
    echo "Starting shard $i..."
    SHARD_ID=$i docker-compose up -d shard-$i 2>/dev/null || true
done

echo "Cluster started. Coordinator at localhost:9000"
echo "Shards at localhost:9001-$((9000+NUM_SHARDS))"
