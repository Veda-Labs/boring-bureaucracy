use alloy::primitives::Address;
use eyre::{Result, eyre};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast, oneshot};
// The different types that can be stored in cache
#[derive(Debug, Clone, PartialEq)]
pub enum CacheValue {
    Address(Address),
    String(String),
    U8(u8),
    U32(u32),
    Bool(bool),
}

// Structure to hold both the value and any pending requests
struct CacheEntry {
    value: Option<CacheValue>,
    // Broadcast channel for multiple readers
    subscribers: broadcast::Sender<CacheValue>,
    // Tag of the block that first set this value
    origin: Option<String>,
}

// The main cache structure
pub struct SharedCache {
    // Main storage
    storage: RwLock<HashMap<String, CacheEntry>>,
    // Configuration
    timeout: Duration,
}

// Create a type alias for the shared cache
pub type SharedCacheRef = Arc<SharedCache>;

impl SharedCache {
    pub fn new(timeout: Duration) -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
            timeout,
        }
    }

    pub async fn get(&self, key: &str, tag: &str) -> Result<CacheValue> {
        let mut storage = self.storage.write().await;

        // Check if value exists
        if let Some(entry) = storage.get(key) {
            if let Some(value) = &entry.value {
                return Ok(value.clone());
            }
        }

        // Value doesn't exist, create new entry with broadcast channel
        let (tx, mut rx) = broadcast::channel(16);
        let entry = CacheEntry {
            value: None,
            subscribers: tx,
            origin: None,
        };
        storage.insert(key.to_string(), entry);

        // Wait for value with timeout
        tokio::select! {
            Ok(value) = rx.recv() => Ok(value),
            _ = tokio::time::sleep(self.timeout) => {
                Err(eyre!("Timeout waiting for cache value: {}, (from {})", key, tag))
            }
        }
    }

    pub async fn set(&self, key: &str, value: CacheValue, tag: &str) -> Result<()> {
        let mut storage = self.storage.write().await;

        if let Some(entry) = storage.get(key) {
            // Check if value matches existing
            if let Some(existing) = &entry.value {
                if existing != &value {
                    return Err(eyre!(
                        "Cache value mismatch for key {}: existing={:?} (from {}), new={:?} (from {})",
                        key,
                        existing,
                        entry.origin.as_deref().unwrap_or("unknown"),
                        value,
                        tag
                    ));
                }
                return Ok(());
            }
        }

        // Create new entry or update existing
        let (tx, _) = broadcast::channel(16);
        let entry = CacheEntry {
            value: Some(value.clone()),
            subscribers: tx,
            origin: Some(tag.to_string()),
        };
        storage.insert(key.to_string(), entry);

        // Notify all waiting readers
        if let Some(entry) = storage.get(key) {
            let _ = entry.subscribers.send(value);
        }

        Ok(())
    }
}
