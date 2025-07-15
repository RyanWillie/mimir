//! Integrated storage manager for coordinating database and vector store operations

use mimir_core::{crypto::CryptoManager, Memory, MemoryClass, MemoryId, Result};
use mimir_db::Database;
use mimir_vector::ThreadSafeVectorStore;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

/// Integrated storage manager that coordinates database and vector store operations
pub struct IntegratedStorage {
    database: Arc<Mutex<Database>>,
    vector_store: Arc<ThreadSafeVectorStore>,
    crypto_manager: Arc<CryptoManager>,
    llm_service: Option<Arc<super::llm_service::LlmService>>,
}

/// Search result with full memory data
#[derive(Debug, Clone)]
pub struct MemorySearchResult {
    pub memory: Memory,
    pub similarity: f32,
    pub distance: f32,
}

/// Memory addition result
#[derive(Debug, Clone)]
pub struct MemoryAddResult {
    pub memory_id: MemoryId,
    pub vector_stored: bool,
    pub database_stored: bool,
}

impl IntegratedStorage {
    /// Create a new integrated storage manager
    pub async fn new(
        database: Database,
        vector_store: ThreadSafeVectorStore,
        crypto_manager: CryptoManager,
    ) -> Result<Self> {
        Ok(Self {
            database: Arc::new(Mutex::new(database)),
            vector_store: Arc::new(vector_store),
            crypto_manager: Arc::new(crypto_manager),
            llm_service: None,
        })
    }

    /// Set the LLM service for memory processing
    pub fn with_llm_service(mut self, llm_service: Arc<super::llm_service::LlmService>) -> Self {
        self.llm_service = Some(llm_service);
        self
    }

    /// Add a memory to both database and vector store
    pub async fn add_memory(&self, memory: Memory) -> Result<MemoryAddResult> {
        info!("Adding memory to integrated storage: {}", memory.id);

        let mut result = MemoryAddResult {
            memory_id: memory.id,
            vector_stored: false,
            database_stored: false,
        };

        // Step 1: Store in database first
        let db_result = {
            let mut db = self.database.lock().await;
            db.store_memory(&memory).await
        };

        match db_result {
            Ok(_) => {
                result.database_stored = true;
                info!("Memory stored in database: {}", memory.id);
            }
            Err(e) => {
                error!("Failed to store memory in database: {}", e);
                return Err(e);
            }
        }

        // Step 2: Generate embedding and store in vector store
        let vector_result = self.add_memory_to_vector_store(&memory).await;
        match vector_result {
            Ok(_) => {
                result.vector_stored = true;
                info!("Memory stored in vector store: {}", memory.id);
            }
            Err(e) => {
                warn!("Failed to store memory in vector store: {}", e);
                // Don't fail the entire operation if vector store fails
                // The memory is still stored in the database
            }
        }

        Ok(result)
    }

    /// Add multiple memories in batch
    pub async fn add_memories(&self, memories: Vec<Memory>) -> Result<Vec<MemoryAddResult>> {
        info!("Adding {} memories to integrated storage", memories.len());

        let mut results = Vec::new();

        for memory in memories {
            let result = self.add_memory(memory).await?;
            results.push(result);
        }

        info!("Successfully added {} memories", results.len());
        Ok(results)
    }

    /// Search memories using vector similarity
    pub async fn search_memories(&self, query: &str, k: usize) -> Result<Vec<MemorySearchResult>> {
        info!("Searching memories with query: '{}' (k={})", query, k);

        // Step 1: Search vector store
        let vector_results = self
            .vector_store
            .search_text(query, k)
            .await
            .map_err(|e| mimir_core::MimirError::VectorStore(e.to_string()))?;

        info!("Found {} vector results", vector_results.len());

        // Step 2: Retrieve full memories from database
        let mut search_results = Vec::new();

        for result in vector_results {
            let memory_result = {
                let mut db = self.database.lock().await;
                db.get_memory(result.id).await
            };

            match memory_result {
                Ok(Some(memory)) => {
                    let distance = 1.0 - result.similarity; // Convert similarity to distance
                    search_results.push(MemorySearchResult {
                        memory,
                        similarity: result.similarity,
                        distance,
                    });
                }
                Ok(None) => {
                    warn!(
                        "Memory {} found in vector store but not in database",
                        result.id
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to retrieve memory {} from database: {}",
                        result.id, e
                    );
                }
            }
        }

        // Sort by similarity (descending)
        search_results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        info!("Returning {} search results", search_results.len());
        Ok(search_results)
    }

    /// Get memory by ID
    pub async fn get_memory(&self, memory_id: MemoryId) -> Result<Option<Memory>> {
        let mut db = self.database.lock().await;
        db.get_memory(memory_id).await
    }

