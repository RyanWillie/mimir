//! Mimir Tray - System tray UI for the memory vault
//!
//! Licensed under AGPL-3.0 to keep derivative UIs open-source

use mimir_core::{Config, Result};
use std::sync::Arc;
use tokio::sync::Mutex;

mod service_manager;
mod memory_client;
mod config_manager;

pub use service_manager::{ServiceManager, ServiceStatus};
pub use memory_client::MemoryClient;
pub use config_manager::ConfigManager;

/// System tray application
pub struct TrayApp {
    service_manager: Arc<Mutex<ServiceManager>>,
    memory_client: Arc<MemoryClient>,
    config_manager: Arc<ConfigManager>,
}

impl TrayApp {
    /// Create a new tray application
    pub fn new() -> Result<Self> {
        let config = Config::load().unwrap_or_else(|_| Config::default());
        
        let service_manager = Arc::new(Mutex::new(ServiceManager::new(config.clone())?));
        let memory_client = Arc::new(MemoryClient::new(config.clone())?);
        let config_manager = Arc::new(ConfigManager::new(config)?);

        Ok(Self {
            service_manager,
            memory_client,
            config_manager,
        })
    }

    /// Run the tray application
    pub async fn run(self) -> Result<()> {
        // For now, just keep the application running
        // In the future, this will integrate with Tauri
        println!("Mimir Tray started. Press Ctrl+C to exit.");
        
        // Keep the application running
        tokio::signal::ctrl_c().await
            .map_err(|e| mimir_core::MimirError::ServerError(format!("Signal error: {}", e)))?;
        
        println!("Shutting down Mimir Tray...");
        Ok(())
    }

    /// Get a reference to the service manager
    pub fn service_manager(&self) -> Arc<Mutex<ServiceManager>> {
        self.service_manager.clone()
    }

    /// Get a reference to the memory client
    pub fn memory_client(&self) -> Arc<MemoryClient> {
        self.memory_client.clone()
    }

    /// Get a reference to the config manager
    pub fn config_manager(&self) -> Arc<ConfigManager> {
        self.config_manager.clone()
    }

    /// Start the Mimir daemon
    pub async fn start_daemon(&self) -> Result<()> {
        let mut manager = self.service_manager.lock().await;
        manager.start_daemon().await
    }

    /// Stop the Mimir daemon
    pub async fn stop_daemon(&self) -> Result<()> {
        let mut manager = self.service_manager.lock().await;
        manager.stop_daemon().await
    }

    /// Get the current service status
    pub async fn get_service_status(&self) -> ServiceStatus {
        let mut manager = self.service_manager.lock().await;
        manager.update_status().await;
        manager.get_status()
    }

    /// Check if the daemon is running
    pub async fn is_daemon_running(&self) -> bool {
        let manager = self.service_manager.lock().await;
        manager.is_daemon_running().await
    }
}
