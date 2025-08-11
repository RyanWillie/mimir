//! Mimir Tray - Main entry point
//!
//! Licensed under AGPL-3.0 to keep derivative UIs open-source

use mimir_tray::TrayApp;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mimir_tray=info,mimir_core=info")
        .init();

    info!("Starting Mimir Tray v{}", env!("CARGO_PKG_VERSION"));

    // Check for command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "start" => {
                info!("Starting Mimir daemon...");
                let tray_app = TrayApp::new()?;
                tray_app.start_daemon().await?;
                info!("Daemon started successfully");
                return Ok(());
            }
            "stop" => {
                info!("Stopping Mimir daemon...");
                let tray_app = TrayApp::new()?;
                tray_app.stop_daemon().await?;
                info!("Daemon stopped successfully");
                return Ok(());
            }
            "status" => {
                let tray_app = TrayApp::new()?;
                let status = tray_app.get_service_status().await;
                let is_running = tray_app.is_daemon_running().await;
                println!("Service Status: {:?}", status);
                println!("Daemon Running: {}", is_running);
                return Ok(());
            }
            "test" => {
                info!("Running tray application test...");
                let tray_app = TrayApp::new()?;
                
                // Test service status
                let status = tray_app.get_service_status().await;
                info!("Initial service status: {:?}", status);
                
                // Test daemon start
                info!("Testing daemon start...");
                if let Err(e) = tray_app.start_daemon().await {
                    error!("Failed to start daemon: {}", e);
                } else {
                    info!("Daemon started successfully");
                    
                    // Wait a moment and check status
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    let status = tray_app.get_service_status().await;
                    let is_running = tray_app.is_daemon_running().await;
                    info!("Service status after start: {:?}", status);
                    info!("Daemon running: {}", is_running);
                    
                    // Test daemon stop
                    info!("Testing daemon stop...");
                    if let Err(e) = tray_app.stop_daemon().await {
                        error!("Failed to stop daemon: {}", e);
                    } else {
                        info!("Daemon stopped successfully");
                    }
                }
                return Ok(());
            }
            _ => {
                println!("Usage: mimir-tray [start|stop|status|test]");
                println!("  start  - Start the Mimir daemon");
                println!("  stop   - Stop the Mimir daemon");
                println!("  status - Show daemon status");
                println!("  test   - Run a test of daemon management");
                return Ok(());
            }
        }
    }

    // Create and run the tray application
    match TrayApp::new() {
        Ok(tray_app) => {
            info!("Tray application initialized successfully");
            if let Err(e) = tray_app.run().await {
                error!("Failed to run tray application: {}", e);
                return Err(Box::new(e) as Box<dyn std::error::Error>);
            }
        }
        Err(e) => {
            error!("Failed to initialize tray application: {}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    }

    info!("Mimir Tray shutdown complete");
    Ok(())
} 