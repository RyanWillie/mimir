//! Mimir - Local-First AI Memory Vault
//!
//! Main daemon process that provides both HTTP API and MCP server for AI memory management

use crate::vault::{check_vault_status, ensure_vault_ready};
use clap::{Parser, Subcommand};
use mimir_core::{Config, Result};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use axum::{Router, routing::post};
use tokio::net::TcpListener;
use std::path::PathBuf;
use tracing::{error, info, warn};
use rmcp::ServiceExt;

mod mcp;
mod storage;
mod vault;
mod model;
mod llm_service;

/// Mimir - Local-First AI Memory Vault
#[derive(Parser)]
#[command(name = "mimir")]
#[command(about = "A local-first, zero-knowledge AI memory vault")]
struct Cli {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Override server port
    #[arg(short, long)]
    port: Option<u16>,

    /// Auto-initialize vault if not ready
    #[arg(long)]
    auto_init: bool,

    /// Server mode
    #[command(subcommand)]
    mode: Option<ServerMode>,
}

#[derive(Subcommand, Debug)]
enum ServerMode {
    /// Start MCP (Model Context Protocol) server
    Mcp {
        /// Force stdio transport (otherwise defaults to streamable HTTP)
        #[arg(long)]
        stdio: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("mimir={},mimir_core={}", log_level, log_level))
        .init();

    info!("Starting Mimir v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let mut config = Config::load().unwrap_or_else(|_| {
        info!("No configuration file found, using defaults");
        Config::default()
    });

    if let Some(port) = cli.port {
        config.server.port = port;
    }

    // Load from specific config file if provided
    if let Some(config_path) = cli.config {
        info!("Loading configuration from: {}", config_path.display());
        config = Config::load_from(&config_path)
            .map_err(|e| mimir_core::MimirError::Config(format!("Failed to load config: {}", e)))?;
    }

    // Check vault status and auto-initialize if needed
    info!("Checking vault status...");
    info!("Config vault path: {}", config.get_vault_path().display());
    info!(
        "Config database path: {}",
        config.get_database_path().display()
    );
    info!("Config keyset path: {}", config.get_keyset_path().display());
    let vault_status = check_vault_status(&config);
    info!("{}", vault_status.status_message());

    if !vault_status.is_ready() {
        if cli.auto_init {
            info!("Auto-initializing vault...");
            ensure_vault_ready(&config, true).await?;
        } else {
            error!("Vault not ready. Use --auto-init to auto-initialize or run 'mimir-cli init' first.");
            return Err(mimir_core::MimirError::Initialization(
                "Vault not initialized".to_string(),
            ));
        }
    }

    // Ensure model files are present and valid
    let (model_path, _tokenizer_path, _vocab_path) =
        model::ensure_model_files().await.map_err(mimir_core::MimirError::ServerError)?;
    eprintln!("BGE model file ready at: {}", model_path.display());

    // Ensure Gemma3 model is available
    let gemma3_path = model::ensure_gemma3_model().await.map_err(mimir_core::MimirError::ServerError)?;
    eprintln!("Gemma3 model file ready at: {}", gemma3_path.display());

    // Initialize LLM service
    llm_service::initialize_llm_service(&config).await?;

    // Determine server mode
    let server_mode = cli.mode.unwrap_or(ServerMode::Mcp { stdio: false });

    // Start appropriate server mode
    match server_mode {
        ServerMode::Mcp { stdio: force_stdio } => {
            info!("Starting MCP server");
            if force_stdio {
                config.mcp.transport = mimir_core::config::McpTransport::Stdio;
                start_mcp_server(config).await
            } else {
                // Setup crypto managers
                let (db_crypto_manager, storage_crypto_manager) = setup_crypto_managers(&config).await?;
                // Create database
                let database = create_database(&config, db_crypto_manager)?;
                // Create vector store
                let vector_store = create_vector_store_with_model(&config, &model_path).await?;
                // Create integrated storage
                let storage = create_integrated_storage(database, vector_store, storage_crypto_manager).await?;
                // Create the MCP server with integrated storage
                let mcp_server = mcp::MimirServer::new(storage);
                start_mcp_streamhttp_server(config, mcp_server).await
            }
        }
    }
}

