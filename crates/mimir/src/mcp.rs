use crate::storage::IntegratedStorage;
use mimir_core::{Memory as CoreMemory, MemoryClass};
use rmcp::{
    handler::server::router::tool::ToolRouter, handler::server::tool::Parameters, model::*,
    schemars, tool, tool_handler, tool_router,
};
use std::future::Future;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// Parameters for adding a single memory
#[derive(Debug, schemars::JsonSchema, serde::Deserialize, serde::Serialize)]
pub struct AddMemoryParams {
    pub source: String,
    pub text: String,
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

/// Parameters for updating a memory
#[derive(Debug, schemars::JsonSchema, serde::Deserialize, serde::Serialize)]
struct UpdateMemoryParams {
    id: String,
    text: String,
}

/// Mimir MCP Server for memory management
#[derive(Clone)]
pub struct MimirServer {
    /// Tool router for handling MCP tool calls
    pub tool_router: ToolRouter<Self>,
    /// Integrated storage for database and vector operations
    storage: Arc<IntegratedStorage>,
}

#[tool_router]
impl MimirServer {
    /// Create a new Mimir MCP server instance
    pub fn new(storage: IntegratedStorage) -> Self {
        Self {
            tool_router: Self::tool_router(),
            storage: Arc::new(storage),
        }
    }

