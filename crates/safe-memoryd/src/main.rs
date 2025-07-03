//! Mimir Safe Memory Daemon
//!
//! Main daemon process that provides the MCP server for AI memory management

use clap::Parser;
use std::path::PathBuf;
use tracing::{info, error};
use mimir_core::{MimirConfig, Result};

mod server;
mod mcp;

/// Safe Memory Daemon - Local-First AI Memory Vault
#[derive(Parser)]
#[command(name = "safe-memoryd")]
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("safe_memoryd={},mimir_core={}", log_level, log_level))
        .init();
    
    info!("Starting Mimir Safe Memory Daemon v{}", env!("CARGO_PKG_VERSION"));
    
    // Load configuration
    let mut config = MimirConfig::default();
    if let Some(port) = cli.port {
        config.server.port = port;
    }
    
    // TODO: Load from config file if provided
    
    // Start the server
    match server::start(config).await {
        Ok(_) => {
            info!("Server shutdown gracefully");
            Ok(())
        }
        Err(e) => {
            error!("Server error: {}", e);
            Err(e)
        }
    }
} 