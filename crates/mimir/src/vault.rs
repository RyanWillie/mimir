//! Vault management - Status checking and auto-initialization

use mimir_core::{config::MimirConfig, crypto::CryptoManager, Result};

use tracing::{info, warn, error};

/// Vault status information
#[derive(Debug, Clone)]
pub struct VaultStatus {
    pub initialized: bool,
    pub app_dir_exists: bool,
    pub keyset_exists: bool,
    pub database_exists: bool,
    pub crypto_ready: bool,
    pub app_dir_path: String,
    pub keyset_path: String,
    pub database_path: String,
}

impl VaultStatus {
    /// Create a new vault status
    pub fn new(config: &MimirConfig) -> Self {
        let app_dir = config.storage.vault_path.parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let keyset_path = app_dir.join("keyset.json");
        
        let app_dir_exists = app_dir.exists();
        let keyset_exists = keyset_path.exists();
        let database_exists = config.storage.vault_path.exists();
        
        // Try to initialize crypto to check if it's ready
        let crypto_ready = CryptoManager::new(&keyset_path).is_ok();
        
        let initialized = app_dir_exists && keyset_exists && database_exists && crypto_ready;
        
        Self {
            initialized,
            app_dir_exists,
            keyset_exists,
            database_exists,
            crypto_ready,
            app_dir_path: app_dir.to_string_lossy().to_string(),
            keyset_path: keyset_path.to_string_lossy().to_string(),
            database_path: config.storage.vault_path.to_string_lossy().to_string(),
        }
    }
    
    /// Check if vault is fully initialized and ready
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    
    /// Get a human-readable status message
    pub fn status_message(&self) -> String {
        if self.is_ready() {
            "✅ Vault is initialized and ready".to_string()
        } else {
            let mut issues = Vec::new();
            
            if !self.app_dir_exists {
                issues.push("Application directory missing");
            }
            if !self.keyset_exists {
                issues.push("Encryption keyset missing");
            }
            if !self.database_exists {
                issues.push("Database file missing");
            }
            if !self.crypto_ready {
                issues.push("Crypto system not ready");
            }
            
            format!("❌ Vault not initialized: {}", issues.join(", "))
        }
    }
}

/// Initialize the vault with all required components
pub async fn initialize_vault(config: &MimirConfig) -> Result<()> {
    info!("Initializing Mimir vault...");
    
    let app_dir = config.storage.vault_path.parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let keyset_path = app_dir.join("keyset.json");
    
    // Create application directory
    if !app_dir.exists() {
        info!("Creating application directory: {}", app_dir.display());
        std::fs::create_dir_all(app_dir)
            .map_err(|e| mimir_core::MimirError::Initialization(
                format!("Failed to create app directory: {}", e)
            ))?;
    }
    
    // Initialize crypto manager (creates keyset if needed)
    info!("Initializing cryptographic system...");
    let _crypto_manager = CryptoManager::new(&keyset_path)
        .map_err(|e| mimir_core::MimirError::Initialization(
            format!("Failed to initialize crypto: {}", e)
        ))?;
    
    // Create database file (placeholder for now)
    if !config.storage.vault_path.exists() {
        info!("Creating database file: {}", config.storage.vault_path.display());
        // TODO: Initialize actual database when mimir-db is implemented
        // For now, just create an empty file as a placeholder
        std::fs::write(&config.storage.vault_path, "")
            .map_err(|e| mimir_core::MimirError::Initialization(
                format!("Failed to create database file: {}", e)
            ))?;
    }
    
    info!("✅ Vault initialized successfully");
    info!("  App directory: {}", app_dir.display());
    info!("  Keyset: {}", keyset_path.display());
    info!("  Database: {}", config.storage.vault_path.display());
    
    Ok(())
}

/// Ensure vault is initialized, auto-initialize if needed
pub async fn ensure_vault_ready(config: &MimirConfig, auto_init: bool) -> Result<()> {
    let status = VaultStatus::new(config);
    
    if status.is_ready() {
        info!("{}", status.status_message());
        return Ok(());
    }
    
    if !auto_init {
        error!("{}", status.status_message());
        return Err(mimir_core::MimirError::Initialization(
            "Vault not initialized. Run 'mimir-cli init' or use --auto-init flag".to_string()
        ));
    }
    
    warn!("Vault not initialized, auto-initializing...");
    warn!("  App directory: {}", status.app_dir_path);
    warn!("  Keyset: {}", status.keyset_path);
    warn!("  Database: {}", status.database_path);
    
    initialize_vault(config).await?;
    
    // Verify initialization was successful
    let final_status = VaultStatus::new(config);
    if !final_status.is_ready() {
        return Err(mimir_core::MimirError::Initialization(
            "Auto-initialization failed".to_string()
        ));
    }
    
    info!("✅ Auto-initialization completed successfully");
    Ok(())
}

/// Check vault status and return detailed information
pub fn check_vault_status(config: &MimirConfig) -> VaultStatus {
    VaultStatus::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_vault_status_creation() {
        let config = MimirConfig::default();
        let status = VaultStatus::new(&config);
        
        // Should have all the expected fields
        assert!(!status.app_dir_path.is_empty());
        assert!(!status.keyset_path.is_empty());
        assert!(!status.database_path.is_empty());
    }
    
    #[test]
    fn test_vault_status_message() {
        let config = MimirConfig::default();
        let status = VaultStatus::new(&config);
        
        let message = status.status_message();
        assert!(!message.is_empty());
        
        // Should contain either ✅ or ❌
        assert!(message.contains("✅") || message.contains("❌"));
    }
    
    #[tokio::test]
    async fn test_initialize_vault() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = MimirConfig::default();
        config.storage.vault_path = temp_dir.path().join("vault.db");
        
        // Should succeed
        let result = initialize_vault(&config).await;
        assert!(result.is_ok());
        
        // Check that files were created
        let app_dir = config.storage.vault_path.parent().unwrap();
        let keyset_path = app_dir.join("keyset.json");
        
        assert!(app_dir.exists());
        assert!(keyset_path.exists());
        assert!(config.storage.vault_path.exists());
    }
    
    #[tokio::test]
    async fn test_ensure_vault_ready_with_auto_init() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = MimirConfig::default();
        config.storage.vault_path = temp_dir.path().join("vault.db");
        
        // Should auto-initialize when not ready
        let result = ensure_vault_ready(&config, true).await;
        assert!(result.is_ok());
        
        // Should be ready after auto-initialization
        let status = check_vault_status(&config);
        assert!(status.is_ready());
    }
    
    #[tokio::test]
    async fn test_ensure_vault_ready_without_auto_init() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = MimirConfig::default();
        config.storage.vault_path = temp_dir.path().join("vault.db");
        
        // Should fail when not ready and auto_init is false
        let result = ensure_vault_ready(&config, false).await;
        assert!(result.is_err());
    }
} 