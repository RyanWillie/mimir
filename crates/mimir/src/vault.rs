//! Vault management - Status checking and auto-initialization

use mimir_core::{Config, crypto::CryptoManager, Result};

use tracing::{info, warn, error};

/// Vault status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum VaultStatusEnum {
    /// Vault is fully initialized and ready
    Ready,
    /// Vault is not initialized at all
    NotInitialized,
    /// Keyset file is missing
    MissingKeyset,
    /// Database file is missing
    MissingDatabase,
}

/// Vault status information
#[derive(Debug, Clone)]
pub struct VaultStatus {
    pub status: VaultStatusEnum,
    pub vault_path: std::path::PathBuf,
    pub keyset_path: std::path::PathBuf,
    pub app_dir: std::path::PathBuf,
}

impl VaultStatus {
    /// Create a new vault status
    pub fn new(config: &Config) -> Self {
        let app_dir = config.get_vault_path().parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        
        let vault_path = config.get_database_path();
        let keyset_path = config.get_keyset_path();
        
        let vault_exists = vault_path.exists();
        let keyset_exists = keyset_path.exists();
        let app_dir_exists = app_dir.exists();
        
        let status = if !app_dir_exists {
            VaultStatusEnum::NotInitialized
        } else if !keyset_exists {
            VaultStatusEnum::MissingKeyset
        } else if !vault_exists {
            VaultStatusEnum::MissingDatabase
        } else {
            VaultStatusEnum::Ready
        };
        
        Self {
            status,
            vault_path,
            keyset_path,
            app_dir: app_dir.to_path_buf(),
        }
    }
    
    /// Check if vault is fully initialized and ready
    pub fn is_ready(&self) -> bool {
        self.status == VaultStatusEnum::Ready
    }
    
    /// Get a human-readable status message
    pub fn status_message(&self) -> String {
        match self.status {
            VaultStatusEnum::Ready => "✅ Vault is initialized and ready".to_string(),
            VaultStatusEnum::NotInitialized => {
                let mut issues = Vec::new();
                issues.push("Application directory missing");
                issues.push("Encryption keyset missing");
                issues.push("Database file missing");
                issues.push("Crypto system not ready");
                format!("❌ Vault not initialized: {}", issues.join(", "))
            }
            VaultStatusEnum::MissingKeyset => {
                let mut issues = Vec::new();
                issues.push("Encryption keyset missing");
                issues.push("Database file missing");
                issues.push("Crypto system not ready");
                format!("❌ Vault not initialized: {}", issues.join(", "))
            }
            VaultStatusEnum::MissingDatabase => {
                let mut issues = Vec::new();
                issues.push("Database file missing");
                issues.push("Crypto system not ready");
                format!("❌ Vault not initialized: {}", issues.join(", "))
            }
        }
    }
}

/// Initialize the vault with all required components
pub async fn initialize_vault(config: &Config) -> Result<()> {
    info!("Initializing Mimir vault...");
    
    let app_dir = config.get_vault_path().parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let keyset_path = config.get_keyset_path();
    
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
    if !config.get_database_path().exists() {
        info!("Creating database file: {}", config.get_database_path().display());
        // TODO: Initialize actual database when mimir-db is implemented
        // For now, just create an empty file as a placeholder
        std::fs::write(&config.get_database_path(), "")
            .map_err(|e| mimir_core::MimirError::Initialization(
                format!("Failed to create database file: {}", e)
            ))?;
    }
    
    info!("✅ Vault initialized successfully");
    info!("  App directory: {}", app_dir.display());
    info!("  Keyset: {}", keyset_path.display());
    info!("  Database: {}", config.get_database_path().display());
    
    Ok(())
}

/// Ensure vault is initialized, auto-initialize if needed
pub async fn ensure_vault_ready(config: &Config, auto_init: bool) -> Result<()> {
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
    warn!("  App directory: {}", status.app_dir.display());
    warn!("  Keyset: {}", status.keyset_path.display());
    warn!("  Database: {}", status.vault_path.display());
    
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
pub fn check_vault_status(config: &Config) -> VaultStatus {
    VaultStatus::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_vault_status_creation() {
        let config = Config::default();
        let status = VaultStatus::new(&config);
        
        // Should have all the expected fields
        assert!(!status.app_dir.to_string_lossy().is_empty());
        assert!(!status.keyset_path.to_string_lossy().is_empty());
        assert!(!status.vault_path.to_string_lossy().is_empty());
    }
    
    #[test]
    fn test_vault_status_message() {
        let config = Config::default();
        let status = VaultStatus::new(&config);
        
        let message = status.status_message();
        assert!(!message.is_empty());
        
        // Should contain either ✅ or ❌
        assert!(message.contains("✅") || message.contains("❌"));
    }
    
    #[tokio::test]
    async fn test_initialize_vault() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.set_database_path(temp_dir.path().join("vault.db"));
        
        // Should succeed
        let result = initialize_vault(&config).await;
        assert!(result.is_ok());
        
        // Check that files were created
        let app_dir = config.get_vault_path();
        let keyset_path = config.get_keyset_path();
        
        assert!(app_dir.exists());
        assert!(keyset_path.exists());
        assert!(config.get_database_path().exists());
    }
    
    #[tokio::test]
    async fn test_ensure_vault_ready_with_auto_init() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.set_database_path(temp_dir.path().join("vault.db"));
        
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
        let mut config = Config::default();
        config.set_database_path(temp_dir.path().join("vault.db"));
        
        // Should fail when not ready and auto_init is false
        let result = ensure_vault_ready(&config, false).await;
        assert!(result.is_err());
    }
} 