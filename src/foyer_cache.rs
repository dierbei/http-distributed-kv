use async_trait::async_trait;
use foyer::{Cache, CacheBuilder};

use crate::cache_trait::BCache;
use anyhow::Result;

/// `FoyerCache` is an implementation of the `BCache` trait using the `foyer` caching library.
///
/// It provides a thread-safe and asynchronous cache with basic cache operations
/// such as insertion, retrieval, and removal of key-value pairs.
///
/// # Example
///
/// ```rust
/// let mut cache = FoyerCache::new(2).await;
/// cache.insert("key".to_string(), "value".to_string()).await;
/// assert_eq!(cache.get("key".to_string()).await.unwrap(), "value");
/// ```
#[derive(Debug, Clone)]
pub struct FoyerCache {
    /// The inner cache structure provided by the `foyer` crate.
    cc: Cache<String, String>,
}

impl FoyerCache {
    /// Creates a new `FoyerCache` instance with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `cache_capacity` - The maximum number of entries the cache can hold.
    ///
    /// # Returns
    ///
    /// * A new `FoyerCache` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// let cache = FoyerCache::new(10).await;
    /// ```
    pub async fn new(cache_capacity: usize) -> Self {
        let cache: Cache<String, String> = CacheBuilder::new(cache_capacity).with_shards(1).build();

        Self { cc: cache }
    }
}

#[async_trait]
impl BCache for FoyerCache {
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
        self.cc.insert(key, val);
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
        let value = match self.cc.get(&key) {
            Some(e) => e.value().clone(),
            None => {
                return Err(anyhow::anyhow!("key not found"));
            }
        };

        Ok(value)
    }

    /// Asynchronously removes the key-value pair from the cache if it exists.
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
        self.cc.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit test for `FoyerCache`.
    ///
    /// This test creates a cache, inserts a key-value pair, retrieves it, and
    /// checks that the correct value is returned.
    #[tokio::test]
    async fn test_foyer_cache() {
        let mut cache = FoyerCache::new(2).await;
        cache.insert("hello".to_string(), "world".to_string()).await;
        assert_eq!(
            cache.get("hello".to_string()).await.unwrap(),
            "world".to_string()
        );
    }
}
