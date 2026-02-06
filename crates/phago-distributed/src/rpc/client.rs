//! tarpc client utilities.
//!
//! This module provides client utilities for connecting to remote shards
//! and the coordinator. It includes connection functions, retry logic,
//! and a connection pool for efficient client reuse.

use crate::rpc::protocol::{CoordinatorServiceClient, ShardServiceClient};
use crate::types::ShardId;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tarpc::client::Config;
use tokio::sync::RwLock;
use tokio_serde::formats::Bincode;
use tracing::{debug, error, info, warn};

/// Default connection timeout in milliseconds.
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 5000;

/// Default number of retry attempts for failed connections.
const DEFAULT_RETRY_ATTEMPTS: u32 = 3;

/// Default delay between retry attempts in milliseconds.
const DEFAULT_RETRY_DELAY_MS: u64 = 500;

/// Configuration for client connections.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Number of retry attempts.
    pub retry_attempts: u32,
    /// Delay between retries.
    pub retry_delay: Duration,
    /// Maximum pending requests per client.
    pub max_pending_requests: usize,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_millis(DEFAULT_CONNECT_TIMEOUT_MS),
            retry_attempts: DEFAULT_RETRY_ATTEMPTS,
            retry_delay: Duration::from_millis(DEFAULT_RETRY_DELAY_MS),
            max_pending_requests: 100,
        }
    }
}

/// Create a client connection to a shard.
///
/// Establishes a TCP connection to the shard at the given address and
/// returns a tarpc client for making RPC calls.
///
/// # Arguments
///
/// * `addr` - The socket address of the shard server
///
/// # Errors
///
/// Returns an error if the connection cannot be established.
///
/// # Example
///
/// ```rust,ignore
/// use phago_distributed::rpc::client::connect_to_shard;
///
/// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
/// let client = connect_to_shard(addr).await?;
///
/// let health = client.health_check(tarpc::context::current()).await?;
/// ```
pub async fn connect_to_shard(addr: SocketAddr) -> Result<ShardServiceClient, std::io::Error> {
    debug!("Connecting to shard at {}", addr);
    let transport = tarpc::serde_transport::tcp::connect(addr, Bincode::default).await?;
    let client = ShardServiceClient::new(Config::default(), transport).spawn();
    info!("Connected to shard at {}", addr);
    Ok(client)
}

/// Create a client connection to a shard with custom configuration.
///
/// # Arguments
///
/// * `addr` - The socket address of the shard server
/// * `config` - Client configuration
///
/// # Errors
///
/// Returns an error if the connection cannot be established.
pub async fn connect_to_shard_with_config(
    addr: SocketAddr,
    config: &ClientConfig,
) -> Result<ShardServiceClient, std::io::Error> {
    debug!("Connecting to shard at {} with custom config", addr);

    let transport = tokio::time::timeout(
        config.connect_timeout,
        tarpc::serde_transport::tcp::connect(addr, Bincode::default),
    )
    .await
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "connection timeout"))??;

    let mut tarpc_config = Config::default();
    tarpc_config.max_in_flight_requests = config.max_pending_requests;

    let client = ShardServiceClient::new(tarpc_config, transport).spawn();
    info!("Connected to shard at {}", addr);
    Ok(client)
}

/// Create a client connection to the coordinator.
///
/// Establishes a TCP connection to the coordinator at the given address and
/// returns a tarpc client for making RPC calls.
///
/// # Arguments
///
/// * `addr` - The socket address of the coordinator server
///
/// # Errors
///
/// Returns an error if the connection cannot be established.
///
/// # Example
///
/// ```rust,ignore
/// use phago_distributed::rpc::client::connect_to_coordinator;
///
/// let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
/// let client = connect_to_coordinator(addr).await?;
///
/// let shards = client.list_shards(tarpc::context::current()).await?;
/// ```
pub async fn connect_to_coordinator(
    addr: SocketAddr,
) -> Result<CoordinatorServiceClient, std::io::Error> {
    debug!("Connecting to coordinator at {}", addr);
    let transport = tarpc::serde_transport::tcp::connect(addr, Bincode::default).await?;
    let client = CoordinatorServiceClient::new(Config::default(), transport).spawn();
    info!("Connected to coordinator at {}", addr);
    Ok(client)
}

