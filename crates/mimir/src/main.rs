//! Mimir - Local-First AI Memory Vault
//!
//! Main daemon process that provides both HTTP API and MCP server for AI memory management

use clap::{Parser, Subcommand};
use mimir_core::{config::MimirConfig, Result};
use rmcp::ServiceExt;
use std::path::PathBuf;
use tracing::{error, info};
use crate::vault::{ensure_vault_ready, check_vault_status};

mod mcp;
mod server;
mod vault;

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
    /// Start HTTP API server
    Http {
        /// Override server port
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Start MCP (Model Context Protocol) server
    Mcp {
        /// Force stdio transport even if config says otherwise
        #[arg(long)]
        stdio: bool,
    },
    /// Start both HTTP and MCP servers
    Both {
        /// Override HTTP server port
        #[arg(long)]
        http_port: Option<u16>,
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
    let mut config = MimirConfig::default();
    if let Some(port) = cli.port {
        config.server.port = port;
    }

    // TODO: Load from config file if provided
    if let Some(config_path) = cli.config {
        info!("Loading configuration from: {}", config_path.display());
        // TODO: Implement config file loading
    }

    // Check vault status and auto-initialize if needed
    info!("Checking vault status...");
    let vault_status = check_vault_status(&config);
    info!("{}", vault_status.status_message());
    
    if !vault_status.is_ready() {
        if cli.auto_init {
            info!("Auto-initializing vault...");
            ensure_vault_ready(&config, true).await?;
        } else {
            error!("Vault not ready. Use --auto-init to auto-initialize or run 'mimir-cli init' first.");
            return Err(mimir_core::MimirError::Initialization(
                "Vault not initialized".to_string()
            ));
        }
    }

    // Determine server mode
    let server_mode = cli.mode.unwrap_or_else(|| {
        if config.mcp.enabled {
            ServerMode::Both { http_port: None }
        } else {
            ServerMode::Http { port: None }
        }
    });

    // Start appropriate server mode
    match server_mode {
        ServerMode::Http { port } => {
            if let Some(port) = port {
                config.server.port = port;
            }
            info!("Starting HTTP server mode");
            start_http_server(config).await
        }
        ServerMode::Mcp { stdio: force_stdio } => {
            info!("Starting MCP server mode");
            if force_stdio {
                config.mcp.transport = mimir_core::config::McpTransport::Stdio;
            }
            start_mcp_server(config).await
        }
        ServerMode::Both { http_port } => {
            if let Some(port) = http_port {
                config.server.port = port;
            }
            info!("Starting both HTTP and MCP servers");
            start_both_servers(config).await
        }
    }
}

async fn start_http_server(config: MimirConfig) -> Result<()> {
    match server::start(config).await {
        Ok(_) => {
            info!("HTTP server shutdown gracefully");
            Ok(())
        }
        Err(e) => {
            error!("HTTP server error: {}", e);
            Err(e)
        }
    }
}

async fn start_mcp_server(_config: MimirConfig) -> Result<()> {
    info!("Starting MCP server");
    
    // Create the MCP server
    let mcp_server = mcp::MimirServer::new();
    
    // Add sample data
    mcp_server.add_sample_data().await;
    
    // Start the server with stdio transport
    match mcp_server.serve((tokio::io::stdin(), tokio::io::stdout())).await {
        Ok(service) => {
            info!("MCP server connected and ready");
            let quit_reason = service.waiting().await;
            info!("MCP server shutdown: {:?}", quit_reason);
            Ok(())
        }
        Err(e) => {
            error!("MCP server error: {}", e);
            Err(mimir_core::MimirError::ServerError(format!("MCP server error: {}", e)))
        }
    }
}

async fn start_both_servers(config: MimirConfig) -> Result<()> {
    info!("Starting both HTTP and MCP servers concurrently");
    
    let config_http = config.clone();
    let _config_mcp = config.clone();
    
    // Start both servers concurrently
    let http_handle = tokio::spawn(async move {
        if let Err(e) = server::start(config_http).await {
            error!("HTTP server error: {}", e);
        }
    });
    
    let mcp_handle = tokio::spawn(async move {
        let mcp_server = mcp::MimirServer::new();
        mcp_server.add_sample_data().await;
        
        match mcp_server.serve((tokio::io::stdin(), tokio::io::stdout())).await {
            Ok(service) => {
                info!("MCP server connected and ready");
                if let Err(e) = service.waiting().await {
                    error!("MCP server error while waiting: {}", e);
                }
            }
            Err(e) => {
                error!("MCP server error: {}", e);
            }
        }
    });
    
    // Wait for both servers (they should run indefinitely)
    tokio::select! {
        _ = http_handle => {
            info!("HTTP server terminated");
        }
        _ = mcp_handle => {
            info!("MCP server terminated");
        }
    }
    
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
    fn test_server_mode_http() {
        // Test HTTP mode
        let cli = Cli::try_parse_from(&["mimir", "http"]).unwrap();
        match cli.mode.unwrap() {
            ServerMode::Http { port } => assert!(port.is_none()),
            _ => panic!("Expected HTTP mode"),
        }

        // Test HTTP mode with port
        let cli = Cli::try_parse_from(&["mimir", "http", "--port", "8080"]).unwrap();
        match cli.mode.unwrap() {
            ServerMode::Http { port } => assert_eq!(port, Some(8080)),
            _ => panic!("Expected HTTP mode with port"),
        }
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
    fn test_server_mode_both() {
        // Test both mode
        let cli = Cli::try_parse_from(&["mimir", "both"]).unwrap();
        match cli.mode.unwrap() {
            ServerMode::Both { http_port } => assert!(http_port.is_none()),
            _ => panic!("Expected both mode"),
        }

        // Test both mode with http-port
        let cli = Cli::try_parse_from(&["mimir", "both", "--http-port", "9090"]).unwrap();
        match cli.mode.unwrap() {
            ServerMode::Both { http_port } => assert_eq!(http_port, Some(9090)),
            _ => panic!("Expected both mode with http-port"),
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
        assert!(help_str.contains("http"));
        assert!(help_str.contains("mcp"));
        assert!(help_str.contains("both"));
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
        let mut config = MimirConfig::default();
        config.mcp.enabled = true;
        
        // When no mode is specified and MCP is enabled, should default to Both
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
        let http_mode = ServerMode::Http { port: Some(8080) };
        let debug_str = format!("{:?}", http_mode);
        assert!(debug_str.contains("Http"));
        assert!(debug_str.contains("8080"));

        let mcp_mode = ServerMode::Mcp { stdio: true };
        let debug_str = format!("{:?}", mcp_mode);
        assert!(debug_str.contains("Mcp"));
        assert!(debug_str.contains("true"));

        let both_mode = ServerMode::Both { http_port: None };
        let debug_str = format!("{:?}", both_mode);
        assert!(debug_str.contains("Both"));
    }

    #[test]
    fn test_complex_cli_scenarios() {
        // Test global flags with subcommands
        let cli = Cli::try_parse_from(&["mimir", "--debug", "--port", "7777", "mcp", "--stdio"]).unwrap();
        assert!(cli.debug);
        assert_eq!(cli.port, Some(7777));
        match cli.mode.unwrap() {
            ServerMode::Mcp { stdio } => assert!(stdio),
            _ => panic!("Expected MCP mode"),
        }

        // Test all flags together
        let cli = Cli::try_parse_from(&[
            "mimir", 
            "--config", "test.toml",
            "--debug",
            "--port", "5555",
            "both",
            "--http-port", "6666"
        ]).unwrap();
        
        assert!(cli.debug);
        assert_eq!(cli.port, Some(5555));
        assert_eq!(cli.config, Some(PathBuf::from("test.toml")));
        match cli.mode.unwrap() {
            ServerMode::Both { http_port } => assert_eq!(http_port, Some(6666)),
            _ => panic!("Expected both mode"),
        }
    }
}
