//! RPC protocol definitions using tarpc.
//!
//! This module implements the tarpc-based RPC layer for
//! inter-node communication including:
//! - Service definitions for colony operations
//! - Client and server implementations
//! - Connection pooling and retry logic
//!
//! # Architecture
//!
//! The RPC layer consists of three main components:
//!
//! - **Protocol**: Trait definitions for `ShardService` and `CoordinatorService`
//! - **Server**: Server implementations that wrap local components
//! - **Client**: Client utilities including connection pooling
//!
//! # Example: Starting a Shard Server
//!
//! ```rust,ignore
//! use phago_distributed::rpc::server::ShardServer;
//! use phago_distributed::shard::ShardedColony;
//!
//! let shard = Arc::new(RwLock::new(ShardedColony::new(...)));
//! let server = ShardServer::new(shard);
//! server.serve("127.0.0.1:8080".parse().unwrap()).await?;
//! ```
//!
//! # Example: Connecting as a Client
//!
//! ```rust,ignore
//! use phago_distributed::rpc::client::{connect_to_shard, ShardClientPool};
//!
//! // Direct connection
//! let client = connect_to_shard("127.0.0.1:8080".parse().unwrap()).await?;
//!
//! // Or use the connection pool
//! let pool = ShardClientPool::new();
//! pool.register_shard(ShardId::new(0), "127.0.0.1:8080".parse().unwrap());
//! let client = pool.get_client(ShardId::new(0)).await?;
//! ```

pub mod client;
pub mod messages;
pub mod protocol;
pub mod server;

// Protocol exports
pub use protocol::{
    CoordinatorService, CoordinatorServiceClient, RpcError, RpcResult, ShardService,
    ShardServiceClient, TickStatus,
};

// Message exports
pub use messages::{
    BatchUpdate, BatchUpdateResult, CrossShardEdgeNotification, CrossShardSignal, HeartbeatMessage,
    HeartbeatResponse, NodeTransferRequest, NodeTransferResponse, QueryGatherResponse,
    QueryScatterRequest, ShardCommand, StartTickMessage, UpdateOperation,
};

// Client exports
pub use client::{
    connect_to_coordinator, connect_to_coordinator_with_config, connect_to_coordinator_with_retry,
    connect_to_shard, connect_to_shard_with_config, connect_to_shard_with_retry, ClientConfig,
    CoordinatorClient, ShardClientPool,
};

// Server exports
pub use server::{CoordinatorServer, ShardServer};
