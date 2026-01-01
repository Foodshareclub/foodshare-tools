//! File-based caching system for expensive operations
//!
//! Provides a simple but effective caching layer for:
//! - Command output caching
//! - File hash caching
//! - Configuration caching
//!
//! # Example
//!
//! ```rust,ignore
//! use foodshare_core::cache::{Cache, CacheConfig};
//!
//! let cache = Cache::new(CacheConfig::default())?;
//!
//! // Cache a value
//! cache.set("key", "value", None)?;
//!
//! // Retrieve with TTL check
//! if let Some(value) = cache.get::<String>("key")? {
//!     println!("Cached: {}", value);
//! }
//! ```

use crate::error::{Error, ErrorCode, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache directory path
    pub cache_dir: PathBuf,
    /// Default TTL in seconds (0 = no expiry)
    pub default_ttl_secs: u64,
    /// Maximum cache size in bytes (0 = unlimited)
    pub max_size_bytes: u64,
    /// Enable in-memory caching
    pub memory_cache: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("foodshare-tools");

        Self {
            cache_dir,
            default_ttl_secs: 3600, // 1 hour
            max_size_bytes: 100 * 1024 * 1024, // 100MB
            memory_cache: true,
        }
    }
}

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// When the entry was created
    created_at: u64,
    /// When the entry expires (0 = never)
    expires_at: u64,
    /// Size of the cached data in bytes
    size_bytes: u64,
    /// Hash of the cached data for integrity
    hash: String,
}

/// File-based cache with optional in-memory layer
pub struct Cache {
    config: CacheConfig,
    memory: Option<RwLock<HashMap<String, (CacheEntry, Vec<u8>)>>>,
}

impl Cache {
    /// Create a new cache instance
    pub fn new(config: CacheConfig) -> Result<Self> {
        // Ensure cache directory exists
        fs::create_dir_all(&config.cache_dir)?;

        let memory = if config.memory_cache {
            Some(RwLock::new(HashMap::new()))
        } else {
            None
        };

        Ok(Self { config, memory })
    }

    /// Create with default configuration
    pub fn default_cache() -> Result<Self> {
        Self::new(CacheConfig::default())
    }

    /// Get a cached value
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let cache_key = self.hash_key(key);

        // Try memory cache first
        if let Some(ref memory) = self.memory {
            let guard = memory.read().map_err(|_| Error::new(
                ErrorCode::Internal,
                "Failed to acquire cache read lock",
            ))?;

            if let Some((entry, data)) = guard.get(&cache_key) {
                if !self.is_expired(entry) {
                    let value: T = serde_json::from_slice(data)?;
                    return Ok(Some(value));
                }
            }
        }

        // Try file cache
        let entry_path = self.entry_path(&cache_key);
        let data_path = self.data_path(&cache_key);

        if !entry_path.exists() || !data_path.exists() {
            return Ok(None);
        }

        let entry: CacheEntry = serde_json::from_str(&fs::read_to_string(&entry_path)?)?;

        if self.is_expired(&entry) {
            // Clean up expired entry
            let _ = fs::remove_file(&entry_path);
            let _ = fs::remove_file(&data_path);
            return Ok(None);
        }

        let data = fs::read(&data_path)?;

        // Verify integrity
        let hash = self.hash_data(&data);
        if hash != entry.hash {
            // Corrupted entry, remove it
            let _ = fs::remove_file(&entry_path);
            let _ = fs::remove_file(&data_path);
            return Ok(None);
        }

        // Update memory cache
        if let Some(ref memory) = self.memory {
            if let Ok(mut guard) = memory.write() {
                guard.insert(cache_key, (entry, data.clone()));
            }
        }

