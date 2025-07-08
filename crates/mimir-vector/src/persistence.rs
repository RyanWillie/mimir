//! Persistence layer for HNSW vector store

use crate::error::{VectorError, VectorResult};
use crate::hnsw_store::SecureVectorStore;
use mimir_core::{crypto::RootKey, MemoryId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::{Read, Write};

/// Metadata for the vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreMetadata {
    /// Version of the metadata format
    pub version: u32,
    /// Dimension of vectors in the store
    pub dimension: usize,
    /// Number of vectors in the store
    pub vector_count: usize,
    /// Next internal ID to use
    pub next_id: usize,
    /// HNSW parameters
    pub hnsw_params: HnswParams,
    /// Whether rotation matrix is enabled
    pub has_rotation: bool,
    /// Whether embedder is configured
    pub has_embedder: bool,
    /// Timestamp of last save
    pub last_saved: chrono::DateTime<chrono::Utc>,
}

/// HNSW algorithm parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswParams {
    pub max_connections: usize,
    pub max_elements: usize,
    pub max_layer: usize,
    pub ef_construction: usize,
}

/// Vector store persistence manager
#[derive(Clone)]
pub struct VectorStorePersistence {
    vault_path: PathBuf,
    metadata_path: PathBuf,
    index_path: PathBuf,
    vectors_path: PathBuf,
}

impl VectorStorePersistence {
    /// Create a new persistence manager
    pub fn new<P: AsRef<Path>>(vault_path: P) -> Self {
        let vault_path = vault_path.as_ref().to_path_buf();
        let metadata_path = vault_path.join("vector_store_metadata.bin");
        let index_path = vault_path.join("vector_store_index.bin");
        let vectors_path = vault_path.join("vector_store_vectors.bin");
        
        Self {
            vault_path,
            metadata_path,
            index_path,
            vectors_path,
        }
    }
    
    /// Save the vector store to disk
    pub async fn save_store(
        &self,
        store: &SecureVectorStore<'static>,
        root_key: Option<&RootKey>,
    ) -> VectorResult<()> {
        // Ensure vault directory exists
        fs::create_dir_all(&self.vault_path)
            .map_err(|e| VectorError::Persistence(format!("Failed to create vault directory: {}", e)))?;
        
        // Create metadata
        let metadata = VectorStoreMetadata {
            version: 1,
            dimension: store.dimension(),
            vector_count: store.len(),
            next_id: store.next_id(),
            hnsw_params: HnswParams {
                max_connections: 32, // Default values - could be made configurable
                max_elements: 10000,
                max_layer: 16,
                ef_construction: 32,
            },
            has_rotation: store.has_rotation(),
            has_embedder: store.has_embedder(),
            last_saved: chrono::Utc::now(),
        };
        
        // Save metadata
        self.save_metadata(&metadata)?;
        
        // Save HNSW index and vectors
        self.save_index_and_vectors(store, root_key).await?;
        
        Ok(())
    }
    
    /// Load the vector store from disk
    pub async fn load_store(
        &self,
        root_key: Option<&RootKey>,
    ) -> VectorResult<Option<SecureVectorStore<'static>>> {
        // Check if metadata exists
        if !self.metadata_path.exists() {
            return Ok(None); // No saved store
        }
        
        // Load metadata
        let metadata = self.load_metadata()?;
        
        // Validate metadata
        if metadata.version != 1 {
            return Err(VectorError::Persistence(format!(
                "Unsupported metadata version: {}",
                metadata.version
            )));
        }
        
        // Load index and vectors
        let store = self.load_index_and_vectors(&metadata, root_key).await?;
        
