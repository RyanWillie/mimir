//! Batch operations for vector store

use crate::error::VectorResult;
use crate::hnsw_store::{SecureVectorStore, SearchResult};
use mimir_core::MemoryId;
use std::sync::Arc;
use parking_lot::Mutex;

/// Batch operation configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Batch size for insertions
    pub insert_batch_size: usize,
    /// Batch size for searches
    pub search_batch_size: usize,
    /// Number of worker threads for batch operations
    pub worker_threads: usize,
    /// Whether to enable parallel processing
    pub parallel_processing: bool,
    /// Timeout for batch operations in seconds
    pub timeout_seconds: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            insert_batch_size: 1000,
            search_batch_size: 100,
            worker_threads: num_cpus::get(),
            parallel_processing: true,
            timeout_seconds: 30,
        }
    }
}

/// Batch insertion result
#[derive(Debug, Clone)]
pub struct BatchInsertResult {
    /// Number of successfully inserted vectors
    pub inserted_count: usize,
    /// Number of failed insertions
    pub failed_count: usize,
    /// List of errors for failed insertions
    pub errors: Vec<(MemoryId, String)>,
}

/// Batch search result
#[derive(Debug, Clone)]
pub struct BatchSearchResult {
    /// Search results for each query
    pub results: Vec<Vec<SearchResult>>,
    /// Number of successful searches
    pub successful_count: usize,
    /// Number of failed searches
    pub failed_count: usize,
    /// List of errors for failed searches
    pub errors: Vec<(usize, String)>, // (query_index, error_message)
}

/// Vector to be inserted in batch
#[derive(Debug, Clone)]
pub struct VectorInsert {
    pub memory_id: MemoryId,
    pub vector: Vec<f32>,
}

/// Search query for batch search
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub query_vector: Vec<f32>,
    pub k: usize,
}

/// Batch operations manager
pub struct BatchOperations {
    config: BatchConfig,
    store: Arc<Mutex<SecureVectorStore<'static>>>,
}

impl BatchOperations {
    /// Create a new batch operations manager
    pub fn new(store: SecureVectorStore<'static>, config: BatchConfig) -> Self {
        Self {
            config,
            store: Arc::new(Mutex::new(store)),
        }
    }
    
    /// Insert multiple vectors in batch
    pub async fn batch_insert(
        &self,
        vectors: Vec<VectorInsert>,
    ) -> VectorResult<BatchInsertResult> {
        // For now, only support sequential processing to avoid Send trait issues
        self.batch_insert_sequential(vectors).await
    }
    
    /// Sequential batch insertion
    async fn batch_insert_sequential(
        &self,
        vectors: Vec<VectorInsert>,
    ) -> VectorResult<BatchInsertResult> {
        let mut result = BatchInsertResult {
            inserted_count: 0,
            failed_count: 0,
            errors: Vec::new(),
        };
        
        let mut store = self.store.lock();
        
        for vector_insert in vectors {
            match store.add_raw_vector(vector_insert.vector, vector_insert.memory_id).await {
                Ok(_) => result.inserted_count += 1,
                Err(e) => {
                    result.failed_count += 1;
                    result.errors.push((vector_insert.memory_id, e.to_string()));
                }
            }
        }
        
        Ok(result)
    }
    
    /// Search multiple vectors in batch
    pub async fn batch_search(
        &self,
        queries: Vec<SearchQuery>,
    ) -> VectorResult<BatchSearchResult> {
        // For now, only support sequential processing to avoid Send trait issues
        self.batch_search_sequential(queries).await
    }
    
    /// Sequential batch search
    async fn batch_search_sequential(
        &self,
        queries: Vec<SearchQuery>,
    ) -> VectorResult<BatchSearchResult> {
        let mut result = BatchSearchResult {
            results: Vec::new(),
            successful_count: 0,
            failed_count: 0,
            errors: Vec::new(),
        };
        
        let store = self.store.lock();
        
        for (i, query) in queries.into_iter().enumerate() {
            match store.search_raw_vector(&query.query_vector, query.k).await {
                Ok(search_results) => {
                    result.results.push(search_results);
                    result.successful_count += 1;
                }
                Err(e) => {
                    result.results.push(Vec::new());
                    result.failed_count += 1;
                    result.errors.push((i, e.to_string()));
                }
            }
        }
        
        Ok(result)
    }
    
