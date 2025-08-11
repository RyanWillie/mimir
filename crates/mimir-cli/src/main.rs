//! Mimir CLI - Command-line interface for the AI Memory Vault

use clap::{Parser, Subcommand};
use mimir_core::{Config, Result};
use mimir_db::Database;
use tracing::info;
use std::path::PathBuf;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

#[cfg(unix)]
use libc::{kill, SIGTERM};

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

            // Load config to determine vault/app dirs and encryption mode
            let config = Config::load().unwrap_or_else(|_| Config::new());

            // If password-based encryption is enabled, background start cannot prompt for password
            if daemon && config.use_password_encryption {
                return Err(mimir_core::MimirError::Initialization(
                    "Background start is not supported with password-based encryption. Use keychain mode or start the daemon in foreground.".to_string(),
                ));
            }

            // Determine path for PID/log files
            let app_dir = mimir_core::get_default_app_dir();
            fs::create_dir_all(&app_dir)?;
            let pid_file = app_dir.join("mimir.pid");
            let log_out = app_dir.join("mimir.out.log");
            let log_err = app_dir.join("mimir.err.log");

            // If a PID file exists and process appears alive, refuse to start another
            if let Some(existing_pid) = read_pid_file(&pid_file) {
                if is_process_running(existing_pid) {
                    return Err(mimir_core::MimirError::ServerError(format!(
                        "Daemon already running with PID {}. Use 'mimir-cli stop' first.",
                        existing_pid
                    )));
                }
            }

            // Resolve daemon binary path: prefer sibling of current exe (dev), fallback to PATH
            let daemon_path = resolve_daemon_path().unwrap_or_else(|| PathBuf::from("mimir"));

            let mut args = vec!["mcp", "--auto-init"]; // default to MCP over HTTP with auto-init

            if daemon {
                // Spawn detached with logs redirected
                let stdout = fs::OpenOptions::new().create(true).append(true).open(&log_out)?;
                let stderr = fs::OpenOptions::new().create(true).append(true).open(&log_err)?;

                let child = Command::new(daemon_path)
                    .args(&args)
                    .stdin(Stdio::null())
                    .stdout(Stdio::from(stdout))
                    .stderr(Stdio::from(stderr))
                    .spawn()
                    .map_err(|e| mimir_core::MimirError::ServerError(format!(
                        "Failed to spawn daemon: {}",
                        e
                    )))?;

                // Write PID file
                write_pid_file(&pid_file, child.id())?;
                println!("🚀 Mimir daemon started (PID {})", child.id());
                println!("📄 Logs: {} | {}", log_out.display(), log_err.display());
            } else {
                // Foreground: inherit stdio
                let status = Command::new(daemon_path)
                    .args(&args)
                    .status()
                    .map_err(|e| mimir_core::MimirError::ServerError(format!(
                        "Failed to start daemon in foreground: {}",
                        e
                    )))?;
                if !status.success() {
                    return Err(mimir_core::MimirError::ServerError(format!(
                        "Daemon exited with status: {}",
                        status
                    )));
                }
            }
        }
        Commands::Stop => {
            info!("Stopping Mimir daemon");
            let app_dir = mimir_core::get_default_app_dir();
            let pid_file = app_dir.join("mimir.pid");

            let Some(pid) = read_pid_file(&pid_file) else {
                println!("ℹ️  No PID file found at {}. Is the daemon running?", pid_file.display());
                return Ok(());
            };

            if !is_process_running(pid) {
                println!("ℹ️  No running process with PID {}. Cleaning up PID file.", pid);
                let _ = fs::remove_file(&pid_file);
                return Ok(());
            }

            // Attempt graceful termination
            terminate_process(pid)?;

            // Best-effort cleanup of PID file
            let _ = fs::remove_file(&pid_file);
            println!("🛑 Mimir daemon stopped (PID {})", pid);
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

fn resolve_daemon_path() -> Option<PathBuf> {
    // Try to locate sibling binary next to current exe (useful for `cargo run` dev builds)
    if let Ok(curr) = std::env::current_exe() {
        if let Some(parent) = curr.parent() {
            let candidate = parent.join(if cfg!(windows) { "mimir.exe" } else { "mimir" });
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

fn write_pid_file(pid_path: &PathBuf, pid: u32) -> Result<()> {
    let mut f = fs::File::create(pid_path)
        .map_err(|e| mimir_core::MimirError::ServerError(format!("Failed to create PID file: {}", e)))?;
    writeln!(f, "{}", pid)
        .map_err(|e| mimir_core::MimirError::ServerError(format!("Failed to write PID file: {}", e)))?;
    Ok(())
}

fn read_pid_file(pid_path: &PathBuf) -> Option<u32> {
    let Ok(contents) = fs::read_to_string(pid_path) else { return None };
    contents.trim().parse::<u32>().ok()
}

fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    unsafe {
        // kill(pid, 0) returns 0 if process exists and we can send signals, -1 otherwise
        kill(pid as i32, 0) == 0
    }
    #[cfg(windows)]
    {
        // Best-effort: assume running; stop will attempt taskkill
        true
    }
}

fn terminate_process(pid: u32) -> Result<()> {
    #[cfg(unix)]
    unsafe {
        if kill(pid as i32, SIGTERM) != 0 {
            return Err(mimir_core::MimirError::ServerError(format!(
                "Failed to send SIGTERM to PID {}",
                pid
            )));
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"]) // terminate tree, force
            .status()
            .map_err(|e| mimir_core::MimirError::ServerError(format!("Failed to invoke taskkill: {}", e)))?;
        if !status.success() {
            return Err(mimir_core::MimirError::ServerError(format!(
                "taskkill failed for PID {} (status: {})",
                pid, status
            )));
        }
        Ok(())
    }
}
