//! Configuration manager for the tray application

use mimir_core::{Config, Result};
use std::path::PathBuf;
use tracing::info;

/// Configuration manager for the tray application
pub struct ConfigManager {
    config: Config,
    config_path: PathBuf,
    vault_path: PathBuf,
    keyset_path: PathBuf,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config: Config) -> Result<Self> {
        let config_path = mimir_core::get_default_config_path();
        let vault_path = config.get_vault_path().clone();
        let keyset_path = config.get_keyset_path().clone();

        Ok(Self {
            config,
            config_path,
            vault_path,
            keyset_path,
        })
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &Config {
        &self.config
    }

    /// Get a mutable reference to the configuration
    pub fn get_config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Save the current configuration
    pub fn save_config(&self) -> Result<()> {
        self.config.save()
    }

    /// Reload configuration from disk
    pub fn reload_config(&mut self) -> Result<()> {
        self.config = Config::load_from(&self.config_path)?;
        info!("Configuration reloaded from: {}", self.config_path.display());
        Ok(())
    }

    /// Get the vault path
    pub fn get_vault_path(&self) -> &PathBuf {
        &self.vault_path
    }

    /// Set the vault path
    pub fn set_vault_path(&mut self, path: &PathBuf) -> Result<()> {
        self.config.set_vault_path(path);
        self.vault_path = path.clone();
        self.save_config()?;
        info!("Vault path updated to: {}", path.display());
        Ok(())
    }

    /// Get the keyset path
    pub fn get_keyset_path(&self) -> &PathBuf {
        &self.keyset_path
    }

    /// Set the keyset path
    pub fn set_keyset_path(&mut self, path: &PathBuf) -> Result<()> {
        self.config.set_keyset_path(path);
        self.keyset_path = path.clone();
        self.save_config()?;
        info!("Keyset path updated to: {}", path.display());
        Ok(())
    }

    /// Get the server port
    pub fn get_server_port(&self) -> u16 {
        self.config.server.port
    }

    /// Set the server port
    pub fn set_server_port(&mut self, port: u16) -> Result<()> {
        self.config.server.port = port;
        self.save_config()?;
        info!("Server port updated to: {}", port);
        Ok(())
    }

    /// Get the encryption mode
    pub fn get_encryption_mode(&self) -> &str {
        &self.config.encryption_mode
    }

    /// Set the encryption mode
    pub fn set_encryption_mode(&mut self, mode: &str) -> Result<()> {
        self.config.set_encryption_mode(mode);
        self.save_config()?;
        info!("Encryption mode updated to: {}", mode);
        Ok(())
    }

    /// Check if vault is initialized
    pub fn is_vault_initialized(&self) -> bool {
        self.vault_path.exists() && self.keyset_path.exists()
    }

    /// Get vault status information
    pub fn get_vault_status(&self) -> VaultStatus {
        let vault_exists = self.vault_path.exists();
        let keyset_exists = self.keyset_path.exists();
        let config_exists = self.config_path.exists();

        if vault_exists && keyset_exists && config_exists {
            VaultStatus::Ready {
                vault_path: self.vault_path.clone(),
                keyset_path: self.keyset_path.clone(),
                config_path: self.config_path.clone(),
            }
        } else if !vault_exists && !keyset_exists {
            VaultStatus::NotInitialized
        } else {
            VaultStatus::Incomplete {
                vault_exists,
                keyset_exists,
                config_exists,
            }
        }
    }

    /// Initialize the vault
    pub async fn initialize_vault(&mut self, password: Option<&str>) -> Result<()> {
        info!("Initializing vault at: {}", self.vault_path.display());

        // Create vault directory if it doesn't exist
        if !self.vault_path.exists() {
            std::fs::create_dir_all(&self.vault_path)?;
            info!("Created vault directory: {}", self.vault_path.display());
        }

        // Create keyset directory if it doesn't exist
        if let Some(keyset_dir) = self.keyset_path.parent() {
            if !keyset_dir.exists() {
                std::fs::create_dir_all(keyset_dir)?;
                info!("Created keyset directory: {}", keyset_dir.display());
            }
        }

        // Initialize crypto manager
        let crypto_manager = if let Some(pwd) = password {
            mimir_core::crypto::CryptoManager::with_password(&self.keyset_path, pwd)?
        } else {
            mimir_core::crypto::CryptoManager::new(&self.keyset_path)?
        };

        // Initialize database
        let db_path = self.config.get_database_path();
        let _db = mimir_db::Database::with_crypto_manager(&db_path, crypto_manager)?;
        info!("Database initialized at: {}", db_path.display());

        // Save configuration
        self.save_config()?;
        info!("Vault initialization completed successfully");

        Ok(())
    }

    /// Rotate encryption keys
    pub async fn rotate_keys(&mut self, key_type: KeyRotationType) -> Result<()> {
        match key_type {
            KeyRotationType::Root => {
                info!("Rotating root encryption key");
                let mut crypto_manager = mimir_core::crypto::CryptoManager::new(&self.keyset_path)?;
                crypto_manager.rotate_root_key()?;
                info!("Root encryption key rotated successfully");
            }
            KeyRotationType::Class { class } => {
                info!("Rotating encryption key for class: {}", class);
                let mut crypto_manager = mimir_core::crypto::CryptoManager::new(&self.keyset_path)?;
                crypto_manager.rotate_class_key(&class)?;
                info!("Class '{}' encryption key rotated successfully", class);
            }
        }
        Ok(())
    }

    /// Get configuration summary
    pub fn get_config_summary(&self) -> ConfigSummary {
        ConfigSummary {
            vault_path: self.vault_path.clone(),
            keyset_path: self.keyset_path.clone(),
            config_path: self.config_path.clone(),
            server_port: self.config.server.port,
            encryption_mode: self.config.encryption_mode.clone(),
            vault_initialized: self.is_vault_initialized(),
        }
    }
}

/// Vault status information
#[derive(Debug, Clone)]
pub enum VaultStatus {
    Ready {
        vault_path: PathBuf,
        keyset_path: PathBuf,
        config_path: PathBuf,
    },
    NotInitialized,
    Incomplete {
        vault_exists: bool,
        keyset_exists: bool,
        config_exists: bool,
    },
}

/// Key rotation type
#[derive(Debug, Clone)]
pub enum KeyRotationType {
    Root,
    Class { class: String },
}

/// Configuration summary
#[derive(Debug, Clone)]
pub struct ConfigSummary {
    pub vault_path: PathBuf,
    pub keyset_path: PathBuf,
    pub config_path: PathBuf,
    pub server_port: u16,
    pub encryption_mode: String,
    pub vault_initialized: bool,
} 