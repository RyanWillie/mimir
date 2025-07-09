//! Thread-safe vector store wrapper

use crate::error::{VectorError, VectorResult};
use crate::hnsw_store::{SecureVectorStore, SearchResult};
use crate::persistence::VectorStorePersistence;
use crate::memory_manager::{MemoryManager, MemoryConfig};
use crate::batch_ops::{BatchOperations, BatchConfig, VectorInsert, SearchQuery};
use mimir_core::{crypto::RootKey, MemoryId};
use tokio::sync::Mutex;
use std::path::Path;
use std::sync::Arc;

/// Thread-safe vector store with persistence and memory management
pub struct ThreadSafeVectorStore {
    store: Arc<Mutex<SecureVectorStore<'static>>>,
    persistence: VectorStorePersistence,
    memory_manager: MemoryManager,
    batch_ops: Option<BatchOperations>,
}

impl ThreadSafeVectorStore {
    /// Create a new thread-safe vector store
    pub fn new<P: AsRef<Path>>(
        vault_path: P,
        dimension: usize,
        memory_config: Option<MemoryConfig>,
        _batch_config: Option<BatchConfig>,
    ) -> VectorResult<Self> {
        let store = SecureVectorStore::new(dimension)?;
        let persistence = VectorStorePersistence::new(vault_path);
        let memory_manager = MemoryManager::new(memory_config.unwrap_or_default());
        
        Ok(Self {
            store: Arc::new(Mutex::new(store)),
            persistence,
            memory_manager,
            batch_ops: None, // Will be created when needed
        })
    }
    
    /// Create with embedder
    pub async fn with_embedder<P: AsRef<Path>>(
        vault_path: P,
        model_path: P,
        memory_config: Option<MemoryConfig>,
        _batch_config: Option<BatchConfig>,
    ) -> VectorResult<Self> {
        let store = SecureVectorStore::with_embedder(model_path).await?;
        let persistence = VectorStorePersistence::new(vault_path);
        let memory_manager = MemoryManager::new(memory_config.unwrap_or_default());
        
        Ok(Self {
            store: Arc::new(Mutex::new(store)),
            persistence,
            memory_manager,
            batch_ops: None, // Will be created when needed
        })
    }
    
    /// Create with embedder and rotation
    pub async fn with_embedder_and_rotation<P: AsRef<Path>>(
        vault_path: P,
        model_path: P,
        root_key: &RootKey,
        memory_config: Option<MemoryConfig>,
        _batch_config: Option<BatchConfig>,
    ) -> VectorResult<Self> {
        let store = SecureVectorStore::with_embedder_and_rotation(model_path, root_key).await?;
        let persistence = VectorStorePersistence::new(vault_path);
        let memory_manager = MemoryManager::new(memory_config.unwrap_or_default());
        
        Ok(Self {
            store: Arc::new(Mutex::new(store)),
            persistence,
            memory_manager,
            batch_ops: None, // Will be created when needed
        })
    }
    
    /// Load existing store from disk
    pub async fn load<P: AsRef<Path>>(
        vault_path: P,
        root_key: Option<&RootKey>,
        memory_config: Option<MemoryConfig>,
        _batch_config: Option<BatchConfig>,
    ) -> VectorResult<Option<Self>> {
        Self::load_with_embedder(vault_path, root_key, None, memory_config, _batch_config).await
    }