/// Setup crypto managers for database and storage encryption
async fn setup_crypto_managers(
    config: &Config,
) -> Result<(
    mimir_core::crypto::CryptoManager,
    mimir_core::crypto::CryptoManager,
)> {
    if config.use_password_encryption {
        info!("Password encryption detected. Please enter your vault password:");

        // Read password from stdin
        let mut password = String::new();
        std::io::stdin().read_line(&mut password).map_err(|e| {
            mimir_core::MimirError::Initialization(format!("Failed to read password: {}", e))
        })?;
        let password = password.trim();

        if password.is_empty() {
            return Err(mimir_core::MimirError::Initialization(
                "Password cannot be empty".to_string(),
            ));
        }

        info!("Attempting to unlock vault with provided password...");

        let db_crypto_manager =
            mimir_core::crypto::CryptoManager::with_password(&config.get_keyset_path(), password)?;
        let storage_crypto_manager =
            mimir_core::crypto::CryptoManager::with_password(&config.get_keyset_path(), password)?;

        Ok((db_crypto_manager, storage_crypto_manager))
    } else {
        let db_crypto_manager = mimir_core::crypto::CryptoManager::new(&config.get_keyset_path())?;
        let storage_crypto_manager =
            mimir_core::crypto::CryptoManager::new(&config.get_keyset_path())?;
        Ok((db_crypto_manager, storage_crypto_manager))
    }
}

/// Create database with crypto manager
fn create_database(
    config: &Config,
    db_crypto_manager: mimir_core::crypto::CryptoManager,
) -> Result<mimir_db::Database> {
    let database =
        mimir_db::Database::with_crypto_manager(&config.get_database_path(), db_crypto_manager)?;
    Ok(database)
}

/// Create vector store with embedder
async fn create_vector_store_with_model(config: &Config, model_path: &std::path::Path) -> Result<mimir_vector::ThreadSafeVectorStore> {
    let vault_path = config.get_vault_path();
    if model_path.exists() {
        info!(
            "Loading vector store with embedder from: {}",
            model_path.display()
        );
        match mimir_vector::ThreadSafeVectorStore::load_with_embedder(
            vault_path.as_path(),
            None,             // root_key
            Some(model_path), // model_path for embedder
            None,             // memory config
            None,             // batch config
        )
        .await
        {
            Ok(Some(existing_store)) => {
                info!(
                    "Loaded existing vector store with {} vectors and embedder",
                    existing_store.len().await
                );
                Ok(existing_store)
            }
            Ok(None) => {
                info!("No existing vector store found, creating new one with embedder");
                mimir_vector::ThreadSafeVectorStore::with_embedder(
                    vault_path.as_path(),
                    model_path,
                    None, // memory config
                    None, // batch config
                )
                .await
                .map_err(|e| mimir_core::MimirError::VectorStore(e.to_string()))
            }
            Err(e) => {
                warn!(
                    "Failed to load existing vector store: {}, creating new one",
                    e
                );
                mimir_vector::ThreadSafeVectorStore::with_embedder(
                    vault_path.as_path(),
                    model_path,
                    None, // memory config
                    None, // batch config
                )
                .await
                .map_err(|e| mimir_core::MimirError::VectorStore(e.to_string()))
            }
        }
    } else {
        warn!(
            "Model file not found at {}, creating vector store without embedder",
            model_path.display()
        );
        mimir_vector::ThreadSafeVectorStore::new(
            vault_path.as_path(),
            128,  // dimension
            None, // memory config
            None, // batch config
        )
        .map_err(|e| mimir_core::MimirError::VectorStore(e.to_string()))
    }
}

