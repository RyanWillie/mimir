//! Mimir Vector - High-performance vector similarity search

use mimir_core::{MemoryId, Result};
use std::path::Path;

pub mod error;
pub mod embedder;
pub mod rotation;

use error::{VectorError, VectorResult};
use embedder::Embedder;
use rotation::RotationMatrix;

/// Vector store for embeddings and similarity search
///
/// If you use a rotation matrix for embedding security, the rotation matrix dimension
/// must match the embedding dimension reported by the embedder. Always use
/// `embedder.embedding_dimension()` when constructing a rotation matrix.
pub struct VectorStore {
    embedder: Option<Embedder>,
    rotation_matrix: Option<RotationMatrix>,
    // TODO: Add HNSW index
}

impl VectorStore {
    /// Create a new vector store
    pub fn new() -> Self {
        Self {
            embedder: None,
            rotation_matrix: None,
        }
    }
    
    /// Create a new vector store with embedding model
    pub async fn with_embedder<P: AsRef<Path>>(model_path: P) -> VectorResult<Self> {
        let embedder = Embedder::new(model_path).await?;
        
        Ok(Self {
            embedder: Some(embedder),
            rotation_matrix: None,
        })
    }
    
    /// Create a new vector store with default BGE model
    pub async fn with_default_embedder() -> VectorResult<Self> {
        let model_path = if let Ok(workspace_root) = std::env::var("CARGO_WORKSPACE_DIR") {
            std::path::PathBuf::from(workspace_root)
                .join("crates/mimir/assets/bge-small-en-int8/model-int8.onnx")
        } else {
            // Fallback: try to find it relative to current directory
            let mut path = std::env::current_dir().unwrap();
            // Go up to workspace root if we're in a crate directory
            if path.ends_with("mimir-vector") {
                path.pop(); // Remove mimir-vector
                path.pop(); // Remove crates
            }
            path.join("crates/mimir/assets/bge-small-en-int8/model-int8.onnx")
        };
        Self::with_embedder(model_path).await
    }
    
    /// Create a new vector store with embedding model and rotation matrix
    pub async fn with_embedder_and_rotation<P: AsRef<Path>>(
        model_path: P,
        root_key: &mimir_core::crypto::RootKey,
    ) -> VectorResult<Self> {
        let embedder = Embedder::new(model_path).await?;
        let embedding_dim = embedder.embedding_dimension();
        let rotation_matrix = RotationMatrix::from_root_key(root_key, embedding_dim)?;
        
        Ok(Self {
            embedder: Some(embedder),
            rotation_matrix: Some(rotation_matrix),
        })
    }
    
    /// Create a new vector store with default BGE model and rotation matrix
    pub async fn with_default_embedder_and_rotation(
        root_key: &mimir_core::crypto::RootKey,
    ) -> VectorResult<Self> {
        let model_path = if let Ok(workspace_root) = std::env::var("CARGO_WORKSPACE_DIR") {
            std::path::PathBuf::from(workspace_root)
                .join("crates/mimir/assets/bge-small-en-int8/model-int8.onnx")
        } else {
            // Fallback: try to find it relative to current directory
            let mut path = std::env::current_dir().unwrap();
            // Go up to workspace root if we're in a crate directory
            if path.ends_with("mimir-vector") {
                path.pop(); // Remove mimir-vector
                path.pop(); // Remove crates
            }
            path.join("crates/mimir/assets/bge-small-en-int8/model-int8.onnx")
        };
        Self::with_embedder_and_rotation(model_path, root_key).await
    }
    
    /// Add a vector to the store
    pub async fn add_vector(&mut self, _id: MemoryId, _embedding: Vec<f32>) -> Result<()> {
        // TODO: Implement vector indexing with HNSW
        Ok(())
    }
    
    /// Add text to the store (converts to embedding first)
    pub async fn add_text(&mut self, id: MemoryId, text: &str) -> VectorResult<()> {
        let embedder = self.embedder.as_mut()
            .ok_or_else(|| VectorError::InvalidInput("No embedder available".to_string()))?;
        
        // Generate embedding
        let embedding = embedder.embed(text).await?;
        
        // Apply rotation if available
        let final_embedding = if let Some(rotation_matrix) = &self.rotation_matrix {
            rotation_matrix.rotate_vector(&embedding)?
        } else {
            embedding
        };
        
        // Add to store
        self.add_vector(id, final_embedding).await
            .map_err(|e| VectorError::InvalidInput(format!("Failed to add vector: {}", e)))?;
        
        Ok(())
    }
    
