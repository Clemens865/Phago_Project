//! Shard registry for tracking active shards.
//!
//! This module maintains a registry of all active shards in the distributed
//! cluster, including their status, heartbeat information, and metrics.

use crate::types::{ShardId, ShardInfo, ShardStatus};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Extended shard info with status tracking.
#[derive(Debug, Clone)]
pub struct RegisteredShard {
    /// Basic shard information.
    pub info: ShardInfo,
    /// Current status of the shard.
    pub status: ShardStatus,
    /// Memory usage in bytes.
    pub memory_bytes: u64,
}

impl RegisteredShard {
    /// Create a new registered shard from info.
    pub fn new(info: ShardInfo) -> Self {
        Self {
            info,
            status: ShardStatus::Online,
            memory_bytes: 0,
        }
    }
}

/// Registry of active shards in the cluster.
///
/// The registry tracks all shards, their current status, and health metrics.
/// It is used by the coordinator to manage the cluster topology and route
/// requests to healthy shards.
pub struct ShardRegistry {
    /// Map of shard IDs to their information.
    shards: HashMap<ShardId, RegisteredShard>,
    /// Counter for assigning new shard IDs.
    next_id: u32,
    /// Timeout for considering a shard dead (milliseconds).
    heartbeat_timeout_ms: u64,
}

impl ShardRegistry {
    /// Create a new empty shard registry.
    pub fn new() -> Self {
        Self {
            shards: HashMap::new(),
            next_id: 0,
            heartbeat_timeout_ms: 30_000, // 30 seconds default
        }
    }

    /// Create a registry with custom heartbeat timeout.
    pub fn with_heartbeat_timeout(timeout_ms: u64) -> Self {
        Self {
            shards: HashMap::new(),
            next_id: 0,
            heartbeat_timeout_ms: timeout_ms,
        }
    }

    /// Register a new shard and return its assigned ID.
    ///
    /// The shard will be assigned a unique ID and added to the registry.
    /// Its initial status will be set to Online.
    pub fn register(&mut self, info: ShardInfo) -> ShardId {
        let id = ShardId::new(self.next_id);
        self.next_id += 1;

        let mut registered = RegisteredShard::new(info);
        registered.info.id = id;
        registered.info.last_heartbeat = Self::current_timestamp();
        registered.status = ShardStatus::Online;

        self.shards.insert(id, registered);
        id
    }

    /// Register a shard with a specific ID.
    ///
    /// This is useful when restoring state or in deterministic testing.
    /// The next_id counter will be updated if necessary.
    pub fn register_with_id(&mut self, info: ShardInfo, id: ShardId) -> ShardId {
        let mut registered = RegisteredShard::new(info);
        registered.info.id = id;
        registered.info.last_heartbeat = Self::current_timestamp();
        registered.status = ShardStatus::Online;

        self.shards.insert(id, registered);

        // Update next_id to avoid conflicts
        if id.0 >= self.next_id {
            self.next_id = id.0 + 1;
        }

        id
    }

    /// Get shard info by ID.
    pub fn get(&self, id: &ShardId) -> Option<&ShardInfo> {
        self.shards.get(id).map(|r| &r.info)
    }

    /// Get registered shard by ID (includes status).
    pub fn get_registered(&self, id: &ShardId) -> Option<&RegisteredShard> {
        self.shards.get(id)
    }

    /// Get mutable registered shard by ID.
    pub fn get_registered_mut(&mut self, id: &ShardId) -> Option<&mut RegisteredShard> {
        self.shards.get_mut(id)
    }

    /// Remove a shard from the registry.
    pub fn remove(&mut self, id: &ShardId) -> Option<ShardInfo> {
        self.shards.remove(id).map(|r| r.info)
    }

    /// Get all shard infos.
    pub fn all(&self) -> Vec<ShardInfo> {
        self.shards.values().map(|r| r.info.clone()).collect()
    }

