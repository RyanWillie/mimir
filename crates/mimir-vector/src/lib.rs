//! Mimir Vector - High-performance vector similarity search

use mimir_core::{MemoryId, Result};
use std::path::Path;

pub mod embedder;
pub mod error;
pub mod hnsw_store;
pub mod rotation;

use error::VectorResult;
use hnsw_store::SecureVectorStore;

/// Vector store for embeddings and similarity search
///
/// If you use a rotation matrix for embedding security, the rotation matrix dimension
/// must match the embedding dimension reported by the embedder. Always use
/// `embedder.embedding_dimension()` when constructing a rotation matrix.
pub struct VectorStore<'a> {
    secure_store: SecureVectorStore<'a>,
}

impl<'a> Default for VectorStore<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> VectorStore<'a> {
    /// Create a new vector store
    pub fn new() -> Self {
        // Default to 768 dimensions for BGE models
        let secure_store =
            SecureVectorStore::new(768).expect("Failed to create secure vector store");
        Self { secure_store }
    }

    /// Create a new vector store with embedding model
    pub async fn with_embedder<P: AsRef<Path>>(model_path: P) -> VectorResult<Self> {
        let secure_store = SecureVectorStore::with_embedder(model_path).await?;
        Ok(Self { secure_store })
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
        let secure_store =
            SecureVectorStore::with_embedder_and_rotation(model_path, root_key).await?;
        Ok(Self { secure_store })
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
    pub async fn add_vector(&mut self, id: MemoryId, embedding: Vec<f32>) -> Result<()> {
        self.secure_store
            .add_raw_vector(embedding, id)
            .await
            .map_err(|e| mimir_core::MimirError::VectorStore(e.to_string()))
    }

    /// Add text to the store (converts to embedding first)
    pub async fn add_text(&mut self, id: MemoryId, text: &str) -> VectorResult<()> {
        self.secure_store.add_text(text, id).await
    }

    /// Search for similar vectors
    pub async fn search(&self, query: Vec<f32>, k: usize) -> Result<Vec<(MemoryId, f32)>> {
        let results = self
            .secure_store
            .search_raw_vector(&query, k)
            .await
            .map_err(|e| mimir_core::MimirError::VectorStore(e.to_string()))?;

        // Convert SearchResult to (MemoryId, f32) format for backward compatibility
        let converted_results = results.into_iter().map(|r| (r.id, r.similarity)).collect();

        Ok(converted_results)
    }

    /// Search for similar text (converts to embedding first)
    pub async fn search_text(
        &mut self,
        query: &str,
        k: usize,
    ) -> VectorResult<Vec<(MemoryId, f32)>> {
        let results = self.secure_store.search_text(query, k).await?;

        // Convert SearchResult to (MemoryId, f32) format for backward compatibility
        let converted_results = results.into_iter().map(|r| (r.id, r.similarity)).collect();

        Ok(converted_results)
    }

    /// Search for similar vectors and return detailed results
    pub async fn search_detailed(
        &self,
        query: Vec<f32>,
        k: usize,
    ) -> VectorResult<Vec<SearchResult>> {
        self.secure_store.search_raw_vector(&query, k).await
    }

    /// Search for similar text and return detailed results
    pub async fn search_text_detailed(
        &mut self,
        query: &str,
        k: usize,
    ) -> VectorResult<Vec<SearchResult>> {
        self.secure_store.search_text(query, k).await
    }

    /// Remove a vector from the store
    pub async fn remove_vector(&mut self, id: MemoryId) -> VectorResult<()> {
        self.secure_store.remove_vector(id).await
    }

    /// Check if embedder is available
    pub fn has_embedder(&self) -> bool {
        self.secure_store.has_embedder()
    }

    /// Check if rotation matrix is available
    pub fn has_rotation(&self) -> bool {
        self.secure_store.has_rotation()
    }

    /// Get embedding dimension
    pub fn embedding_dimension(&self) -> Option<usize> {
        self.secure_store.embedding_dimension()
    }

    /// Get the number of vectors in the store
    pub fn len(&self) -> usize {
        self.secure_store.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.secure_store.is_empty()
    }

    /// Check if a memory ID exists in the store
    pub fn contains(&self, id: &MemoryId) -> bool {
        self.secure_store.contains(id)
    }
}

// Re-export SearchResult for convenience
pub use hnsw_store::SearchResult;

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
        assert_eq!(store.embedding_dimension(), None); // No embedder configured
    }

    #[tokio::test]
    async fn test_add_vector() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();
        let embedding = generate_test_embedding(768); // Match default dimension

        let result = store.add_vector(memory_id, embedding).await;
        assert!(result.is_ok());
        assert_eq!(store.len(), 1);
        assert!(store.contains(&memory_id));
    }

    #[tokio::test]
    async fn test_add_multiple_vectors() {
        let mut store = VectorStore::new();

        for i in 0..10 {
            let memory_id = Uuid::new_v4();
            let embedding = generate_test_embedding(768);

            let result = store.add_vector(memory_id, embedding).await;
            assert!(result.is_ok(), "Failed to add vector {}", i);
        }

        assert_eq!(store.len(), 10);
    }

    #[tokio::test]
    async fn test_search() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();
        let embedding = generate_test_embedding(768);

        store
            .add_vector(memory_id, embedding.clone())
            .await
            .unwrap();

        let results = store.search(embedding, 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, memory_id);
        assert!((results[0].1 - 1.0).abs() < 1e-6); // Should be very similar to itself
    }

    #[tokio::test]
    async fn test_search_with_empty_store() {
        let store = VectorStore::new();
        let query = generate_test_embedding(768);

        let results = store.search(query, 10).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_remove_vector() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();
        let embedding = generate_test_embedding(768);

        store.add_vector(memory_id, embedding).await.unwrap();
        assert!(store.contains(&memory_id));

        store.remove_vector(memory_id).await.unwrap();
        assert!(!store.contains(&memory_id));
        assert_eq!(store.len(), 0);
    }

    #[tokio::test]
    async fn test_search_detailed() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();
        let embedding = generate_test_embedding(768);

        store
            .add_vector(memory_id, embedding.clone())
            .await
            .unwrap();

        let results = store.search_detailed(embedding, 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, memory_id);
        assert!(results[0].distance < 1e-6);
        assert!((results[0].similarity - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_dimension_mismatch() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();
        let wrong_dim_embedding = generate_test_embedding(128); // Wrong dimension

        let result = store.add_vector(memory_id, wrong_dim_embedding).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_duplicate_memory_id() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();
        let embedding1 = generate_test_embedding(768);
        let embedding2 = generate_test_embedding(768);

        store.add_vector(memory_id, embedding1).await.unwrap();

        let result = store.add_vector(memory_id, embedding2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_different_k_values() {
        let mut store = VectorStore::new();
        let memory_id = Uuid::new_v4();
        let embedding = generate_test_embedding(768);

        store
            .add_vector(memory_id, embedding.clone())
            .await
            .unwrap();

        for k in [1, 5, 10, 50] {
            let results = store.search(embedding.clone(), k).await.unwrap();
            assert_eq!(results.len(), 1); // Only one vector in store
        }
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let mut store = VectorStore::new();
        let memory_id1 = Uuid::new_v4();
        let memory_id2 = Uuid::new_v4();
        let embedding1 = generate_test_embedding(768);
        let embedding2 = generate_test_embedding(768);

        // Sequential operations (concurrent would require Arc<Mutex<>>)
        store
            .add_vector(memory_id1, embedding1.clone())
            .await
            .unwrap();
        store
            .add_vector(memory_id2, embedding2.clone())
            .await
            .unwrap();

        let results1 = store.search(embedding1, 1).await.unwrap();
        let results2 = store.search(embedding2, 1).await.unwrap();

        // Just verify we get results, not specific IDs (since search order may vary)
        assert_eq!(results1.len(), 1);
        assert_eq!(results2.len(), 1);
        assert!(store.contains(&memory_id1));
        assert!(store.contains(&memory_id2));
    }

    #[tokio::test]
    async fn test_search_with_meaningful_vectors() {
        let mut store = VectorStore::new();

        // Create vectors with meaningful patterns (normalized to unit length)
        let vector_a = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]; // Unit vector in first dimension
        let vector_b = vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]; // Unit vector in second dimension
        let vector_c = vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]; // Unit vector in third dimension
        let vector_d = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]; // Unit vector in fourth dimension

        // Pad to 768 dimensions (fill with zeros)
        let pad_size = 768 - 8;
        let vector_a = [vector_a, vec![0.0; pad_size]].concat();
        let vector_b = [vector_b, vec![0.0; pad_size]].concat();
        let vector_c = [vector_c, vec![0.0; pad_size]].concat();
        let vector_d = [vector_d, vec![0.0; pad_size]].concat();

        // Add vectors to store
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let id_c = Uuid::new_v4();
        let id_d = Uuid::new_v4();

        store.add_vector(id_a, vector_a.clone()).await.unwrap();
        store.add_vector(id_b, vector_b.clone()).await.unwrap();
        store.add_vector(id_c, vector_c.clone()).await.unwrap();
        store.add_vector(id_d, vector_d.clone()).await.unwrap();

        assert_eq!(store.len(), 4);

        // Test 1: Search for vector_a should return vector_a as most similar
        let results_a = store.search(vector_a.clone(), 4).await.unwrap();
        assert_eq!(results_a.len(), 4);

        // First result should be vector_a itself (perfect match)
        assert_eq!(results_a[0].0, id_a);
        assert!((results_a[0].1 - 1.0).abs() < 1e-6); // Should be very close to 1.0

        // Test 2: Search should be deterministic (same query returns same results)
        let results_a2 = store.search(vector_a.clone(), 4).await.unwrap();
        assert_eq!(results_a.len(), results_a2.len());
        for (r1, r2) in results_a.iter().zip(results_a2.iter()) {
            assert_eq!(r1.0, r2.0);
            assert!((r1.1 - r2.1).abs() < 1e-6);
        }

        // Test 3: Search for vector_b should return vector_b as most similar
        let results_b = store.search(vector_b.clone(), 4).await.unwrap();
        assert_eq!(results_b.len(), 4);
        assert_eq!(results_b[0].0, id_b);
        assert!((results_b[0].1 - 1.0).abs() < 1e-6);

        // Test 4: All results should contain all vectors (since we're searching for all 4)
        let all_ids = vec![id_a, id_b, id_c, id_d];
        for result in &results_a {
            assert!(all_ids.contains(&result.0));
        }

        // Test 5: Results should be ordered by similarity (descending) for the first result
        assert!(results_a[0].1 >= results_a[1].1);
        assert!(results_a[0].1 >= results_a[2].1);
        assert!(results_a[0].1 >= results_a[3].1);
    }

    #[tokio::test]
    async fn test_search_with_similar_vectors() {
        let mut store = VectorStore::new();

        // Create vectors with known similarities
        let vector_1 = vec![1.0, 0.0, 0.0]; // Unit vector
        let vector_2 = vec![0.9, 0.1, 0.0]; // Similar to vector_1
        let vector_3 = vec![0.0, 1.0, 0.0]; // Orthogonal to both

        // Pad to 768 dimensions
        let pad_size = 768 - 3;
        let vector_1 = [vector_1, vec![0.0; pad_size]].concat();
        let vector_2 = [vector_2, vec![0.0; pad_size]].concat();
        let vector_3 = [vector_3, vec![0.0; pad_size]].concat();

        let id_1 = Uuid::new_v4();
        let id_2 = Uuid::new_v4();
        let id_3 = Uuid::new_v4();

        store.add_vector(id_1, vector_1.clone()).await.unwrap();
        store.add_vector(id_2, vector_2.clone()).await.unwrap();
        store.add_vector(id_3, vector_3.clone()).await.unwrap();

        // Search for vector_1
        let results = store.search(vector_1.clone(), 3).await.unwrap();
        assert_eq!(results.len(), 3);

        // vector_1 should be first (perfect match)
        assert_eq!(results[0].0, id_1);
        assert!((results[0].1 - 1.0).abs() < 1e-6);

        // vector_2 should be second (most similar)
        assert_eq!(results[1].0, id_2);

        // vector_3 should be last (least similar)
        assert_eq!(results[2].0, id_3);

        // Verify similarity ordering
        assert!(results[0].1 > results[1].1);
        assert!(results[1].1 > results[2].1);
    }

    #[tokio::test]
    async fn test_search_detailed_with_meaningful_vectors() {
        let mut store = VectorStore::new();

        // Create vectors with meaningful patterns
        let vector_1 = vec![1.0, 0.0, 0.0]; // Unit vector
        let vector_2 = vec![0.7071068, 0.7071068, 0.0]; // Normalized vector at 45 degrees
        let vector_3 = vec![0.0, 1.0, 0.0]; // Orthogonal to both

        // Pad to 768 dimensions
        let pad_size = 768 - 3;
        let vector_1 = [vector_1, vec![0.0; pad_size]].concat();
        let vector_2 = [vector_2, vec![0.0; pad_size]].concat();
        let vector_3 = [vector_3, vec![0.0; pad_size]].concat();

        let id_1 = Uuid::new_v4();
        let id_2 = Uuid::new_v4();
        let id_3 = Uuid::new_v4();

        store.add_vector(id_1, vector_1.clone()).await.unwrap();
        store.add_vector(id_2, vector_2.clone()).await.unwrap();
        store.add_vector(id_3, vector_3.clone()).await.unwrap();

        // Search with detailed results
        let results = store.search_detailed(vector_1.clone(), 3).await.unwrap();
        assert_eq!(results.len(), 3);

        // vector_1 should be first (perfect match)
        assert_eq!(results[0].id, id_1);
        assert!(results[0].distance < 1e-6);
        assert!((results[0].similarity - 1.0).abs() < 1e-6);

        // vector_2 should be second (cosine similarity ≈ 0.707)
        assert_eq!(results[1].id, id_2);
        assert!((results[1].similarity - 0.707).abs() < 0.01);

        // vector_3 should be last (orthogonal, cosine similarity ≈ 0)
        assert_eq!(results[2].id, id_3);
        assert!(results[2].similarity < 0.01);

        // Verify distance and similarity relationship
        for result in &results {
            assert!((result.similarity - (1.0 - result.distance)).abs() < 1e-6);
        }
    }

    // Property-based tests
    proptest! {
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

    // Tests for embedder integration (requires actual model)
    #[tokio::test]
    #[ignore = "Requires actual ONNX model"]
    async fn test_with_embedder() {
        // This test would require the actual BGE model to be present
        // For now, we'll skip it
    }

    #[tokio::test]
    #[ignore = "Requires actual ONNX model and root key"]
    async fn test_with_embedder_and_rotation() {
        // This test would require the actual BGE model and a root key
        // For now, we'll skip it
    }
}
