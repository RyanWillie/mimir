//! Mimir LLM - Large Language Model integration for memory processing

pub mod error;
pub mod gemma;
pub mod prompts;
pub mod config;

// Re-export main types
pub use error::{LlmError, LlmResult};
pub use gemma::{GemmaService, GemmaTask};
pub use config::GemmaConfig;

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic_module_structure() {
        // Basic smoke test to ensure modules compile
        assert!(true);
    }
} 