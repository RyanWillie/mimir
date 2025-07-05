use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Get the default Mimir application directory
/// 
/// Returns OS-appropriate paths:
/// - macOS: ~/Library/Application Support/Mimir/
/// - Linux: ~/.local/share/mimir/
/// - Windows: %USERPROFILE%\AppData\Local\Mimir\
pub fn get_default_app_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "Mimir")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("./mimir"))
}

/// Get the default vault database path
pub fn get_default_vault_path() -> PathBuf {
    get_default_app_dir().join("vault.db")
}

/// Get the default keyset path
pub fn get_default_keyset_path() -> PathBuf {
    get_default_app_dir().join("keyset.json")
}

/// Main configuration for Mimir daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimirConfig {
    pub server: ServerConfig,
    pub mcp: McpConfig,
    pub storage: StorageConfig,
    pub security: SecurityConfig,
    pub processing: ProcessingConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_tls: bool,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
}

/// MCP (Model Context Protocol) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub enabled: bool,
    pub transport: McpTransport,
    pub server_name: String,
    pub server_version: String,
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

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub vault_path: PathBuf,
    pub max_memory_age_days: u64,
    pub compression_threshold_days: u32,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub master_key_path: PathBuf,
    pub enable_pii_detection: bool,
    pub strict_access_control: bool,
}

/// Processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    pub worker_threads: usize,
    pub embedding_model: String,
    pub compression_model: String,
}

