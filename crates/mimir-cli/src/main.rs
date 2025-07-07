//! Mimir CLI - Command-line interface for the AI Memory Vault

use clap::{Parser, Subcommand};
use mimir_core::{Config, Result};
use mimir_db::Database;
use tracing::info;

  /// Mimir CLI - Manage your local AI memory vault
  #[derive(Parser)]
  #[command(name = "mimir")]
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
        /// Use password-based encryption instead of OS keychain
        #[arg(long)]
        password: bool,
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
        Commands::Init { path, password } => {
            // Load existing config or create new one
            let mut config = Config::load().unwrap_or_else(|_| Config::new());
            
            // Update vault path if provided
            let vault_dir = match path {
                Some(p) => {
                    let vault_path = std::path::PathBuf::from(p);
                    config.set_vault_path(&vault_path);
                    vault_path
                }
                None => config.get_vault_path().clone(),
            };
            
            info!("Initializing memory vault at: {}", vault_dir.display());
            
            // Create the directory if it doesn't exist
            std::fs::create_dir_all(&vault_dir)?;
            
            // Set encryption mode
            if password {
                config.set_encryption_mode("password");
                println!("🔐 Using password-based encryption");
                println!("Enter a strong password for your memory vault:");
                
                let mut password_input = String::new();
                std::io::stdin().read_line(&mut password_input)?;
                let password = password_input.trim();
                
                if password.is_empty() {
                    return Err(mimir_core::MimirError::Config("Password cannot be empty".to_string()));
                }
                
                let keyset_path = config.get_keyset_path();
                let crypto_manager = mimir_core::crypto::CryptoManager::with_password(&keyset_path, password)?;
                println!("✅ Memory vault initialized with password-based encryption at {}", vault_dir.display());
                
                // Initialize database with the password-based crypto manager
                let db_path = config.get_database_path();
                let _db = Database::with_crypto_manager(&db_path, crypto_manager)?;
                println!("✅ Database initialized at {}", db_path.display());
            } else {
                config.set_encryption_mode("keychain");
                println!("🔑 Using OS keychain for encryption");
                let keyset_path = config.get_keyset_path();
                let crypto_manager = mimir_core::crypto::CryptoManager::new(&keyset_path)?;
                println!("✅ Memory vault initialized with OS keychain at {}", vault_dir.display());
                
                // Initialize database with the keychain-based crypto manager
                let db_path = config.get_database_path();
                let _db = Database::with_crypto_manager(&db_path, crypto_manager)?;
                println!("✅ Database initialized at {}", db_path.display());
            }
            
            // Save configuration
            config.save()?;
            println!("✅ Configuration saved to {}", mimir_core::get_default_config_path().display());
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
            let keyset_path = mimir_core::config::get_default_keyset_path();
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
            let keyset_path = mimir_core::config::get_default_keyset_path();
            let mut crypto_manager = mimir_core::crypto::CryptoManager::new(&keyset_path)?;
            crypto_manager.rotate_class_key(&class)?;
            
            println!("🔄 Class '{}' encryption key rotated successfully", class);
        }
    }

    Ok(())
}
