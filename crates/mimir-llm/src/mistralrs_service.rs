//! MistralRS service for memory processing tasks
//! 
//! This module provides a service for running LLM models locally using MistralRS.
//! It supports both GGUF and SafeTensors formats and provides memory processing capabilities.

use crate::{config::*, error::*, prompts::*};
use mimir_core::MemoryClass;
use mistralrs::{
    IsqType, PagedAttentionMetaBuilder, RequestBuilder, TextMessageRole, TextMessages,
    TextModelBuilder, GgufModelBuilder, VisionModelBuilder, Model,
};
use std::time::Instant;
use tracing::{debug, info};

/// Available tasks for LLM model
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmTask {
    /// Extract memorable content from text
    Extract,
    /// Summarize memory content
    Summarize,
    /// Resolve conflicts between memories
    Resolve,
    /// Classify memory content
    Classify,
}

/// MistralRS service for memory processing
pub struct MistralRSService {
    model: Option<Model>,
    config: LlmConfig,
    prompt_manager: PromptManager,
    last_used: Option<Instant>,
}

impl MistralRSService {
    /// Create a new MistralRS service with the given configuration
    pub fn new(config: LlmConfig) -> Self {
        Self {
            model: None,
            config,
            prompt_manager: PromptManager::new(),
            last_used: None,
        }
    }

    /// Load the LLM model
    pub async fn load_model(&mut self) -> LlmResult<()> {
        let start = Instant::now();
        info!("Loading LLM model from: {}", self.config.model_path.display());

        // Check if model path exists
        if !self.config.model_path.exists() {
            return Err(LlmError::ModelLoading(format!(
                "Model path does not exist: {}. Please provide a valid model path.",
                self.config.model_path.display()
            )));
        }

        // Determine model ID
        let model_id = if let Some(id) = &self.config.model_id {
            id.clone()
        } else if let Some(model_type) = &self.config.model_type {
            model_type.default_model_id().to_string()
        } else {
            return Err(LlmError::Config("Either model_id or model_type must be set".to_string()));
        };

        info!("Using model ID: {}", model_id);

        // Build the model based on format
        let model = if self.config.use_gguf {
            // GGUF format
            info!("Loading GGUF model...");
            
            let filename = self.config.model_path.to_string_lossy().to_string();
            
            GgufModelBuilder::new(model_id, vec![filename])
                .with_paged_attn(|| PagedAttentionMetaBuilder::default().build())?
                .build()
                .await?
        } else {
            // SafeTensors format - determine builder type
            let requires_vision_builder = self.config.model_type
                .map(|mt| mt.requires_vision_builder())
                .unwrap_or(false);

            if requires_vision_builder {
                // Use VisionModelBuilder for Gemma3 models
                info!("Loading SafeTensors model with VisionModelBuilder...");
                
                // Convert quantization type
                let isq_type = match self.config.quantization {
                    QuantizationType::Q4_0 => IsqType::Q4_0,
                    QuantizationType::Q4_1 => IsqType::Q4_1,
                    QuantizationType::Q8_0 => IsqType::Q8_0,
                    QuantizationType::Q8_1 => IsqType::Q8_1,
                    QuantizationType::Q4K => IsqType::Q4K,
                    QuantizationType::None => IsqType::Q4_0, // Default fallback
                };
                
                VisionModelBuilder::new(model_id)
                    .with_paged_attn(|| PagedAttentionMetaBuilder::default().build())?
                    .build()
                    .await?
            } else {
                // Use TextModelBuilder for other models
                info!("Loading SafeTensors model with TextModelBuilder...");
                
                // Convert quantization type
                let isq_type = match self.config.quantization {
                    QuantizationType::Q4_0 => IsqType::Q4_0,
                    QuantizationType::Q4_1 => IsqType::Q4_1,
                    QuantizationType::Q8_0 => IsqType::Q8_0,
                    QuantizationType::Q8_1 => IsqType::Q8_1,
                    QuantizationType::Q4K => IsqType::Q4K,
                    QuantizationType::None => IsqType::Q4_0, // Default fallback
                };
                
                TextModelBuilder::new(model_id)
                    .with_isq(isq_type)
                    .with_paged_attn(|| PagedAttentionMetaBuilder::default().build())?
                    .build()
                    .await?
            }
        };

        self.model = Some(model);
        self.last_used = Some(Instant::now());

        info!("Model loaded successfully in {:?}", start.elapsed());
        Ok(())
    }

    /// Check if model is loaded
    pub fn is_loaded(&self) -> bool {
        self.model.is_some()
    }

    /// Extract memories from text
    pub async fn extract_memories(&mut self, text: &str) -> LlmResult<Vec<ExtractedMemory>> {
        self.ensure_loaded().await?;
        
        let prompt = self.prompt_manager.build_extract_prompt(text);
        let response = self.generate_response(&prompt).await?;
        println!("Memory Extraction Response: {}", response);
        // Parse JSON response
        let extraction: ExtractionResponse = serde_json::from_str(&response)
            .map_err(|e| LlmError::InvalidInput(format!("Failed to parse extraction response: {}", e)))?;
        
        // Convert to internal type and filter by relevance
        let memories = extraction.memories.into_iter()
            .filter(|m| m.relevance > 0.3) // Only keep memories with relevance > 0.3
            .map(|m| ExtractedMemory {
                content: m.content,
                relevance: m.relevance, // Use relevance as confidence
            })
            .collect();
        
        Ok(memories)
    }
    
    /// Summarize memory content
    pub async fn summarize_memory(&mut self, content: &str, max_tokens: usize) -> LlmResult<String> {
        self.ensure_loaded().await?;
        
        let prompt = self.prompt_manager.build_summarize_prompt(content, max_tokens);
        let response = self.generate_response(&prompt).await?;
        
        Ok(response.trim().to_string())
    }
    
