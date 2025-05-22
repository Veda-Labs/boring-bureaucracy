use alloy::primitives::Address;
use eyre::{Result, eyre};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
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
    value: CacheValue,
    // Tag of the block that first set this value
    origin: String,
}

// The main cache structure
pub struct SharedCache {
    // Main storage
    storage: RwLock<HashMap<String, CacheEntry>>,
}

// Create a type alias for the shared cache
pub type SharedCacheRef = Arc<SharedCache>;

impl SharedCache {
    pub fn new() -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
        }
    }

    // Try to get a value, returning None if it doesn't exist
    pub async fn get(&self, key: &str) -> Option<CacheValue> {
        let storage = self.storage.read().await;
        storage.get(key).map(|entry| entry.value.clone())
    }

    // Get a value with error if missing
    pub async fn get_required(&self, key: &str, requester: &str) -> Result<CacheValue> {
        if let Some(value) = self.get(key).await {
            Ok(value)
        } else {
            Err(eyre!(
                "Required cache value not found: {} (requested by {})",
                key,
                requester
            ))
        }
    }

    // Convenience method to get an address
    pub async fn get_address(&self, key: &str) -> Option<Address> {
        if let Some(value) = self.get(key).await {
            match value {
                CacheValue::Address(addr) => Some(addr),
                _ => None,
            }
        } else {
            None
        }
    }

    // Set a value with origin tracking and conflict detection
    pub async fn set(&self, key: &str, value: CacheValue, origin: &str) -> Result<()> {
        let mut storage = self.storage.write().await;

        if let Some(entry) = storage.get(key) {
            // Check for value conflicts
            if entry.value != value {
                return Err(eyre!(
                    "Cache value conflict for key {}: existing={:?} (from {}), new={:?} (from {})",
                    key,
                    entry.value,
                    entry.origin,
                    value,
                    origin
                ));
            }
            // Value already exists and matches, nothing to do
            return Ok(());
        }

        // Create new entry
        let entry = CacheEntry {
            value,
            origin: origin.to_string(),
        };

        storage.insert(key.to_string(), entry);
        Ok(())
    }

    // Get all keys (useful for debugging)
    pub async fn get_all_keys(&self) -> Vec<String> {
        let storage = self.storage.read().await;
        storage.keys().cloned().collect()
    }

    // Check if a key exists
    pub async fn has_key(&self, key: &str) -> bool {
        let storage = self.storage.read().await;
        storage.contains_key(key)
    }

    #[cfg(test)]
    pub async fn clear(&self, key: &str) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.remove(key);
        Ok(())
    }

    #[cfg(test)]
    pub async fn clear_all(&self) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::address;

    #[tokio::test]
    async fn test_basic_operations() {
        let cache = SharedCache::new();

        // Set a value
        cache
            .set(
                "test_key",
                CacheValue::String("test_value".to_string()),
                "test_origin",
            )
            .await
            .unwrap();

        // Get the value
        let value = cache.get("test_key").await.unwrap();
        assert_eq!(value, CacheValue::String("test_value".to_string()));

        // Try to get non-existent value
        let missing = cache.get("missing_key").await;
        assert!(missing.is_none());

        // Set same value again (should work)
        cache
            .set(
                "test_key",
                CacheValue::String("test_value".to_string()),
                "other_origin",
            )
            .await
            .unwrap();

        // Try to set conflicting value
        let result = cache
            .set("test_key", CacheValue::U8(123), "conflict_origin")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_address_handling() {
        let cache = SharedCache::new();
        let addr = address!("0x1234567890123456789012345678901234567890");

        // Set an address
        cache
            .set("test_addr", CacheValue::Address(addr), "test_origin")
            .await
            .unwrap();

        // Get the address directly
        let retrieved = cache.get_address("test_addr").await.unwrap();
        assert_eq!(retrieved, addr);

        // Try to get non-existent address
        let missing = cache.get_address("missing_addr").await;
        assert!(missing.is_none());

        // Try to get wrong type as address
        cache
            .set("test_u8", CacheValue::U8(123), "test_origin")
            .await
            .unwrap();
        let wrong_type = cache.get_address("test_u8").await;
        assert!(wrong_type.is_none());
    }

    #[tokio::test]
    async fn test_clear_operations() {
        let cache = SharedCache::new();

        // Set multiple values
        cache
            .set("key1", CacheValue::U8(1), "origin1")
            .await
            .unwrap();
        cache
            .set("key2", CacheValue::U8(2), "origin2")
            .await
            .unwrap();

        // Clear one key
        cache.clear("key1").await.unwrap();
        assert!(cache.get("key1").await.is_none());
        assert!(cache.get("key2").await.is_some());

        // Clear all
        cache.clear_all().await.unwrap();
        assert!(cache.get("key2").await.is_none());
    }
}
