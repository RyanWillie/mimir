//! Mimir Configuration - Persistent settings for the AI Memory Vault
//!
//! This module provides configuration management for Mimir, including:
//! - Database and vault paths
//! - Encryption settings
//! - Future extensible configuration options

use crate::{MimirError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for the Mimir AI Memory Vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Version of the configuration format
    #[serde(default = "default_config_version")]
    pub version: u32,

    /// Path to the vault directory (where database and keyset are stored)
    #[serde(default = "default_vault_path")]
    pub vault_path: PathBuf,

    /// Path to the database file (relative to vault_path if not absolute)
    #[serde(default = "default_database_path")]
    pub database_path: PathBuf,

    /// Path to the keyset file (relative to vault_path if not absolute)
    #[serde(default = "default_keyset_path")]
    pub keyset_path: PathBuf,

    /// Encryption mode: "keychain" or "password"
    #[serde(default = "default_encryption_mode")]
    pub encryption_mode: String,

    /// Whether to use password-based encryption
    #[serde(default)]
    pub use_password_encryption: bool,

    /// Maximum number of memories to return in queries (0 = unlimited)
    #[serde(default = "default_max_memories")]
    pub max_memories: usize,

    /// Whether to enable debug logging
    #[serde(default)]
    pub debug_logging: bool,

    /// Auto-backup settings
    #[serde(default)]
    pub auto_backup: AutoBackupConfig,

    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// MCP (Model Context Protocol) configuration
    #[serde(default)]
    pub mcp: McpConfig,

    /// Future extensible configuration options
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Auto-backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoBackupConfig {
    /// Whether auto-backup is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Backup interval in hours (0 = disabled)
    #[serde(default = "default_backup_interval")]
    pub interval_hours: u32,

    /// Maximum number of backup files to keep
    #[serde(default = "default_max_backups")]
    pub max_backups: usize,

    /// Backup directory path (relative to vault_path if not absolute)
    #[serde(default = "default_backup_path")]
    pub backup_path: PathBuf,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host address
    #[serde(default = "default_server_host")]
    pub host: String,

    /// Server port
    #[serde(default = "default_server_port")]
    pub port: u16,

    /// Whether to enable TLS
    #[serde(default)]
    pub enable_tls: bool,

    /// TLS certificate path (if using TLS)
    #[serde(default)]
    pub tls_cert_path: Option<PathBuf>,

    /// TLS key path (if using TLS)
    #[serde(default)]
    pub tls_key_path: Option<PathBuf>,
}

/// MCP (Model Context Protocol) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Whether MCP server is enabled
    #[serde(default = "default_mcp_enabled")]
    pub enabled: bool,

    /// MCP transport type
    #[serde(default = "default_mcp_transport")]
    pub transport: McpTransport,

    /// Server name for MCP
    #[serde(default = "default_mcp_server_name")]
    pub server_name: String,

    /// Server version for MCP
    #[serde(default = "default_mcp_server_version")]
    pub server_version: String,

    /// Maximum number of MCP connections
    #[serde(default = "default_mcp_max_connections")]
    pub max_connections: u32,
}

/// MCP transport type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpTransport {
    /// Standard input/output streams
    Stdio,
    /// SSE (Server-Sent Events) transport
    Sse,
}

impl Config {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from the default location
    pub fn load() -> Result<Self> {
        let config_path = get_default_config_path();
        Self::load_from(&config_path)
    }

    /// Load configuration from a specific path
    pub fn load_from<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let config_path = config_path.as_ref();

        if !config_path.exists() {
            return Ok(Self::new());
        }

        let config_data = fs::read_to_string(config_path)
            .map_err(|e| MimirError::Config(format!("Failed to read config file: {}", e)))?;

        let mut config: Config = serde_json::from_str(&config_data)
            .map_err(|e| MimirError::Config(format!("Failed to parse config file: {}", e)))?;

        // Ensure paths are resolved relative to config file location
        config.resolve_paths(config_path.parent().unwrap_or(Path::new("")));