    /// Delete memory from both storage systems
    pub async fn delete_memory(&self, memory_id: MemoryId) -> Result<bool> {
        info!("Deleting memory: {}", memory_id);

        // Delete from database first
        {
            let db = self.database.lock().await;
            db.delete_memory(memory_id).await?;
        }

        // Delete from vector store
        let vector_result = self.vector_store.remove_vector(memory_id).await;
        match vector_result {
            Ok(_) => {
                info!("Memory deleted from vector store: {}", memory_id);
            }
            Err(e) => {
                warn!("Failed to delete memory from vector store: {}", e);
                // Don't fail if vector store deletion fails
            }
        }

        Ok(true)
    }

    /// Get memories by class
    pub async fn get_memories_by_class(&self, class: &MemoryClass) -> Result<Vec<Memory>> {
        let mut db = self.database.lock().await;
        db.get_memories_by_class(class).await
    }

    /// Get last N memories for a user
    pub async fn get_last_memories(&self, source: &str, limit: usize) -> Result<Vec<Memory>> {
        let mut db = self.database.lock().await;
        db.get_last_memories(source, limit).await
    }

    /// Update an existing memory in both database and vector store
    pub async fn update_memory(&self, memory: Memory) -> Result<MemoryAddResult> {
        info!("Updating memory in integrated storage: {}", memory.id);

        let mut result = MemoryAddResult {
            memory_id: memory.id,
            vector_stored: false,
            database_stored: false,
        };

        // Step 1: Update in database first
        let db_result = {
            let mut db = self.database.lock().await;
            db.update_memory(&memory).await
        };

        match db_result {
            Ok(_) => {
                result.database_stored = true;
                info!("Memory updated in database: {}", memory.id);
            }
            Err(e) => {
                error!("Failed to update memory in database: {}", e);
                return Err(e);
            }
        }

        // Step 2: Update in vector store (remove old, add new)
        let vector_result = self.update_memory_in_vector_store(&memory).await;
        match vector_result {
            Ok(_) => {
                result.vector_stored = true;
                info!("Memory updated in vector store: {}", memory.id);
            }
            Err(e) => {
                warn!("Failed to update memory in vector store: {}", e);
                // Don't fail the entire operation if vector store fails
                // The memory is still updated in the database
            }
        }

        Ok(result)
    }