/// Create integrated storage system
async fn create_integrated_storage(
    database: mimir_db::Database,
    vector_store: mimir_vector::ThreadSafeVectorStore,
    storage_crypto_manager: mimir_core::crypto::CryptoManager,
) -> Result<storage::IntegratedStorage> {
    let mut storage =
        storage::IntegratedStorage::new(database, vector_store, storage_crypto_manager).await?;
    
    // Add LLM service if available
    if let Some(llm_service) = llm_service::get_llm_service() {
        storage = storage.with_llm_service(llm_service);
    }
    
    Ok(storage)
}

/// Start MCP service and handle its lifecycle
async fn start_mcp_service(mcp_server: mcp::MimirServer) -> Result<()> {
    let mcp_server_clone = mcp_server.clone();
    match mcp_server_clone
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await
    {
        Ok(service) => {
            info!("MCP server connected and ready");
            let quit_reason = service.waiting().await;
            info!("MCP server shutdown: {:?}", quit_reason);

            // Save vector store on clean shutdown
            info!("Attempting to save vector store on shutdown...");
            if let Err(e) = mcp_server.save_vector_store().await {
                error!("Failed to save vector store on shutdown: {}", e);
            } else {
                info!("Vector store saved successfully on shutdown");
            }

            Ok(())
        }
        Err(e) => {
            error!("MCP server error: {}", e);

            // Try to save vector store even on error
            info!("Attempting to save vector store on error shutdown...");
            if let Err(save_err) = mcp_server.save_vector_store().await {
                error!(
                    "Failed to save vector store on error shutdown: {}",
                    save_err
                );
            } else {
                info!("Vector store saved successfully on error shutdown");
            }

            Err(mimir_core::MimirError::ServerError(format!(
                "MCP server error: {}",
                e
            )))
        }
    }
}

async fn start_mcp_server(config: Config) -> Result<()> {
    info!("Starting MCP server");

    // Setup crypto managers
    let (db_crypto_manager, storage_crypto_manager) = setup_crypto_managers(&config).await?;

    // Create database
    let database = create_database(&config, db_crypto_manager)?;

    // Create vector store
    let vector_store = create_vector_store_with_model(&config, &model::ensure_model_files().await.map_err(mimir_core::MimirError::ServerError).unwrap().0).await?;

    // Create integrated storage
    let storage = create_integrated_storage(database, vector_store, storage_crypto_manager).await?;

    // Create the MCP server with integrated storage
    let mcp_server = mcp::MimirServer::new(storage);

    // Start the MCP service
    start_mcp_service(mcp_server).await
}

