//! Search Result Summarization Example
//!
//! This example demonstrates how to use the LLM service to summarize
//! search results for relevance and reduced token output.

use mimir_llm::{LlmConfig, ModelType, MistralRSService};
use std::error::Error;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging (only if not already initialized)
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mimir_llm=info")
        .try_init();

    println!("🔍 Search Result Summarization Example");

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

    // Test cases with different search scenarios
    let test_cases = vec![
        (
            "What meetings do I have scheduled?",
            vec![
                "Meeting with John tomorrow at 3pm about project deadline".to_string(),
                "Team standup meeting every Monday at 9am".to_string(),
                "Dentist appointment next Friday at 2pm".to_string(),
                "Project review meeting with Sarah on Wednesday at 1pm".to_string(),
                "Lunch with client on Thursday at 12pm".to_string(),
            ]
        ),
        (
            "What are my preferences for UI design?",
            vec![
                "I prefer dark mode interfaces and minimalist design".to_string(),
                "User prefers clean, uncluttered layouts with good contrast".to_string(),
                "I like responsive design that works on mobile devices".to_string(),
                "User mentioned preferring blue color schemes".to_string(),
                "I want intuitive navigation with clear call-to-action buttons".to_string(),
            ]
        ),
        (
            "What tasks do I need to complete?",
            vec![
                "Call John tomorrow at 3pm about the project deadline".to_string(),
                "Review the quarterly budget report by Friday".to_string(),
                "Schedule a meeting with the marketing team".to_string(),
                "Update the project documentation".to_string(),
                "Prepare presentation for next week's client meeting".to_string(),
                "Order office supplies for the team".to_string(),
            ]
        ),
        (
            "What is the weather like?",
            vec![
                "Meeting with John tomorrow at 3pm about project deadline".to_string(),
                "I prefer dark mode interfaces".to_string(),
                "The project deadline is next Friday".to_string(),
            ]
        ),
    ];

    for (query, results) in test_cases {
        println!("\n🔍 Query: '{}'", query);
        println!("📊 Found {} search results", results.len());
        
        // Show original results
        println!("\n📋 Original Results:");
        for (i, result) in results.iter().enumerate() {
            println!("  {}. {}", i + 1, result);
        }
        
        // Summarize results
        println!("\n📝 Summarizing results...");
        let start_time = Instant::now();
        match service.summarize_search_results(query, &results).await {
            Ok(summary) => {
                let duration = start_time.elapsed();
                println!("✅ Summary (took {:.2?}):", duration);
                println!("{}", summary);
                
                // Show token reduction
                let original_tokens = results.iter().map(|r| r.len()).sum::<usize>();
                let summary_tokens = summary.len();
                let reduction = if original_tokens > summary_tokens {
                    ((original_tokens - summary_tokens) as f32 / original_tokens as f32) * 100.0
                } else {
                    0.0
                };
                println!("📊 Token reduction: {:.1}% ({} → {} chars)", reduction, original_tokens, summary_tokens);
            }
            Err(e) => {
                let duration = start_time.elapsed();
                println!("❌ Failed after {:.2?}: {}", duration, e);
            }
        }
    }

    println!("\n✅ Search summarization test completed!");
    
    Ok(())
} 