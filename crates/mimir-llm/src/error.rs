//! Error types for LLM operations

use thiserror::Error;

/// Result type for LLM operations
pub type LlmResult<T> = Result<T, LlmError>;

/// Errors that can occur during LLM operations
#[derive(Error, Debug)]
pub enum LlmError {
    /// Model loading errors
    #[error("Failed to load model: {0}")]
    ModelLoading(String),

    /// Model inference errors
    #[error("Inference failed: {0}")]
    Inference(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Model not loaded when inference attempted
    #[error("Model not loaded - call load_model() first")]
    ModelNotLoaded,

    /// Invalid prompt or input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Token limit exceeded
    #[error("Token limit exceeded: {current} > {max}")]
    TokenLimitExceeded { current: usize, max: usize },

    /// Timeout during inference
    #[error("Inference timeout after {seconds} seconds")]
    Timeout { seconds: u64 },
}

// Note: llama-cpp-2 uses different error handling, converting via string for now
impl From<String> for LlmError {
    fn from(err: String) -> Self {
        LlmError::Inference(err)
    }
} 