//! Memory management for large vector datasets

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Memory management configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum number of vectors to keep in memory
    pub max_vectors: usize,
    /// Maximum memory usage in bytes (approximate)
    pub max_memory_bytes: usize,
    /// Whether to enable automatic cleanup
    pub auto_cleanup: bool,
    /// Cleanup threshold (percentage of max_vectors)
    pub cleanup_threshold: f32,
    /// Whether to use memory mapping for large datasets
    pub use_memory_mapping: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_vectors: 100_000,
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            auto_cleanup: true,
            cleanup_threshold: 0.8, // 80%
            use_memory_mapping: false,
        }
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Current number of vectors in memory
    pub vector_count: usize,
    /// Estimated memory usage in bytes
    pub memory_bytes: usize,
    /// Number of vectors evicted
    pub evicted_count: usize,
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of cache misses
    pub cache_misses: usize,
}

/// Memory manager for vector store
pub struct MemoryManager {
    config: MemoryConfig,
    stats: Arc<RwLock<MemoryStats>>,
    vector_count: AtomicUsize,
    memory_usage: AtomicUsize,
    evicted_count: AtomicUsize,
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            stats: Arc::new(RwLock::new(MemoryStats {
                vector_count: 0,
                memory_bytes: 0,
                evicted_count: 0,
                cache_hits: 0,
                cache_misses: 0,
            })),
            vector_count: AtomicUsize::new(0),
            memory_usage: AtomicUsize::new(0),
            evicted_count: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
            config,
        }
    }

    /// Check if adding a vector would exceed memory limits
    pub fn can_add_vector(&self, vector_size_bytes: usize) -> bool {
        let current_count = self.vector_count.load(Ordering::Relaxed);
        let current_memory = self.memory_usage.load(Ordering::Relaxed);

        // Check vector count limit
        if current_count >= self.config.max_vectors {
            return false;
        }

        // Check memory limit
        if current_memory + vector_size_bytes > self.config.max_memory_bytes {
            return false;
        }

        true
    }

    /// Record addition of a vector
    pub fn record_vector_added(&self, vector_size_bytes: usize) {
        self.vector_count.fetch_add(1, Ordering::Relaxed);
        self.memory_usage
            .fetch_add(vector_size_bytes, Ordering::Relaxed);
        self.update_stats();
    }

    /// Record removal of a vector
    pub fn record_vector_removed(&self, vector_size_bytes: usize) {
        self.vector_count.fetch_sub(1, Ordering::Relaxed);
        self.memory_usage
            .fetch_sub(vector_size_bytes, Ordering::Relaxed);
        self.update_stats();
    }

    /// Record cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
        self.update_stats();
    }

    /// Record cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        self.update_stats();
    }

    /// Check if cleanup is needed
    pub fn needs_cleanup(&self) -> bool {
        if !self.config.auto_cleanup {
            return false;
        }

        let current_count = self.vector_count.load(Ordering::Relaxed);
        let threshold = (self.config.max_vectors as f32 * self.config.cleanup_threshold) as usize;

        current_count >= threshold
    }

    /// Get current memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        self.stats.read().clone()
    }

    /// Get memory usage percentage
    pub fn get_memory_usage_percentage(&self) -> f32 {
        let current_memory = self.memory_usage.load(Ordering::Relaxed);
        (current_memory as f32 / self.config.max_memory_bytes as f32) * 100.0
    }

    /// Get vector count percentage
    pub fn get_vector_count_percentage(&self) -> f32 {
        let current_count = self.vector_count.load(Ordering::Relaxed);
        (current_count as f32 / self.config.max_vectors as f32) * 100.0
    }

    /// Update internal statistics
    fn update_stats(&self) {
        let mut stats = self.stats.write();
        stats.vector_count = self.vector_count.load(Ordering::Relaxed);
        stats.memory_bytes = self.memory_usage.load(Ordering::Relaxed);
        stats.evicted_count = self.evicted_count.load(Ordering::Relaxed);
        stats.cache_hits = self.cache_hits.load(Ordering::Relaxed);
        stats.cache_misses = self.cache_misses.load(Ordering::Relaxed);
    }

    /// Get configuration
    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: MemoryConfig) {
        self.config = config.clone();
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.vector_count.store(0, Ordering::Relaxed);
        self.memory_usage.store(0, Ordering::Relaxed);
        self.evicted_count.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.update_stats();
    }
}

/// LRU cache for vector storage
pub struct LruCache<K, V> {
    capacity: usize,
    cache: HashMap<K, (V, usize)>, // (value, access_count)
    access_counter: usize,
}

impl<K, V> LruCache<K, V>
where
    K: Clone + std::hash::Hash + Eq,
{
    /// Create a new LRU cache
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            cache: HashMap::new(),
            access_counter: 0,
        }
    }

    /// Get a value from the cache
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some((_, access_count)) = self.cache.get_mut(key) {
            self.access_counter += 1;
            *access_count = self.access_counter;
            self.cache.get(key).map(|(v, _)| v)
        } else {
            None
        }
    }

    /// Insert a value into the cache
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.access_counter += 1;

        if self.cache.len() >= self.capacity {
            // Remove least recently used item
            let lru_key = self.find_lru_key();
            if let Some(key) = lru_key {
                self.cache.remove(&key);
            }
        }

        self.cache
            .insert(key, (value, self.access_counter))
            .map(|(v, _)| v)
    }

    /// Remove a value from the cache
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.cache.remove(key).map(|(v, _)| v)
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Find the least recently used key
    fn find_lru_key(&self) -> Option<K> {
        self.cache
            .iter()
            .min_by_key(|(_, (_, access_count))| access_count)
            .map(|(key, _)| key.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config);

        let stats = manager.get_stats();
        assert_eq!(stats.vector_count, 0);
        assert_eq!(stats.memory_bytes, 0);
    }

    #[test]
    fn test_memory_limits() {
        let config = MemoryConfig {
            max_vectors: 10,
            max_memory_bytes: 1000,
            ..Default::default()
        };
        let manager = MemoryManager::new(config.clone());

        // Should be able to add vectors within limits
        assert!(manager.can_add_vector(100));
        manager.record_vector_added(100);

        // Should not be able to exceed vector count
        for _ in 0..10 {
            manager.record_vector_added(50);
        }
        assert!(!manager.can_add_vector(50));

        // Should not be able to exceed memory limit
        let manager = MemoryManager::new(config);
        assert!(!manager.can_add_vector(2000));
    }

    #[test]
    fn test_lru_cache() {
        let mut cache = LruCache::new(3);

        // Insert values
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);

        assert_eq!(cache.len(), 3);

        // Access to update LRU
        cache.get(&"a");

        // Insert one more to trigger eviction
        cache.insert("d", 4);

        assert_eq!(cache.len(), 3);
        assert!(cache.get(&"b").is_none()); // Should be evicted
        assert!(cache.get(&"a").is_some()); // Should still be there
    }

    #[test]
    fn test_memory_stats() {
        let manager = MemoryManager::new(MemoryConfig::default());

        manager.record_vector_added(100);
        manager.record_cache_hit();
        manager.record_cache_miss();

        let stats = manager.get_stats();
        assert_eq!(stats.vector_count, 1);
        assert_eq!(stats.memory_bytes, 100);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
    }
}
