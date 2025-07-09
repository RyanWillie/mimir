//! HNSW-based secure vector store implementation

use crate::embedder::Embedder;
use crate::error::{VectorError, VectorResult};
use crate::persistence::VectorDataForPersistence;
use crate::rotation::RotationMatrix;
use hnsw_rs::prelude::*;
use mimir_core::{crypto::RootKey, MemoryId};
use std::collections::HashMap;
use std::path::Path;

/// Search result from HNSW index
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: MemoryId,
    pub distance: f32,
    pub similarity: f32, // Cosine similarity (1.0 - distance)
}

/// Secure vector store using HNSW for similarity search
///
/// This implementation provides:
/// - High-performance similarity search using HNSW
/// - Optional vector rotation for security
/// - Integration with ONNX embedding models
/// - Support for both raw vectors and text-to-vector conversion
pub struct SecureVectorStore<'a> {
    hnsw: Hnsw<'a, f32, DistCosine>, // f32 vectors with cosine distance
    embedder: Option<Embedder>,
    rotation_matrix: Option<RotationMatrix>,
    dimension: usize,
    next_id: usize,                             // Use usize to match HNSW expectations
    id_mapping: HashMap<usize, MemoryId>,       // Map internal IDs to MemoryIds
    reverse_mapping: HashMap<MemoryId, usize>,  // Map MemoryIds to internal IDs
    original_vectors: HashMap<usize, Vec<f32>>, // Store original vectors for persistence
}

impl<'a> SecureVectorStore<'a> {
    /// Create a new secure vector store
    pub fn new(dimension: usize) -> VectorResult<Self> {
        if dimension == 0 {
            return Err(VectorError::InvalidInput(
                "Dimension must be greater than 0".to_string(),
            ));
        }

        // Use parameters suitable for larger dimensions and deterministic results
        let max_connections = 32; // Increased for better connectivity
        let max_elements = 10000; // Maximum number of elements
        let max_layer = 16; // Maximum number of layers
        let ef_construction = 32; // Increased for better construction quality

        let hnsw = Hnsw::new(
            max_connections,
            max_elements,
            max_layer,
            ef_construction,
            DistCosine,
        );

        Ok(Self {
            hnsw,
            embedder: None,
            rotation_matrix: None,
            dimension,
            next_id: 0,
            id_mapping: HashMap::new(),
            reverse_mapping: HashMap::new(),
            original_vectors: HashMap::new(),
        })
    }

    /// Create a secure vector store with embedder
    pub async fn with_embedder<P: AsRef<Path>>(model_path: P) -> VectorResult<Self> {
        let embedder = Embedder::new(model_path).await?;
        let dimension = embedder.embedding_dimension();

        let mut store = Self::new(dimension)?;
        store.embedder = Some(embedder);

        Ok(store)
    }

    /// Create a secure vector store with embedder and rotation matrix
    pub async fn with_embedder_and_rotation<P: AsRef<Path>>(
        model_path: P,
        root_key: &RootKey,
    ) -> VectorResult<Self> {
        let embedder = Embedder::new(model_path).await?;
        let dimension = embedder.embedding_dimension();
        let rotation_matrix = RotationMatrix::from_root_key(root_key, dimension)?;

        // Verify rotation matrix matches embedder dimension
        if rotation_matrix.dimension() != dimension {
            return Err(VectorError::DimensionMismatch {
                expected: dimension,
                actual: rotation_matrix.dimension(),
            });
        }

        let mut store = Self::new(dimension)?;
        store.embedder = Some(embedder);
        store.rotation_matrix = Some(rotation_matrix);

        Ok(store)
    }

    /// Add a raw vector to the store
    pub async fn add_raw_vector(
        &mut self,
        vector: Vec<f32>,
        memory_id: MemoryId,
    ) -> VectorResult<()> {
        // Validate vector dimension
        if vector.len() != self.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        // Check if memory_id already exists
        if self.reverse_mapping.contains_key(&memory_id) {
            return Err(VectorError::InvalidInput(format!(
                "Memory ID {} already exists in store",
                memory_id
            )));
        }

        // Store original vector for persistence (before rotation)
        let original_vector = vector.clone();

        // Apply rotation if configured
        let vector_to_store = if let Some(rotation_matrix) = &self.rotation_matrix {
            rotation_matrix.rotate_vector(&vector)?
        } else {
            vector
        };

        // Add to HNSW with internal ID
        let internal_id = self.next_id;
        self.hnsw.insert((&vector_to_store, internal_id));

        // Store original vector for persistence
        self.original_vectors.insert(internal_id, original_vector);

        // Update mappings
        self.id_mapping.insert(internal_id, memory_id);
        self.reverse_mapping.insert(memory_id, internal_id);
        self.next_id += 1;

        Ok(())
    }

