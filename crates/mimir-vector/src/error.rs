//! Error types for Mimir Vector operations

use thiserror::Error;

/// Error type for vector store operations
#[derive(Error, Debug)]
pub enum VectorError {
    #[error("ONNX model error: {0}")]
    OnnxModel(String),

    #[error("Embedding generation failed: {0}")]
    EmbeddingGeneration(String),

    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("HNSW index error: {0}")]
    HnswIndex(String),

    #[error("Persistence error: {0}")]
    Persistence(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Result type for vector store operations
pub type VectorResult<T> = Result<T, VectorError>;

impl From<ort::Error> for VectorError {
    fn from(err: ort::Error) -> Self {
        VectorError::OnnxModel(err.to_string())
    }
}

impl From<serde_json::Error> for VectorError {
    fn from(err: serde_json::Error) -> Self {
        VectorError::Serialization(err.to_string())
    }
} 