        let value: T = serde_json::from_slice(&data)?;
        Ok(Some(value))
    }

    /// Set a cached value
    pub fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()> {
        let cache_key = self.hash_key(key);
        let data = serde_json::to_vec(value)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let ttl_secs = ttl
            .map(|d| d.as_secs())
            .unwrap_or(self.config.default_ttl_secs);

        let entry = CacheEntry {
            created_at: now,
            expires_at: if ttl_secs > 0 { now + ttl_secs } else { 0 },
            size_bytes: data.len() as u64,
            hash: self.hash_data(&data),
        };

        // Write to file cache
        let entry_path = self.entry_path(&cache_key);
        let data_path = self.data_path(&cache_key);

        fs::write(&entry_path, serde_json::to_string(&entry)?)?;
        fs::write(&data_path, &data)?;

        // Update memory cache
        if let Some(ref memory) = self.memory {
            if let Ok(mut guard) = memory.write() {
                guard.insert(cache_key, (entry, data));
            }
        }

        Ok(())
    }

    /// Remove a cached value
    pub fn remove(&self, key: &str) -> Result<bool> {
        let cache_key = self.hash_key(key);

        // Remove from memory
        if let Some(ref memory) = self.memory {
            if let Ok(mut guard) = memory.write() {
                guard.remove(&cache_key);
            }
        }

        // Remove from disk
        let entry_path = self.entry_path(&cache_key);
        let data_path = self.data_path(&cache_key);

        let existed = entry_path.exists();
        let _ = fs::remove_file(&entry_path);
        let _ = fs::remove_file(&data_path);

        Ok(existed)
    }

    /// Clear all cached values
    pub fn clear(&self) -> Result<()> {
        // Clear memory
        if let Some(ref memory) = self.memory {
            if let Ok(mut guard) = memory.write() {
                guard.clear();
            }
        }

        // Clear disk
        if self.config.cache_dir.exists() {
            for entry in fs::read_dir(&self.config.cache_dir)? {
                let entry = entry?;
                let _ = fs::remove_file(entry.path());
            }
        }

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats> {
        let mut total_size = 0u64;
        let mut entry_count = 0usize;
        let mut expired_count = 0usize;

        if self.config.cache_dir.exists() {
            for entry in fs::read_dir(&self.config.cache_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().map_or(false, |e| e == "meta") {
                    entry_count += 1;

                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(cache_entry) = serde_json::from_str::<CacheEntry>(&content) {
                            total_size += cache_entry.size_bytes;
                            if self.is_expired(&cache_entry) {
                                expired_count += 1;
                            }
                        }
                    }
                }
            }
        }

        let memory_entries = self.memory
            .as_ref()
            .and_then(|m| m.read().ok())
            .map(|g| g.len())
            .unwrap_or(0);

        Ok(CacheStats {
            total_entries: entry_count,
            expired_entries: expired_count,
            total_size_bytes: total_size,
            memory_entries,
            cache_dir: self.config.cache_dir.clone(),
        })
    }

    /// Clean up expired entries
    pub fn cleanup(&self) -> Result<usize> {
        let mut removed = 0;

        // Clean memory cache
        if let Some(ref memory) = self.memory {
            if let Ok(mut guard) = memory.write() {
                let expired_keys: Vec<String> = guard
                    .iter()
                    .filter(|(_, (entry, _))| self.is_expired(entry))
                    .map(|(k, _)| k.clone())
                    .collect();

                for key in expired_keys {
                    guard.remove(&key);
                    removed += 1;
                }
            }
        }

        // Clean disk cache
        if self.config.cache_dir.exists() {
            for entry in fs::read_dir(&self.config.cache_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().map_or(false, |e| e == "meta") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(cache_entry) = serde_json::from_str::<CacheEntry>(&content) {
                            if self.is_expired(&cache_entry) {
                                let data_path = path.with_extension("data");
                                let _ = fs::remove_file(&path);
                                let _ = fs::remove_file(&data_path);
                                removed += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(removed)
    }

    // Helper methods

    fn hash_key(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn hash_data(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    fn entry_path(&self, cache_key: &str) -> PathBuf {
        self.config.cache_dir.join(format!("{}.meta", cache_key))
    }

    fn data_path(&self, cache_key: &str) -> PathBuf {
        self.config.cache_dir.join(format!("{}.data", cache_key))
    }

    fn is_expired(&self, entry: &CacheEntry) -> bool {
        if entry.expires_at == 0 {
            return false;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now > entry.expires_at
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    /// Total number of cache entries
    pub total_entries: usize,
    /// Number of expired entries
    pub expired_entries: usize,
    /// Total size of cached data in bytes
    pub total_size_bytes: u64,
    /// Number of entries in memory cache
    pub memory_entries: usize,
    /// Path to the cache directory
    pub cache_dir: PathBuf,
}

/// Cached command execution
pub fn cached_command<F, T>(
    cache: &Cache,
    key: &str,
    ttl: Option<Duration>,
    f: F,
) -> Result<T>
where
    F: FnOnce() -> Result<T>,
    T: Serialize + DeserializeOwned,
{
    // Try cache first
    if let Some(value) = cache.get::<T>(key)? {
        return Ok(value);
    }

    // Execute and cache
    let value = f()?;
    cache.set(key, &value, ttl)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_cache() -> (Cache, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            default_ttl_secs: 3600,
            max_size_bytes: 0,
            memory_cache: true,
        };
        let cache = Cache::new(config).unwrap();
        (cache, temp_dir)
    }

    #[test]
    fn test_set_and_get() {
        let (cache, _temp) = test_cache();

        cache.set("test_key", &"test_value".to_string(), None).unwrap();
        let value: Option<String> = cache.get("test_key").unwrap();

        assert_eq!(value, Some("test_value".to_string()));
    }

    #[test]
    fn test_get_missing() {
        let (cache, _temp) = test_cache();

        let value: Option<String> = cache.get("nonexistent").unwrap();
        assert!(value.is_none());
    }

    #[test]
    fn test_remove() {
        let (cache, _temp) = test_cache();

        cache.set("to_remove", &42i32, None).unwrap();
        assert!(cache.get::<i32>("to_remove").unwrap().is_some());

        cache.remove("to_remove").unwrap();
        assert!(cache.get::<i32>("to_remove").unwrap().is_none());
    }

    #[test]
    fn test_expiry() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            default_ttl_secs: 1,
            max_size_bytes: 0,
            memory_cache: false, // Disable memory cache to test file-based expiry
        };
        let cache = Cache::new(config).unwrap();

        // Set with 1 second TTL
        cache.set("expires", &"value".to_string(), Some(Duration::from_secs(1))).unwrap();

        // Should exist immediately
        let value: Option<String> = cache.get("expires").unwrap();
        assert!(value.is_some(), "Value should exist immediately after setting");

        // Wait for expiry (2 seconds to ensure we're past the expiry time)
        std::thread::sleep(Duration::from_secs(2));
        let value: Option<String> = cache.get("expires").unwrap();
        assert!(value.is_none(), "Value should be expired after 2 seconds");
    }

    #[test]
    fn test_stats() {
        let (cache, _temp) = test_cache();

        cache.set("key1", &"value1".to_string(), None).unwrap();
        cache.set("key2", &"value2".to_string(), None).unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 2);
    }
}
