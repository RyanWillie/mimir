//! Simple Gemma3 test with MistralRSService
//!
//! This example shows a minimal test case for Gemma3 model generation using the new MistralRSService.

use mimir_llm::{LlmConfig, ModelType, QuantizationType, MistralRSService};
use std::error::Error;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging (only if not already initialized)
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mimir_llm=debug")
        .try_init();

    println!("🚀 Simple Gemma3 Test with MistralRSService");

    // Create config with a shorter context and very limited generation
    let config = LlmConfig::new()
        .with_model_path("/Users/ryanwilliamson/Library/Application Support/Mimir/models/gemma-3-1b-it-standard")
        .with_model_type(ModelType::Gemma3_1bIt)
        .with_gguf(false)
        .with_temperature(0.7);
    
    println!("📁 Model path: {}", config.model_path.display());
    
    // Create service
    let mut service = MistralRSService::new(config);
    
    // Load the model first
    println!("\nLoading model...");
    match service.load_model().await {
        Ok(_) => println!("✅ Model loaded successfully"),
        Err(e) => {
            println!("❌ Failed to load model: {}", e);
            return Err(e.into());
        }
    }
    
    // Test with increasingly longer prompts to find the breaking point
    let test_prompts = vec![
        ("Short", "Hello"),
        ("Medium", "Hello, how are you today? I hope you are doing well."),
        ("Long", "Hello, how are you today? I hope you are doing well. This is a longer test to see if we can generate text with more context. The model should be able to handle this length of input without issues."),
        ("Very Long", "Hello, how are you today? I hope you are doing well. This is a longer test to see if we can generate text with more context. The model should be able to handle this length of input without issues. Let me add even more text here to make it longer and see where the breaking point is. This should be getting close to the limit now. I wonder if this will work or if we will see the same shape mismatch error that we encountered before."),
    ];
    
    for (name, prompt) in test_prompts {
        println!("\n🔍 Testing {} prompt (length: {} chars)", name, prompt.len());
        let display_prompt = if prompt.len() > 50 { 
            format!("{}...", &prompt[..50]) 
        } else { 
            prompt.to_string() 
        };
        println!("Prompt: '{}'", display_prompt);
        
        let start_time = Instant::now();
        match service.generate_response(prompt).await {
            Ok(response) => {
                let duration = start_time.elapsed();
                println!("✅ {} generation success! (took {:.2?})", name, duration);
                println!("Response: '{}'", response);
            }
            Err(e) => {
                let duration = start_time.elapsed();
                println!("❌ {} generation failed after {:.2?}: {}", name, duration, e);
                println!("Error details: {:?}", e);
                
                // If we hit an error, stop testing longer prompts
                break;
            }
        }
    }

    println!("\n✅ Test completed!");
    
    Ok(())
} 