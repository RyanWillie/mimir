//! Rotation matrix implementation for vector scrambling

use crate::error::{VectorError, VectorResult};
use mimir_core::crypto::RootKey;
use nalgebra::{DMatrix, DVector};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use ring::hkdf;

/// Rotation matrix for vector scrambling
///
/// The rotation matrix dimension must match the embedding dimension of the model used for embedding.
/// Always use the value returned by `Embedder::embedding_dimension()` when constructing a rotation matrix.
pub struct RotationMatrix {
    matrix: DMatrix<f32>,
    dimension: usize,
}

impl RotationMatrix {
    /// Create a rotation matrix from root key using HKDF-SHA256
    ///
    /// # Arguments
    /// * `root_key` - The root key for deterministic rotation
    /// * `dimension` - The embedding dimension (must match the embedder's output)
    ///
    /// # Panics
    /// Panics if `dimension` is zero.
    pub fn from_root_key(root_key: &RootKey, dimension: usize) -> VectorResult<Self> {
        if dimension == 0 {
            return Err(VectorError::InvalidInput(
                "Dimension must be greater than 0".to_string(),
            ));
        }

        // Use HKDF to derive a seed for the CSPRNG
        let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"mimir-rotation-matrix");
        let prk = salt.extract(root_key.as_bytes());

        let info: [&[u8]; 1] = [b"rotation-matrix-seed" as &[u8]];
        let okm = prk
            .expand(&info, hkdf::HKDF_SHA256)
            .map_err(|_| VectorError::Crypto("Failed to expand HKDF".to_string()))?;

        // Extract seed bytes for CSPRNG
        let mut seed_bytes = [0u8; 32];
        okm.fill(&mut seed_bytes)
            .map_err(|_| VectorError::Crypto("Failed to extract seed bytes".to_string()))?;

        // Create CSPRNG from seed
        let mut rng = ChaCha20Rng::from_seed(seed_bytes);

        // Generate random matrix data
        let mut matrix_data = Vec::with_capacity(dimension * dimension);
        for _ in 0..dimension * dimension {
            matrix_data.push(rng.gen::<f32>());
        }

        let matrix = DMatrix::from_row_slice(dimension, dimension, &matrix_data);

        // QR decomposition to get orthonormal matrix
        let qr = matrix.qr();
        let q = qr.q();

        Ok(RotationMatrix {
            matrix: q.clone(),
            dimension,
        })
    }

    /// Apply rotation to a vector: v' = R·v
    pub fn rotate_vector(&self, vector: &[f32]) -> VectorResult<Vec<f32>> {
        if vector.len() != self.dimension {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        let v = DVector::from_column_slice(vector);
        let rotated = &self.matrix * v;

        Ok(rotated.as_slice().to_vec())
    }

    /// Get matrix dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (self.matrix.nrows(), self.matrix.ncols())
    }

    /// Get the dimension this rotation matrix operates on
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Check if matrix is orthonormal (R^T * R = I)
    pub fn is_orthonormal(&self) -> bool {
        let product = self.matrix.transpose() * &self.matrix;

        // Check if product is close to identity (allowing for floating point errors)
        // Use a more relaxed tolerance for high-dimensional matrices
        let tolerance = 1e-4;
        for i in 0..self.dimension {
            for j in 0..self.dimension {
                let expected = if i == j { 1.0 } else { 0.0 };
                if (product[(i, j)] - expected).abs() > tolerance {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mimir_core::crypto::RootKey;

    #[test]
    fn test_rotation_matrix_creation() {
        let root_key = RootKey::new().unwrap();
        let rotation_matrix = RotationMatrix::from_root_key(&root_key, 768).unwrap();

        assert_eq!(rotation_matrix.dimensions(), (768, 768));
        assert_eq!(rotation_matrix.dimension(), 768);
        assert!(rotation_matrix.is_orthonormal());
    }

    #[test]
    fn test_vector_rotation() {
        let root_key = RootKey::new().unwrap();
        let rotation_matrix = RotationMatrix::from_root_key(&root_key, 768).unwrap();

        let test_vector = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let padded_vector = vec![test_vector, vec![0.0; 762]].concat();

        let rotated = rotation_matrix.rotate_vector(&padded_vector).unwrap();

        assert_eq!(rotated.len(), 768);
        assert_ne!(rotated, padded_vector); // Should be different after rotation
    }

    #[test]
    fn test_deterministic_rotation() {
        let root_key = RootKey::new().unwrap();
        let rotation_matrix1 = RotationMatrix::from_root_key(&root_key, 768).unwrap();
        let rotation_matrix2 = RotationMatrix::from_root_key(&root_key, 768).unwrap();

        let test_vector = vec![1.0; 768];
        let rotated1 = rotation_matrix1.rotate_vector(&test_vector).unwrap();
        let rotated2 = rotation_matrix2.rotate_vector(&test_vector).unwrap();

        assert_eq!(rotated1, rotated2); // Same root key should produce same rotation
    }

    #[test]
    fn test_different_root_keys_produce_different_rotations() {
        let root_key1 = RootKey::new().unwrap();
        let root_key2 = RootKey::new().unwrap();

        let rotation_matrix1 = RotationMatrix::from_root_key(&root_key1, 768).unwrap();
        let rotation_matrix2 = RotationMatrix::from_root_key(&root_key2, 768).unwrap();

        let test_vector = vec![1.0; 768];
        let rotated1 = rotation_matrix1.rotate_vector(&test_vector).unwrap();
        let rotated2 = rotation_matrix2.rotate_vector(&test_vector).unwrap();

        assert_ne!(rotated1, rotated2); // Different root keys should produce different rotations
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let root_key = RootKey::new().unwrap();
        let rotation_matrix = RotationMatrix::from_root_key(&root_key, 768).unwrap();

        let wrong_size_vector = vec![1.0; 100];
        let result = rotation_matrix.rotate_vector(&wrong_size_vector);

        assert!(result.is_err());
        match result.unwrap_err() {
            VectorError::DimensionMismatch { expected, actual } => {
                assert_eq!(expected, 768);
                assert_eq!(actual, 100);
            }
            _ => panic!("Expected DimensionMismatch error"),
        }
    }
}
