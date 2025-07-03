use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use anyhow::Result;
use mimir_core::config::MimirConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, warn};

/// Memory entry for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

/// Simple in-memory storage implementation
#[derive(Debug, Default)]
pub struct InMemoryStorage {
    entries: Arc<RwLock<HashMap<String, MemoryEntry>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn store(&self, entry: MemoryEntry) -> Result<()> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.id.clone(), entry);
        Ok(())
    }

    pub async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let entries = self.entries.read().await;
        Ok(entries.get(id).cloned())
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        let entries = self.entries.read().await;
        
        // Return empty results for empty queries
        if query.trim().is_empty() {
            return Ok(vec![]);
        }
        
        let query_lower = query.to_lowercase();
        
        let mut results: Vec<MemoryEntry> = entries
            .values()
            .filter(|entry| {
                entry.content.to_lowercase().contains(&query_lower) ||
                entry.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect();

        // Sort by updated_at (most recent first)
        results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        results.truncate(limit);
        Ok(results)
    }

    pub async fn list_recent(&self, limit: usize) -> Result<Vec<MemoryEntry>> {
        let entries = self.entries.read().await;
        let mut results: Vec<MemoryEntry> = entries.values().cloned().collect();
        results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        results.truncate(limit);
        Ok(results)
    }

    pub async fn delete(&self, id: &str) -> Result<bool> {
        let mut entries = self.entries.write().await;
        Ok(entries.remove(id).is_some())
    }

    pub async fn store_memory(&self, content: &str, tags: Vec<String>, metadata: HashMap<String, Value>) -> Result<String> {
        let now = chrono::Utc::now();
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content: content.to_string(),
            metadata,
            created_at: now,
            updated_at: now,
            tags,
        };

        let id = entry.id.clone();
        self.store(entry).await?;
        Ok(id)
    }
}

/// MCP server for Mimir memory management
pub struct MimirMcpServer {
    config: MimirConfig,
    storage: Arc<InMemoryStorage>,
}

impl MimirMcpServer {
    pub fn new(config: MimirConfig) -> Self {
        Self {
            config,
            storage: Arc::new(InMemoryStorage::new()),
        }
    }

    pub fn with_storage(config: MimirConfig, storage: Arc<InMemoryStorage>) -> Self {
        Self {
            config,
            storage,
        }
    }

    /// Start the MCP server
    pub async fn start(&self) -> Result<()> {
        info!("Starting Mimir MCP server");
        
        // For now, we'll implement a basic server that can be extended
        // TODO: Implement proper MCP protocol handlers once we have the correct imports
        
        warn!("MCP server implementation is currently a placeholder");
        warn!("The following tools would be available:");
        warn!("- store_memory: Store a memory entry");
        warn!("- retrieve_memory: Retrieve a memory by ID");
        warn!("- search_memories: Search for memories");
        warn!("- list_recent_memories: List recent memories");
        warn!("- delete_memory: Delete a memory by ID");
        
        info!("MCP server configuration:");
        info!("  Name: {}", self.config.mcp.server_name);
        info!("  Version: {}", self.config.mcp.server_version);
        info!("  Transport: {:?}", self.config.mcp.transport);
        
        // For now, just keep the server running
        // TODO: Replace with actual MCP protocol implementation
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        
        Ok(())
    }