/// Create a client connection to the coordinator with custom configuration.
///
/// # Arguments
///
/// * `addr` - The socket address of the coordinator server
/// * `config` - Client configuration
///
/// # Errors
///
/// Returns an error if the connection cannot be established.
pub async fn connect_to_coordinator_with_config(
    addr: SocketAddr,
    config: &ClientConfig,
) -> Result<CoordinatorServiceClient, std::io::Error> {
    debug!("Connecting to coordinator at {} with custom config", addr);

    let transport = tokio::time::timeout(
        config.connect_timeout,
        tarpc::serde_transport::tcp::connect(addr, Bincode::default),
    )
    .await
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "connection timeout"))??;

    let mut tarpc_config = Config::default();
    tarpc_config.max_in_flight_requests = config.max_pending_requests;

    let client = CoordinatorServiceClient::new(tarpc_config, transport).spawn();
    info!("Connected to coordinator at {}", addr);
    Ok(client)
}

/// Connect to a shard with automatic retry on failure.
///
/// Attempts to connect to the shard, retrying on failure according to
/// the configuration.
///
/// # Arguments
///
/// * `addr` - The socket address of the shard server
/// * `config` - Client configuration including retry settings
///
/// # Errors
///
/// Returns an error if all retry attempts fail.
pub async fn connect_to_shard_with_retry(
    addr: SocketAddr,
    config: &ClientConfig,
) -> Result<ShardServiceClient, std::io::Error> {
    let mut last_error = None;

    for attempt in 0..config.retry_attempts {
        if attempt > 0 {
            warn!(
                "Retry attempt {} connecting to shard at {}",
                attempt + 1,
                addr
            );
            tokio::time::sleep(config.retry_delay).await;
        }

        match connect_to_shard_with_config(addr, config).await {
            Ok(client) => {
                if attempt > 0 {
                    info!(
                        "Successfully connected to shard at {} after {} attempts",
                        addr,
                        attempt + 1
                    );
                }
                return Ok(client);
            }
            Err(e) => {
                warn!("Failed to connect to shard at {}: {}", addr, e);
                last_error = Some(e);
            }
        }
    }

    error!(
        "Failed to connect to shard at {} after {} attempts",
        addr, config.retry_attempts
    );
    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotConnected, "connection failed")
    }))
}

/// Connect to the coordinator with automatic retry on failure.
///
/// # Arguments
///
/// * `addr` - The socket address of the coordinator server
/// * `config` - Client configuration including retry settings
///
/// # Errors
///
/// Returns an error if all retry attempts fail.
pub async fn connect_to_coordinator_with_retry(
    addr: SocketAddr,
    config: &ClientConfig,
) -> Result<CoordinatorServiceClient, std::io::Error> {
    let mut last_error = None;

    for attempt in 0..config.retry_attempts {
        if attempt > 0 {
            warn!(
                "Retry attempt {} connecting to coordinator at {}",
                attempt + 1,
                addr
            );
            tokio::time::sleep(config.retry_delay).await;
        }

        match connect_to_coordinator_with_config(addr, config).await {
            Ok(client) => {
                if attempt > 0 {
                    info!(
                        "Successfully connected to coordinator at {} after {} attempts",
                        addr,
                        attempt + 1
                    );
                }
                return Ok(client);
            }
            Err(e) => {
                warn!("Failed to connect to coordinator at {}: {}", addr, e);
                last_error = Some(e);
            }
        }
    }

    error!(
        "Failed to connect to coordinator at {} after {} attempts",
        addr, config.retry_attempts
    );
    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotConnected, "connection failed")
    }))
}

/// A pool of shard client connections.
///
/// The connection pool maintains a cached set of connections to shards,
/// creating new connections on demand and reusing existing ones.
///
/// # Thread Safety
///
/// The pool is thread-safe and can be shared across multiple tasks.
///
/// # Example
///
/// ```rust,ignore
/// use phago_distributed::rpc::client::ShardClientPool;
///
/// let pool = ShardClientPool::new();
/// pool.register_shard(ShardId::new(0), "127.0.0.1:8080".parse().unwrap());
///
/// let client = pool.get_client(ShardId::new(0)).await?;
/// let health = client.health_check(tarpc::context::current()).await?;
/// ```
pub struct ShardClientPool {
    /// Mapping from shard ID to address.
    addresses: Arc<RwLock<HashMap<ShardId, SocketAddr>>>,
    /// Cached client connections.
    clients: Arc<RwLock<HashMap<ShardId, ShardServiceClient>>>,
    /// Client configuration.
    config: ClientConfig,
}

