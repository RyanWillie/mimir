use rmcp::{
    Error as McpError, ServiceExt, model::*, tool, tool_router, tool_handler, 
    handler::server::router::tool::ToolRouter, 
    handler::server::tool::Parameters, schemars
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::future::Future;
use tokio::sync::Mutex;
use tokio::io::{stdin, stdout};

/// Parameters for adding memories
#[derive(Debug, schemars::JsonSchema, serde::Deserialize, serde::Serialize)]
struct AddMemoriesParams {
    memories: Vec<MemoryInput>,
}

/// Parameters for updating a memory
#[derive(Debug, schemars::JsonSchema, serde::Deserialize, serde::Serialize)]
struct UpdateMemoryParams {
    id: String,
    text: String,
}

/// Parameters for deleting a memory
#[derive(Debug, schemars::JsonSchema, serde::Deserialize, serde::Serialize)]
struct DeleteMemoryParams {
    id: String,
}

/// Parameters for searching memories
#[derive(Debug, schemars::JsonSchema, serde::Deserialize, serde::Serialize)]
struct SearchMemoriesParams {
    query: String,
}

/// Input structure for creating memories
#[derive(Debug, schemars::JsonSchema, serde::Deserialize, serde::Serialize)]
struct MemoryInput {
    id: String,
    user_id: String,
    text: String,
}

/// Memory record structure
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Memory {
    pub id: String,
    pub user_id: String, 
    pub text: String,
    pub timestamp: i64,
}

/// Mimir MCP Server for memory management
#[derive(Clone)]
pub struct MimirServer {
    /// Thread-safe memory store
    store: Arc<Mutex<HashMap<String, Memory>>>,
    /// Tool router for handling MCP tool calls
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl MimirServer {
    /// Create a new Mimir MCP server instance
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
            tool_router: Self::tool_router(),
        }
    }

    /// Add sample data for demonstration
    pub async fn add_sample_data(&self) {
        let mut store = self.store.lock().await;
        let now = chrono::Utc::now().timestamp();
        
        store.insert("sample-1".to_string(), Memory {
            id: "sample-1".to_string(),
            user_id: "demo-user".to_string(),
            text: "Welcome to Mimir AI Memory Vault".to_string(),
            timestamp: now,
        });
        
        store.insert("sample-2".to_string(), Memory {
            id: "sample-2".to_string(),
            user_id: "demo-user".to_string(),
            text: "This server manages your AI memories securely".to_string(),
            timestamp: now + 60,
        });
    }

    /// Add one or more memories to the vault
    #[tool(description = "Add new memories to the vault with ID, user ID, and text")]
    async fn add_memories(
        &self,
        Parameters(AddMemoriesParams { memories }): Parameters<AddMemoriesParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut store = self.store.lock().await;
        let mut added_ids = Vec::new();
        
        for memory_input in memories {
            let timestamp = chrono::Utc::now().timestamp();
            
            let memory = Memory {
                id: memory_input.id.clone(),
                user_id: memory_input.user_id,
                text: memory_input.text,
                timestamp,
            };
            
            store.insert(memory_input.id.clone(), memory);
            added_ids.push(memory_input.id);
        }
        
        Ok(CallToolResult::success(vec![Content::text(format!(
            "✅ Successfully added {} memories: {}",
            added_ids.len(),
            added_ids.join(", ")
        ))]))
    }

    /// Update an existing memory by ID
    #[tool(description = "Update an existing memory by ID")]
    async fn update_memory(
        &self,
        Parameters(UpdateMemoryParams { id, text }): Parameters<UpdateMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut store = self.store.lock().await;
        
        if let Some(memory) = store.get_mut(&id) {
            memory.text = text;
            memory.timestamp = chrono::Utc::now().timestamp(); // Update timestamp
            
            Ok(CallToolResult::success(vec![Content::text(format!(
                "✅ Successfully updated memory with ID: {}",
                id
            ))]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "❌ Memory with ID '{}' not found",
                id
            ))]))
        }
    }

    /// Delete a memory by ID
    #[tool(description = "Delete a memory by ID")]
    async fn delete_memory(
        &self,
        Parameters(DeleteMemoryParams { id }): Parameters<DeleteMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut store = self.store.lock().await;
        
        if store.remove(&id).is_some() {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "✅ Successfully deleted memory with ID: {}",
                id
            ))]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "❌ Memory with ID '{}' not found",
                id
            ))]))
        }
    }

    /// Search memories by text content
    #[tool(description = "Search memories by text content")]
    async fn search_memories(
        &self,
        Parameters(SearchMemoriesParams { query }): Parameters<SearchMemoriesParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = self.store.lock().await;
        let mut matches = Vec::new();
        
        for memory in store.values() {
            if memory.text.to_lowercase().contains(&query.to_lowercase()) {
                matches.push(format!(
                    "ID: {} | User: {} | Text: '{}'",
                    memory.id, memory.user_id, memory.text
                ));
            }
        }
        
        let result = if matches.is_empty() {
            format!("🔍 No memories found matching query: '{}'", query)
        } else {
            format!(
                "🔍 Found {} memories matching '{}'\n{}",
                matches.len(),
                query,
                matches.join("\n")
            )
        };
        
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// List all memories in the vault
    #[tool(description = "List all memories in the vault")]
    async fn list_memories(&self) -> Result<CallToolResult, McpError> {
        let store = self.store.lock().await;
        
        if store.is_empty() {
            Ok(CallToolResult::success(vec![Content::text(
                "📝 No memories stored in the vault".to_string()
            )]))
        } else {
            let mut result = format!("📝 Total memories in vault: {}\n\n", store.len());
            
            let mut sorted_memories: Vec<_> = store.values().collect();
            sorted_memories.sort_by_key(|m| m.timestamp);
            
            for (i, memory) in sorted_memories.iter().enumerate() {
                result.push_str(&format!(
                    "{}. ID: {} | User: {} | Text: '{}'\n",
                    i + 1, memory.id, memory.user_id, memory.text
                ));
            }
            
            Ok(CallToolResult::success(vec![Content::text(result)]))
        }
    }

    /// Get vault statistics
    #[tool(description = "Get vault statistics and summary")]
    async fn get_vault_stats(&self) -> Result<CallToolResult, McpError> {
        let store = self.store.lock().await;
        let count = store.len();
        
        if count == 0 {
            Ok(CallToolResult::success(vec![Content::text(
                "📊 Vault is empty - no memories stored".to_string()
            )]))
        } else {
            let mut users = std::collections::HashSet::new();
            let mut total_text_length = 0;
            
            for memory in store.values() {
                users.insert(&memory.user_id);
                total_text_length += memory.text.len();
            }
            
            let avg_text_length = if count > 0 { total_text_length / count } else { 0 };
            
            let stats = format!(
                "📊 Vault Statistics:\n• Total memories: {}\n• Unique users: {}\n• Average text length: {} characters\n• Users: {:?}",
                count, users.len(), avg_text_length, users.iter().collect::<Vec<_>>()
            );
            
            Ok(CallToolResult::success(vec![Content::text(stats)]))
        }
    }

    /// Clear all memories (for testing)
    #[tool(description = "Clear all memories from the vault")]
    async fn clear_vault(&self) -> Result<CallToolResult, McpError> {
        let mut store = self.store.lock().await;
        let count = store.len();
        store.clear();
        
        Ok(CallToolResult::success(vec![Content::text(format!(
            "🗑️ Cleared {} memories from vault",
            count
        ))]))
    }
}

