use thiserror::Error;

/// Main error type for Mimir operations
#[derive(Error, Debug)]
pub enum MimirError {
    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),
    
    #[error("Vector store error: {0}")]
    VectorStore(String),
    
    #[error("Encryption error: {0}")]
    Encryption(String),
    
    #[error("Guardrails error: {0}")]
    Guardrails(String),
    
    #[error("Compression error: {0}")]
    Compression(String),
    
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Server error: {0}")]
    ServerError(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convenience Result type
pub type Result<T> = std::result::Result<T, MimirError>; 