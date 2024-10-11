use async_trait::async_trait;
use moka::future::Cache;

use crate::cache_trait::BCache;
use anyhow::Result;

/// `MokaCache` is an implementation of the `BCache` trait using the `moka` asynchronous cache.
///
/// It allows asynchronous insertion, retrieval, and removal of key-value pairs, providing
/// a simple interface for caching with automatic expiration.
///
/// # Example
///
/// ```rust
/// let mut cache = MokaCache::new(100).await;
/// cache.insert("key".to_string(), "value".to_string()).await;
/// let value = cache.get("key".to_string()).await.unwrap();
/// assert_eq!(value, "value".to_string());
/// ```
#[derive(Debug, Clone)]
pub struct MokaCache {
    /// The underlying cache instance provided by the `moka` crate.
    cc: Cache<String, String>,
}

impl MokaCache {
    /// Creates a new `MokaCache` with the given capacity.
    ///
    /// # Arguments
    ///
    /// * `cache_capacity` - The maximum number of entries the cache can hold before it starts evicting items.
    ///
    /// # Returns
    ///
    /// * A new instance of `MokaCache`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let cache = MokaCache::new(10).await;
    /// ```
    pub async fn new(cache_capacity: usize) -> Self {
        let cache = Cache::new(cache_capacity as u64);

        Self { cc: cache }
    }
}

#[async_trait]
impl BCache for MokaCache {
    /// Asynchronously inserts a key-value pair into the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - A `String` representing the key.
    /// * `val` - A `String` representing the value associated with the key.
    ///
    /// # Example
    ///
    /// ```rust
    /// cache.insert("key".to_string(), "value".to_string()).await;
    /// ```
    async fn insert(&mut self, key: String, val: String) {
        self.cc.insert(key, val).await;
    }

    /// Asynchronously retrieves the value associated with the given key from the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - A `String` representing the key to retrieve.
    ///
    /// # Returns
    ///
    /// * A `Result<String>` containing the value if found, or an error if the key is not found.
    ///
    /// # Errors
    ///
    /// If the key does not exist in the cache, an `anyhow::Error` is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// let value = cache.get("key".to_string()).await.unwrap();
    /// assert_eq!(value, "value".to_string());
    /// ```
    async fn get(&mut self, key: String) -> Result<String> {
        let value = match self.cc.get(&key).await {
            Some(e) => e,
            None => {
                return Err(anyhow::anyhow!("key not found"));
            }
        };

        Ok(value)
    }

    /// Asynchronously removes the key-value pair from the cache, if it exists.
    ///
    /// # Arguments
    ///
    /// * `key` - A `String` representing the key to remove.
    ///
    /// # Example
    ///
    /// ```rust
    /// cache.remove("key".to_string()).await;
    /// ```
    async fn remove(&mut self, key: String) {
        self.cc.remove(&key).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit test for `MokaCache`.
    ///
    /// This test creates a cache, inserts a key-value pair, retrieves it, and
    /// checks that the correct value is returned.
    #[tokio::test]
    async fn test_moka_cache() {
        let mut cache = MokaCache::new(2).await;
        cache.insert("hello".to_string(), "world".to_string()).await;
        assert_eq!(
            cache.get("hello".to_string()).await.unwrap(),
            "world".to_string()
        );
    }
}
