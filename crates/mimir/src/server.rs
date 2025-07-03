use mimir_core::{config::MimirConfig, Result};
use tracing::info;
use axum::{routing::get, Router};
use tokio::net::TcpListener;

/// Create the Axum application with all routes configured
pub async fn create_app(_config: MimirConfig) -> Result<Router> {
    // Create a simple health check endpoint
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/", get(root_handler));

    Ok(app)
}

/// Start the Mimir server with the given configuration
pub async fn start(config: MimirConfig) -> Result<()> {
    info!("Starting Mimir server on {}:{}", config.server.host, config.server.port);
    
    // Create the application
    let app = create_app(config.clone()).await?;
    
    let bind_address = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&bind_address).await
        .map_err(|e| mimir_core::MimirError::ServerError(format!("Failed to bind to {}: {}", bind_address, e)))?;
    
    info!("🚀 Mimir server is running at http://{}", bind_address);
    info!("📊 Health check available at http://{}/health", bind_address);
    
    // Start the server
    axum::serve(listener, app).await
        .map_err(|e| mimir_core::MimirError::ServerError(format!("Server error: {}", e)))?;
    
    Ok(())
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// Root endpoint with welcome message
async fn root_handler() -> &'static str {
    "🧠 Mimir AI Memory Vault - Server is running!"
} 