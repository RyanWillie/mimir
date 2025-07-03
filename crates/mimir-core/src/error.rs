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

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

/// Convenience Result type
pub type Result<T> = std::result::Result<T, MimirError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_display_messages() {
        let vector_error = MimirError::VectorStore("Index not found".to_string());
        assert_eq!(
            vector_error.to_string(),
            "Vector store error: Index not found"
        );

        let encryption_error = MimirError::Encryption("Key derivation failed".to_string());
        assert_eq!(
            encryption_error.to_string(),
            "Encryption error: Key derivation failed"
        );

        let guardrails_error = MimirError::Guardrails("PII detected".to_string());
        assert_eq!(
            guardrails_error.to_string(),
            "Guardrails error: PII detected"
        );

        let compression_error = MimirError::Compression("Model load failed".to_string());
        assert_eq!(
            compression_error.to_string(),
            "Compression error: Model load failed"
        );

        let access_error = MimirError::AccessDenied("Invalid app_id".to_string());
        assert_eq!(access_error.to_string(), "Access denied: Invalid app_id");

        let config_error = MimirError::Config("Invalid port".to_string());
        assert_eq!(
            config_error.to_string(),
            "Configuration error: Invalid port"
        );

        let server_error = MimirError::ServerError("Bind failed".to_string());
        assert_eq!(server_error.to_string(), "Server error: Bind failed");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let mimir_error = MimirError::from(io_error);

        match mimir_error {
            MimirError::Io(err) => {
                assert_eq!(err.kind(), io::ErrorKind::NotFound);
                assert_eq!(err.to_string(), "File not found");
            }
            _ => panic!("Expected IO error"),
        }
    }

    #[test]
    fn test_serde_error_conversion() {
        // Create a malformed JSON to trigger serde error
        let malformed_json = r#"{"missing_quote: true}"#;
        let parse_result: std::result::Result<serde_json::Value, serde_json::Error> =
            serde_json::from_str(malformed_json);

        let serde_error = parse_result.unwrap_err();
        let mimir_error = MimirError::from(serde_error);

        match mimir_error {
            MimirError::Serialization(_) => {
                assert!(mimir_error.to_string().contains("Serialization error"));
            }
            _ => panic!("Expected Serialization error"),
        }
    }

    #[test]
    fn test_anyhow_error_conversion() {
        let anyhow_error = anyhow::anyhow!("Something went wrong");
        let mimir_error = MimirError::from(anyhow_error);

        match mimir_error {
            MimirError::Database(_) => {
                assert!(mimir_error.to_string().contains("Database error"));
                assert!(mimir_error.to_string().contains("Something went wrong"));
            }
            _ => panic!("Expected Database error"),
        }
    }

    #[test]
    fn test_result_type_usage() {
        fn success_function() -> Result<String> {
            Ok("success".to_string())
        }

        fn error_function() -> Result<String> {
            Err(MimirError::Config("Test error".to_string()))
        }

        assert!(success_function().is_ok());
        assert_eq!(success_function().unwrap(), "success");

        assert!(error_function().is_err());
        let error = error_function().unwrap_err();
        assert!(matches!(error, MimirError::Config(_)));
    }

    #[test]
    fn test_error_propagation() {
        fn inner_function() -> Result<()> {
            Err(MimirError::VectorStore("Inner error".to_string()))
        }

        fn outer_function() -> Result<String> {
            inner_function()?;
            Ok("success".to_string())
        }

        let result = outer_function();
        assert!(result.is_err());

        match result.unwrap_err() {
            MimirError::VectorStore(msg) => assert_eq!(msg, "Inner error"),
            _ => panic!("Expected VectorStore error"),
        }
    }

    #[test]
    fn test_error_chain() {
        // Test that we can chain errors appropriately
        let root_cause = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        let mimir_error: MimirError = root_cause.into();

        // Verify the error chain is preserved
        assert!(mimir_error.to_string().contains("IO error"));
        assert!(mimir_error.to_string().contains("Access denied"));
    }

    #[test]
    fn test_all_error_variants_debug() {
        // Ensure all error variants can be debugged (useful for logging)
        let errors = vec![
            MimirError::VectorStore("test".to_string()),
            MimirError::Encryption("test".to_string()),
            MimirError::Guardrails("test".to_string()),
            MimirError::Compression("test".to_string()),
            MimirError::AccessDenied("test".to_string()),
            MimirError::Config("test".to_string()),
            MimirError::ServerError("test".to_string()),
        ];

        for error in errors {
            let debug_str = format!("{:?}", error);
            assert!(!debug_str.is_empty());
            assert!(debug_str.contains("test"));
        }
    }
}
