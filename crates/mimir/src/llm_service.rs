//! LLM service integration for memory processing
//! 
//! This module provides a wrapper around the mimir-llm service for use in the main Mimir server.

use mimir_llm::{LlmConfig, ModelType, QuantizationType, MistralRSService, LlmResult};
use mimir_core::{Config, Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Global LLM service instance
static mut LLM_SERVICE: Option<Arc<LlmService>> = None;

/// LLM service wrapper for the main Mimir server
pub struct LlmService {
    /// The underlying MistralRS service
    service: Arc<Mutex<MistralRSService>>,
    /// Whether the service is initialized
    initialized: bool,
}

impl LlmService {
    /// Create a new LLM service with default Gemma3 configuration
    pub fn new() -> Self {
        let config = LlmConfig::new()
            .with_model_type(ModelType::Gemma3_1bIt)
            .with_gguf(false)
            .with_quantization(QuantizationType::Q4_0)
            .with_temperature(0.7)
            .with_max_tokens(200);

        Self {
            service: Arc::new(Mutex::new(MistralRSService::new(config))),
            initialized: false,
        }
    }

    /// Create a new LLM service with custom configuration
    pub fn with_config(config: LlmConfig) -> Self {
        Self {
            service: Arc::new(Mutex::new(MistralRSService::new(config))),
            initialized: false,
        }
    }

    /// Initialize the LLM service (load the model)
    pub async fn initialize(&mut self) -> LlmResult<()> {
        if self.initialized {
            return Ok(());
        }

        info!("Initializing LLM service...");
        
        let mut service = self.service.lock().await;
        service.load_model().await?;
        
        self.initialized = true;
        info!("LLM service initialized successfully");
        
        Ok(())
    }

    /// Check if the service is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Extract memories from text
    pub async fn extract_memories(&self, text: &str) -> LlmResult<Vec<mimir_llm::ExtractedMemory>> {
        if !self.initialized {
            return Err(mimir_llm::LlmError::ModelNotLoaded);
        }

        let mut service = self.service.lock().await;
        service.extract_memories(text).await
    }

    /// Summarize memory content
    pub async fn summarize_memory(&self, content: &str, max_tokens: usize) -> LlmResult<String> {
        if !self.initialized {
            return Err(mimir_llm::LlmError::ModelNotLoaded);
        }

        let mut service = self.service.lock().await;
        service.summarize_memory(content, max_tokens).await
    }

    /// Resolve conflicts between memories
    pub async fn resolve_conflict(&self, existing: &str, new: &str, similarity: f32) -> LlmResult<mimir_llm::ConflictResolution> {
        if !self.initialized {
            return Err(mimir_llm::LlmError::ModelNotLoaded);
        }

        let mut service = self.service.lock().await;
        service.resolve_conflict(existing, new, similarity).await
    }

    /// Classify memory content
    pub async fn classify_memory(&self, content: &str) -> LlmResult<mimir_core::MemoryClass> {
        if !self.initialized {
            return Err(mimir_llm::LlmError::ModelNotLoaded);
        }

        let mut service = self.service.lock().await;
        service.classify_memory(content).await
    }

    /// Summarize search results
    pub async fn summarize_search_results(&self, query: &str, results: &[String]) -> LlmResult<String> {
        if !self.initialized {
            return Err(mimir_llm::LlmError::ModelNotLoaded);
        }

        let mut service = self.service.lock().await;
        service.summarize_search_results(query, results).await
    }

    /// Generate a response using the LLM
    pub async fn generate_response(&self, prompt: &str) -> LlmResult<String> {
        if !self.initialized {
            return Err(mimir_llm::LlmError::ModelNotLoaded);
        }

        let mut service = self.service.lock().await;
        service.generate_response(prompt).await
    }
}

impl Clone for LlmService {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            initialized: self.initialized,
        }
    }
}

impl Default for LlmService {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the global LLM service
pub async fn initialize_llm_service(config: &Config) -> Result<()> {
    info!("Initializing global LLM service...");
    
    // Get the default LLM model path
    let llm_model_path = mimir_core::get_default_app_dir()
        .join("models")
        .join("gemma-3-1b-it-standard");
    
    // Check if the LLM model directory exists
    if !llm_model_path.exists() {
        return Err(mimir_core::MimirError::ServerError(format!(
            "LLM model directory not found at: {}. Please ensure the Gemma3 model is properly installed.",
            llm_model_path.display()
        )));
    }
    
    info!("LLM model ready at: {}", llm_model_path.display());
    
    // Create LLM service with model path
    let mut llm_service = LlmService::with_config(
        LlmConfig::new()
            .with_model_path(llm_model_path)
            .with_model_type(ModelType::Gemma3_1bIt)
            .with_gguf(false)
            .with_quantization(QuantizationType::Q4_0)
            .with_temperature(0.7)
            .with_max_tokens(200)
    );
    
    // Initialize the service
    llm_service.initialize().await
        .map_err(|e| mimir_core::MimirError::ServerError(format!("Failed to initialize LLM service: {}", e)))?;
    
    // Store the global instance
    unsafe {
        LLM_SERVICE = Some(Arc::new(llm_service));
    }
    
    info!("Global LLM service initialized successfully");
    Ok(())
}

/// Get the global LLM service instance
pub fn get_llm_service() -> Option<Arc<LlmService>> {
    unsafe {
        LLM_SERVICE.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_service_creation() {
        let service = LlmService::new();
        assert!(!service.is_initialized());
    }

    #[test]
    fn test_llm_service_with_config() {
        let config = LlmConfig::new()
            .with_model_type(ModelType::Gemma3_1bIt)
            .with_temperature(0.8);
        
        let service = LlmService::with_config(config);
        assert!(!service.is_initialized());
    }

    #[test]
    fn test_llm_service_clone() {
        let service = LlmService::new();
        let cloned = service.clone();
        assert_eq!(service.is_initialized(), cloned.is_initialized());
    }
} 