    /// Resolve conflicts between memories
    pub async fn resolve_conflict(&mut self, existing: &str, new: &str, similarity: f32) -> LlmResult<ConflictResolution> {
        self.ensure_loaded().await?;
        
        let prompt = self.prompt_manager.build_resolve_prompt(existing, new, similarity);
        let response = self.generate_response(&prompt).await?;
        println!("Conflict Resolution Response: {}", response);
        // Parse JSON response
        let resolution: ConflictResolutionResponse = serde_json::from_str(&response)
            .map_err(|e| LlmError::InvalidInput(format!("Failed to parse conflict resolution response: {}", e)))?;
        
        let action = match resolution.action.as_str() {
            "MERGE" => ConflictAction::Merge,
            "REPLACE" => ConflictAction::Replace,
            "KEEP_BOTH" => ConflictAction::KeepBoth,
            "DISCARD" => ConflictAction::Discard,
            _ => return Err(LlmError::InvalidInput(format!("Unknown conflict action: {}", resolution.action))),
        };
        
        Ok(ConflictResolution {
            action,
            reason: resolution.reason,
            result: resolution.result,
        })
    }
    
    /// Classify memory content
    pub async fn classify_memory(&mut self, content: &str) -> LlmResult<MemoryClass> {
        self.ensure_loaded().await?;
        
        let prompt = self.prompt_manager.build_classify_prompt(content);
        let response = self.generate_response(&prompt).await?;
        
        let category = response.trim().to_lowercase();
        Ok(self.parse_memory_class(&category))
    }
    
    /// Summarize search results for relevance and reduced token output
    pub async fn summarize_search_results(&mut self, query: &str, results: &[String]) -> LlmResult<String> {
        self.ensure_loaded().await?;
        
        let prompt = self.prompt_manager.build_search_summary_prompt(query, results);
        let response = self.generate_response(&prompt).await?;
        
        Ok(response.trim().to_string())
    }
    
    /// Generate a response using the MistralRS model
    pub async fn generate_response(&mut self, prompt: &str) -> LlmResult<String> {
        let model = self.model.as_ref()
            .ok_or_else(|| LlmError::ModelNotLoaded)?;
        
        debug!("Generating response for prompt: {}", prompt);
        
        let start_time = Instant::now();
        
        // Create messages
        let messages = TextMessages::new()
            .add_message(TextMessageRole::User, prompt);

        // Create request with configuration
        let request = RequestBuilder::from(messages)
            .set_sampler_max_len(self.config.inference.max_tokens)
            .set_sampler_temperature(self.config.inference.temperature)
            .set_sampler_topp(self.config.inference.top_p)
            .set_sampler_frequency_penalty(self.config.inference.repeat_penalty);

        // Generate response
        let response = model.send_chat_request(request).await?;
        
        // Extract content from response
        let content = response.choices.first()
            .and_then(|choice| choice.message.content.as_ref())
            .ok_or_else(|| LlmError::Inference("No content in response".to_string()))?;
        
        let duration = start_time.elapsed();
        debug!("Generated response in {:?}: {}", duration, content);
        Ok(content.clone())
    }
    
    /// Ensure model is loaded
    async fn ensure_loaded(&mut self) -> LlmResult<()> {
        if !self.is_loaded() {
            self.load_model().await?;
        }
        Ok(())
    }
    
    /// Parse memory class from string
    fn parse_memory_class(&self, category: &str) -> MemoryClass {
        match category.to_lowercase().as_str() {
            "personal" => MemoryClass::Personal,
            "work" => MemoryClass::Work,
            "health" => MemoryClass::Health,
            "financial" => MemoryClass::Financial,
            _ => MemoryClass::Other(category.to_string()),
        }
    }
}

/// Extracted memory information
#[derive(Debug, Clone)]
pub struct ExtractedMemory {
    pub content: String,
    pub relevance: f32,
}

/// Conflict resolution result
#[derive(Debug, Clone)]
pub struct ConflictResolution {
    pub action: ConflictAction,
    pub reason: String,
    pub result: Option<String>,
}

/// Actions for resolving memory conflicts
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictAction {
    Merge,
    Replace,
    KeepBoth,
    Discard,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InferenceConfig;
    use std::path::PathBuf;
    
    fn create_test_config() -> LlmConfig {
        LlmConfig {
            model_path: PathBuf::from("test/model"),
            model_type: Some(ModelType::Gemma3_1bIt),
            use_gguf: true,
            quantization: QuantizationType::Q4_0,
            inference: InferenceConfig {
                max_tokens: 100,
                temperature: 0.7,
                top_p: 0.9,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    
    #[test]
    fn test_service_creation() {
        let config = create_test_config();
        let service = MistralRSService::new(config);
        assert!(!service.is_loaded());
    }
    
    #[test]
    fn test_memory_class_parsing() {
        let config = create_test_config();
        let service = MistralRSService::new(config);
        
        assert_eq!(service.parse_memory_class("personal"), MemoryClass::Personal);
        assert_eq!(service.parse_memory_class("work"), MemoryClass::Work);
        assert_eq!(service.parse_memory_class("health"), MemoryClass::Health);
        assert_eq!(service.parse_memory_class("financial"), MemoryClass::Financial);
        
        if let MemoryClass::Other(category) = service.parse_memory_class("custom") {
            assert_eq!(category, "custom");
        } else {
            panic!("Expected Other category");
        }
    }
    
    #[tokio::test]
    async fn test_model_loading_with_missing_file() {
        let config = create_test_config();
        let mut service = MistralRSService::new(config);
        
        // Test that we get proper error message for missing file
        let result = service.load_model().await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Model path does not exist"));
    }
} 