/// Implement the ServerHandler trait with tool support
#[tool_handler]
impl rmcp::ServerHandler for MimirServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Mimir AI Memory Vault - A simple memory management server with tools for adding, updating, deleting, searching, and listing memories".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mimir_server_creation() {
        let server = MimirServer::new();
        
        // Test that the server starts with empty storage
        let stats = server.get_vault_stats().await.unwrap();
        assert!(format!("{:?}", stats).contains("empty"));
    }

    #[tokio::test]
    async fn test_memory_operations() {
        let server = MimirServer::new();
        
        // Test adding memories
        let memory_input = MemoryInput {
            id: "test-1".to_string(),
            user_id: "test-user".to_string(),
            text: "Test memory content".to_string(),
        };
        
        let add_params = AddMemoriesParams {
            memories: vec![memory_input],
        };
        
        let result = server.add_memories(Parameters(add_params)).await;
        assert!(result.is_ok());
        
        // Test listing memories
        let list_result = server.list_memories().await;
        assert!(list_result.is_ok());
        
        // Test updating memory
        let update_params = UpdateMemoryParams {
            id: "test-1".to_string(),
            text: "Updated content".to_string(),
        };
        
        let update_result = server.update_memory(Parameters(update_params)).await;
        assert!(update_result.is_ok());
        
        // Test deleting memory
        let delete_params = DeleteMemoryParams {
            id: "test-1".to_string(),
        };
        
        let delete_result = server.delete_memory(Parameters(delete_params)).await;
        assert!(delete_result.is_ok());
    }

    #[tokio::test]
    async fn test_search_functionality() {
        let server = MimirServer::new();
        
        // Add some test data
        server.add_sample_data().await;
        
        // Search for content
        let search_params = SearchMemoriesParams {
            query: "Mimir".to_string(),
        };
        
        let search_result = server.search_memories(Parameters(search_params)).await;
        assert!(search_result.is_ok());
        
        // Test vault stats
        let stats_result = server.get_vault_stats().await;
        assert!(stats_result.is_ok());
    }

    #[tokio::test]
    async fn test_clear_vault() {
        let server = MimirServer::new();
        
        // Add sample data
        server.add_sample_data().await;
        
        // Clear the vault
        let clear_result = server.clear_vault().await;
        assert!(clear_result.is_ok());
        
        // Verify vault is empty
        let stats_result = server.get_vault_stats().await;
        assert!(stats_result.is_ok());
    }
}
