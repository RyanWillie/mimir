use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration for Mimir daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimirConfig {
    pub server: ServerConfig,
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
            storage: StorageConfig {
                vault_path: directories::ProjectDirs::from("", "", "mimir")
                    .map(|d| d.data_dir().join("vault.db"))
                    .unwrap_or_else(|| PathBuf::from("./vault.db")),
                max_memory_age_days: 90,
                compression_threshold_days: 30,
            },
            security: SecurityConfig {
                master_key_path: directories::ProjectDirs::from("", "", "mimir")
                    .map(|d| d.config_dir().join("master.key"))
                    .unwrap_or_else(|| PathBuf::from("./master.key")),
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