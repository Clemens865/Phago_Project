# Multi-stage build for phago distributed cluster
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY poc/ poc/

# Build release binary
RUN cargo build --release -p phago-distributed

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/phago-* /app/

# Default environment
ENV RUST_LOG=info
ENV SHARD_ID=0
ENV COORDINATOR_ADDR=coordinator:9000
ENV SHARD_PORT=9001

EXPOSE 9000 9001

# Entry point will be specified in docker-compose
CMD ["./phago-shard"]