        Ok(config)
    }

    /// Save configuration to the default location
    pub fn save(&self) -> Result<()> {
        let config_path = get_default_config_path();
        self.save_to(&config_path)
    }

    /// Save configuration to a specific path
    pub fn save_to<P: AsRef<Path>>(&self, config_path: P) -> Result<()> {
        let config_path = config_path.as_ref();

        // Ensure the parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                MimirError::Config(format!("Failed to create config directory: {}", e))
            })?;
        }

        let config_data = serde_json::to_string_pretty(self)
            .map_err(|e| MimirError::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(config_path, config_data)
            .map_err(|e| MimirError::Config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Get the absolute path to the database file
    pub fn get_database_path(&self) -> PathBuf {
        if self.database_path.is_absolute() {
            self.database_path.clone()
        } else {
            self.vault_path.join(&self.database_path)
        }
    }

    /// Get the absolute path to the keyset file
    pub fn get_keyset_path(&self) -> PathBuf {
        if self.keyset_path.is_absolute() {
            self.keyset_path.clone()
        } else {
            self.vault_path.join(&self.keyset_path)
        }
    }

    /// Get the vault path
    pub fn get_vault_path(&self) -> &PathBuf {
        &self.vault_path
    }

    /// Get the absolute path to the backup directory
    pub fn get_backup_path(&self) -> PathBuf {
        if self.auto_backup.backup_path.is_absolute() {
            self.auto_backup.backup_path.clone()
        } else {
            self.vault_path.join(&self.auto_backup.backup_path)
        }
    }

    /// Set the vault path and update relative paths accordingly
    pub fn set_vault_path<P: AsRef<Path>>(&mut self, vault_path: P) {
        self.vault_path = vault_path.as_ref().to_path_buf();
    }

    /// Set the database path (can be relative or absolute)
    pub fn set_database_path<P: AsRef<Path>>(&mut self, database_path: P) {
        self.database_path = database_path.as_ref().to_path_buf();
    }

    /// Set the keyset path (can be relative or absolute)
    pub fn set_keyset_path<P: AsRef<Path>>(&mut self, keyset_path: P) {
        self.keyset_path = keyset_path.as_ref().to_path_buf();
    }

    /// Set encryption mode
    pub fn set_encryption_mode(&mut self, mode: &str) {
        self.encryption_mode = mode.to_string();
        self.use_password_encryption = mode == "password";
    }

    /// Resolve relative paths based on a base directory
    fn resolve_paths(&mut self, base_dir: &Path) {
        // Only resolve paths that are relative and not already resolved
        if !self.vault_path.is_absolute() {
            self.vault_path = base_dir.join(&self.vault_path);
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.version < 1 {
            return Err(MimirError::Config("Invalid config version".to_string()));
        }

        if self.max_memories > 1_000_000 {
            return Err(MimirError::Config(
                "max_memories cannot exceed 1,000,000".to_string(),
            ));
        }

        if self.auto_backup.interval_hours > 8760 {
            // 1 year
            return Err(MimirError::Config(
                "backup_interval_hours cannot exceed 8760".to_string(),
            ));
        }

        if self.auto_backup.max_backups > 1000 {
            return Err(MimirError::Config(
                "max_backups cannot exceed 1000".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_config_version(),
            vault_path: default_vault_path(),
            database_path: default_database_path(),
            keyset_path: default_keyset_path(),
            encryption_mode: default_encryption_mode(),
            use_password_encryption: false,
            max_memories: default_max_memories(),
            debug_logging: false,
            auto_backup: AutoBackupConfig::default(),
            server: ServerConfig::default(),
            mcp: McpConfig::default(),
            extra: std::collections::HashMap::new(),
        }
    }
}

impl Default for AutoBackupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_hours: default_backup_interval(),
            max_backups: default_max_backups(),
            backup_path: default_backup_path(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_server_host(),
            port: default_server_port(),
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            transport: McpTransport::Stdio,
            server_name: "Mimir".to_string(),
            server_version: "0.1.0".to_string(),
            max_connections: 10,
        }
    }
}

// Default value functions
fn default_config_version() -> u32 {
    1
}
fn default_vault_path() -> PathBuf {
    get_default_app_dir()
}
fn default_database_path() -> PathBuf {
    PathBuf::from("mimir.db")
}
fn default_keyset_path() -> PathBuf {
    PathBuf::from("keyset.json")
}
fn default_encryption_mode() -> String {
    "keychain".to_string()
}
fn default_max_memories() -> usize {
    1000
}
fn default_backup_interval() -> u32 {
    24
}
fn default_max_backups() -> usize {
    10
}
fn default_backup_path() -> PathBuf {
    PathBuf::from("backups")
}
fn default_server_host() -> String {
    "localhost".to_string()
}
fn default_server_port() -> u16 {
    61827
}
fn default_mcp_enabled() -> bool {
    false
}
fn default_mcp_transport() -> McpTransport {
    McpTransport::Stdio
}
fn default_mcp_server_name() -> String {
    "Mimir".to_string()
}
fn default_mcp_server_version() -> String {
    "0.1.0".to_string()
}
fn default_mcp_max_connections() -> u32 {
    10
}

/// Get the default application directory for Mimir
pub fn get_default_app_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "Mimir")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("./mimir"))
}

/// Get the default configuration file path
pub fn get_default_config_path() -> PathBuf {
    get_default_app_dir().join("config.json")
}

/// Get the default keyset path (for backward compatibility)
pub fn get_default_keyset_path() -> PathBuf {
    get_default_app_dir().join("keyset.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_defaults() {
        let config = Config::new();
        assert_eq!(config.version, 1);
        assert_eq!(config.encryption_mode, "keychain");
        assert!(!config.use_password_encryption);
        assert_eq!(config.max_memories, 1000);
        assert!(!config.debug_logging);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::new();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config.version, deserialized.version);
        assert_eq!(config.encryption_mode, deserialized.encryption_mode);
    }

    #[test]
    fn test_config_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.json");

        let mut config = Config::new();
        config.set_vault_path(temp_dir.path());
        config.set_encryption_mode("password");
        config.max_memories = 500;

        // Save config
        config.save_to(&config_path).unwrap();
        assert!(config_path.exists());

        // Load config
        let loaded_config = Config::load_from(&config_path).unwrap();
        assert_eq!(loaded_config.encryption_mode, "password");
        assert_eq!(loaded_config.max_memories, 500);
        assert!(loaded_config.use_password_encryption);
    }

    #[test]
    fn test_path_resolution() {
        let mut config = Config::new();
        config.set_vault_path("/custom/vault");
        config.set_database_path("custom.db");
        config.set_keyset_path("custom_keyset.json");

        assert_eq!(
            config.get_database_path(),
            PathBuf::from("/custom/vault/custom.db")
        );
        assert_eq!(
            config.get_keyset_path(),
            PathBuf::from("/custom/vault/custom_keyset.json")
        );
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::new();
        assert!(config.validate().is_ok());

        // Test invalid max_memories
        config.max_memories = 2_000_000;
        assert!(config.validate().is_err());

        // Test invalid backup interval
        config.max_memories = 1000;
        config.auto_backup.interval_hours = 10000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_encryption_mode_setting() {
        let mut config = Config::new();

        config.set_encryption_mode("password");
        assert_eq!(config.encryption_mode, "password");
        assert!(config.use_password_encryption);

        config.set_encryption_mode("keychain");
        assert_eq!(config.encryption_mode, "keychain");
        assert!(!config.use_password_encryption);
    }
}
