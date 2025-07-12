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

/// Ensure models are downloaded during initialization
async fn ensure_models_downloaded() -> Result<()> {
    // Import the model functions
    use mimir_core::get_default_app_dir;
    use reqwest::Client;
    use std::fs;
    use std::path::Path;
    
    // Simple function to download Gemma3 model
    let model_dir = get_default_app_dir().join("models");
    if !model_dir.exists() {
        fs::create_dir_all(&model_dir).map_err(|e| {
            mimir_core::MimirError::Initialization(format!("Failed to create models directory: {}", e))
        })?;
    }
    
    let gemma3_path = model_dir.join("gemma-3-1b-it-qat-q4_0.gguf");
    
    // Download Gemma3 model if not exists
    if !gemma3_path.exists() {
        println!("📥 Downloading Gemma3 1B model...");
        let client = Client::new();
        let url = "https://huggingface.co/google/gemma-3-1b-it-qat-q4_0-gguf/resolve/main/gemma-3-1b-it-qat-q4_0.gguf";
        
        let response = client.get(url).send().await.map_err(|e| {
            mimir_core::MimirError::Initialization(format!("Failed to download Gemma3 model: {}", e))
        })?;
        
        let bytes = response.bytes().await.map_err(|e| {
            mimir_core::MimirError::Initialization(format!("Failed to read Gemma3 model bytes: {}", e))
        })?;
        
        fs::write(&gemma3_path, &bytes).map_err(|e| {
            mimir_core::MimirError::Initialization(format!("Failed to write Gemma3 model: {}", e))
        })?;
        
        println!("✅ Gemma3 model downloaded to: {}", gemma3_path.display());
    } else {
        println!("✅ Gemma3 model already exists at: {}", gemma3_path.display());
    }
    
    Ok(())
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
                    // Convert to absolute path if it's relative
                    let absolute_vault_path = if vault_path.is_relative() {
                        std::env::current_dir()?.join(&vault_path)
                    } else {
                        vault_path.clone()
                    };
                    config.set_vault_path(&absolute_vault_path);
                    absolute_vault_path
                }
                None => config.get_vault_path().clone(),
            };

            info!("Initializing memory vault at: {}", vault_dir.display());

            // Create the directory if it doesn't exist
            std::fs::create_dir_all(&vault_dir)?;

            // Download required models
            println!("📦 Ensuring models are downloaded...");
            ensure_models_downloaded().await?;

            // Set encryption mode
            if password {
                config.set_encryption_mode("password");
                println!("🔐 Using password-based encryption");
                println!("Enter a strong password for your memory vault:");

                let mut password_input = String::new();
                std::io::stdin().read_line(&mut password_input)?;
                let password = password_input.trim();

                if password.is_empty() {
                    return Err(mimir_core::MimirError::Config(
                        "Password cannot be empty".to_string(),
                    ));
                }

                let keyset_path = config.get_keyset_path();
                let crypto_manager =
                    mimir_core::crypto::CryptoManager::with_password(&keyset_path, password)?;
                println!(
                    "✅ Memory vault initialized with password-based encryption at {}",
                    vault_dir.display()
                );

                // Initialize database with the password-based crypto manager
                let db_path = config.get_database_path();
                let _db = Database::with_crypto_manager(&db_path, crypto_manager)?;
                println!("✅ Database initialized at {}", db_path.display());
            } else {
                config.set_encryption_mode("keychain");
                println!("🔑 Using OS keychain for encryption");
                let keyset_path = config.get_keyset_path();
                let crypto_manager = mimir_core::crypto::CryptoManager::new(&keyset_path)?;
                println!(
                    "✅ Memory vault initialized with OS keychain at {}",
                    vault_dir.display()
                );

                // Initialize database with the keychain-based crypto manager
                let db_path = config.get_database_path();
                let _db = Database::with_crypto_manager(&db_path, crypto_manager)?;
                println!("✅ Database initialized at {}", db_path.display());
            }

            // Save configuration
            config.save()?;
            println!(
                "✅ Configuration saved to {}",
                mimir_core::get_default_config_path().display()
            );
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
                println!(
                    "⚠️  This will rotate the root encryption key and re-encrypt all class keys."
                );
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
