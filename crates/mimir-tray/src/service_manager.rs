//! Service manager for handling Mimir daemon process

use mimir_core::{Config, Result};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn};

/// Service status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Running { pid: u32, uptime: Duration },
    Stopped,
    Starting,
    Stopping,
    Error { message: String },
}

/// Service manager for handling Mimir daemon
pub struct ServiceManager {
    config: Config,
    daemon_process: Option<Child>,
    status: ServiceStatus,
    start_time: Option<Instant>,
}

impl ServiceManager {
    /// Create a new service manager
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            daemon_process: None,
            status: ServiceStatus::Stopped,
            start_time: None,
        })
    }

    /// Start the Mimir daemon
    pub async fn start_daemon(&mut self) -> Result<()> {
        if matches!(self.status, ServiceStatus::Running { .. } | ServiceStatus::Starting) {
            info!("Daemon is already running or starting");
            return Ok(());
        }

        self.status = ServiceStatus::Starting;
        info!("Starting Mimir daemon...");

        // Check if daemon is already running
        if self.is_daemon_running().await {
            info!("Daemon is already running");
            self.status = ServiceStatus::Running {
                pid: 0, // We'll get the actual PID later
                uptime: Duration::from_secs(0),
            };
            return Ok(());
        }

        // Start the daemon process using cargo run
        // Equivalent to: cargo run --bin mimir -- --auto-init --port <PORT> [--config <PATH>] mcp
        let cargo_path = self.get_cargo_path()?;
        let mut command = Command::new(cargo_path);

        // Build program args for the mimir binary (must be after "--")
        let mut program_args: Vec<String> = Vec::new();
        program_args.push("--auto-init".to_string());
        // Pin the port to what the tray expects
        program_args.push("--port".to_string());
        program_args.push(self.config.server.port.to_string());

        // Add configuration if available
        let config_path = mimir_core::get_default_config_path();
        if config_path.exists() {
            program_args.push("--config".to_string());
            program_args.push(config_path.to_string_lossy().to_string());
        }

        // Subcommand last so clap parses global flags first
        program_args.push("mcp".to_string());

        command
            .arg("run")
            .arg("--bin")
            .arg("mimir")
            .arg("--")
            .args(program_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match command.spawn() {
            Ok(child) => {
                self.daemon_process = Some(child);
                self.start_time = Some(Instant::now());
                
                // Wait a moment for the process to start
                sleep(Duration::from_millis(500)).await;
                
                // Check if process is still running
                if let Some(ref mut child) = self.daemon_process {
                    match child.try_wait() {
                        Ok(None) => {
                            self.status = ServiceStatus::Running {
                                pid: child.id(),
                                uptime: Duration::from_secs(0),
                            };
                            info!("Daemon started successfully with PID: {}", child.id());
                            Ok(())
                        }
                        Ok(Some(exit_status)) => {
                            self.status = ServiceStatus::Error {
                                message: format!("Daemon exited with status: {}", exit_status),
                            };
                            self.daemon_process = None;
                            self.start_time = None;
                            Err(mimir_core::MimirError::ServerError(
                                format!("Daemon exited with status: {}", exit_status)
                            ))
                        }
                        Err(e) => {
                            self.status = ServiceStatus::Error {
                                message: format!("Failed to check daemon status: {}", e),
                            };
                            self.daemon_process = None;
                            self.start_time = None;
                            Err(mimir_core::MimirError::ServerError(
                                format!("Failed to check daemon status: {}", e)
                            ))
                        }
                    }
                } else {
                    self.status = ServiceStatus::Error {
                        message: "Failed to start daemon process".to_string(),
                    };
                    Err(mimir_core::MimirError::ServerError(
                        "Failed to start daemon process".to_string()
                    ))
                }
            }
            Err(e) => {
                self.status = ServiceStatus::Error {
                    message: format!("Failed to spawn daemon process: {}", e),
                };
                Err(mimir_core::MimirError::ServerError(
                    format!("Failed to spawn daemon process: {}", e)
                ))
            }
        }
    }

    /// Stop the Mimir daemon
    pub async fn stop_daemon(&mut self) -> Result<()> {
        if matches!(self.status, ServiceStatus::Stopped | ServiceStatus::Stopping) {
            info!("Daemon is already stopped or stopping");
            return Ok(());
        }

        self.status = ServiceStatus::Stopping;
        info!("Stopping Mimir daemon...");

        // Try to stop our managed process first
        if let Some(mut child) = self.daemon_process.take() {
            match child.kill() {
                Ok(_) => {
                    info!("Sent kill signal to daemon process");
                    // Wait for process to terminate
                    match child.wait() {
                        Ok(_) => {
                            info!("Daemon process terminated");
                            self.status = ServiceStatus::Stopped;
                            self.start_time = None;
                            Ok(())
                        }
                        Err(e) => {
                            warn!("Failed to wait for daemon process: {}", e);
                            self.status = ServiceStatus::Stopped;
                            self.start_time = None;
                            Ok(())
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to kill daemon process: {}", e);
                    // Try to stop any running daemon
                    self.stop_any_daemon().await
                }
            }
        } else {
            // No managed process, try to stop any running daemon
            self.stop_any_daemon().await
        }
    }

    /// Stop any running daemon process
    async fn stop_any_daemon(&mut self) -> Result<()> {
        // Try to find and stop any running mimir daemon
        // Since we're using cargo run, look for the mimir binary process
        let daemon_name = "mimir";

        // On Unix-like systems, try to find and kill the process
        #[cfg(unix)]
        {
            let output = Command::new("pgrep")
                .arg("-f")
                .arg(daemon_name)
                .output();

            if let Ok(output) = output {
                if !output.stdout.is_empty() {
                    let pids: Vec<&str> = std::str::from_utf8(&output.stdout)
                        .unwrap_or("")
                        .trim()
                        .split('\n')
                        .collect();

                    for pid in pids {
                        if let Ok(pid_num) = pid.parse::<u32>() {
                            if let Err(e) = Command::new("kill").arg(pid).output() {
                                warn!("Failed to kill process {}: {}", pid_num, e);
                            } else {
                                info!("Killed daemon process: {}", pid_num);
                            }
                        }
                    }
                }
            }
        }

        // On Windows, try to find and kill the process
        #[cfg(windows)]
        {
            let output = Command::new("tasklist")
                .arg("/FI")
                .arg(format!("IMAGENAME eq {}", daemon_name))
                .output();

            if let Ok(output) = output {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains(daemon_name) {
                    if let Err(e) = Command::new("taskkill")
                        .arg("/F")
                        .arg("/IM")
                        .arg(daemon_name)
                        .output() {
                        warn!("Failed to kill daemon process: {}", e);
                    } else {
                        info!("Killed daemon process: {}", daemon_name);
                    }
                }
            }
        }

        self.status = ServiceStatus::Stopped;
        self.start_time = None;
        Ok(())
    }

    /// Get the current service status
    pub fn get_status(&self) -> ServiceStatus {
        self.status.clone()
    }

    /// Check if the daemon is running
    pub async fn is_daemon_running(&self) -> bool {
        // If we have already marked the service as running, consider it running
        if matches!(self.status, ServiceStatus::Running { .. }) {
            return true;
        }

        // Try to connect to the daemon's HTTP API with a few quick retries
        let client = reqwest::Client::new();
        let url = format!("http://localhost:{}/health", self.config.server.port);

        let max_attempts = 10u8; // ~5s total with 500ms backoff
        for _attempt in 1..=max_attempts {
            match client
                .get(&url)
                .timeout(Duration::from_secs(1))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => return true,
                _ => {
                    // brief backoff before trying again
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }

        // Fallback: if a mimir process exists, consider it running (likely initializing)
        #[cfg(unix)]
        {
            if let Ok(output) = Command::new("pgrep").arg("-f").arg("mimir").output() {
                if !output.stdout.is_empty() {
                    return true;
                }
            }
        }

        #[cfg(windows)]
        {
            if let Ok(output) = Command::new("tasklist").output() {
                let out = String::from_utf8_lossy(&output.stdout);
                if out.contains("mimir.exe") || out.contains("mimir") {
                    return true;
                }
            }
        }

        false
    }

    /// Get daemon uptime
    pub fn get_uptime(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }

    /// Get the path to cargo for running the daemon
    fn get_cargo_path(&self) -> Result<std::path::PathBuf> {
        // Try to find cargo in PATH
        if let Ok(path) = std::env::var("PATH") {
            for dir in path.split(':') {
                let cargo_path = std::path::Path::new(dir).join("cargo");
                if cargo_path.exists() {
                    return Ok(cargo_path);
                }
            }
        }

        Err(mimir_core::MimirError::Initialization(
            "Could not find cargo executable".to_string()
        ))
    }

    /// Update the service status based on current state
    pub async fn update_status(&mut self) {
        if let ServiceStatus::Running { pid, .. } = self.status {
            // Check if our managed process is still running
            if let Some(ref mut child) = self.daemon_process {
                match child.try_wait() {
                    Ok(None) => {
                        // Process is still running, update uptime
                        if let Some(start_time) = self.start_time {
                            self.status = ServiceStatus::Running {
                                pid,
                                uptime: start_time.elapsed(),
                            };
                        }
                    }
                    Ok(Some(_)) => {
                        // Process has exited
                        self.status = ServiceStatus::Stopped;
                        self.daemon_process = None;
                        self.start_time = None;
                    }
                    Err(_) => {
                        // Error checking process, assume it's stopped
                        self.status = ServiceStatus::Stopped;
                        self.daemon_process = None;
                        self.start_time = None;
                    }
                }
            } else {
                // No managed process, check if daemon is running via API or process presence
                if self.is_daemon_running().await {
                    // We don't know PID; use uptime if we have it, else zero
                    let uptime = self.start_time.map(|s| s.elapsed()).unwrap_or_default();
                    self.status = ServiceStatus::Running { pid: 0, uptime };
                } else {
                    self.status = ServiceStatus::Stopped;
                    self.start_time = None;
                }
            }
        }
    }
} 