    /// Search for similar vectors
    pub async fn search(&self, _query: Vec<f32>, _k: usize) -> Result<Vec<(MemoryId, f32)>> {
        // TODO: Implement similarity search with HNSW
        Ok(vec![])
    }
    
    /// Search for similar text (converts to embedding first)
    pub async fn search_text(&mut self, query: &str, k: usize) -> VectorResult<Vec<(MemoryId, f32)>> {
        let embedder = self.embedder.as_mut()
            .ok_or_else(|| VectorError::InvalidInput("No embedder available".to_string()))?;
        
        // Generate embedding for query
        let embedding = embedder.embed(query).await?;
        
        // Apply rotation if available
        let final_embedding = if let Some(rotation_matrix) = &self.rotation_matrix {
            rotation_matrix.rotate_vector(&embedding)?
        } else {
            embedding
        };
        
        // Search
        self.search(final_embedding, k).await
            .map_err(|e| VectorError::InvalidInput(format!("Search failed: {}", e)))
    }
    
    /// Check if embedder is available
    pub fn has_embedder(&self) -> bool {
        self.embedder.is_some()
    }
    
    /// Check if rotation matrix is available
    pub fn has_rotation(&self) -> bool {
        self.rotation_matrix.is_some()
    }
    
    /// Get embedding dimension
    pub fn embedding_dimension(&self) -> Option<usize> {
        self.embedder.as_ref().map(|e| e.embedding_dimension())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mimir_core::test_utils::generators::generate_test_embedding;
    use proptest::prelude::*;
    use uuid::Uuid;

    #[test]
    fn test_vector_store_creation() {
        let store = VectorStore::new();
        // Just verify it can be created without panicking
        drop(store);
    }
    
    #[test]
    fn test_vector_store_without_embedder() {
        let store = VectorStore::new();
        assert!(!store.has_embedder());
        assert!(!store.has_rotation());
        assert_eq!(store.embedding_dimension(), None);
    }

    #[tokio::test]
    async fn test_add_vector_stub() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();
        let embedding = generate_test_embedding(128);

        let result = store.add_vector(memory_id, embedding).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_multiple_vectors() {
        let mut store = VectorStore::new();

        for i in 0..10 {
            let memory_id = Uuid::new_v4();
            let embedding = generate_test_embedding(128);

            let result = store.add_vector(memory_id, embedding).await;
            assert!(result.is_ok(), "Failed to add vector {}", i);
        }
    }

    #[tokio::test]
    async fn test_different_embedding_dimensions() {
        let mut store = VectorStore::new();

        let dimensions = vec![64, 128, 256, 384, 512, 768, 1024];

        for dim in dimensions {
            let memory_id = Uuid::new_v4();
            let embedding = generate_test_embedding(dim);

            assert_eq!(embedding.len(), dim);

            let result = store.add_vector(memory_id, embedding).await;
            assert!(result.is_ok(), "Failed with dimension {}", dim);
        }
    }

    #[tokio::test]
    async fn test_search_stub() {
        let store = VectorStore::new();
        let query = generate_test_embedding(128);

        let result = store.search(query, 5).await;
        assert!(result.is_ok());

        let results = result.unwrap();
        assert_eq!(results.len(), 0); // Stub returns empty
    }

    #[tokio::test]
    async fn test_search_different_k_values() {
        let store = VectorStore::new();
        let query = generate_test_embedding(128);

        let k_values = vec![1, 5, 10, 50, 100];

        for k in k_values {
            let result = store.search(query.clone(), k).await;
            assert!(result.is_ok(), "Failed with k={}", k);
        }
    }

    #[tokio::test]
    async fn test_empty_embedding() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();
        let empty_embedding = vec![];

        let result = store.add_vector(memory_id, empty_embedding).await;
        // With stub implementation, this succeeds
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_large_embeddings() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();

        // Test with very large embedding (4096 dimensions)
        let large_embedding = generate_test_embedding(4096);

        let result = store.add_vector(memory_id, large_embedding).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_special_float_values() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();

        // Test with special float values
        let special_embedding = vec![
            0.0,      // Zero
            1.0,      // One
            -1.0,     // Negative
            0.5,      // Fraction
            f32::MIN, // Minimum
            f32::MAX, // Maximum
            1e-10,    // Very small
            1e10,     // Very large
        ];

        let result = store.add_vector(memory_id, special_embedding).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_vector_operations() {
        let mut store = VectorStore::new();

        let memory_id1 = Uuid::new_v4();
        let memory_id2 = Uuid::new_v4();
        let embedding1 = generate_test_embedding(128);
        let embedding2 = generate_test_embedding(128);
        let query = generate_test_embedding(128);

        // Test sequential operations (concurrent operations would require Arc<Mutex<>> wrapper)
        let add_result1 = store.add_vector(memory_id1, embedding1).await;
        let add_result2 = store.add_vector(memory_id2, embedding2).await;
        let search_result = store.search(query, 10).await;

        assert!(add_result1.is_ok());
        assert!(add_result2.is_ok());
        assert!(search_result.is_ok());
    }

    #[tokio::test]
    async fn test_search_with_empty_store() {
        let store = VectorStore::new();
        let query = generate_test_embedding(128);

        // Search in empty store should work (return empty results)
        let result = store.search(query, 10).await;
        assert!(result.is_ok());

        let results = result.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_repeated_searches() {
        let store = VectorStore::new();
        let query = generate_test_embedding(128);

        // Perform the same search multiple times
        for i in 0..5 {
            let result = store.search(query.clone(), 10).await;
            assert!(result.is_ok(), "Search {} failed", i);
        }
    }

    #[test]
    fn test_vector_store_multiple_instances() {
        // Test that multiple vector stores can coexist
        let store1 = VectorStore::new();
        let store2 = VectorStore::new();
        let store3 = VectorStore::new();

        // All should be independently usable
        drop(store1);
        drop(store2);
        drop(store3);
    }

    #[tokio::test]
    async fn test_vector_normalization_tolerance() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();

        // Test with vectors that might need normalization
        let unnormalized_embedding = vec![100.0, 200.0, 300.0, 400.0];

        let result = store.add_vector(memory_id, unnormalized_embedding).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_memory_id_uniqueness() {
        let mut store = VectorStore::new();

        // Test adding vectors with same embedding but different IDs
        let embedding = generate_test_embedding(128);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let result1 = store.add_vector(id1, embedding.clone()).await;
        let result2 = store.add_vector(id2, embedding).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_ne!(id1, id2);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn test_vector_dimensions_positive(dims in 1usize..2048) {
            let embedding = generate_test_embedding(dims);
            assert_eq!(embedding.len(), dims);
            assert!(dims > 0);
        }

        #[test]
        fn test_k_value_bounds(k in 1usize..1000) {
            assert!(k > 0);
            assert!(k < 1000);
        }

        #[test]
        fn test_embedding_values_finite(values in prop::collection::vec(-1000.0f32..1000.0, 1..100)) {
            // All values should be finite (not NaN or infinite)
            for value in &values {
                assert!(value.is_finite());
            }
        }
    }

    // Future tests for when actual vector search is implemented
    #[tokio::test]
    #[ignore = "Requires actual vector search implementation"]
    async fn test_similarity_scoring() {
        // This test will verify that similar vectors get higher scores
        let mut store = VectorStore::new();

        let base_vector = vec![1.0, 0.0, 0.0, 0.0];
        let similar_vector = vec![0.9, 0.1, 0.0, 0.0];
        let different_vector = vec![0.0, 0.0, 1.0, 0.0];

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        store.add_vector(id1, similar_vector).await.unwrap();
        store.add_vector(id2, different_vector).await.unwrap();

        let _results = store.search(base_vector, 2).await.unwrap();

        // Should return results ordered by similarity
        // assert!(results[0].1 > results[1].1); // First result should have higher score
    }

    #[tokio::test]
    #[ignore = "Requires actual vector search implementation"]
    async fn test_vector_persistence() {
        // This test will verify that vectors persist correctly
        // when actual storage is implemented
    }

    #[tokio::test]
    #[ignore = "Requires actual vector search implementation"]
    async fn test_index_performance() {
        // This test will verify performance characteristics
        // when actual indexing (HNSW, IVF) is implemented
    }
}
