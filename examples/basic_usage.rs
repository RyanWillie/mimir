//! Basic usage example for Mimir Memory Vault
//! 
//! This example demonstrates how to:
//! 1. Start the Mimir daemon
//! 2. Connect a client 
//! 3. Store and retrieve memories
//! 4. Handle different memory classes

use mimir_sdk::MemoryClient;
use mimir_core::{MemoryIngestion, MemoryQuery, MemoryClass};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 Mimir Basic Usage Example");
    
    // Create a memory client for our application
    let client = MemoryClient::new("http://localhost:8100", "example-app");
    
    // Check if the daemon is healthy
    println!("🔍 Checking daemon health...");
    match client.health().await {
        Ok(true) => println!("✅ Daemon is healthy"),
        Ok(false) => println!("⚠️ Daemon is not healthy"),
        Err(e) => {
            println!("❌ Failed to connect to daemon: {}", e);
            println!("💡 Make sure to start the daemon first: mimir start");
            return Ok(());
        }
    }
    
    // Example 1: Store personal memories
    println!("\n📝 Storing personal memories...");
    let personal_memory = MemoryIngestion {
        content: "I love Ethiopian single-origin coffee, especially from the Yirgacheffe region".to_string(),
        class: Some(MemoryClass::Personal),
        scope: Some("preferences".to_string()),
        tags: vec!["coffee".to_string(), "food".to_string()],
        app_id: "example-app".to_string(),
    };
    
    client.ingest(personal_memory).await?;
    println!("✅ Stored personal coffee preference");
    
    // Example 2: Store work-related memory
    println!("📝 Storing work memory...");
    let work_memory = MemoryIngestion {
        content: "Team standup is every Tuesday at 10 AM in the main conference room".to_string(),
        class: Some(MemoryClass::Work),
        scope: Some("meetings".to_string()),
        tags: vec!["schedule".to_string(), "team".to_string()],
        app_id: "example-app".to_string(),
    };
    
    client.ingest(work_memory).await?;
    println!("✅ Stored work schedule information");
    
    // Example 3: Store health information
    println!("📝 Storing health information...");
    let health_memory = MemoryIngestion {
        content: "I'm allergic to penicillin and shellfish. Always check before prescribing antibiotics.".to_string(),
        class: Some(MemoryClass::Health),
        scope: None,
        tags: vec!["allergies".to_string(), "medication".to_string()],
        app_id: "example-app".to_string(),
    };
    
    client.ingest(health_memory).await?;
    println!("✅ Stored health allergy information");
    
    // Example 4: Retrieve relevant memories
    println!("\n🔍 Retrieving memories...");
    
    // Query for coffee-related memories
    let coffee_query = MemoryQuery {
        query: "What kind of coffee do I like?".to_string(),
        class_filter: Some(vec![MemoryClass::Personal]),
        scope_filter: None,
        app_id: "example-app".to_string(),
        top_k: 3,
    };
    
    let coffee_results = client.retrieve(coffee_query).await?;
    println!("☕ Found {} coffee-related memories", coffee_results.len());
    
    // Query for work schedule
    let schedule_query = MemoryQuery {
        query: "When is the team meeting?".to_string(),
        class_filter: Some(vec![MemoryClass::Work]),
        scope_filter: Some("meetings".to_string()),
        app_id: "example-app".to_string(),
        top_k: 5,
    };
    
    let schedule_results = client.retrieve(schedule_query).await?;
    println!("📅 Found {} schedule-related memories", schedule_results.len());
    
    // Query for medical information
    let medical_query = MemoryQuery {
        query: "What are my medication allergies?".to_string(),
        class_filter: Some(vec![MemoryClass::Health]),
        scope_filter: None,
        app_id: "example-app".to_string(),
        top_k: 3,
    };
    
    let medical_results = client.retrieve(medical_query).await?;
    println!("🏥 Found {} medical memories", medical_results.len());
    
    println!("\n🎉 Example completed successfully!");
    println!("💡 Try running the CLI commands:");
    println!("   safe-memory status");
    println!("   safe-memory burn personal  # (with confirmation)");
    
    Ok(())
} 