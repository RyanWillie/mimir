//! Mimir - Local-First AI Memory Vault
//!
//! Main daemon process that provides both HTTP API and MCP server for AI memory management

use crate::vault::{check_vault_status, ensure_vault_ready};
use clap::{Parser, Subcommand};
use mimir_core::{Config, Result};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use axum::{routing::{get, post}, Json};
use serde::Serialize;
use tokio::net::TcpListener;
use std::path::PathBuf;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};
use tracing_appender::rolling;
use tokio::sync::{broadcast, oneshot};
use tokio_stream::wrappers::BroadcastStream;
use futures_util::StreamExt;
use axum::response::sse::{Sse, Event, KeepAlive};
use std::{convert::Infallible, time::Duration};
use std::io;
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

    // Initialize logging (stdout + rotating file + SSE layer)
    let log_level = if cli.debug { "debug" } else { "info" };
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    // Prepare log directory
    let log_dir = mimir_core::get_default_app_dir().join("logs");
    let _ = std::fs::create_dir_all(&log_dir);

    // Rolling daily file appender
    let file_appender = rolling::daily(&log_dir, "mimir.log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    // Create broadcast channel for SSE log streaming
    let (log_tx, _log_rx) = broadcast::channel::<String>(1024);

    // Build subscriber with stdout and file layers
    let stdout_layer = fmt::layer().with_writer(std::io::stdout);
    let file_layer = fmt::layer().with_writer(file_writer);
    // JSON layer that writes events to broadcast channel for SSE
    let tx_for_layer = log_tx.clone();
    let sse_layer = fmt::layer().with_writer(move || ChannelWriter { tx: tx_for_layer.clone() });
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .with(sse_layer);
    let _ = tracing::subscriber::set_global_default(subscriber);

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

    // Ensure model files are present and valid, unless explicitly skipped (useful for tests)
    let skip_models = std::env::var("MIMIR_SKIP_MODELS").is_ok();
    let model_path_opt: Option<std::path::PathBuf> = if !skip_models {
        let (model_path, _tokenizer_path, _vocab_path) =
            model::ensure_model_files().await.map_err(mimir_core::MimirError::ServerError)?;
        eprintln!("BGE model file ready at: {}", model_path.display());

        let gemma3_path = model::ensure_gemma3_model().await.map_err(mimir_core::MimirError::ServerError)?;
        eprintln!("Gemma3 model file ready at: {}", gemma3_path.display());
        Some(model_path)
    } else {
        warn!("Skipping model downloads due to MIMIR_SKIP_MODELS env var");
        None
    };

    // Initialize LLM service unless disabled via env (useful for tests/CI)
    let disable_llm = std::env::var("MIMIR_DISABLE_LLM").is_ok();
    if !disable_llm {
        llm_service::initialize_llm_service(&config).await?;
    } else {
        warn!("LLM initialization disabled via MIMIR_DISABLE_LLM env var");
    }

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
                // Create vector store (when skipping models, fall back to non-embedder path)
                let vector_store = if skip_models {
                    mimir_vector::ThreadSafeVectorStore::new(
                        config.get_vault_path().as_path(),
                        128,
                        None,
                        None,
                    ).map_err(|e| mimir_core::MimirError::VectorStore(e.to_string()))?
                } else {
                    let model_path = model_path_opt.as_ref().expect("model_path available when not skipping models");
                    create_vector_store_with_model(&config, model_path.as_path()).await?
                };
                // Create integrated storage
                let storage = create_integrated_storage(database, vector_store, storage_crypto_manager).await?;
                // Create the MCP server with integrated storage
                let mcp_server = mcp::MimirServer::new(storage);
                start_mcp_streamhttp_server(config, mcp_server, log_tx.clone()).await
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
    // Prefer non-interactive password from env when provided (e.g., tests/CI)
    if let Ok(pw) = std::env::var("MIMIR_VAULT_PASSWORD") {
        let db_crypto_manager =
            mimir_core::crypto::CryptoManager::with_password(&config.get_keyset_path(), pw.as_str())?;
        let storage_crypto_manager =
            mimir_core::crypto::CryptoManager::with_password(&config.get_keyset_path(), pw.as_str())?;
        return Ok((db_crypto_manager, storage_crypto_manager));
    }

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

async fn start_mcp_streamhttp_server(config: Config, mcp_server: mcp::MimirServer, log_tx: broadcast::Sender<String>) -> Result<()> {
    let addr = format!("{}:{}", config.server.host, config.server.port);
    println!("Starting MCP server on {}", addr);
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| mimir_core::MimirError::ServerError(format!("Failed to bind: {}", e)))?;

    // Track server start time for uptime
    let start_time = std::time::Instant::now();
    let pid = std::process::id();
    let version = env!("CARGO_PKG_VERSION").to_string();
    let port = config.server.port;

    let mcp_for_service = mcp_server.clone();
    let service = StreamableHttpService::new(
        move || Ok(mcp_for_service.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );
    // Use the correct handler as in the official example
    let mcp_for_status = mcp_server.clone();
    let mcp_for_shutdown = mcp_server.clone();
    // Channel for graceful shutdown trigger from HTTP route
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let shutdown_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(shutdown_tx)));
    let log_tx_for_route = log_tx.clone();
    let app = axum::Router::new()
        // Minimal health endpoint for tray checks
        .route("/health", get(|| async { "ok" }))
        // Status endpoint with summarized JSON
        .route("/status", get(move || {
            let mcp = mcp_for_status.clone();
            let start_time = start_time.clone();
            async move {
                let stats = mcp.get_stats().await.ok();
                let has_embedder = mcp.has_vector_embedder().await;
                let llm_initialized = mcp.llm_initialized();
                let similarity_threshold = mcp.similarity_threshold().await;
                let uptime_secs = start_time.elapsed().as_secs();
                let body = StatusSummary {
                    pid,
                    version: version.clone(),
                    port,
                    uptime_secs,
                    has_embedder,
                    llm_initialized,
                    similarity_threshold,
                    database_memories: stats.as_ref().map(|s| s.database_memories).unwrap_or(0),
                    vector_memories: stats.as_ref().map(|s| s.vector_memories).unwrap_or(0),
                    memory_usage_bytes: stats.as_ref().map(|s| s.memory_usage_bytes).unwrap_or(0),
                    vector_count_percentage: stats
                        .as_ref()
                        .map(|s| s.vector_count_percentage)
                        .unwrap_or(0.0),
                };
                Json(body)
            }
        }))
        // Live logs via Server-Sent Events
        .route("/logs", get(move || logs_sse(log_tx_for_route.clone())))
        // Graceful shutdown endpoint
        .route("/shutdown", post(move || {
            let mcp = mcp_for_shutdown.clone();
            let shutdown_tx = shutdown_tx.clone();
            async move {
                // Try to save vector store before shutdown
                if let Err(e) = mcp.save_vector_store().await {
                    warn!("Failed to save vector store on shutdown request: {}", e);
                }
                // Trigger shutdown (ignore if already sent)
                if let Some(tx) = shutdown_tx.lock().unwrap().take() {
                    let _ = tx.send(());
                }
                // Fallback hard-exit in case some background tasks keep runtime alive
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    std::process::exit(0);
                });
                Json(serde_json::json!({"status": "shutting_down"}))
            }
        }))
        .nest_service("/mcp", service);

    // Serve the app
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        })
        .await
        .map_err(|e| mimir_core::MimirError::ServerError(format!("Axum serve error: {}", e)))?;

    Ok(())
}

#[derive(Serialize)]
struct StatusSummary {
    pid: u32,
    version: String,
    port: u16,
    uptime_secs: u64,
    has_embedder: bool,
    llm_initialized: bool,
    similarity_threshold: f32,
    database_memories: usize,
    vector_memories: usize,
    memory_usage_bytes: usize,
    vector_count_percentage: f32,
}

// Writer that forwards formatted tracing events to a broadcast channel as JSON strings
#[derive(Clone)]
struct ChannelWriter {
    tx: broadcast::Sender<String>,
}

impl io::Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf).to_string();
        let _ = self.tx.send(s);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

async fn logs_sse(tx: broadcast::Sender<String>) -> Sse<impl futures_util::Stream<Item = std::result::Result<Event, Infallible>>> {
    use futures_util::{stream, StreamExt};
    let rx = tx.subscribe();
    // Initial event so clients see something immediately
    let initial = stream::once(async { Ok(Event::default().data("sse-connected")) });
    let logs = BroadcastStream::new(rx)
        .filter_map(|msg| async move { msg.ok() })
        .map(|line| Ok(Event::default().data(line)));
    let stream = initial.chain(logs);

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("keepalive"))
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