impl ShardClientPool {
    /// Create a new empty connection pool.
    pub fn new() -> Self {
        Self {
            addresses: Arc::new(RwLock::new(HashMap::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            config: ClientConfig::default(),
        }
    }

    /// Create a new connection pool with custom configuration.
    pub fn with_config(config: ClientConfig) -> Self {
        Self {
            addresses: Arc::new(RwLock::new(HashMap::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Register a shard's address.
    ///
    /// This does not establish a connection immediately; connections
    /// are created lazily when `get_client` is called.
    pub async fn register_shard(&self, shard_id: ShardId, addr: SocketAddr) {
        let mut addresses = self.addresses.write().await;
        addresses.insert(shard_id, addr);
        debug!("Registered shard {:?} at {}", shard_id, addr);
    }

    /// Unregister a shard and close any cached connection.
    pub async fn unregister_shard(&self, shard_id: ShardId) {
        let mut addresses = self.addresses.write().await;
        addresses.remove(&shard_id);

        let mut clients = self.clients.write().await;
        clients.remove(&shard_id);
        debug!("Unregistered shard {:?}", shard_id);
    }

    /// Get a client for the specified shard.
    ///
    /// Returns a cached client if available, otherwise creates a new connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the shard is not registered or if the connection fails.
    pub async fn get_client(
        &self,
        shard_id: ShardId,
    ) -> Result<ShardServiceClient, std::io::Error> {
        // Check for cached client
        {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(&shard_id) {
                return Ok(client.clone());
            }
        }

        // Get address
        let addr = {
            let addresses = self.addresses.read().await;
            addresses.get(&shard_id).copied()
        };

        let addr = addr.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("shard {:?} not registered", shard_id),
            )
        })?;

        // Create new connection
        let client = connect_to_shard_with_retry(addr, &self.config).await?;

        // Cache the client
        {
            let mut clients = self.clients.write().await;
            clients.insert(shard_id, client.clone());
        }

        Ok(client)
    }

    /// Get clients for all registered shards.
    ///
    /// Attempts to connect to all shards, returning successfully connected clients.
    /// Failed connections are logged but do not cause the entire operation to fail.
    pub async fn get_all_clients(&self) -> Vec<(ShardId, ShardServiceClient)> {
        let addresses: Vec<_> = {
            let addresses = self.addresses.read().await;
            addresses.iter().map(|(&id, &addr)| (id, addr)).collect()
        };

        let mut results = Vec::with_capacity(addresses.len());
        for (shard_id, _) in addresses {
            match self.get_client(shard_id).await {
                Ok(client) => results.push((shard_id, client)),
                Err(e) => warn!("Failed to get client for shard {:?}: {}", shard_id, e),
            }
        }

        results
    }

    /// Check if a shard is registered.
    pub async fn has_shard(&self, shard_id: ShardId) -> bool {
        let addresses = self.addresses.read().await;
        addresses.contains_key(&shard_id)
    }

    /// Get the number of registered shards.
    pub async fn shard_count(&self) -> usize {
        let addresses = self.addresses.read().await;
        addresses.len()
    }

    /// Get the number of cached connections.
    pub async fn cached_connection_count(&self) -> usize {
        let clients = self.clients.read().await;
        clients.len()
    }

    /// Clear all cached connections.
    ///
    /// This forces new connections to be created on the next `get_client` call.
    pub async fn clear_cache(&self) {
        let mut clients = self.clients.write().await;
        clients.clear();
        debug!("Cleared connection cache");
    }

    /// Remove a cached connection for a specific shard.
    ///
    /// Useful for forcing a reconnection after a failure.
    pub async fn invalidate_client(&self, shard_id: ShardId) {
        let mut clients = self.clients.write().await;
        clients.remove(&shard_id);
        debug!("Invalidated cached client for shard {:?}", shard_id);
    }
}

impl Default for ShardClientPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ShardClientPool {
    fn clone(&self) -> Self {
        Self {
            addresses: Arc::clone(&self.addresses),
            clients: Arc::clone(&self.clients),
            config: self.config.clone(),
        }
    }
}

/// A coordinator client wrapper with automatic reconnection.
///
/// This wrapper maintains a connection to the coordinator and
/// automatically attempts to reconnect if the connection is lost.
pub struct CoordinatorClient {
    /// The coordinator's address.
    addr: SocketAddr,
    /// The cached client connection.
    client: Arc<RwLock<Option<CoordinatorServiceClient>>>,
    /// Client configuration.
    config: ClientConfig,
}

impl CoordinatorClient {
    /// Create a new coordinator client.
    ///
    /// This does not establish a connection immediately; the connection
    /// is created lazily on the first call to `get`.
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            client: Arc::new(RwLock::new(None)),
            config: ClientConfig::default(),
        }
    }

    /// Create a new coordinator client with custom configuration.
    pub fn with_config(addr: SocketAddr, config: ClientConfig) -> Self {
        Self {
            addr,
            client: Arc::new(RwLock::new(None)),
            config,
        }
    }

    /// Get the coordinator client, creating a connection if needed.
    pub async fn get(&self) -> Result<CoordinatorServiceClient, std::io::Error> {
        // Check for cached client
        {
            let client = self.client.read().await;
            if let Some(ref c) = *client {
                return Ok(c.clone());
            }
        }

        // Create new connection
        let new_client = connect_to_coordinator_with_retry(self.addr, &self.config).await?;

        // Cache the client
        {
            let mut client = self.client.write().await;
            *client = Some(new_client.clone());
        }

        Ok(new_client)
    }

    /// Force a reconnection.
    ///
    /// Useful after detecting a connection failure.
    pub async fn reconnect(&self) -> Result<CoordinatorServiceClient, std::io::Error> {
        // Clear the cached client
        {
            let mut client = self.client.write().await;
            *client = None;
        }

        // Get a new connection
        self.get().await
    }

    /// Invalidate the cached connection without reconnecting.
    pub async fn invalidate(&self) {
        let mut client = self.client.write().await;
        *client = None;
        debug!("Invalidated coordinator client");
    }

    /// Get the coordinator's address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Clone for CoordinatorClient {
    fn clone(&self) -> Self {
        Self {
            addr: self.addr,
            client: Arc::clone(&self.client),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(
            config.connect_timeout.as_millis(),
            DEFAULT_CONNECT_TIMEOUT_MS as u128
        );
        assert_eq!(config.retry_attempts, DEFAULT_RETRY_ATTEMPTS);
        assert_eq!(
            config.retry_delay.as_millis(),
            DEFAULT_RETRY_DELAY_MS as u128
        );
    }

    #[tokio::test]
    async fn test_shard_client_pool_register() {
        let pool = ShardClientPool::new();

        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        pool.register_shard(ShardId::new(0), addr).await;

        assert!(pool.has_shard(ShardId::new(0)).await);
        assert!(!pool.has_shard(ShardId::new(1)).await);
        assert_eq!(pool.shard_count().await, 1);
    }

    #[tokio::test]
    async fn test_shard_client_pool_unregister() {
        let pool = ShardClientPool::new();

        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        pool.register_shard(ShardId::new(0), addr).await;
        pool.unregister_shard(ShardId::new(0)).await;

        assert!(!pool.has_shard(ShardId::new(0)).await);
        assert_eq!(pool.shard_count().await, 0);
    }

    #[tokio::test]
    async fn test_shard_client_pool_get_client_not_registered() {
        let pool = ShardClientPool::new();

        let result = pool.get_client(ShardId::new(0)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_shard_client_pool_clear_cache() {
        let pool = ShardClientPool::new();

        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        pool.register_shard(ShardId::new(0), addr).await;

        // The cache should be empty initially
        assert_eq!(pool.cached_connection_count().await, 0);

        // Clear should work even when empty
        pool.clear_cache().await;
        assert_eq!(pool.cached_connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_shard_client_pool_invalidate_client() {
        let pool = ShardClientPool::new();

        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        pool.register_shard(ShardId::new(0), addr).await;

        // Invalidate should work even without a cached client
        pool.invalidate_client(ShardId::new(0)).await;
        assert_eq!(pool.cached_connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_shard_client_pool_clone() {
        let pool = ShardClientPool::new();

        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        pool.register_shard(ShardId::new(0), addr).await;

        let pool_clone = pool.clone();
        assert!(pool_clone.has_shard(ShardId::new(0)).await);

        // Changes should be visible in both
        pool_clone
            .register_shard(ShardId::new(1), "127.0.0.1:8081".parse().unwrap())
            .await;
        assert!(pool.has_shard(ShardId::new(1)).await);
    }

    #[test]
    fn test_coordinator_client_new() {
        let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
        let client = CoordinatorClient::new(addr);

        assert_eq!(client.addr(), addr);
    }

    #[tokio::test]
    async fn test_coordinator_client_invalidate() {
        let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
        let client = CoordinatorClient::new(addr);

        // Invalidate should work even without a cached connection
        client.invalidate().await;
    }

    #[tokio::test]
    async fn test_coordinator_client_clone() {
        let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
        let client = CoordinatorClient::new(addr);
        let client_clone = client.clone();

        assert_eq!(client_clone.addr(), addr);
    }

    #[tokio::test]
    async fn test_shard_client_pool_with_config() {
        let config = ClientConfig {
            connect_timeout: Duration::from_secs(10),
            retry_attempts: 5,
            retry_delay: Duration::from_millis(200),
            max_pending_requests: 50,
        };

        let pool = ShardClientPool::with_config(config);
        assert_eq!(pool.shard_count().await, 0);
    }

    #[test]
    fn test_coordinator_client_with_config() {
        let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
        let config = ClientConfig {
            connect_timeout: Duration::from_secs(10),
            retry_attempts: 5,
            retry_delay: Duration::from_millis(200),
            max_pending_requests: 50,
        };

        let client = CoordinatorClient::with_config(addr, config);
        assert_eq!(client.addr(), addr);
    }
}