        Ok(Some(store))
    }
    
    /// Save metadata to disk
    fn save_metadata(&self, metadata: &VectorStoreMetadata) -> VectorResult<()> {
        let data = bincode::serialize(metadata)
            .map_err(|e| VectorError::Serialization(format!("Failed to serialize metadata: {}", e)))?;
        
        let mut file = fs::File::create(&self.metadata_path)
            .map_err(|e| VectorError::Io(e))?;
        
        file.write_all(&data)
            .map_err(|e| VectorError::Io(e))?;
        
        Ok(())
    }
    
    /// Load metadata from disk
    fn load_metadata(&self) -> VectorResult<VectorStoreMetadata> {
        let mut file = fs::File::open(&self.metadata_path)
            .map_err(|e| VectorError::Io(e))?;
        
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| VectorError::Io(e))?;
        
        let metadata: VectorStoreMetadata = bincode::deserialize(&data)
            .map_err(|e| VectorError::Serialization(format!("Failed to deserialize metadata: {}", e)))?;
        
        Ok(metadata)
    }
    
    /// Save HNSW index and vectors
    async fn save_index_and_vectors(
        &self,
        store: &SecureVectorStore<'static>,
        _root_key: Option<&RootKey>,
    ) -> VectorResult<()> {
        // For now, we'll save a simplified representation
        // In a full implementation, we'd serialize the actual HNSW index
        
        // Save vector data and ID mappings
        let vector_data = store.get_vector_data_for_persistence()?;
        let data = bincode::serialize(&vector_data)
            .map_err(|e| VectorError::Serialization(format!("Failed to serialize vector data: {}", e)))?;
        
        let mut file = fs::File::create(&self.vectors_path)
            .map_err(|e| VectorError::Io(e))?;
        
        file.write_all(&data)
            .map_err(|e| VectorError::Io(e))?;
        
        // Save index structure (placeholder for now)
        let index_data = b"placeholder_index_data";
        let mut index_file = fs::File::create(&self.index_path)
            .map_err(|e| VectorError::Io(e))?;
        
        index_file.write_all(index_data)
            .map_err(|e| VectorError::Io(e))?;
        
        Ok(())
    }
    
    /// Load HNSW index and vectors
    async fn load_index_and_vectors(
        &self,
        metadata: &VectorStoreMetadata,
        _root_key: Option<&RootKey>,
    ) -> VectorResult<SecureVectorStore<'static>> {
        // Load vector data
        let mut file = fs::File::open(&self.vectors_path)
            .map_err(|e| VectorError::Io(e))?;
        
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| VectorError::Io(e))?;
        
        let vector_data: VectorDataForPersistence = bincode::deserialize(&data)
            .map_err(|e| VectorError::Serialization(format!("Failed to deserialize vector data: {}", e)))?;
        
        // Recreate store from loaded data
        let mut store = SecureVectorStore::new(metadata.dimension)?;
        store.restore_from_persistence_data(vector_data, metadata.next_id)?;
        
        Ok(store)
    }
    
    /// Check if a saved store exists
    pub fn store_exists(&self) -> bool {
        self.metadata_path.exists() && self.vectors_path.exists()
    }
    
    /// Get the path to the vault directory
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }
    
    /// Delete the saved store
    pub fn delete_store(&self) -> VectorResult<()> {
        if self.metadata_path.exists() {
            fs::remove_file(&self.metadata_path)
                .map_err(|e| VectorError::Io(e))?;
        }
        
        if self.index_path.exists() {
            fs::remove_file(&self.index_path)
                .map_err(|e| VectorError::Io(e))?;
        }
        
        if self.vectors_path.exists() {
            fs::remove_file(&self.vectors_path)
                .map_err(|e| VectorError::Io(e))?;
        }
        
        Ok(())
    }
}

/// Data structure for persisting vector store data
#[derive(Debug, Serialize, Deserialize)]
pub struct VectorDataForPersistence {
    pub vectors: Vec<(usize, Vec<f32>)>, // (internal_id, vector)
    pub id_mapping: HashMap<usize, MemoryId>, // internal_id -> MemoryId
    pub reverse_mapping: HashMap<MemoryId, usize>, // MemoryId -> internal_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_persistence_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = VectorStorePersistence::new(temp_dir.path());
        
        assert_eq!(persistence.vault_path(), temp_dir.path());
        assert!(!persistence.store_exists());
    }
    
    #[test]
    fn test_metadata_serialization() {
        let metadata = VectorStoreMetadata {
            version: 1,
            dimension: 768,
            vector_count: 100,
            next_id: 100,
            hnsw_params: HnswParams {
                max_connections: 32,
                max_elements: 10000,
                max_layer: 16,
                ef_construction: 32,
            },
            has_rotation: false,
            has_embedder: true,
            last_saved: chrono::Utc::now(),
        };
        
        let temp_dir = TempDir::new().unwrap();
        let persistence = VectorStorePersistence::new(temp_dir.path());
        
        // Save metadata
        persistence.save_metadata(&metadata).unwrap();
        
        // Load metadata
        let loaded = persistence.load_metadata().unwrap();
        
        assert_eq!(loaded.version, metadata.version);
        assert_eq!(loaded.dimension, metadata.dimension);
        assert_eq!(loaded.vector_count, metadata.vector_count);
    }
    
    #[test]
    fn test_store_deletion() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = VectorStorePersistence::new(temp_dir.path());
        
        // Create some dummy files
        fs::write(&persistence.metadata_path, b"dummy").unwrap();
        fs::write(&persistence.vectors_path, b"dummy").unwrap();
        
        assert!(persistence.store_exists());
        
        // Delete store
        persistence.delete_store().unwrap();
        
        assert!(!persistence.store_exists());
    }
} 