//! Mimir LLM - Large Language Model integration for memory processing

pub mod error;
pub mod prompts;
pub mod config;
pub mod mistralrs_service;

// Re-export main types
pub use error::{LlmError, LlmResult};
pub use config::{LlmConfig, ModelType, QuantizationType, InferenceConfig, DeviceConfig};
pub use mistralrs_service::{MistralRSService, LlmTask, ExtractedMemory, ConflictResolution, ConflictAction};

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic_module_structure() {
        // Basic smoke test to ensure modules compile
        assert!(true);
    }
} 