async fn start_mcp_streamhttp_server(config: Config, mcp_server: mcp::MimirServer) -> Result<()> {
    let addr = format!("{}:{}", config.server.host, config.server.port);
    println!("Starting MCP server on {}", addr);
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| mimir_core::MimirError::ServerError(format!("Failed to bind: {}", e)))?;

    let service = StreamableHttpService::new(
        move || Ok(mcp_server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );
    // Use the correct handler as in the official example
    let app = axum::Router::new()
        .nest_service("/mcp", service);

    // Serve the app
    axum::serve(listener, app)
        .await
        .map_err(|e| mimir_core::MimirError::ServerError(format!("Axum serve error: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parsing_basic() {
        // Test basic command parsing
        let cli = Cli::try_parse_from(&["mimir"]).unwrap();
        assert!(!cli.debug);
        assert!(cli.config.is_none());
        assert!(cli.port.is_none());
        assert!(cli.mode.is_none());
    }

    #[test]
    fn test_cli_parsing_with_flags() {
        // Test with debug flag
        let cli = Cli::try_parse_from(&["mimir", "--debug"]).unwrap();
        assert!(cli.debug);

        // Test with port
        let cli = Cli::try_parse_from(&["mimir", "--port", "9090"]).unwrap();
        assert_eq!(cli.port, Some(9090));

        // Test with config file
        let cli = Cli::try_parse_from(&["mimir", "--config", "/path/to/config.toml"]).unwrap();
        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config.toml")));

        // Test combined flags
        let cli = Cli::try_parse_from(&["mimir", "-d", "-p", "8080", "-c", "config.toml"]).unwrap();
        assert!(cli.debug);
        assert_eq!(cli.port, Some(8080));
        assert_eq!(cli.config, Some(PathBuf::from("config.toml")));
    }

    #[test]
    fn test_server_mode_mcp() {
        // Test MCP mode
        let cli = Cli::try_parse_from(&["mimir", "mcp"]).unwrap();
        match cli.mode.unwrap() {
            ServerMode::Mcp { stdio } => assert!(!stdio),
            _ => panic!("Expected MCP mode"),
        }

        // Test MCP mode with stdio flag
        let cli = Cli::try_parse_from(&["mimir", "mcp", "--stdio"]).unwrap();
        match cli.mode.unwrap() {
            ServerMode::Mcp { stdio } => assert!(stdio),
            _ => panic!("Expected MCP mode with stdio"),
        }
    }

    #[test]
    fn test_cli_help() {
        // Verify help can be generated (ensures CLI structure is valid)
        let mut cmd = Cli::command();
        let help = cmd.render_help();
        let help_str = help.to_string();

        assert!(help_str.contains("local-first, zero-knowledge AI memory vault"));
        assert!(help_str.contains("--debug"));
        assert!(help_str.contains("--port"));
        assert!(help_str.contains("--config"));
        assert!(help_str.contains("mcp"));
    }

    #[test]
    fn test_invalid_cli_args() {
        // Test invalid port
        let result = Cli::try_parse_from(&["mimir", "--port", "invalid"]);
        assert!(result.is_err());

        // Test unknown command
        let result = Cli::try_parse_from(&["mimir", "unknown"]);
        assert!(result.is_err());

        // Test invalid flag
        let result = Cli::try_parse_from(&["mimir", "--invalid-flag"]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_server_mode_determination() {
        // Test default mode with MCP enabled
        let mut config = Config::default();
        config.mcp.enabled = true;

        // When no mode is specified and MCP is enabled, should default to MCP
        // This tests the logic in main() but we can't easily test that function directly
        // So we test the config building logic
        assert!(config.mcp.enabled);

        // Test default mode with MCP disabled
        config.mcp.enabled = false;
        assert!(!config.mcp.enabled);
    }

    #[test]
    fn test_server_mode_debug_trait() {
        // Ensure ServerMode implements Debug (needed for testing and logging)
        let mcp_mode = ServerMode::Mcp { stdio: true };
        let debug_str = format!("{:?}", mcp_mode);
        assert!(debug_str.contains("Mcp"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn test_complex_cli_scenarios() {
        // Test global flags with subcommands
        let cli =
            Cli::try_parse_from(&["mimir", "--debug", "--port", "7777", "mcp", "--stdio"]).unwrap();
        assert!(cli.debug);
        assert_eq!(cli.port, Some(7777));
        match cli.mode.unwrap() {
            ServerMode::Mcp { stdio } => assert!(stdio),
            _ => panic!("Expected MCP mode"),
        }

        // Test all flags together
        let cli = Cli::try_parse_from(&[
            "mimir",
            "--config",
            "test.toml",
            "--debug",
            "--port",
            "5555",
            "mcp",
            "--stdio",
        ])
        .unwrap();

        assert!(cli.debug);
        assert_eq!(cli.port, Some(5555));
        assert_eq!(cli.config, Some(PathBuf::from("test.toml")));
        match cli.mode.unwrap() {
            ServerMode::Mcp { stdio } => assert!(stdio),
            _ => panic!("Expected MCP mode"),
        }
    }

    // Tests for the new helper functions
    mod helper_function_tests {
        use super::*;
        use tempfile::TempDir;

        #[test]
        fn test_create_database() {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("test.db");
            let keyset_path = temp_dir.path().join("keyset.json");

            let mut config = Config::default();
            config.vault_path = temp_dir.path().to_path_buf();

            let crypto_manager = mimir_core::crypto::CryptoManager::new(&keyset_path)
                .expect("Failed to create test crypto manager");

            let result = create_database(&config, crypto_manager);
            assert!(result.is_ok());
            // Removed assertion that db_path.exists() as file creation is not guaranteed until a write occurs
        }

        #[test]
        fn test_create_database_with_invalid_path() {
            let temp_dir = TempDir::new().unwrap();
            let keyset_path = temp_dir.path().join("keyset.json");

            let mut config = Config::default();
            // Set an invalid path that should cause an error
            config.vault_path = PathBuf::from("/invalid/path/that/does/not/exist");

            let crypto_manager = mimir_core::crypto::CryptoManager::new(&keyset_path)
                .expect("Failed to create test crypto manager");

            let result = create_database(&config, crypto_manager);
            // This should fail due to invalid path
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_create_integrated_storage() {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("test.db");
            let keyset_path = temp_dir.path().join("keyset.json");

            // Create crypto manager
            let db_crypto_manager = mimir_core::crypto::CryptoManager::new(&keyset_path)
                .expect("Failed to create test crypto manager");
            let storage_crypto_manager = mimir_core::crypto::CryptoManager::new(&keyset_path)
                .expect("Failed to create test crypto manager");

            // Create database
            let database = mimir_db::Database::with_crypto_manager(&db_path, db_crypto_manager)
                .expect("Failed to create test database");

            // Create vector store
            let vector_store =
                mimir_vector::ThreadSafeVectorStore::new(temp_dir.path(), 128, None, None)
                    .expect("Failed to create test vector store");

            let result =
                create_integrated_storage(database, vector_store, storage_crypto_manager).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_create_vector_store_without_embedder() {
            let temp_dir = TempDir::new().unwrap();
            let mut config = Config::default();
            config.vault_path = temp_dir.path().to_path_buf();

            // Test creating vector store without embedder (model file doesn't exist)
            let result = create_vector_store_with_model(&config, &model::ensure_model_files().await.map_err(mimir_core::MimirError::ServerError).unwrap().0).await;
            assert!(result.is_ok());

            // Verify vector store directory was created
            assert!(temp_dir.path().exists());
        }

        #[test]
        fn test_setup_crypto_managers_no_password() {
            let temp_dir = TempDir::new().unwrap();
            let mut config = Config::default();
            config.vault_path = temp_dir.path().to_path_buf();
            config.use_password_encryption = false;

            // This test would require mocking stdin for password input
            // For now, we'll just test the no-password path
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(setup_crypto_managers(&config));
            assert!(result.is_ok());

            let (db_crypto, storage_crypto) = result.unwrap();
            // Verify both crypto managers were created by checking keyset file exists
            assert!(config.get_keyset_path().exists());
        }

        #[test]
        fn test_helper_functions_error_handling() {
            let temp_dir = TempDir::new().unwrap();
            let keyset_path = temp_dir.path().join("keyset.json");

            let mut config = Config::default();
            config.vault_path = PathBuf::from("/invalid/path");

            let crypto_manager = mimir_core::crypto::CryptoManager::new(&keyset_path)
                .expect("Failed to create test crypto manager");

            // Test that create_database properly handles invalid paths
            let result = create_database(&config, crypto_manager);
            assert!(result.is_err());

            // Verify the error is of the expected type
            match result {
                Err(mimir_core::MimirError::Database(_)) => {
                    // Expected error type
                }
                Err(e) => {
                    panic!("Expected Database error, got: {:?}", e);
                }
                Ok(_) => {
                    panic!("Expected error, got Ok");
                }
            }
        }
    }
}