    /// Get the storage instance for testing
    pub fn storage(&self) -> &Arc<InMemoryStorage> {
        &self.storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::time::{sleep, Duration};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_memory_entry_creation() {
        let storage = InMemoryStorage::new();
        
        // Test basic memory creation
        let id = storage.store_memory(
            "Test memory content",
            vec!["test".to_string(), "demo".to_string()],
            HashMap::from([("source".to_string(), json!("test"))])
        ).await.unwrap();
        
        let retrieved = storage.retrieve(&id).await.unwrap();
        assert!(retrieved.is_some());
        
        let memory = retrieved.unwrap();
        assert_eq!(memory.content, "Test memory content");
        assert_eq!(memory.tags, vec!["test", "demo"]);
        assert_eq!(memory.metadata.get("source").unwrap(), &json!("test"));
        assert!(memory.created_at <= chrono::Utc::now());
        assert!(memory.updated_at <= chrono::Utc::now());
        assert_eq!(memory.created_at, memory.updated_at);
    }

    #[tokio::test]
    async fn test_memory_entry_edge_cases() {
        let storage = InMemoryStorage::new();
        
        // Test empty content
        let id = storage.store_memory("", vec![], HashMap::new()).await.unwrap();
        let retrieved = storage.retrieve(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "");
        assert!(retrieved.tags.is_empty());
        assert!(retrieved.metadata.is_empty());
        
        // Test special characters and unicode
        let special_content = "Special chars: !@#$%^&*()_+{}|:<>?[]\\;'\",./ 🚀🧠💾";
        let id = storage.store_memory(special_content, vec!["unicode".to_string()], HashMap::new()).await.unwrap();
        let retrieved = storage.retrieve(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, special_content);
        
        // Test very long content
        let long_content = "a".repeat(10000);
        let id = storage.store_memory(&long_content, vec![], HashMap::new()).await.unwrap();
        let retrieved = storage.retrieve(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, long_content);
        
        // Test complex metadata
        let complex_metadata = HashMap::from([
            ("nested".to_string(), json!({"level1": {"level2": "deep"}})),
            ("array".to_string(), json!([1, 2, 3, "mixed", true])),
            ("null".to_string(), json!(null)),
            ("number".to_string(), json!(42.5)),
            ("boolean".to_string(), json!(true)),
        ]);
        let id = storage.store_memory("Complex metadata test", vec![], complex_metadata.clone()).await.unwrap();
        let retrieved = storage.retrieve(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.metadata, complex_metadata);
    }

    #[tokio::test]
    async fn test_storage_crud_operations() {
        let storage = InMemoryStorage::new();
        
        // Test retrieve non-existent
        let result = storage.retrieve("non-existent").await.unwrap();
        assert!(result.is_none());
        
        // Test delete non-existent
        let deleted = storage.delete("non-existent").await.unwrap();
        assert!(!deleted);
        
        // Create a memory entry
        let now = chrono::Utc::now();
        let entry = MemoryEntry {
            id: "test-id".to_string(),
            content: "Test content".to_string(),
            metadata: HashMap::from([("key".to_string(), json!("value"))]),
            created_at: now,
            updated_at: now,
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };
        
        // Test direct store
        storage.store(entry.clone()).await.unwrap();
        
        // Test retrieve
        let retrieved = storage.retrieve("test-id").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_entry = retrieved.unwrap();
        assert_eq!(retrieved_entry.id, entry.id);
        assert_eq!(retrieved_entry.content, entry.content);
        assert_eq!(retrieved_entry.tags, entry.tags);
        
        // Test delete
        let deleted = storage.delete("test-id").await.unwrap();
        assert!(deleted);
        
        // Verify deletion
        let result = storage.retrieve("test-id").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_search_functionality() {
        let storage = InMemoryStorage::new();
        
        // Create test data
        let entries = vec![
            ("Rust programming tutorial", vec!["rust", "programming"], json!({"difficulty": "beginner"})),
            ("Advanced Rust concepts", vec!["rust", "advanced"], json!({"difficulty": "expert"})),
            ("Python data science", vec!["python", "data"], json!({"difficulty": "intermediate"})),
            ("Machine learning with Rust", vec!["rust", "ml"], json!({"difficulty": "advanced"})),
            ("Web development basics", vec!["web", "html"], json!({"difficulty": "beginner"})),
        ];
        
        let mut ids = Vec::new();
        for (content, tags, metadata) in entries {
            let tags: Vec<String> = tags.into_iter().map(String::from).collect();
            let metadata = HashMap::from([("meta".to_string(), metadata)]);
            let id = storage.store_memory(content, tags, metadata).await.unwrap();
            ids.push(id);
            // Add small delay to ensure different timestamps
            sleep(Duration::from_millis(1)).await;
        }
        
        // Test content search
        let results = storage.search("Rust", 10).await.unwrap();
        assert_eq!(results.len(), 3); // Should find all Rust-related entries
        
        // Test tag search
        let results = storage.search("programming", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("tutorial"));
        
        // Test case insensitive search
        let results = storage.search("RUST", 10).await.unwrap();
        assert_eq!(results.len(), 3);
        
        // Test limit functionality
        let results = storage.search("rust", 2).await.unwrap();
        assert_eq!(results.len(), 2);
        
        // Test no results
        let results = storage.search("nonexistent", 10).await.unwrap();
        assert_eq!(results.len(), 0);
        
        // Test empty query
        let results = storage.search("", 10).await.unwrap();
        assert_eq!(results.len(), 0);
        
        // Verify results are sorted by updated_at (most recent first)
        let all_results = storage.search("tutorial", 10).await.unwrap();
        if all_results.len() > 1 {
            for i in 0..all_results.len()-1 {
                assert!(all_results[i].updated_at >= all_results[i+1].updated_at);
            }
        }
    }

    #[tokio::test]
    async fn test_list_recent_functionality() {
        let storage = InMemoryStorage::new();
        
        // Test empty storage
        let results = storage.list_recent(10).await.unwrap();
        assert_eq!(results.len(), 0);
        
        // Add some entries with delays to ensure different timestamps
        let mut ids = Vec::new();
        for i in 0..5 {
            let id = storage.store_memory(
                &format!("Memory {}", i),
                vec![format!("tag{}", i)],
                HashMap::new()
            ).await.unwrap();
            ids.push(id);
            sleep(Duration::from_millis(2)).await;
        }
        
        // Test list all
        let results = storage.list_recent(10).await.unwrap();
        assert_eq!(results.len(), 5);
        
        // Verify order (most recent first)
        for i in 0..results.len()-1 {
            assert!(results[i].updated_at >= results[i+1].updated_at);
        }
        
        // Test limit
        let results = storage.list_recent(3).await.unwrap();
        assert_eq!(results.len(), 3);
        assert!(results[0].content.contains("Memory 4")); // Most recent
        
        // Test zero limit
        let results = storage.list_recent(0).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let storage = Arc::new(InMemoryStorage::new());
        let counter = Arc::new(AtomicUsize::new(0));
        
        // Spawn multiple tasks to store memories concurrently
        let mut handles = Vec::new();
        for i in 0..20 {
            let storage_clone = storage.clone();
            let counter_clone = counter.clone();
            
            let handle = tokio::spawn(async move {
                let content = format!("Concurrent memory {}", i);
                let tags = vec![format!("concurrent-{}", i % 5)];
                
                match storage_clone.store_memory(&content, tags, HashMap::new()).await {
                    Ok(_) => {
                        counter_clone.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(e) => panic!("Failed to store memory: {}", e),
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Verify all memories were stored
        assert_eq!(counter.load(Ordering::SeqCst), 20);
        
        let all_memories = storage.list_recent(25).await.unwrap();
        assert_eq!(all_memories.len(), 20);
        
        // Test concurrent reads
        let mut read_handles = Vec::new();
        let first_id = all_memories[0].id.clone();
        
        for _ in 0..10 {
            let storage_clone = storage.clone();
            let id_clone = first_id.clone();
            
            let handle = tokio::spawn(async move {
                let result = storage_clone.retrieve(&id_clone).await.unwrap();
                assert!(result.is_some());
            });
            
            read_handles.push(handle);
        }
        
        for handle in read_handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_mcp_server_creation() {
        let config = MimirConfig::default();
        let server = MimirMcpServer::new(config.clone());
        
        // Test default configuration
        assert_eq!(server.config.mcp.server_name, "mimir");
        assert_eq!(server.config.mcp.server_version, "0.1.0");
        assert!(server.config.mcp.enabled);
        assert_eq!(server.config.mcp.max_connections, 10);
        
        // Test that we can store a memory through the server
        let storage = server.storage();
        let id = storage.store_memory(
            "Server test memory",
            vec!["server".to_string()],
            HashMap::new()
        ).await.unwrap();
        
        let retrieved = storage.retrieve(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Server test memory");
    }

    #[tokio::test]
    async fn test_mcp_server_with_custom_storage() {
        let config = MimirConfig::default();
        let custom_storage = Arc::new(InMemoryStorage::new());
        
        // Pre-populate custom storage
        let id = custom_storage.store_memory(
            "Pre-existing memory",
            vec!["existing".to_string()],
            HashMap::new()
        ).await.unwrap();
        
        let server = MimirMcpServer::with_storage(config, custom_storage.clone());
        
        // Verify server uses the custom storage
        let server_storage = server.storage();
        let retrieved = server_storage.retrieve(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Pre-existing memory");
        
        // Verify it's the same storage instance
        assert!(Arc::ptr_eq(server_storage, &custom_storage));
    }

    #[tokio::test]
    async fn test_mcp_server_configuration_variants() {
        use mimir_core::config::{McpConfig, McpTransport};
        
        // Test with custom MCP configuration
        let mut config = MimirConfig::default();
        config.mcp = McpConfig {
            enabled: false,
            transport: McpTransport::Sse,
            server_name: "custom-mimir".to_string(),
            server_version: "2.0.0".to_string(),
            max_connections: 50,
        };
        
        let server = MimirMcpServer::new(config.clone());
        assert_eq!(server.config.mcp.server_name, "custom-mimir");
        assert_eq!(server.config.mcp.server_version, "2.0.0");
        assert!(!server.config.mcp.enabled);
        assert_eq!(server.config.mcp.max_connections, 50);
        
        // Test transport type
        match server.config.mcp.transport {
            McpTransport::Sse => (),
            _ => panic!("Expected SSE transport"),
        }
    }

    #[tokio::test]
    async fn test_memory_entry_serialization() {
        let entry = MemoryEntry {
            id: "test-123".to_string(),
            content: "Test content with unicode: 🚀".to_string(),
            metadata: HashMap::from([
                ("key1".to_string(), json!("value1")),
                ("key2".to_string(), json!(42)),
                ("key3".to_string(), json!({"nested": true})),
            ]),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };
        
        // Test serialization
        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(serialized.contains("test-123"));
        assert!(serialized.contains("Test content"));
        assert!(serialized.contains("🚀"));
        
        // Test deserialization
        let deserialized: MemoryEntry = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id, entry.id);
        assert_eq!(deserialized.content, entry.content);
        assert_eq!(deserialized.tags, entry.tags);
        assert_eq!(deserialized.metadata, entry.metadata);
    }

    #[tokio::test]
    async fn test_storage_edge_cases() {
        let storage = InMemoryStorage::new();
        
        // Test storing entry with same ID (should overwrite)
        let entry1 = MemoryEntry {
            id: "duplicate-id".to_string(),
            content: "First content".to_string(),
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec!["first".to_string()],
        };
        
        let entry2 = MemoryEntry {
            id: "duplicate-id".to_string(),
            content: "Second content".to_string(),
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec!["second".to_string()],
        };
        
        storage.store(entry1).await.unwrap();
        storage.store(entry2).await.unwrap();
        
        let retrieved = storage.retrieve("duplicate-id").await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Second content");
        assert_eq!(retrieved.tags, vec!["second"]);
        
        // Test search with partial matches
        let storage = InMemoryStorage::new();
        storage.store_memory("JavaScript tutorial", vec!["js".to_string()], HashMap::new()).await.unwrap();
        storage.store_memory("Java programming", vec!["java".to_string()], HashMap::new()).await.unwrap();
        
        let results = storage.search("Java", 10).await.unwrap();
        assert_eq!(results.len(), 2); // Should match both JavaScript and Java
        
        let results = storage.search("Script", 10).await.unwrap();
        assert_eq!(results.len(), 1); // Should only match JavaScript
    }

    #[tokio::test]
    async fn test_large_dataset_performance() {
        let storage = InMemoryStorage::new();
        
        // Create a reasonably large dataset
        for i in 0..1000 {
            let content = format!("Memory entry number {} with some content", i);
            let tags = vec![
                format!("tag-{}", i % 10),
                format!("category-{}", i % 5),
                "performance-test".to_string(),
            ];
            let metadata = HashMap::from([
                ("index".to_string(), json!(i)),
                ("even".to_string(), json!(i % 2 == 0)),
            ]);
            
            storage.store_memory(&content, tags, metadata).await.unwrap();
        }
        
        // Test search performance
        let start = std::time::Instant::now();
        let results = storage.search("performance-test", 50).await.unwrap();
        let search_duration = start.elapsed();
        
        assert_eq!(results.len(), 50); // Limited by the limit parameter
        assert!(search_duration.as_millis() < 100); // Should be fast
        
        // Test list_recent performance
        let start = std::time::Instant::now();
        let recent = storage.list_recent(100).await.unwrap();
        let list_duration = start.elapsed();
        
        assert_eq!(recent.len(), 100);
        assert!(list_duration.as_millis() < 50);
        
        // Verify ordering is maintained
        for i in 0..recent.len()-1 {
            assert!(recent[i].updated_at >= recent[i+1].updated_at);
        }
    }
}