    /// Get all shard IDs.
    pub fn all_ids(&self) -> Vec<ShardId> {
        self.shards.keys().copied().collect()
    }

    /// Get the number of registered shards.
    pub fn count(&self) -> usize {
        self.shards.len()
    }

    /// Check if a shard exists.
    pub fn contains(&self, id: &ShardId) -> bool {
        self.shards.contains_key(id)
    }

    /// Update heartbeat timestamp for a shard.
    pub fn heartbeat(&mut self, id: &ShardId) {
        if let Some(registered) = self.shards.get_mut(id) {
            registered.info.last_heartbeat = Self::current_timestamp();
            // Restore online status if it was marked as recovering
            if registered.status == ShardStatus::Recovering {
                registered.status = ShardStatus::Online;
            }
        }
    }

    /// Update heartbeat with explicit timestamp (for testing or remote sync).
    pub fn heartbeat_with_timestamp(&mut self, id: &ShardId, timestamp: u64) {
        if let Some(registered) = self.shards.get_mut(id) {
            registered.info.last_heartbeat = timestamp;
        }
    }

    /// Update shard status.
    pub fn set_status(&mut self, id: &ShardId, status: ShardStatus) {
        if let Some(registered) = self.shards.get_mut(id) {
            registered.status = status;
        }
    }

    /// Get the status of a shard.
    pub fn get_status(&self, id: &ShardId) -> Option<ShardStatus> {
        self.shards.get(id).map(|r| r.status)
    }

    /// Update shard metrics.
    pub fn update_metrics(&mut self, id: &ShardId, document_count: usize, memory_bytes: u64) {
        if let Some(registered) = self.shards.get_mut(id) {
            registered.info.document_count = document_count;
            registered.memory_bytes = memory_bytes;
        }
    }

    /// Get all online shards.
    pub fn online_shards(&self) -> Vec<ShardInfo> {
        self.shards
            .values()
            .filter(|r| r.status == ShardStatus::Online)
            .map(|r| r.info.clone())
            .collect()
    }

    /// Get all shards with a specific status.
    pub fn shards_with_status(&self, status: ShardStatus) -> Vec<ShardInfo> {
        self.shards
            .values()
            .filter(|r| r.status == status)
            .map(|r| r.info.clone())
            .collect()
    }

    /// Check for and mark dead shards based on heartbeat timeout.
    ///
    /// Returns the IDs of shards that were marked as offline.
    pub fn check_dead_shards(&mut self) -> Vec<ShardId> {
        let now = Self::current_timestamp();
        let timeout = self.heartbeat_timeout_ms;
        let mut dead_shards = Vec::new();

        for (id, registered) in self.shards.iter_mut() {
            if registered.status == ShardStatus::Online
                && now - registered.info.last_heartbeat > timeout
            {
                registered.status = ShardStatus::Offline;
                dead_shards.push(*id);
            }
        }

        dead_shards
    }

    /// Get total document count across all shards.
    pub fn total_documents(&self) -> u64 {
        self.shards
            .values()
            .map(|r| r.info.document_count as u64)
            .sum()
    }

    /// Get total memory usage across all shards.
    pub fn total_memory(&self) -> u64 {
        self.shards.values().map(|r| r.memory_bytes).sum()
    }

    /// Get the shard with the least documents (for load balancing).
    pub fn least_loaded_shard(&self) -> Option<ShardId> {
        self.shards
            .values()
            .filter(|r| r.status == ShardStatus::Online)
            .min_by_key(|r| r.info.document_count)
            .map(|r| r.info.id)
    }

    /// Get current Unix timestamp in milliseconds.
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

impl Default for ShardRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_shard_info() -> ShardInfo {
        ShardInfo::new(ShardId::new(0), "127.0.0.1:8080".to_string())
    }