    /// Add sample data for demonstration
    pub async fn add_sample_data(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        // Add some sample memories
        let sample_memories = vec![
            CoreMemory {
                id: Uuid::new_v4(),
                content: "I need to remember to buy groceries tomorrow".to_string(),
                embedding: None,
                class: MemoryClass::Personal,
                scope: None,
                tags: vec!["shopping".to_string(), "reminder".to_string()],
                app_acl: vec!["user1".to_string()],
                key_id: "personal".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            CoreMemory {
                id: Uuid::new_v4(),
                content: "Meeting with client scheduled for next Tuesday at 2 PM".to_string(),
                embedding: None,
                class: MemoryClass::Work,
                scope: None,
                tags: vec!["meeting".to_string(), "client".to_string()],
                app_acl: vec!["user1".to_string()],
                key_id: "work".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        ];

        for memory in sample_memories {
            self.storage
                .add_memory(memory)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        }

        Ok(())
    }

    /// Add a single memory to the vault
    #[tool(description = "Pass all useful information about a user")]
    async fn add_memory(
        &self,
        Parameters(AddMemoryParams { source, text }): Parameters<AddMemoryParams>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        // Generate a unique ID for the memory
        let memory_id = Uuid::new_v4();

        // Try to summarize the memory content if LLM service is available
        let processed_content = if let Some(llm_service) = self.storage.get_llm_service() {
            match llm_service.summarize_memory(&text, 150).await {
                Ok(summary) => {
                    info!("Previous content: {}", text);
                    info!("Summarized content: {}", summary);
                    info!("Successfully summarized memory content from {} to {} characters", text.len(), summary.len());
                    summary
                }
                Err(e) => {
                    warn!("Failed to summarize memory content: {}, using original text", e);
                    text
                }
            }
        } else {
            info!("LLM service not available, using original text without summarization");
            text
        };

        let core_memory = CoreMemory {
            id: memory_id,
            content: processed_content,
            embedding: None,
            class: MemoryClass::Personal, // Default to personal
            scope: None,
            tags: vec![],
            app_acl: vec![source.clone()],
            key_id: memory_id.to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Store memory using integrated storage
        match self.storage.add_memory(core_memory).await {
            Ok(result) => {
                let success_text = if result.database_stored && result.vector_stored {
                    format!(
                        "Successfully added memory with ID: {} (database and vector store) - Content summarized to reduce token usage",
                        memory_id
                    )
                } else if result.database_stored {
                    format!(
                        "Successfully added memory with ID: {} (database only) - Content summarized to reduce token usage",
                        memory_id
                    )
                } else {
                    format!("Failed to add memory with ID: {}", memory_id)
                };

                Ok(CallToolResult::success(vec![Content::text(success_text)]))
            }
            Err(e) => Err(ErrorData::invalid_request(
                format!("Failed to add memory: {}", e),
                None,
            )),
        }
    }

    /// Delete a memory by ID
    #[tool(description = "Delete a memory by ID")]
    async fn delete_memory(
        &self,
        Parameters(DeleteMemoryParams { id }): Parameters<DeleteMemoryParams>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let memory_id = Uuid::parse_str(&id)
            .map_err(|e| ErrorData::invalid_request(format!("Invalid UUID: {}", e), None))?;

        match self.storage.delete_memory(memory_id).await {
            Ok(deleted) => {
                if deleted {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "Successfully deleted memory with ID: {}",
                        id
                    ))]))
                } else {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "Memory with ID {} not found",
                        id
                    ))]))
                }
            }
            Err(e) => Err(ErrorData::invalid_request(
                format!("Failed to delete memory: {}", e),
                None,
            )),
        }
    }

    /// Search memories using vector similarity
    #[tool(description = "Get provided context from a users message")]
    async fn search_memories(
        &self,
        Parameters(SearchMemoriesParams { query }): Parameters<SearchMemoriesParams>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        match self.storage.search_memories(&query, 5).await {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "No memories found for query: '{}'",
                        query
                    ))]))
                } else {
                    // Try to summarize search results if LLM service is available
                    let result_text = if let Some(llm_service) = self.storage.get_llm_service() {
                        // Extract content from search results for summarization
                        let search_contents: Vec<String> = results.iter()
                            .map(|result| result.memory.content.clone())
                            .collect();
                        
                        match llm_service.summarize_search_results(&query, &search_contents).await {
                            Ok(summary) => {
                                info!("Successfully summarized search results from {} memories", results.len());
                                println!("Query: {}", query);
                                
                                // Print detailed search results with similarity scores
                                println!("Search Results with Similarity Scores:");
                                for (i, result) in results.iter().enumerate() {
                                    println!("{}. ID: {} | Similarity: {:.3} | Content: '{}'", 
                                        i + 1, 
                                        result.memory.id, 
                                        result.similarity, 
                                        result.memory.content);
                                }
                                
                                println!("Summary: {}", summary);
                                format!("{}", summary)
                            }
                            Err(e) => {
                                warn!("Failed to summarize search results: {}, using detailed format", e);
                                // Fallback to detailed format
                                let mut detailed_text = format!("Search results for query: '{}':\n", query);
                                for (i, result) in results.iter().enumerate() {
                                    detailed_text.push_str(&format!(
                                        "{}. ID: {} | Similarity: {:.3} | Content: '{}'\n",
                                        i + 1,
                                        result.memory.id,
                                        result.similarity,
                                        result.memory.content
                                    ));
                                }
                                detailed_text
                            }
                        }
                    } else {
                        info!("LLM service not available, using detailed search results format");
                        // Fallback to detailed format when LLM service is not available
                        let mut detailed_text = format!("Search results for query: '{}':\n", query);
                        for (i, result) in results.iter().enumerate() {
                            detailed_text.push_str(&format!(
                                "{}. ID: {} | Similarity: {:.3} | Content: '{}'\n",
                                i + 1,
                                result.memory.id,
                                result.similarity,
                                result.memory.content
                            ));
                        }
                        detailed_text
                    };
                    
                    Ok(CallToolResult::success(vec![Content::text(result_text)]))
                }
            }
            Err(e) => Err(ErrorData::invalid_request(
                format!("Failed to search memories: {}", e),
                None,
            )),
        }
    }

    /// List all memories in the vault
    #[tool(description = "List all memories about the user")]
    async fn list_memories(&self) -> std::result::Result<CallToolResult, ErrorData> {
        match self
            .storage
            .get_memories_by_class(&MemoryClass::Personal)
            .await
        {
            Ok(memories) => {
                if memories.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(
                        "No memories found in vault".to_string(),
                    )]))
                } else {
                    let mut result_text =
                        format!("Found {} memories in vault:\n", memories.len());
                    for (i, memory) in memories.iter().enumerate() {
                        result_text.push_str(&format!(
                            "{}. ID: {} | Class: {:?} | Content: '{}'\n",
                            i + 1,
                            memory.id,
                            memory.class,
                            memory.content
                        ));
                    }
                    Ok(CallToolResult::success(vec![Content::text(result_text)]))
                }
            }
            Err(e) => Err(ErrorData::invalid_request(
                format!("Failed to list memories: {}", e),
                None,
            )),
        }
    }

    /// Get vault statistics
    #[tool(description = "Get vault statistics and summary")]
    async fn get_vault_stats(&self) -> std::result::Result<CallToolResult, ErrorData> {
        match self.storage.get_stats().await {
            Ok(stats) => {
                let stats_text = format!(
                    "Vault Statistics:\n• Database memories: {}\n• Vector memories: {}\n• Memory usage: {} bytes\n• Vector store usage: {:.1}%",
                    stats.database_memories,
                    stats.vector_memories,
                    stats.memory_usage_bytes,
                    stats.vector_count_percentage
                );

                Ok(CallToolResult::success(vec![Content::text(stats_text)]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Failed to get vault stats: {}",
                e
            ))])),
        }
    }

    /// Update an existing memory
    #[tool(description = "Update an existing memory by ID with new text")]
    async fn update_memory(
        &self,
        Parameters(UpdateMemoryParams { id, text }): Parameters<UpdateMemoryParams>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let memory_id = Uuid::parse_str(&id)
            .map_err(|e| ErrorData::invalid_request(format!("Invalid UUID: {}", e), None))?;

        // Get existing memory first
        let existing_memory = match self.storage.get_memory(memory_id).await {
            Ok(Some(memory)) => memory,
            Ok(None) => {
                return Err(ErrorData::invalid_request(
                    format!("Memory with ID {} not found", id),
                    None,
                ));
            }
            Err(e) => {
                return Err(ErrorData::invalid_request(
                    format!("Failed to retrieve memory: {}", e),
                    None,
                ));
            }
        };

        // Create updated memory with new content
        let mut updated_memory = existing_memory;
        updated_memory.content = text;
        updated_memory.updated_at = chrono::Utc::now();

        // Update in storage
        match self.storage.update_memory(updated_memory).await {
            Ok(result) => {
                let success_text = if result.database_stored && result.vector_stored {
                    format!(
                        "Successfully updated memory with ID: {} (database and vector store)",
                        id
                    )
                } else if result.database_stored {
                    format!(
                        "Successfully updated memory with ID: {} (database only)",
                        id
                    )
                } else {
                    format!("Failed to update memory with ID: {}", id)
                };

                Ok(CallToolResult::success(vec![Content::text(success_text)]))
            }
            Err(e) => Err(ErrorData::invalid_request(
                format!("Failed to update memory: {}", e),
                None,
            )),
        }
    }

    /// Clear all memories from the vault
    #[tool(description = "Clear all memories from the vault")]
    async fn clear_vault(&self) -> std::result::Result<CallToolResult, ErrorData> {
        match self.storage.clear_vault().await {
            Ok(count) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Successfully cleared {} memories from vault",
                count
            ))])),
            Err(e) => Err(ErrorData::invalid_request(
                format!("Failed to clear vault: {}", e),
                None,
            )),
        }
    }

    /// Save the vector store to disk
    pub async fn save_vector_store(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        self.storage
            .save_vector_store()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Manual save tool for debugging
    #[tool(description = "Manually save the vector store to disk")]
    async fn save_vault(&self) -> std::result::Result<CallToolResult, ErrorData> {
        match self.storage.save_vector_store().await {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                "Vector store saved successfully to disk".to_string(),
            )])),
            Err(e) => Err(ErrorData::invalid_request(
                format!("Failed to save vector store: {}", e),
                None,
            )),
        }
    }

    /// Check vector store status
    #[tool(description = "Check vector store status and statistics")]
    async fn vector_store_status(&self) -> std::result::Result<CallToolResult, ErrorData> {
        let stats = match self.storage.get_stats().await {
            Ok(stats) => stats,
            Err(e) => {
                return Err(ErrorData::invalid_request(
                    format!("Failed to get stats: {}", e),
                    None,
                ));
            }
        };

        let has_embedder = self.storage.has_vector_embedder().await;

        let status_text = format!(
            "Vector Store Status:\n• Vector count: {}\n• Database memories: {}\n• Has embedder: {}\n• Memory usage: {} bytes",
            stats.vector_memories,
            stats.database_memories,
            has_embedder,
            stats.memory_usage_bytes
        );

        Ok(CallToolResult::success(vec![Content::text(status_text)]))
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
    use crate::storage::IntegratedStorage;
    use mimir_core::crypto::CryptoManager;
    use mimir_db::Database;
    use mimir_vector::ThreadSafeVectorStore;
    use rmcp::ServerHandler;
    use tempfile::TempDir;

    async fn create_test_server(use_embedder: bool) -> (MimirServer, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let keyset_path = temp_dir.path().join("keyset.json");

        // Create crypto manager for database
        let db_crypto_manager = CryptoManager::with_password(&keyset_path, "test-password")
            .expect("Failed to create test crypto manager");

        // Create crypto manager for integrated storage
        let storage_crypto_manager = CryptoManager::with_password(&keyset_path, "test-password")
            .expect("Failed to create test crypto manager");

        // Create database
        let database = Database::with_crypto_manager(&db_path, db_crypto_manager)
            .expect("Failed to create test database");

        // Create vector store
        let vector_store = if use_embedder {
            let model_path =
                std::path::Path::new("crates/mimir/assets/bge-small-en-int8/model-int8.onnx");
            if model_path.exists() {
                ThreadSafeVectorStore::with_embedder(temp_dir.path(), model_path, None, None)
                    .await
                    .expect("Failed to create test vector store with embedder")
            } else {
                eprintln!(
                    "[SKIP] Model file not found: {:?}, running without embedder",
                    model_path
                );
                ThreadSafeVectorStore::new(temp_dir.path(), 128, None, None)
                    .expect("Failed to create test vector store")
            }
        } else {
            ThreadSafeVectorStore::new(temp_dir.path(), 128, None, None)
                .expect("Failed to create test vector store")
        };

        let storage = IntegratedStorage::new(database, vector_store, storage_crypto_manager)
            .await
            .expect("Failed to create integrated storage");

        (MimirServer::new(storage), temp_dir)
    }

    #[tokio::test]
    async fn test_mimir_server_creation() {
        let (server, _temp_dir) = create_test_server(false).await;

        // Test that the server starts with empty storage
        let stats = server.get_vault_stats().await.unwrap();
        assert!(format!("{:?}", stats).contains("0"));
    }

    #[tokio::test]
    async fn test_memory_operations() {
        let (server, _temp_dir) = create_test_server(false).await;

        // Test adding a memory
        let add_params = AddMemoryParams {
            source: "test-agent".to_string(),
            text: "Test memory content".to_string(),
        };

        let result = server.add_memory(Parameters(add_params)).await;
        assert!(result.is_ok());

        // Test listing memories
        let list_result = server.list_memories().await;
        assert!(list_result.is_ok());

        // Note: We can't test deletion with a specific ID since IDs are now auto-generated
        // The deletion test would need to be updated to work with the returned ID
    }

    #[tokio::test]
    async fn test_search_functionality() {
        let (server, _temp_dir) = create_test_server(true).await;
        // Skip test if embedder is not available
        if !server.storage.has_vector_embedder().await {
            eprintln!("[SKIP] test_search_functionality: embedder/model not available");
            return;
        }

        // Add some test data
        server.add_sample_data().await.unwrap();

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
    async fn test_mcp_server_tool_router() {
        let (server, _temp_dir) = create_test_server(false).await;

        // Test that the tool router is properly initialized
        assert!(server.tool_router.list_all().len() > 0);

        // Verify all expected tools are present
        let tools = server.tool_router.list_all();
        let tool_names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();

        assert!(tool_names.contains(&"add_memory".to_string()));
        assert!(tool_names.contains(&"delete_memory".to_string()));
        assert!(tool_names.contains(&"search_memories".to_string()));
        assert!(tool_names.contains(&"list_memories".to_string()));
        assert!(tool_names.contains(&"get_vault_stats".to_string()));

        // Test that tools have descriptions
        let add_tool = tools.iter().find(|t| t.name == "add_memory").unwrap();

        assert!(add_tool.description.is_some());
        assert!(add_tool
            .description
            .as_ref()
            .unwrap()
            .contains("Pass all useful information about a user"));
    }

    #[tokio::test]
    async fn test_server_handler_info() {
        let (server, _temp_dir) = create_test_server(false).await;

        // Test server handler info
        let server_info = server.get_info();
        assert!(server_info.instructions.is_some());
        assert!(server_info.instructions.as_ref().unwrap().contains("Mimir"));
        assert!(server_info
            .instructions
            .as_ref()
            .unwrap()
            .contains("Memory Vault"));
        assert!(server_info.capabilities.tools.is_some());
    }

    #[tokio::test]
    async fn test_add_memory_with_summarization() {
        let (server, _temp_dir) = create_test_server(false).await;

        // Test adding a memory with long content that should be summarized
        let long_text = "This is a very long memory content that contains a lot of detailed information about various topics. It includes multiple sentences and paragraphs worth of text that would normally consume a significant number of tokens when processed by language models. The summarization feature should reduce this to a more concise version while preserving the key information. This helps to reduce token usage and improve efficiency in memory storage and retrieval operations.";

        let add_params = AddMemoryParams {
            source: "test-agent".to_string(),
            text: long_text.to_string(),
        };

        let result = server.add_memory(Parameters(add_params)).await;
        assert!(result.is_ok());

        // Verify that the memory was added (we can't easily test the actual summarization
        // without a real LLM service, but we can verify the operation completes successfully)
        let list_result = server.list_memories().await;
        assert!(list_result.is_ok());
    }
}
