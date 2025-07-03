//! Mimir CLI - Command-line interface for the AI Memory Vault

use clap::{Parser, Subcommand};
use mimir_core::Result;
use tracing::info;

/// Safe Memory CLI - Manage your local AI memory vault
#[derive(Parser)]
#[command(name = "safe-memory")]
#[command(about = "A CLI for managing Mimir AI Memory Vault")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new memory vault
    Init {
        /// Vault directory path
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Show vault status
    Status,
    /// Start the daemon
    Start {
        /// Run in background
        #[arg(short, long)]
        daemon: bool,
    },
    /// Stop the daemon
    Stop,
    /// Burn (delete) memories by class
    Burn {
        /// Memory class to burn
        #[arg(value_enum)]
        class: BurnTarget,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum BurnTarget {
    Personal,
    Work,
    Health,
    Financial,
    All,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { path } => {
            let vault_path = path.unwrap_or_else(|| "./vault".to_string());
            info!("Initializing memory vault at: {}", vault_path);
            // TODO: Implement vault initialization
            println!("✅ Memory vault initialized at {}", vault_path);
        }
        Commands::Status => {
            info!("Checking vault status");
            // TODO: Implement status check
            println!("🔍 Vault status: Ready");
        }
        Commands::Start { daemon } => {
            info!("Starting Mimir daemon (daemon={})", daemon);
            // TODO: Implement daemon start
            println!("🚀 Mimir daemon started");
        }
        Commands::Stop => {
            info!("Stopping Mimir daemon");
            // TODO: Implement daemon stop
            println!("🛑 Mimir daemon stopped");
        }
        Commands::Burn { class } => {
            info!("Burning memories: {:?}", class);
            // TODO: Implement memory burning with confirmation
            println!("🔥 Memories burned: {:?}", class);
        }
    }
    
    Ok(())
} 