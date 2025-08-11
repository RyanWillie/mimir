//! Memory client for communicating with Mimir daemon

use mimir_core::{Config, Memory, MemoryClass, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Memory search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub memory: Memory,
    pub similarity: f32,
    pub distance: f32,
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_memories: usize,
    pub memories_by_class: std::collections::HashMap<MemoryClass, usize>,
    pub storage_size_bytes: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Connection status
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error { message: String },
}

/// Memory client for communicating with Mimir daemon
pub struct MemoryClient {
    config: Config,
    http_client: Client,
    connection_status: ConnectionStatus,
    base_url: String,
}

impl MemoryClient {
    /// Create a new memory client
    pub fn new(config: Config) -> Result<Self> {
        let base_url = format!("http://localhost:{}", config.server.port);
        
        let http_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to create HTTP client: {}", e)
            ))?;

        Ok(Self {
            config,
            http_client,
            connection_status: ConnectionStatus::Disconnected,
            base_url,
        })
    }

    /// Check connection to the daemon
    pub async fn check_connection(&mut self) -> bool {
        self.connection_status = ConnectionStatus::Connecting;
        
        let health_url = format!("{}/health", self.base_url);
        match self.http_client.get(&health_url).timeout(Duration::from_secs(2)).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    self.connection_status = ConnectionStatus::Connected;
                    true
                } else {
                    self.connection_status = ConnectionStatus::Error {
                        message: format!("Health check failed with status: {}", response.status()),
                    };
                    false
                }
            }
            Err(e) => {
                self.connection_status = ConnectionStatus::Error {
                    message: format!("Connection failed: {}", e),
                };
                false
            }
        }
    }

    /// Get connection status
    pub fn get_connection_status(&self) -> ConnectionStatus {
        self.connection_status.clone()
    }

    /// Search memories
    pub async fn search_memories(&self, query: &str, limit: usize) -> Result<Vec<MemorySearchResult>> {
        let url = format!("{}/memories/search", self.base_url);
        
        let request_body = serde_json::json!({
            "query": query,
            "limit": limit
        });

        let response = self.http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to search memories: {}", e)
            ))?;

        if !response.status().is_success() {
            return Err(mimir_core::MimirError::ServerError(
                format!("Search request failed with status: {}", response.status())
            ));
        }

        let results: Vec<MemorySearchResult> = response.json().await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to parse search results: {}", e)
            ))?;

        Ok(results)
    }

    /// Get memories by class
    pub async fn get_memories_by_class(&self, class: &MemoryClass) -> Result<Vec<Memory>> {
        let class_str = match class {
            MemoryClass::Personal => "personal",
            MemoryClass::Work => "work",
            MemoryClass::Health => "health",
            MemoryClass::Financial => "financial",
            MemoryClass::Other(ref s) => s,
        };
        let url = format!("{}/memories/class/{}", self.base_url, class_str);
        
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to get memories by class: {}", e)
            ))?;

        if !response.status().is_success() {
            return Err(mimir_core::MimirError::ServerError(
                format!("Get memories request failed with status: {}", response.status())
            ));
        }

        let memories: Vec<Memory> = response.json().await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to parse memories: {}", e)
            ))?;

        Ok(memories)
    }

    /// Get all memories
    pub async fn get_all_memories(&self, limit: Option<usize>) -> Result<Vec<Memory>> {
        let mut url = format!("{}/memories", self.base_url);
        if let Some(limit) = limit {
            url.push_str(&format!("?limit={}", limit));
        }
        
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to get all memories: {}", e)
            ))?;

        if !response.status().is_success() {
            return Err(mimir_core::MimirError::ServerError(
                format!("Get all memories request failed with status: {}", response.status())
            ));
        }

        let memories: Vec<Memory> = response.json().await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to parse memories: {}", e)
            ))?;

        Ok(memories)
    }

    /// Add a memory
    pub async fn add_memory(&self, memory: &Memory) -> Result<()> {
        let url = format!("{}/memories", self.base_url);
        
        let response = self.http_client
            .post(&url)
            .json(memory)
            .send()
            .await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to add memory: {}", e)
            ))?;

        if !response.status().is_success() {
            return Err(mimir_core::MimirError::ServerError(
                format!("Add memory request failed with status: {}", response.status())
            ));
        }

        info!("Successfully added memory: {}", memory.id);
        Ok(())
    }

    /// Update a memory
    pub async fn update_memory(&self, memory: &Memory) -> Result<()> {
        let url = format!("{}/memories/{}", self.base_url, memory.id);
        
        let response = self.http_client
            .put(&url)
            .json(memory)
            .send()
            .await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to update memory: {}", e)
            ))?;

        if !response.status().is_success() {
            return Err(mimir_core::MimirError::ServerError(
                format!("Update memory request failed with status: {}", response.status())
            ));
        }

        info!("Successfully updated memory: {}", memory.id);
        Ok(())
    }

    /// Delete a memory
    pub async fn delete_memory(&self, memory_id: &str) -> Result<bool> {
        let url = format!("{}/memories/{}", self.base_url, memory_id);
        
        let response = self.http_client
            .delete(&url)
            .send()
            .await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to delete memory: {}", e)
            ))?;

        match response.status().as_u16() {
            200 => {
                info!("Successfully deleted memory: {}", memory_id);
                Ok(true)
            }
            404 => {
                info!("Memory not found: {}", memory_id);
                Ok(false)
            }
            _ => {
                Err(mimir_core::MimirError::ServerError(
                    format!("Delete memory request failed with status: {}", response.status())
                ))
            }
        }
    }

    /// Get memory statistics
    pub async fn get_stats(&self) -> Result<MemoryStats> {
        let url = format!("{}/memories/stats", self.base_url);
        
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to get memory stats: {}", e)
            ))?;

        if !response.status().is_success() {
            return Err(mimir_core::MimirError::ServerError(
                format!("Get stats request failed with status: {}", response.status())
            ));
        }

        let stats: MemoryStats = response.json().await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to parse memory stats: {}", e)
            ))?;

        Ok(stats)
    }

    /// Get vault status
    pub async fn get_vault_status(&self) -> Result<String> {
        let url = format!("{}/vault/status", self.base_url);
        
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to get vault status: {}", e)
            ))?;

        if !response.status().is_success() {
            return Err(mimir_core::MimirError::ServerError(
                format!("Get vault status request failed with status: {}", response.status())
            ));
        }

        let status: serde_json::Value = response.json().await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to parse vault status: {}", e)
            ))?;

        Ok(status["status"].as_str().unwrap_or("Unknown").to_string())
    }

    /// Clear all memories
    pub async fn clear_vault(&self) -> Result<usize> {
        let url = format!("{}/memories", self.base_url);
        
        let response = self.http_client
            .delete(&url)
            .send()
            .await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to clear vault: {}", e)
            ))?;

        if !response.status().is_success() {
            return Err(mimir_core::MimirError::ServerError(
                format!("Clear vault request failed with status: {}", response.status())
            ));
        }

        let result: serde_json::Value = response.json().await
            .map_err(|e| mimir_core::MimirError::ServerError(
                format!("Failed to parse clear vault result: {}", e)
            ))?;

        let deleted_count = result["deleted_count"].as_u64().unwrap_or(0) as usize;
        info!("Successfully cleared vault, deleted {} memories", deleted_count);
        Ok(deleted_count)
    }
} 