    /// Get configuration
    pub fn config(&self) -> &BatchConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: BatchConfig) {
        self.config = config;
    }
}

/// Builder for batch operations
pub struct BatchOperationsBuilder {
    config: BatchConfig,
}

impl BatchOperationsBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: BatchConfig::default(),
        }
    }
    
    /// Set insert batch size
    pub fn insert_batch_size(mut self, size: usize) -> Self {
        self.config.insert_batch_size = size;
        self
    }
    
    /// Set search batch size
    pub fn search_batch_size(mut self, size: usize) -> Self {
        self.config.search_batch_size = size;
        self
    }
    
    /// Set number of worker threads
    pub fn worker_threads(mut self, threads: usize) -> Self {
        self.config.worker_threads = threads;
        self
    }
    
    /// Enable/disable parallel processing
    pub fn parallel_processing(mut self, enabled: bool) -> Self {
        self.config.parallel_processing = enabled;
        self
    }
    
    /// Set timeout
    pub fn timeout_seconds(mut self, timeout: u64) -> Self {
        self.config.timeout_seconds = timeout;
        self
    }
    
    /// Build batch operations
    pub fn build(self, store: SecureVectorStore<'static>) -> BatchOperations {
        BatchOperations::new(store, self.config)
    }
}

impl Default for BatchOperationsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mimir_core::test_utils::generators::generate_test_embedding;
    use uuid::Uuid;
    
    #[tokio::test]
    async fn test_batch_insert_sequential() {
        let store = SecureVectorStore::new(128).unwrap();
        let batch_ops = BatchOperationsBuilder::new()
            .parallel_processing(false)
            .build(store);
        
        let vectors = vec![
            VectorInsert {
                memory_id: Uuid::new_v4(),
                vector: generate_test_embedding(128),
            },
            VectorInsert {
                memory_id: Uuid::new_v4(),
                vector: generate_test_embedding(128),
            },
        ];
        
        let result = batch_ops.batch_insert(vectors).await.unwrap();
        assert_eq!(result.inserted_count, 2);
        assert_eq!(result.failed_count, 0);
    }
    
    #[tokio::test]
    async fn test_batch_search_sequential() {
        let mut store = SecureVectorStore::new(128).unwrap();
        
        // Add some test vectors
        let memory_id = Uuid::new_v4();
        let vector = generate_test_embedding(128);
        store.add_raw_vector(vector.clone(), memory_id).await.unwrap();
        
        let batch_ops = BatchOperationsBuilder::new()
            .parallel_processing(false)
            .build(store);
        
        let queries = vec![
            SearchQuery {
                query_vector: vector,
                k: 5,
            },
        ];
        
        let result = batch_ops.batch_search(queries).await.unwrap();
        assert_eq!(result.successful_count, 1);
        assert_eq!(result.failed_count, 0);
        assert_eq!(result.results.len(), 1);
    }
    
    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.insert_batch_size, 1000);
        assert_eq!(config.search_batch_size, 100);
        assert!(config.parallel_processing);
    }
    
    #[test]
    fn test_batch_operations_builder() {
        let batch_ops = BatchOperationsBuilder::new()
            .insert_batch_size(500)
            .search_batch_size(50)
            .worker_threads(4)
            .parallel_processing(false)
            .timeout_seconds(60)
            .build(SecureVectorStore::new(128).unwrap());
        
        let config = batch_ops.config();
        assert_eq!(config.insert_batch_size, 500);
        assert_eq!(config.search_batch_size, 50);
        assert_eq!(config.worker_threads, 4);
        assert!(!config.parallel_processing);
        assert_eq!(config.timeout_seconds, 60);
    }
} 