    /// Clear all memories from both storage systems
    pub async fn clear_vault(&self) -> Result<usize> {
        info!("Clearing all memories from vault");

        // Step 1: Clear database
        let db_count = {
            let mut db = self.database.lock().await;
            db.clear_all_memories().await
        };

        let deleted_count = match db_count {
            Ok(count) => {
                info!("Cleared {} memories from database", count);
                count
            }
            Err(e) => {
                error!("Failed to clear database: {}", e);
                return Err(e);
            }
        };

        // Step 2: Clear vector store (not implemented - vector store will be out of sync)
        // TODO: Implement clear_all_vectors in vector store or get all IDs and remove individually
        warn!("Vector store not cleared - will be out of sync with database");

        Ok(deleted_count)
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> Result<StorageStats> {
        let db_count = {
            // Note: We don't have a direct count method in Database yet
            // For now, we'll estimate based on vector store count
            self.vector_store.len().await
        };

        let vector_count = self.vector_store.len().await;
        let vector_stats = self.vector_store.get_memory_stats();

        Ok(StorageStats {
            database_memories: db_count,
            vector_memories: vector_count,
            memory_usage_bytes: vector_stats.memory_bytes,
            vector_count_percentage: self.vector_store.get_vector_count_percentage(),
        })
    }

    /// Add memory to vector store (internal method)
    async fn add_memory_to_vector_store(&self, memory: &Memory) -> Result<()> {
        // Check if vector store has embedder
        if !self.vector_store.has_embedder().await {
            return Err(mimir_core::MimirError::VectorStore(
                "Vector store does not have an embedder configured".to_string(),
            ));
        }

        // Add text to vector store (this will generate embedding internally)
        self.vector_store
            .add_text(memory.id, &memory.content)
            .await
            .map_err(|e| mimir_core::MimirError::VectorStore(e.to_string()))?;

        Ok(())
    }

    /// Update memory in vector store (internal method)
    async fn update_memory_in_vector_store(&self, memory: &Memory) -> Result<()> {
        // Check if vector store has embedder
        if !self.vector_store.has_embedder().await {
            return Err(mimir_core::MimirError::VectorStore(
                "Vector store does not have an embedder configured".to_string(),
            ));
        }

        // Remove old vector first
        let _ = self.vector_store.remove_vector(memory.id).await;

        // Add new text to vector store (this will generate embedding internally)
        self.vector_store
            .add_text(memory.id, &memory.content)
            .await
            .map_err(|e| mimir_core::MimirError::VectorStore(e.to_string()))?;

        Ok(())
    }

    pub async fn has_vector_embedder(&self) -> bool {
        self.vector_store.has_embedder().await
    }

    /// Save the vector store to disk
    pub async fn save_vector_store(&self) -> Result<()> {
        info!("Saving vector store to disk");
        let vector_count = self.vector_store.len().await;
        info!("Vector store has {} vectors to save", vector_count);

        let result = self.vector_store.save(None).await;
        match result {
            Ok(_) => {
                info!("Vector store saved successfully to disk");
                Ok(())
            }
            Err(e) => {
                error!("Failed to save vector store: {}", e);
                Err(mimir_core::MimirError::VectorStore(format!(
                    "Failed to save vector store: {}",
                    e
                )))
            }
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub database_memories: usize,
    pub vector_memories: usize,
    pub memory_usage_bytes: usize,
    pub vector_count_percentage: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mimir_core::test_utils::MemoryBuilder;
    use mimir_core::MemoryClass;
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn create_test_storage() -> (IntegratedStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let keyset_path = temp_dir.path().join("keyset.json");

        // Create crypto manager for database
        let db_crypto_manager =
            mimir_core::crypto::CryptoManager::with_password(&keyset_path, "test-password")
                .expect("Failed to create test crypto manager");

        // Create crypto manager for integrated storage
        let storage_crypto_manager =
            mimir_core::crypto::CryptoManager::with_password(&keyset_path, "test-password")
                .expect("Failed to create test crypto manager");

        // Create database
        let database = Database::with_crypto_manager(&db_path, db_crypto_manager)
            .expect("Failed to create test database");

        // Create vector store (without embedder for testing)
        let vector_store = ThreadSafeVectorStore::new(temp_dir.path(), 128, None, None)
            .expect("Failed to create test vector store");

        let storage = IntegratedStorage::new(database, vector_store, storage_crypto_manager)
            .await
            .expect("Failed to create integrated storage");

        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_add_memory() {
        let (storage, _temp_dir) = create_test_storage().await;

        let memory = MemoryBuilder::new()
            .with_content("Test memory content")
            .with_class(MemoryClass::Personal)
            .build();
        let result = storage.add_memory(memory.clone()).await.unwrap();

        assert_eq!(result.memory_id, memory.id);
        assert!(result.database_stored);
        // Vector store won't work without embedder, so this should be false
        assert!(!result.vector_stored);
    }

    #[tokio::test]
    async fn test_get_memory() {
        let (storage, _temp_dir) = create_test_storage().await;

        let memory = MemoryBuilder::new()
            .with_content("Test memory content")
            .with_class(MemoryClass::Personal)
            .build();
        storage.add_memory(memory.clone()).await.unwrap();

        let retrieved = storage.get_memory(memory.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, memory.id);
    }

    #[tokio::test]
    async fn test_delete_memory() {
        let (storage, _temp_dir) = create_test_storage().await;

        let memory = MemoryBuilder::new()
            .with_content("Test memory content")
            .with_class(MemoryClass::Personal)
            .build();
        storage.add_memory(memory.clone()).await.unwrap();

        let deleted = storage.delete_memory(memory.id).await.unwrap();
        assert!(deleted);

        let retrieved = storage.get_memory(memory.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_get_memories_by_class() {
        let (storage, _temp_dir) = create_test_storage().await;

        let memory1 = MemoryBuilder::new()
            .with_content("Test personal memory")
            .with_class(MemoryClass::Personal)
            .build();
        let memory2 = MemoryBuilder::new()
            .with_content("Test work memory")
            .with_class(MemoryClass::Work)
            .build();

        storage.add_memory(memory1.clone()).await.unwrap();
        storage.add_memory(memory2.clone()).await.unwrap();

        let personal_memories = storage
            .get_memories_by_class(&MemoryClass::Personal)
            .await
            .unwrap();
        assert_eq!(personal_memories.len(), 1);
        assert_eq!(personal_memories[0].id, memory1.id);

        let work_memories = storage
            .get_memories_by_class(&MemoryClass::Work)
            .await
            .unwrap();
        assert_eq!(work_memories.len(), 1);
        assert_eq!(work_memories[0].id, memory2.id);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let (storage, _temp_dir) = create_test_storage().await;

        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.database_memories, 0);
        assert_eq!(stats.vector_memories, 0);
    }
}
