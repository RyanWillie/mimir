//! Test memory extraction with relevance scoring
//!
//! This example tests the improved memory extraction that properly evaluates
//! whether user inputs are worth remembering based on relevance.

use mimir_llm::{LlmConfig, ModelType, QuantizationType, MistralRSService};
use std::error::Error;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging (only if not already initialized)
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mimir_llm=info")
        .try_init();

    println!("🧠 Testing Memory Extraction with Relevance Scoring");

    // Create config
    let config = LlmConfig::new()
        .with_model_path("/Users/ryanwilliamson/Library/Application Support/Mimir/models/gemma-3-1b-it-standard")
        .with_model_type(ModelType::Gemma3_1bIt)
        .with_gguf(false)
        .with_temperature(0.7);
    
    // Create service
    let mut service = MistralRSService::new(config);
    
    // Load the model
    println!("\nLoading model...");
    service.load_model().await?;
    println!("✅ Model loaded successfully");

    // Test cases with different relevance levels
    let test_cases = vec![
        ("High Relevance - Goal", "I want to build this to be a robust system that can handle complex memory operations"),
        ("High Relevance - Task", "I need to call John tomorrow at 3pm about the project deadline"),
        ("High Relevance - Preference", "I prefer dark mode interfaces and minimalist design"),
        ("Medium Relevance - Information", "The meeting is scheduled for next Tuesday"),
        ("Low Relevance - Clarification", "Can you explain that again?"),
        ("Low Relevance - Acknowledgment", "Thanks for your help"),
        ("No Relevance - Greeting", "Hello"),
        ("No Relevance - Casual", "How's it going?"),
    ];

    for (test_name, input) in test_cases {
        println!("\n🔍 Testing: {}", test_name);
        println!("Input: '{}'", input);
        
        let start_time = Instant::now();
        match service.extract_memories(input).await {
            Ok(memories) => {
                let duration = start_time.elapsed();
                if memories.is_empty() {
                    println!("✅ No memories extracted (correctly filtered out) - took {:.2?}", duration);
                } else {
                    println!("✅ Extracted {} memories in {:.2?}:", memories.len(), duration);
                    for (i, memory) in memories.iter().enumerate() {
                        println!("  {}. {} (relevance: {:.2})", 
                            i + 1, memory.content, memory.relevance);
                    }
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();
                println!("❌ Failed after {:.2?}: {}", duration, e);
            }
        }
    }

    println!("\n✅ Relevance test completed!");
    
    Ok(())
} 