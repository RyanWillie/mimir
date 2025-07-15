//! Memory processing example using MistralRSService
//!
//! This example demonstrates how to use the MistralRSService for memory processing tasks:
//! - Extracting memories from text
//! - Summarizing memories
//! - Resolving conflicts
//! - Classifying memories

use mimir_llm::{LlmConfig, ModelType, QuantizationType, MistralRSService};
use std::error::Error;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging (only if not already initialized)
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mimir_llm=info")
        .try_init();

    println!("🚀 Memory Processing Example with MistralRSService");

    // Create config
    let config = LlmConfig::new()
        .with_model_path("/Users/ryanwilliamson/Library/Application Support/Mimir/models/gemma-3-1b-it-standard")
        .with_model_type(ModelType::Gemma3_1bIt)
        .with_gguf(false)
        .with_quantization(QuantizationType::Q4_0)
        .with_temperature(0.7)
        .with_max_tokens(200);
    
    println!("📁 Model path: {}", config.model_path.display());
    
    // Create service
    let mut service = MistralRSService::new(config);
    
    // Load the model
    println!("\nLoading model...");
    service.load_model().await?;
    println!("✅ Model loaded successfully");

    // Example conversation text
    let conversation = r#"
    I’m having trouble getting gemma3 1B to work with candle core, should this be straight forward?
    "#;

    println!("\n📝 Processing conversation:");
    println!("{}", conversation);

    // 1. Extract memories
    println!("\n🔍 Extracting memories...");
    let start_time = Instant::now();
    match service.extract_memories(conversation).await {
        Ok(memories) => {
            let duration = start_time.elapsed();
            println!("✅ Extracted {} memories in {:.2?}:", memories.len(), duration);
            for (i, memory) in memories.iter().enumerate() {
                println!("  {}. {} (relevance: {:.2})", 
                    i + 1, memory.content, memory.relevance);
            }
        }
        Err(e) => {
            let duration = start_time.elapsed();
            println!("❌ Failed to extract memories after {:.2?}: {}", duration, e);
        }
    }

    // 2. Summarize a memory
    println!("\n📝 Summarizing memory...");
    let long_memory = "I need to call John tomorrow at 3pm about the project deadline which is next Friday, and I also need to pick up groceries on my way home today, and I should also schedule a dentist appointment for next month.";
    let test_message = "I think I want to use gemma3 to drive the internal mechanisms of the memory store, dealing with memory conflict resolution, merging, summarising etc. what does this look like for me?";
    let next_message = "Shared detailed YouTube transcript about implementing four-type memory systems for AI agents (working, episodic, semantic, procedural memory) by Adam LK - comprehensive guide covering cognitive architectures, memory frameworks, and practical implementation approaches for agentic language model systems";
    let start_time = Instant::now();
    match service.summarize_memory(next_message, 50).await {
        Ok(summary) => {
            let duration = start_time.elapsed();
            println!("✅ Summary (took {:.2?}): {}", duration, summary);
        }
        Err(e) => {
            let duration = start_time.elapsed();
            println!("❌ Failed to summarize after {:.2?}: {}", duration, e);
        }
    }

    // 3. Classify a memory
    println!("\n🏷️ Classifying memory...");
    let memory_to_classify = "Meeting with the team tomorrow at 2pm to discuss Q4 budget";
    let other_message = "User weighs 80kg";
    let start_time = Instant::now();
    match service.classify_memory(other_message).await {
        Ok(class) => {
            let duration = start_time.elapsed();
            println!("✅ Classified as {:?} (took {:.2?})", class, duration);
        }
        Err(e) => {
            let duration = start_time.elapsed();
            println!("❌ Failed to classify after {:.2?}: {}", duration, e);
        }
    }

    // 4. Resolve a conflict
    println!("\n⚖️ Resolving memory conflict...");
    let existing_memory = "Meeting with John tomorrow at 3pm about project deadline";
    let new_memory = "Meeting with Alice on Thursday at 4pm for coffee";
    let similarity = 0.85;
    
    let start_time = Instant::now();
    match service.resolve_conflict(existing_memory, new_memory, similarity).await {
        Ok(resolution) => {
            let duration = start_time.elapsed();
            println!("✅ Resolution (took {:.2?}): {:?}", duration, resolution.action);
            println!("   Reason: {}", resolution.reason);
            if let Some(result) = resolution.result {
                println!("   Result: {}", result);
            }
        }
        Err(e) => {
            let duration = start_time.elapsed();
            println!("❌ Failed to resolve conflict after {:.2?}: {}", duration, e);
        }
    }

    println!("\n✅ Memory processing example completed!");
    
    Ok(())
} 