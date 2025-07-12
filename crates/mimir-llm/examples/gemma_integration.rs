//! Example demonstrating Gemma3 integration with Mimir
//!
//! This example shows how to use the Gemma3 model for memory processing tasks
//! including extraction, summarization, conflict resolution, and classification.

use mimir_llm::{GemmaConfig, GemmaService};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mimir_llm=info")
        .init();

    println!("🚀 Gemma3 Integration Example");

    // Create default configuration (uses downloaded model)
    let config = GemmaConfig::default();
    
    // Or customize the configuration
    let _custom_config = GemmaConfig::new()
        .with_threads(4)
        .with_temperature(0.7)
        .with_context_length(2048)
        .with_max_tokens(150);
    
    println!("📁 Model path: {}", config.model_path.display());
    
    // Create service with default config
    let mut service = GemmaService::new(config);
    
    // Example 1: Memory extraction
    println!("\n🔍 Example 1: Memory Extraction");
    let conversation = "I need to remember to call mom tomorrow at 3 PM and also pick up the dry cleaning on my way home from work.";
    
    let memories = service.extract_memories(conversation).await?;
    println!("Extracted {} memories:", memories.len());
    for (i, memory) in memories.iter().enumerate() {
        println!("  {}. {}", i + 1, memory.content);
    }

    // Example 2: Memory summarization
    println!("\n📝 Example 2: Memory Summarization");
    let long_memory = "Today I had a very long and detailed discussion with my team about the upcoming project. We talked about the requirements, the timeline, the budget constraints, the technical challenges we might face, the resources we'll need, and the potential risks. The meeting lasted for 2 hours and we covered a lot of ground. We also discussed the stakeholder expectations and the success metrics we need to track.";
    
    let summary = service.summarize_memory(long_memory, 100).await?;
    println!("Original length: {} chars", long_memory.len());
    println!("Summary: {}", summary);

    // Example 3: Conflict resolution
    println!("\n⚖️ Example 3: Conflict Resolution");
    let existing_memory = "Meeting with Sarah scheduled for Tuesday at 3 PM";
    let new_memory = "Meeting with Sarah moved to Wednesday at 2 PM";
    
    let resolution = service.resolve_conflict(existing_memory, new_memory, 0.85).await?;
    println!("Existing: {}", existing_memory);
    println!("New: {}", new_memory);
    println!("Resolution action: {:?}", resolution.action);
    println!("Reason: {}", resolution.reason);
    if let Some(result) = resolution.result {
        println!("Result: {}", result);
    }

    // Example 4: Memory classification
    println!("\n🏷️ Example 4: Memory Classification");
    let memories_to_classify = [
        "Doctor appointment on Friday at 10 AM",
        "Quarterly sales report due next Monday", 
        "Buy groceries: milk, bread, eggs",
        "Investment portfolio review with financial advisor",
    ];
    
    for memory in memories_to_classify {
        let classification = service.classify_memory(memory).await?;
        println!("Memory: \"{}\" -> Class: {:?}", memory, classification);
    }

    // Example 5: Custom configuration
    println!("\n🎛️ Example 5: Custom Configuration");
    let high_creativity_config = GemmaConfig::new()
        .with_temperature(0.9)
        .with_top_p(0.95)
        .with_max_tokens(200);
    
    let _creative_service = GemmaService::new(high_creativity_config);

    println!("\n✅ All examples completed successfully!");
    
    Ok(())
} 