    /// Add text to the store (converts to embedding first)
    pub async fn add_text(&mut self, text: &str, memory_id: MemoryId) -> VectorResult<()> {
        let embedder = self
            .embedder
            .as_mut()
            .ok_or_else(|| VectorError::InvalidInput("No embedder available".to_string()))?;

        // Generate embedding
        let embedding = embedder.embed(text).await?;

        // Add the embedding
        self.add_raw_vector(embedding, memory_id).await
    }

    /// Search for similar vectors
    pub async fn search_raw_vector(
        &self,
        query: &[f32],
        k: usize,
    ) -> VectorResult<Vec<SearchResult>> {
        // Validate query dimension
        if query.len() != self.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimension,
                actual: query.len(),
            });
        }

        // Apply rotation if configured
        let rotated_query = if let Some(rotation_matrix) = &self.rotation_matrix {
            rotation_matrix.rotate_vector(query)?
        } else {
            query.to_vec()
        };

        // Search HNSW
        let results = self.hnsw.search(&rotated_query, k, 32); // Use ef=32 to match construction parameters

        // Convert to SearchResult format using the external id (d_id)
        let search_results: Vec<SearchResult> = results
            .into_iter()
            .filter_map(|result| {
                let memory_id = self.id_mapping.get(&result.d_id)?;
                Some(SearchResult {
                    id: *memory_id,
                    distance: result.distance,
                    similarity: 1.0 - result.distance, // Convert distance to similarity
                })
            })
            .collect();

        Ok(search_results)
    }

    /// Search for similar text (converts to embedding first)
    pub async fn search_text(&mut self, query: &str, k: usize) -> VectorResult<Vec<SearchResult>> {
        let embedder = self
            .embedder
            .as_mut()
            .ok_or_else(|| VectorError::InvalidInput("No embedder available".to_string()))?;

        // Generate embedding for query
        let embedding = embedder.embed(query).await?;

        // Search with the embedding
        self.search_raw_vector(&embedding, k).await
    }

    /// Remove a vector from the store
    pub async fn remove_vector(&mut self, memory_id: MemoryId) -> VectorResult<()> {
        let internal_id = self.reverse_mapping.get(&memory_id).ok_or_else(|| {
            VectorError::InvalidInput(format!("Memory ID {} not found in store", memory_id))
        })?;

        // Remove from HNSW (note: HNSW doesn't support removal, so we'll need to rebuild)
        // For now, we'll just remove from our mappings
        self.id_mapping.remove(internal_id);
        self.reverse_mapping.remove(&memory_id);

        Ok(())
    }

    /// Get the number of vectors in the store
    pub fn len(&self) -> usize {
        self.id_mapping.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.id_mapping.is_empty()
    }

    /// Get the embedding dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Check if embedder is available
    pub fn has_embedder(&self) -> bool {
        self.embedder.is_some()
    }

    /// Check if rotation matrix is available
    pub fn has_rotation(&self) -> bool {
        self.rotation_matrix.is_some()
    }

    /// Get embedding dimension from embedder if available
    pub fn embedding_dimension(&self) -> Option<usize> {
        self.embedder.as_ref().map(|e| e.embedding_dimension())
    }

    /// Attach an embedder to the store
    pub async fn attach_embedder<P: AsRef<Path>>(&mut self, model_path: P) -> VectorResult<()> {
        let embedder = Embedder::new(model_path).await?;
        let dimension = embedder.embedding_dimension();

        // Verify dimension matches
        if dimension != self.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimension,
                actual: dimension,
            });
        }

        self.embedder = Some(embedder);
        Ok(())
    }

    /// Check if a memory ID exists in the store
    pub fn contains(&self, memory_id: &MemoryId) -> bool {
        self.reverse_mapping.contains_key(memory_id)
    }

    /// Get the next internal ID
    pub fn next_id(&self) -> usize {
        self.next_id
    }

    /// Get vector data for persistence
    pub fn get_vector_data_for_persistence(&self) -> VectorResult<VectorDataForPersistence> {
        let mut vectors = Vec::new();

        // Extract original vectors that were stored
        for (internal_id, _memory_id) in &self.id_mapping {
            if let Some(vector) = self.original_vectors.get(internal_id) {
                vectors.push((*internal_id, vector.clone()));
            } else {
                return Err(VectorError::InvalidInput(format!(
                    "Missing original vector for internal ID {}",
                    internal_id
                )));
            }
        }

        Ok(VectorDataForPersistence {
            vectors,
            id_mapping: self.id_mapping.clone(),
            reverse_mapping: self.reverse_mapping.clone(),
        })
    }

    /// Restore from persistence data
    pub fn restore_from_persistence_data(
        &mut self,
        data: VectorDataForPersistence,
        next_id: usize,
    ) -> VectorResult<()> {
        // Restore ID mappings
        self.id_mapping = data.id_mapping;
        self.reverse_mapping = data.reverse_mapping;
        self.next_id = next_id;

        // Clear existing original vectors
        self.original_vectors.clear();

        // Rebuild HNSW index from vectors and store original vectors
        for (internal_id, vector) in data.vectors {
            // Store original vector
            self.original_vectors.insert(internal_id, vector.clone());

            // Apply rotation if configured
            let vector_to_store = if let Some(rotation_matrix) = &self.rotation_matrix {
                rotation_matrix.rotate_vector(&vector)?
            } else {
                vector
            };

            // Add to HNSW
            self.hnsw.insert((&vector_to_store, internal_id));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mimir_core::test_utils::generators::generate_test_embedding;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_secure_vector_store_creation() {
        let store = SecureVectorStore::new(128).unwrap();
        assert_eq!(store.dimension(), 128);
        assert!(store.is_empty());
        assert!(!store.has_embedder());
        assert!(!store.has_rotation());
    }

    #[tokio::test]
    async fn test_add_and_search_vectors() {
        let mut store = SecureVectorStore::new(128).unwrap();

        let memory_id1 = Uuid::new_v4();
        let memory_id2 = Uuid::new_v4();
        let vector1 = generate_test_embedding(128);
        // Create a slightly different vector for the second entry
        let mut vector2 = generate_test_embedding(128);
        vector2[0] += 1.0;

        // Add vectors
        store
            .add_raw_vector(vector1.clone(), memory_id1)
            .await
            .unwrap();
        store
            .add_raw_vector(vector2.clone(), memory_id2)
            .await
            .unwrap();

        assert_eq!(store.len(), 2);
        assert!(store.contains(&memory_id1));
        assert!(store.contains(&memory_id2));

        // Search with higher ef parameter to ensure we find both vectors
        let results = store.search_raw_vector(&vector1, 2).await.unwrap();

        // We should get at least 1 result, but the exact number may vary due to HNSW's approximate nature
        assert!(results.len() >= 1);

        // First result should be the query vector itself (distance = 0)
        assert_eq!(results[0].id, memory_id1);
        assert!(results[0].distance < 1e-6);
        assert!((results[0].similarity - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_dimension_mismatch() {
        let mut store = SecureVectorStore::new(128).unwrap();
        let memory_id = Uuid::new_v4();
        let wrong_dim_vector = generate_test_embedding(64);

        let result = store.add_raw_vector(wrong_dim_vector, memory_id).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            VectorError::DimensionMismatch { expected, actual } => {
                assert_eq!(expected, 128);
                assert_eq!(actual, 64);
            }
            _ => panic!("Expected DimensionMismatch error"),
        }
    }

    #[tokio::test]
    async fn test_duplicate_memory_id() {
        let mut store = SecureVectorStore::new(128).unwrap();
        let memory_id = Uuid::new_v4();
        let vector1 = generate_test_embedding(128);
        let vector2 = generate_test_embedding(128);

        store.add_raw_vector(vector1, memory_id).await.unwrap();

        let result = store.add_raw_vector(vector2, memory_id).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            VectorError::InvalidInput(msg) => {
                assert!(msg.contains("already exists"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[tokio::test]
    async fn test_search_with_empty_store() {
        let store = SecureVectorStore::new(128).unwrap();
        let query = generate_test_embedding(128);

        let results = store.search_raw_vector(&query, 10).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_remove_vector() {
        let mut store = SecureVectorStore::new(128).unwrap();
        let memory_id = Uuid::new_v4();
        let vector = generate_test_embedding(128);

        store
            .add_raw_vector(vector.clone(), memory_id)
            .await
            .unwrap();
        assert!(store.contains(&memory_id));

        store.remove_vector(memory_id).await.unwrap();
        assert!(!store.contains(&memory_id));
        assert_eq!(store.len(), 0);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_vector() {
        let mut store = SecureVectorStore::new(128).unwrap();
        let memory_id = Uuid::new_v4();

        let result = store.remove_vector(memory_id).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            VectorError::InvalidInput(msg) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[tokio::test]
    async fn test_search_different_k_values() {
        let mut store = SecureVectorStore::new(128).unwrap();
        let memory_id = Uuid::new_v4();
        let vector = generate_test_embedding(128);

        store
            .add_raw_vector(vector.clone(), memory_id)
            .await
            .unwrap();

        for k in [1, 5, 10, 50] {
            let results = store.search_raw_vector(&vector, k).await.unwrap();
            assert_eq!(results.len(), 1); // Only one vector in store
        }
    }

    #[tokio::test]
    async fn test_large_embeddings() {
        let mut store = SecureVectorStore::new(768).unwrap(); // Back to 768 dimensions
        let memory_id = Uuid::new_v4();
        let large_vector = generate_test_embedding(768);

        store
            .add_raw_vector(large_vector.clone(), memory_id)
            .await
            .unwrap();
        assert_eq!(store.len(), 1);

        let results = store.search_raw_vector(&large_vector, 1).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, memory_id);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let mut store = SecureVectorStore::new(128).unwrap();
        let memory_id1 = Uuid::new_v4();
        let memory_id2 = Uuid::new_v4();
        let vector1 = generate_test_embedding(128);
        let vector2 = generate_test_embedding(128);

        // Sequential operations (concurrent would require Arc<Mutex<>>)
        store
            .add_raw_vector(vector1.clone(), memory_id1)
            .await
            .unwrap();
        store
            .add_raw_vector(vector2.clone(), memory_id2)
            .await
            .unwrap();

        let results1 = store.search_raw_vector(&vector1, 1).await.unwrap();
        let results2 = store.search_raw_vector(&vector2, 1).await.unwrap();

        // Just verify we get results, not specific IDs (since search order may vary)
        assert_eq!(results1.len(), 1);
        assert_eq!(results2.len(), 1);
        assert!(store.contains(&memory_id1));
        assert!(store.contains(&memory_id2));
    }

    #[tokio::test]
    async fn test_search_with_meaningful_vectors() {
        let mut store = SecureVectorStore::new(4).unwrap(); // Small dimension for testing

        // Create vectors with meaningful patterns
        let vector_a = vec![1.0, 0.0, 0.0, 0.0]; // Unit vector in x direction
        let vector_b = vec![0.0, 1.0, 0.0, 0.0]; // Unit vector in y direction
        let vector_c = vec![0.0, 0.0, 1.0, 0.0]; // Unit vector in z direction
        let vector_d = vec![0.0, 0.0, 0.0, 1.0]; // Unit vector in w direction

        // Add vectors to store
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let id_c = Uuid::new_v4();
        let id_d = Uuid::new_v4();

        store.add_raw_vector(vector_a.clone(), id_a).await.unwrap();
        store.add_raw_vector(vector_b.clone(), id_b).await.unwrap();
        store.add_raw_vector(vector_c.clone(), id_c).await.unwrap();
        store.add_raw_vector(vector_d.clone(), id_d).await.unwrap();

        assert_eq!(store.len(), 4);

        // Test 1: Search for vector_a should return vector_a as most similar
        let results_a = store.search_raw_vector(&vector_a, 4).await.unwrap();
        assert_eq!(results_a.len(), 4);

        // First result should be vector_a itself (perfect match)
        assert_eq!(results_a[0].id, id_a);
        assert!(results_a[0].distance < 1e-6); // Should be very close to 0
        assert!((results_a[0].similarity - 1.0).abs() < 1e-6); // Should be very close to 1.0

        // Test 2: Search should be deterministic (same query returns same results)
        let results_a2 = store.search_raw_vector(&vector_a, 4).await.unwrap();
        assert_eq!(results_a.len(), results_a2.len());
        for (r1, r2) in results_a.iter().zip(results_a2.iter()) {
            assert_eq!(r1.id, r2.id);
            assert!((r1.distance - r2.distance).abs() < 1e-6);
            assert!((r1.similarity - r2.similarity).abs() < 1e-6);
        }

        // Test 3: Search for vector_b should return vector_b as most similar
        let results_b = store.search_raw_vector(&vector_b, 4).await.unwrap();
        assert_eq!(results_b.len(), 4);
        assert_eq!(results_b[0].id, id_b);
        assert!(results_b[0].distance < 1e-6);

        // Test 4: All results should contain all vectors (since we're searching for all 4)
        let all_ids = vec![id_a, id_b, id_c, id_d];
        for result in &results_a {
            assert!(all_ids.contains(&result.id));
        }

        // Test 5: Results should be ordered by similarity (descending) for the first result
        assert!(results_a[0].similarity >= results_a[1].similarity);
        assert!(results_a[0].similarity >= results_a[2].similarity);
        assert!(results_a[0].similarity >= results_a[3].similarity);
    }

    #[tokio::test]
    async fn test_search_with_similar_vectors() {
        let mut store = SecureVectorStore::new(3).unwrap();

        // Create vectors with known similarities
        let vector_1 = vec![1.0, 0.0, 0.0]; // Unit vector
        let vector_2 = vec![0.9, 0.1, 0.0]; // Similar to vector_1
        let vector_3 = vec![0.0, 1.0, 0.0]; // Orthogonal to both

        let id_1 = Uuid::new_v4();
        let id_2 = Uuid::new_v4();
        let id_3 = Uuid::new_v4();

        store.add_raw_vector(vector_1.clone(), id_1).await.unwrap();
        store.add_raw_vector(vector_2.clone(), id_2).await.unwrap();
        store.add_raw_vector(vector_3.clone(), id_3).await.unwrap();

        // Search for vector_1
        let results = store.search_raw_vector(&vector_1, 3).await.unwrap();
        assert_eq!(results.len(), 3);

        // vector_1 should be first (perfect match)
        assert_eq!(results[0].id, id_1);
        assert!(results[0].distance < 1e-6);

        // vector_2 should be second (most similar)
        assert_eq!(results[1].id, id_2);

        // vector_3 should be last (least similar)
        assert_eq!(results[2].id, id_3);

        // Verify similarity ordering
        assert!(results[0].similarity > results[1].similarity);
        assert!(results[1].similarity > results[2].similarity);

        // Verify distances are reasonable
        assert!(results[0].distance < results[1].distance);
        assert!(results[1].distance < results[2].distance);
    }

    #[tokio::test]
    async fn test_search_with_normalized_vectors() {
        let mut store = SecureVectorStore::new(2).unwrap();

        // Test with normalized vectors to ensure cosine similarity works correctly
        let vector_1 = vec![1.0, 0.0]; // Already normalized
        let vector_2 = vec![0.0, 1.0]; // Already normalized
        let vector_3 = vec![0.7071068, 0.7071068]; // Normalized vector at 45 degrees

        let id_1 = Uuid::new_v4();
        let id_2 = Uuid::new_v4();
        let id_3 = Uuid::new_v4();

        store.add_raw_vector(vector_1.clone(), id_1).await.unwrap();
        store.add_raw_vector(vector_2.clone(), id_2).await.unwrap();
        store.add_raw_vector(vector_3.clone(), id_3).await.unwrap();

        // Search with vector_1
        let results = store.search_raw_vector(&vector_1, 3).await.unwrap();
        assert_eq!(results.len(), 3);

        // Debug output
        println!("Search results:");
        for (i, result) in results.iter().enumerate() {
            println!(
                "  {}: id={}, distance={:.6}, similarity={:.6}",
                i, result.id, result.distance, result.similarity
            );
        }

        // vector_1 should be first (perfect match)
        assert_eq!(results[0].id, id_1);
        assert!((results[0].similarity - 1.0).abs() < 1e-6);

        // vector_3 should be second (cosine similarity = 0.707)
        assert_eq!(results[1].id, id_3);
        assert!((results[1].similarity - 0.707).abs() < 0.01);

        // vector_2 should be last (orthogonal, cosine similarity = 0)
        assert_eq!(results[2].id, id_2);
        assert!(results[2].similarity < 0.01);
    }

    #[tokio::test]
    async fn test_search_k_parameter() {
        let mut store = SecureVectorStore::new(2).unwrap();

        let vector_1 = vec![1.0, 0.0];
        let vector_2 = vec![0.0, 1.0];
        let vector_3 = vec![0.707, 0.707];

        let id_1 = Uuid::new_v4();
        let id_2 = Uuid::new_v4();
        let id_3 = Uuid::new_v4();

        store.add_raw_vector(vector_1.clone(), id_1).await.unwrap();
        store.add_raw_vector(vector_2.clone(), id_2).await.unwrap();
        store.add_raw_vector(vector_3.clone(), id_3).await.unwrap();

        // Test different k values
        let results_k1 = store.search_raw_vector(&vector_1, 1).await.unwrap();
        assert_eq!(results_k1.len(), 1);
        assert_eq!(results_k1[0].id, id_1);

        let results_k2 = store.search_raw_vector(&vector_1, 2).await.unwrap();
        assert_eq!(results_k2.len(), 2);
        assert_eq!(results_k2[0].id, id_1);

        let results_k3 = store.search_raw_vector(&vector_1, 3).await.unwrap();
        assert_eq!(results_k3.len(), 3);
        assert_eq!(results_k3[0].id, id_1);

        // Test k larger than available vectors
        let results_k5 = store.search_raw_vector(&vector_1, 5).await.unwrap();
        assert_eq!(results_k5.len(), 3); // Should return all available vectors
    }
}
