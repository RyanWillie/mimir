//! Model swapping example using MistralRSService
//!
//! This example demonstrates how to easily swap between different model types:
//! - Gemma3 models (using VisionModelBuilder)
//! - Qwen models (using TextModelBuilder)
//! - Custom model IDs

use mimir_llm::{LlmConfig, ModelType, QuantizationType, MistralRSService};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging (only if not already initialized)
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mimir_llm=info")
        .try_init();

    println!("🚀 Model Swapping Example with MistralRSService");

    // Example 1: Gemma3 model (uses VisionModelBuilder)
    println!("\n=== Example 1: Gemma3 Model ===");
    let gemma_config = LlmConfig::new()
        .with_model_path("/Users/ryanwilliamson/Library/Application Support/Mimir/models/gemma-3-1b-it-standard")
        .with_model_type(ModelType::Gemma3_1bIt)
        .with_gguf(false)
        .with_quantization(QuantizationType::Q4_0)
        .with_temperature(0.7)
        .with_max_tokens(50);
    
    println!("📁 Model path: {}", gemma_config.model_path.display());
    println!("🤖 Model type: {:?}", gemma_config.model_type.unwrap());
    println!("🔧 Builder: VisionModelBuilder (required for Gemma3)");
    
    let mut gemma_service = MistralRSService::new(gemma_config);
    
    // Note: We're not actually loading the model here since it might not exist
    // In a real scenario, you would load it and test it
    println!("✅ Gemma3 configuration ready (model not loaded for demo)");

    // Example 2: Qwen model (uses TextModelBuilder)
    println!("\n=== Example 2: Qwen Model ===");
    let qwen_config = LlmConfig::new()
        .with_model_path("/Users/ryanwilliamson/Library/Application Support/Mimir/models/qwen-0.6b")
        .with_model_type(ModelType::Qwen06b)
        .with_gguf(false)
        .with_quantization(QuantizationType::Q4_0)
        .with_temperature(0.7)
        .with_max_tokens(50);
    
    println!("📁 Model path: {}", qwen_config.model_path.display());
    println!("🤖 Model type: {:?}", qwen_config.model_type.unwrap());
    println!("🔧 Builder: TextModelBuilder (standard for Qwen)");
    
    let mut qwen_service = MistralRSService::new(qwen_config);
    println!("✅ Qwen configuration ready (model not loaded for demo)");

    // Example 3: Custom model ID
    println!("\n=== Example 3: Custom Model ID ===");
    let custom_config = LlmConfig::new()
        .with_model_path("/Users/ryanwilliamson/Library/Application Support/Mimir/models/custom-model")
        .with_model_id("microsoft/DialoGPT-medium".to_string()) // Override with custom model ID
        .with_gguf(false)
        .with_quantization(QuantizationType::Q4_0)
        .with_temperature(0.7)
        .with_max_tokens(50);
    
    println!("📁 Model path: {}", custom_config.model_path.display());
    println!("🆔 Model ID: {}", custom_config.model_id.as_ref().unwrap());
    println!("🔧 Builder: TextModelBuilder (default for unknown models)");
    
    let mut custom_service = MistralRSService::new(custom_config);
    println!("✅ Custom model configuration ready (model not loaded for demo)");

    // Example 4: GGUF model
    println!("\n=== Example 4: GGUF Model ===");
    let gguf_config = LlmConfig::new()
        .with_model_path("/Users/ryanwilliamson/Library/Application Support/Mimir/models/gemma-3-1b-it-qat-q4_0.gguf")
        .with_model_type(ModelType::Gemma3_1bIt)
        .with_gguf(true) // Use GGUF format
        .with_temperature(0.7)
        .with_max_tokens(50);
    
    println!("📁 Model path: {}", gguf_config.model_path.display());
    println!("🤖 Model type: {:?}", gguf_config.model_type.unwrap());
    println!("🔧 Builder: GgufModelBuilder (for GGUF format)");
    
    let mut gguf_service = MistralRSService::new(gguf_config);
    println!("✅ GGUF configuration ready (model not loaded for demo)");

    // Demonstrate the helper methods
    println!("\n=== Model Type Helper Methods ===");
    let model_types = vec![
        ModelType::Gemma3_1bIt,
        ModelType::Qwen06b,
        ModelType::Llama27b,
    ];

    for model_type in model_types {
        println!("🤖 {:?}:", model_type);
        println!("   Default ID: {}", model_type.default_model_id());
        println!("   Requires Vision Builder: {}", model_type.requires_vision_builder());
        println!("   Is Gemma: {}", model_type.is_gemma());
        println!("   Is Qwen: {}", model_type.is_qwen());
        println!("   Is Llama: {}", model_type.is_llama());
    }

    println!("\n✅ Model swapping example completed!");
    println!("\n💡 Key Benefits:");
    println!("   • Easy model swapping by changing ModelType");
    println!("   • Automatic builder selection (VisionModelBuilder for Gemma3)");
    println!("   • Custom model ID support for any HuggingFace model");
    println!("   • GGUF and SafeTensors format support");
    println!("   • Helper methods for model type detection");
    
    Ok(())
} 