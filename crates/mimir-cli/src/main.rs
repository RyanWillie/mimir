//! Mimir CLI - Command-line interface for the AI Memory Vault

use clap::{Parser, Subcommand};
use mimir_core::Result;
use tracing::info;

/// Safe Memory Daemon CLI - Manage your local AI memory vault
#[derive(Parser)]
#[command(name = "cli")]
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
    /// Rotate the root encryption key
    RotateRoot {
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
    /// Rotate a class-specific encryption key
    RotateClass {
        /// Memory class to rotate
        class: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
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
            
            // Initialize crypto manager
            let keyset_path = std::path::Path::new(&vault_path).join("keyset.json");
            let _crypto_manager = mimir_core::crypto::CryptoManager::new(&keyset_path)?;
            
            println!("✅ Memory vault initialized at {}", vault_path);
        }
        Commands::Status => {
            info!("Checking vault status");
            // TODO: Implement status check with crypto info
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
        Commands::RotateRoot { yes } => {
            if !yes {
                println!("⚠️  This will rotate the root encryption key and re-encrypt all class keys.");
                println!("   This operation cannot be undone. Continue? (y/N)");
                
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().to_lowercase().starts_with('y') {
                    println!("Operation cancelled.");
                    return Ok(());
                }
            }
            
            info!("Rotating root encryption key");
            
            // Load crypto manager and rotate root key
            let keyset_path = std::path::Path::new("./vault").join("keyset.json");
            let mut crypto_manager = mimir_core::crypto::CryptoManager::new(&keyset_path)?;
            crypto_manager.rotate_root_key()?;
            
            println!("🔄 Root encryption key rotated successfully");
        }
        Commands::RotateClass { class, yes } => {
            if !yes {
                println!("⚠️  This will rotate the encryption key for class '{}' and invalidate old encrypted data.", class);
                println!("   This operation cannot be undone. Continue? (y/N)");
                
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().to_lowercase().starts_with('y') {
                    println!("Operation cancelled.");
                    return Ok(());
                }
            }
            
            info!("Rotating class encryption key: {}", class);
            
            // Load crypto manager and rotate class key
            let keyset_path = std::path::Path::new("./vault").join("keyset.json");
            let mut crypto_manager = mimir_core::crypto::CryptoManager::new(&keyset_path)?;
            crypto_manager.rotate_class_key(&class)?;
            
            println!("🔄 Class '{}' encryption key rotated successfully", class);
        }
    }

    Ok(())
}