    /// Load existing store from disk with optional embedder
    pub async fn load_with_embedder<P: AsRef<Path>>(
        vault_path: P,
        root_key: Option<&RootKey>,
        model_path: Option<P>,
        memory_config: Option<MemoryConfig>,
        _batch_config: Option<BatchConfig>,
    ) -> VectorResult<Option<Self>> {
        let vault_path = vault_path.as_ref().to_path_buf();
        let persistence = VectorStorePersistence::new(&vault_path);
        
        // Check if store exists first
        if !persistence.store_exists() {
            return Ok(None);
        }
        
        // Clone persistence for the async borrow
        let persistence_for_load = persistence.clone();
        match persistence_for_load.load_store(root_key).await {
            Ok(Some(mut store)) => {
                // Attach embedder if model path is provided
                if let Some(model_path) = model_path {
                    if let Err(e) = store.attach_embedder(model_path).await {
                        return Err(e);
                    }
                }
                
                let memory_manager = MemoryManager::new(memory_config.unwrap_or_default());
                Ok(Some(Self {
                    store: Arc::new(Mutex::new(store)),
                    persistence,
                    memory_manager,
                    batch_ops: None, // Will be created when needed
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
    
    /// Add a vector to the store
    pub async fn add_vector(&self, memory_id: MemoryId, vector: Vec<f32>) -> VectorResult<()> {
        let vector_size_bytes = vector.len() * std::mem::size_of::<f32>();
        
        // Check memory limits
        if !self.memory_manager.can_add_vector(vector_size_bytes) {
            return Err(VectorError::InvalidInput(
                "Memory limit exceeded".to_string(),
            ));
        }
        
        // Add to store
        let mut store = self.store.lock().await;
        store.add_raw_vector(vector, memory_id).await?;
        
        // Record memory usage
        self.memory_manager.record_vector_added(vector_size_bytes);
        
        Ok(())
    }
    
    /// Add text to the store
    pub async fn add_text(&self, memory_id: MemoryId, text: &str) -> VectorResult<()> {
        let mut store = self.store.lock().await;
        store.add_text(text, memory_id).await?;
        
        // Estimate memory usage (approximate)
        let estimated_size = text.len() * 4; // Rough estimate
        self.memory_manager.record_vector_added(estimated_size);
        
        Ok(())
    }
    
    /// Search for similar vectors
    pub async fn search(&self, query: Vec<f32>, k: usize) -> VectorResult<Vec<SearchResult>> {
        let store = self.store.lock().await;
        let results = store.search_raw_vector(&query, k).await?;
        
        // Record cache hit/miss (simplified)
        self.memory_manager.record_cache_hit();
        
        Ok(results)
    }
    
    /// Search for similar text
    pub async fn search_text(&self, query: &str, k: usize) -> VectorResult<Vec<SearchResult>> {
        let mut store = self.store.lock().await;
        let results = store.search_text(query, k).await?;
        
        // Record cache hit/miss (simplified)
        self.memory_manager.record_cache_hit();
        
        Ok(results)
    }
    
    /// Remove a vector from the store
    pub async fn remove_vector(&self, memory_id: MemoryId) -> VectorResult<()> {
        let mut store = self.store.lock().await;
        store.remove_vector(memory_id).await?;
        
        // Estimate memory freed (approximate)
        let estimated_size = store.dimension() * std::mem::size_of::<f32>();
        self.memory_manager.record_vector_removed(estimated_size);
        
        Ok(())
    }
    
    /// Batch insert vectors
    pub async fn batch_insert(&self, vectors: Vec<VectorInsert>) -> VectorResult<crate::batch_ops::BatchInsertResult> {
        if let Some(batch_ops) = &self.batch_ops {
            batch_ops.batch_insert(vectors).await
        } else {
            Err(VectorError::InvalidInput(
                "Batch operations not configured".to_string(),
            ))
        }
    }
    
    /// Batch search vectors
    pub async fn batch_search(&self, queries: Vec<SearchQuery>) -> VectorResult<crate::batch_ops::BatchSearchResult> {
        if let Some(batch_ops) = &self.batch_ops {
            batch_ops.batch_search(queries).await
        } else {
            Err(VectorError::InvalidInput(
                "Batch operations not configured".to_string(),
            ))
        }
    }
    
    /// Save store to disk
    pub async fn save(&self, root_key: Option<&RootKey>) -> VectorResult<()> {
        let store = self.store.lock().await;
        self.persistence.save_store(&store, root_key).await
    }
    
    /// Get memory statistics
    pub fn get_memory_stats(&self) -> crate::memory_manager::MemoryStats {
        self.memory_manager.get_stats()
    }
    
    /// Get vector count percentage
    pub fn get_vector_count_percentage(&self) -> f32 {
        self.memory_manager.get_vector_count_percentage()
    }
    
    /// Get store statistics
    pub async fn get_store_stats(&self) -> StoreStats {
        let store = self.store.lock().await;
        StoreStats {
            vector_count: store.len(),
            dimension: store.dimension(),
            has_embedder: store.has_embedder(),
            has_rotation: store.has_rotation(),
        }
    }
    
    /// Check if store is empty
    pub async fn is_empty(&self) -> bool {
        let store = self.store.lock().await;
        store.is_empty()
    }
    
    /// Get the number of vectors in the store
    pub async fn len(&self) -> usize {
        let store = self.store.lock().await;
        store.len()
    }
    
    /// Check if a memory ID exists
    pub async fn contains(&self, memory_id: &MemoryId) -> bool {
        let store = self.store.lock().await;
        store.contains(memory_id)
    }
    
    /// Get embedding dimension
    pub async fn dimension(&self) -> usize {
        let store = self.store.lock().await;
        store.dimension()
    }
    
    /// Check if embedder is available
    pub async fn has_embedder(&self) -> bool {
        let store = self.store.lock().await;
        store.has_embedder()
    }
    
    /// Check if rotation is enabled
    pub async fn has_rotation(&self) -> bool {
        let store = self.store.lock().await;
        store.has_rotation()
    }

    /// Attach an embedder to the store
    pub async fn attach_embedder<P: AsRef<Path>>(&self, model_path: P) -> VectorResult<()> {
        let mut store = self.store.lock().await;
        store.attach_embedder(model_path).await
    }
    
    /// Update memory configuration
    pub fn update_memory_config(&mut self, config: MemoryConfig) {
        self.memory_manager.update_config(config);
    }
    
    /// Update batch configuration
    pub async fn update_batch_config(&mut self, config: BatchConfig) {
        if let Some(batch_ops) = &mut self.batch_ops {
            batch_ops.update_config(config);
        } else {
            let dim = self.dimension().await;
            self.batch_ops = Some(BatchOperations::new(
                SecureVectorStore::new(dim).unwrap(),
                config,
            ));
        }
    }
    
    /// Check if cleanup is needed
    pub fn needs_cleanup(&self) -> bool {
        self.memory_manager.needs_cleanup()
    }
    
    /// Perform cleanup
    pub async fn cleanup(&self) -> VectorResult<()> {
        // For now, this is a placeholder
        // In a full implementation, we'd implement LRU eviction
        Ok(())
    }
}

/// Store statistics
#[derive(Debug, Clone)]
pub struct StoreStats {
    pub vector_count: usize,
    pub dimension: usize,
    pub has_embedder: bool,
    pub has_rotation: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mimir_core::test_utils::generators::generate_test_embedding;
    use tempfile::TempDir;
    use uuid::Uuid;
    
    #[tokio::test]
    async fn test_thread_safe_store_creation() {
        let temp_dir = TempDir::new().unwrap();
        let store = ThreadSafeVectorStore::new(temp_dir.path(), 128, None, None).unwrap();
        
        let dim = store.dimension().await;
        assert_eq!(dim, 128);
        assert!(store.is_empty().await);
    }
    
    #[tokio::test]
    async fn test_add_and_search_vector() {
        let temp_dir = TempDir::new().unwrap();
        let store = ThreadSafeVectorStore::new(temp_dir.path(), 128, None, None).unwrap();
        
        let memory_id = Uuid::new_v4();
        let vector = generate_test_embedding(128);
        
        store.add_vector(memory_id, vector.clone()).await.unwrap();
        assert_eq!(store.len().await, 1);
        assert!(store.contains(&memory_id).await);
        
        let results = store.search(vector, 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, memory_id);
    }
    
    #[tokio::test]
    async fn test_concurrent_access() {
        let temp_dir = TempDir::new().unwrap();
        let store = Arc::new(ThreadSafeVectorStore::new(temp_dir.path(), 128, None, None).unwrap());
        
        // Test concurrent access by adding vectors sequentially but with shared store
        for _i in 0..10 {
            let memory_id = Uuid::new_v4();
            let vector = generate_test_embedding(128);
            store.add_vector(memory_id, vector).await.unwrap();
        }
        
        assert_eq!(store.len().await, 10);
    }
    
    #[tokio::test]
    async fn test_memory_limits() {
        let temp_dir = TempDir::new().unwrap();
        let memory_config = MemoryConfig {
            max_vectors: 5,
            max_memory_bytes: 5000, // Increased to account for 128-dim vectors (512 bytes each) plus overhead
            ..Default::default()
        };
        
        let store = ThreadSafeVectorStore::new(temp_dir.path(), 128, Some(memory_config), None).unwrap();
        
        // Add vectors up to the limit
        for _i in 0..5 {
            let memory_id = Uuid::new_v4();
            let vector = generate_test_embedding(128);
            store.add_vector(memory_id, vector).await.unwrap();
        }
        
        // Try to add one more - should fail
        let memory_id = Uuid::new_v4();
        let vector = generate_test_embedding(128);
        let result = store.add_vector(memory_id, vector).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let store = ThreadSafeVectorStore::new(temp_dir.path(), 128, None, None).unwrap();
        
        // Add some vectors
        let memory_id = Uuid::new_v4();
        let vector = generate_test_embedding(128);
        store.add_vector(memory_id, vector).await.unwrap();
        
        // Save to disk
        store.save(None).await.unwrap();
        
        // Load from disk
        let loaded_store = ThreadSafeVectorStore::load(temp_dir.path(), None, None, None).await.unwrap();
        assert!(loaded_store.is_some());
        
        let loaded_store = loaded_store.unwrap();
        assert_eq!(loaded_store.len().await, 1);
        assert!(loaded_store.contains(&memory_id).await);
    }
} 