impl Default for MimirConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8100,
                enable_tls: false,
                tls_cert_path: None,
                tls_key_path: None,
            },
            mcp: McpConfig {
                enabled: true,
                transport: McpTransport::Stdio,
                server_name: "mimir".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
                max_connections: 10,
            },
            storage: StorageConfig {
                vault_path: get_default_vault_path(),
                max_memory_age_days: 90,
                compression_threshold_days: 30,
            },
            security: SecurityConfig {
                master_key_path: get_default_app_dir().join("master.key"),
                enable_pii_detection: true,
                strict_access_control: true,
            },
            processing: ProcessingConfig {
                worker_threads: num_cpus::get(),
                embedding_model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
                compression_model: "microsoft/DialoGPT-small".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config_values() {
        let config = MimirConfig::default();

        // Test server defaults
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8100);
        assert!(!config.server.enable_tls);
        assert!(config.server.tls_cert_path.is_none());
        assert!(config.server.tls_key_path.is_none());

        // Test MCP defaults
        assert!(config.mcp.enabled);
        assert_eq!(config.mcp.server_name, "mimir");
        assert_eq!(config.mcp.server_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(config.mcp.max_connections, 10);
        match config.mcp.transport {
            McpTransport::Stdio => (),
            _ => panic!("Expected default transport to be Stdio"),
        }

        // Test storage defaults
        assert_eq!(config.storage.max_memory_age_days, 90);
        assert_eq!(config.storage.compression_threshold_days, 30);
        assert!(config
            .storage
            .vault_path
            .to_string_lossy()
            .contains("vault.db"));

        // Test security defaults
        assert!(config.security.enable_pii_detection);
        assert!(config.security.strict_access_control);
        assert!(config
            .security
            .master_key_path
            .to_string_lossy()
            .contains("master.key"));

        // Test processing defaults
        assert!(config.processing.worker_threads > 0);
        assert!(config.processing.embedding_model.contains("MiniLM"));
        assert!(config.processing.compression_model.contains("DialoGPT"));
    }

    #[test]
    fn test_mcp_config_creation() {
        let mcp_config = McpConfig {
            enabled: false,
            transport: McpTransport::Sse,
            server_name: "test-server".to_string(),
            server_version: "1.2.3".to_string(),
            max_connections: 25,
        };

        assert!(!mcp_config.enabled);
        assert_eq!(mcp_config.server_name, "test-server");
        assert_eq!(mcp_config.server_version, "1.2.3");
        assert_eq!(mcp_config.max_connections, 25);
        
        match mcp_config.transport {
            McpTransport::Sse => (),
            _ => panic!("Expected SSE transport"),
        }
    }

    #[test]
    fn test_mcp_transport_variants() {
        // Test Stdio variant
        let stdio_transport = McpTransport::Stdio;
        match stdio_transport {
            McpTransport::Stdio => (),
            _ => panic!("Expected Stdio transport"),
        }

        // Test SSE variant
        let sse_transport = McpTransport::Sse;
        match sse_transport {
            McpTransport::Sse => (),
            _ => panic!("Expected SSE transport"),
        }
    }

    #[test]
    fn test_mcp_config_serialization() {
        let mcp_config = McpConfig {
            enabled: true,
            transport: McpTransport::Stdio,
            server_name: "test-mimir".to_string(),
            server_version: "2.0.0".to_string(),
            max_connections: 15,
        };

        // Test serialization
        let serialized = serde_json::to_string(&mcp_config).unwrap();
        assert!(serialized.contains("test-mimir"));
        assert!(serialized.contains("2.0.0"));
        assert!(serialized.contains("Stdio"));
        assert!(serialized.contains("15"));

        // Test deserialization
        let deserialized: McpConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.enabled, mcp_config.enabled);
        assert_eq!(deserialized.server_name, mcp_config.server_name);
        assert_eq!(deserialized.server_version, mcp_config.server_version);
        assert_eq!(deserialized.max_connections, mcp_config.max_connections);
    }

    #[test]
    fn test_mcp_transport_serialization() {
        // Test Stdio serialization
        let stdio = McpTransport::Stdio;
        let serialized = serde_json::to_string(&stdio).unwrap();
        assert_eq!(serialized, "\"Stdio\"");
        
        let deserialized: McpTransport = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            McpTransport::Stdio => (),
            _ => panic!("Expected Stdio after deserialization"),
        }

        // Test SSE serialization
        let sse = McpTransport::Sse;
        let serialized = serde_json::to_string(&sse).unwrap();
        assert_eq!(serialized, "\"Sse\"");
        
        let deserialized: McpTransport = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            McpTransport::Sse => (),
            _ => panic!("Expected SSE after deserialization"),
        }
    }

    #[test]
    fn test_config_serialization() {
        let config = MimirConfig::default();

        let serialized = serde_json::to_string(&config).unwrap();
        assert!(!serialized.is_empty());
        assert!(serialized.contains("127.0.0.1"));
        assert!(serialized.contains("8100"));

        let deserialized: MimirConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config.server.host, deserialized.server.host);
        assert_eq!(config.server.port, deserialized.server.port);
    }

    #[test]
    fn test_server_config_custom_values() {
        let server_config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 9090,
            enable_tls: true,
            tls_cert_path: Some(PathBuf::from("/path/to/cert.pem")),
            tls_key_path: Some(PathBuf::from("/path/to/key.pem")),
        };

        assert_eq!(server_config.host, "0.0.0.0");
        assert_eq!(server_config.port, 9090);
        assert!(server_config.enable_tls);
        assert!(server_config.tls_cert_path.is_some());
        assert!(server_config.tls_key_path.is_some());
    }

    #[test]
    fn test_storage_config_paths() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("custom_vault.db");

        let storage_config = StorageConfig {
            vault_path: vault_path.clone(),
            max_memory_age_days: 30,
            compression_threshold_days: 7,
        };

        assert_eq!(storage_config.vault_path, vault_path);
        assert_eq!(storage_config.max_memory_age_days, 30);
        assert_eq!(storage_config.compression_threshold_days, 7);
    }

    #[test]
    fn test_security_config_settings() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("custom.key");

        let security_config = SecurityConfig {
            master_key_path: key_path.clone(),
            enable_pii_detection: false,
            strict_access_control: false,
        };

        assert_eq!(security_config.master_key_path, key_path);
        assert!(!security_config.enable_pii_detection);
        assert!(!security_config.strict_access_control);
    }

    #[test]
    fn test_processing_config_threads() {
        let processing_config = ProcessingConfig {
            worker_threads: 4,
            embedding_model: "custom/model".to_string(),
            compression_model: "custom/compression".to_string(),
        };

        assert_eq!(processing_config.worker_threads, 4);
        assert_eq!(processing_config.embedding_model, "custom/model");
        assert_eq!(processing_config.compression_model, "custom/compression");
    }

    #[test]
    fn test_config_roundtrip_serialization() {
        let original_config = MimirConfig {
            server: ServerConfig {
                host: "test.example.com".to_string(),
                port: 8080,
                enable_tls: true,
                tls_cert_path: Some(PathBuf::from("/test/cert.pem")),
                tls_key_path: Some(PathBuf::from("/test/key.pem")),
            },
            mcp: McpConfig {
                enabled: false,
                transport: McpTransport::Sse,
                server_name: "test-mimir".to_string(),
                server_version: "0.1.0".to_string(),
                max_connections: 5,
            },
            storage: StorageConfig {
                vault_path: PathBuf::from("/test/vault.db"),
                max_memory_age_days: 60,
                compression_threshold_days: 14,
            },
            security: SecurityConfig {
                master_key_path: PathBuf::from("/test/master.key"),
                enable_pii_detection: false,
                strict_access_control: false,
            },
            processing: ProcessingConfig {
                worker_threads: 8,
                embedding_model: "test/embedding".to_string(),
                compression_model: "test/compression".to_string(),
            },
        };

        let serialized = serde_json::to_string(&original_config).unwrap();
        let deserialized: MimirConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original_config.server.host, deserialized.server.host);
        assert_eq!(original_config.server.port, deserialized.server.port);
        assert_eq!(
            original_config.server.enable_tls,
            deserialized.server.enable_tls
        );
        assert_eq!(
            original_config.storage.vault_path,
            deserialized.storage.vault_path
        );
        assert_eq!(
            original_config.security.enable_pii_detection,
            deserialized.security.enable_pii_detection
        );
        assert_eq!(
            original_config.processing.worker_threads,
            deserialized.processing.worker_threads
        );
    }

    #[test]
    fn test_worker_threads_positive() {
        let config = MimirConfig::default();
        assert!(config.processing.worker_threads > 0);

        // Test that default uses system CPU count
        let system_cpus = num_cpus::get();
        assert_eq!(config.processing.worker_threads, system_cpus);
    }

    #[test]
    fn test_port_range_validity() {
        let config = MimirConfig::default();
        assert!(config.server.port > 0);
        // Note: u16 is always <= 65535, but this documents our expectation
        assert!(config.server.port > 1024); // Should be above well-known ports
    }

    #[test]
    fn test_age_thresholds_logical() {
        let config = MimirConfig::default();

        // Compression threshold should be less than max age
        assert!(
            config.storage.compression_threshold_days < config.storage.max_memory_age_days as u32
        );

        // Both should be positive
        assert!(config.storage.max_memory_age_days > 0);
        assert!(config.storage.compression_threshold_days > 0);
    }

    #[test]
    fn test_model_names_not_empty() {
        let config = MimirConfig::default();

        assert!(!config.processing.embedding_model.is_empty());
        assert!(!config.processing.compression_model.is_empty());

        // Should contain model identifiers
        assert!(config.processing.embedding_model.contains('/'));
        assert!(config.processing.compression_model.contains('/'));
    }

    #[test]
    fn test_default_paths_are_appropriate() {
        let app_dir = get_default_app_dir();
        let vault_path = get_default_vault_path();
        let keyset_path = get_default_keyset_path();

        // All paths should be absolute or contain Mimir/mimir
        let app_dir_str = app_dir.to_string_lossy().to_lowercase();
        assert!(app_dir_str.contains("mimir") || app_dir_str.starts_with("./mimir"));

        // Vault path should be in the app directory
        assert!(vault_path.starts_with(&app_dir));
        assert!(vault_path.file_name().unwrap() == "vault.db");

        // Keyset path should be in the app directory
        assert!(keyset_path.starts_with(&app_dir));
        assert!(keyset_path.file_name().unwrap() == "keyset.json");
    }

    #[test]
    fn test_default_config_uses_proper_paths() {
        let config = MimirConfig::default();

        // Storage path should be the default vault path
        assert_eq!(config.storage.vault_path, get_default_vault_path());

        // Security path should be in the app directory
        assert!(config.security.master_key_path.starts_with(&get_default_app_dir()));
        assert!(config.security.master_key_path.file_name().unwrap() == "master.key");
    }
}