    #[test]
    fn test_registry_creation() {
        let registry = ShardRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_register_shard() {
        let mut registry = ShardRegistry::new();
        let info = test_shard_info();

        let id = registry.register(info);
        assert_eq!(id, ShardId::new(0));
        assert_eq!(registry.count(), 1);

        let info2 = test_shard_info();
        let id2 = registry.register(info2);
        assert_eq!(id2, ShardId::new(1));
        assert_eq!(registry.count(), 2);
    }

    #[test]
    fn test_get_shard() {
        let mut registry = ShardRegistry::new();
        let info = test_shard_info();
        let id = registry.register(info);

        let retrieved = registry.get(&id).unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(registry.get_status(&id), Some(ShardStatus::Online));
    }

    #[test]
    fn test_remove_shard() {
        let mut registry = ShardRegistry::new();
        let info = test_shard_info();
        let id = registry.register(info);

        assert!(registry.contains(&id));
        let removed = registry.remove(&id);
        assert!(removed.is_some());
        assert!(!registry.contains(&id));
    }

    #[test]
    fn test_set_status() {
        let mut registry = ShardRegistry::new();
        let info = test_shard_info();
        let id = registry.register(info);

        assert_eq!(registry.get_status(&id), Some(ShardStatus::Online));

        registry.set_status(&id, ShardStatus::Draining);
        assert_eq!(registry.get_status(&id), Some(ShardStatus::Draining));
    }

    #[test]
    fn test_update_metrics() {
        let mut registry = ShardRegistry::new();
        let info = test_shard_info();
        let id = registry.register(info);

        registry.update_metrics(&id, 100, 1024 * 1024);

        let shard = registry.get(&id).unwrap();
        assert_eq!(shard.document_count, 100);
        let registered = registry.get_registered(&id).unwrap();
        assert_eq!(registered.memory_bytes, 1024 * 1024);
    }

    #[test]
    fn test_online_shards() {
        let mut registry = ShardRegistry::new();

        let id1 = registry.register(test_shard_info());
        let id2 = registry.register(test_shard_info());
        let _id3 = registry.register(test_shard_info());

        registry.set_status(&id2, ShardStatus::Offline);

        let online = registry.online_shards();
        assert_eq!(online.len(), 2);
        assert!(online.iter().all(|s| s.id != id2));
    }

    #[test]
    fn test_total_documents() {
        let mut registry = ShardRegistry::new();

        let id1 = registry.register(test_shard_info());
        let id2 = registry.register(test_shard_info());

        registry.update_metrics(&id1, 100, 1000);
        registry.update_metrics(&id2, 200, 2000);

        assert_eq!(registry.total_documents(), 300);
        assert_eq!(registry.total_memory(), 3000);
    }

    #[test]
    fn test_least_loaded_shard() {
        let mut registry = ShardRegistry::new();

        let id1 = registry.register(test_shard_info());
        let id2 = registry.register(test_shard_info());
        let id3 = registry.register(test_shard_info());

        registry.update_metrics(&id1, 100, 1000);
        registry.update_metrics(&id2, 50, 500);
        registry.update_metrics(&id3, 200, 2000);

        assert_eq!(registry.least_loaded_shard(), Some(id2));
    }

    #[test]
    fn test_check_dead_shards() {
        let mut registry = ShardRegistry::with_heartbeat_timeout(100);
        let info = test_shard_info();
        let id = registry.register(info);

        // Set heartbeat to a very old timestamp
        registry.heartbeat_with_timestamp(&id, 0);

        let dead = registry.check_dead_shards();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0], id);
        assert_eq!(registry.get_status(&id), Some(ShardStatus::Offline));
    }

    #[test]
    fn test_register_with_specific_id() {
        let mut registry = ShardRegistry::new();
        let info = test_shard_info();

        let id = registry.register_with_id(info, ShardId::new(42));
        assert_eq!(id, ShardId::new(42));
        assert!(registry.contains(&id));

        // Next auto-assigned ID should be 43
        let info2 = test_shard_info();
        let id2 = registry.register(info2);
        assert_eq!(id2, ShardId::new(43));
    }
}
