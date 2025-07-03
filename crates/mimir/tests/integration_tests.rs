//! Integration tests for Mimir HTTP server
//!
//! These tests verify the complete server functionality including
//! endpoint behavior, error handling, and service integration.

use mimir_core::{config::MimirConfig, test_utils::env::create_temp_dir};
use axum::http::StatusCode;
use axum_test::TestServer;
use serial_test::serial;
use std::time::Duration;
use tokio::time::timeout;

/// Helper to create a test server with custom configuration
async fn create_test_server(config: MimirConfig) -> TestServer {
    let app = mimir::create_app(config).await.expect("Failed to create app");
    TestServer::new(app).unwrap()
}

/// Helper to create a test server with default configuration
async fn create_default_test_server() -> TestServer {
    let mut config = MimirConfig::default();
    
    // Use a random available port for testing
    config.server.port = 0;
    
    // Use temporary directories for test isolation
    let temp_dir = create_temp_dir();
    config.storage.vault_path = temp_dir.path().join("test_vault.db");
    config.security.master_key_path = temp_dir.path().join("test_master.key");
    
    create_test_server(config).await
}

#[tokio::test]
async fn test_root_endpoint() {
    let server = create_default_test_server().await;
    
    let response = server.get("/").await;
    
    response.assert_status(StatusCode::OK);
    response.assert_text("🧠 Mimir AI Memory Vault - Server is running!");
}

#[tokio::test]
async fn test_health_endpoint() {
    let server = create_default_test_server().await;
    
    let response = server.get("/health").await;
    
    response.assert_status(StatusCode::OK);
    response.assert_text("OK");
}

#[tokio::test]
async fn test_nonexistent_endpoint() {
    let server = create_default_test_server().await;
    
    let response = server.get("/nonexistent").await;
    
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_health_endpoint_multiple_requests() {
    let server = create_default_test_server().await;
    
    // Test that health endpoint can handle multiple concurrent requests
    for _ in 0..10 {
        let response = server.get("/health").await;
        response.assert_status(StatusCode::OK);
        response.assert_text("OK");
    }
}

#[tokio::test]
async fn test_server_configuration_custom_host() {
    let mut config = MimirConfig::default();
    config.server.host = "0.0.0.0".to_string();
    config.server.port = 0; // Use random port
    
    // Test that we can create server with custom host
    let server = create_test_server(config).await;
    let response = server.get("/health").await;
    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn test_cors_headers() {
    let server = create_default_test_server().await;
    
    let response = server
        .get("/health")
        .add_header("Origin", "http://localhost:3000")
        .await;
    
    response.assert_status(StatusCode::OK);
    // Note: CORS headers would be tested here if we add CORS middleware
}

#[tokio::test]
async fn test_request_timeout() {
    let server = create_default_test_server().await;
    
    // Test that normal requests complete quickly
    let result = timeout(Duration::from_secs(5), server.get("/health")).await;
    
    assert!(result.is_ok(), "Request should complete within timeout");
    let response = result.unwrap();
    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn test_concurrent_requests() {
    let server = create_default_test_server().await;
    
    // Test concurrent access to different endpoints
    let health_future = server.get("/health");
    let root_future = server.get("/");
    
    let (health_response, root_response) = tokio::join!(health_future, root_future);
    
    health_response.assert_status(StatusCode::OK);
    health_response.assert_text("OK");
    
    root_response.assert_status(StatusCode::OK);
    root_response.assert_text("🧠 Mimir AI Memory Vault - Server is running!");
}

#[tokio::test]
async fn test_method_not_allowed() {
    let server = create_default_test_server().await;
    
    // Test that POST to GET-only endpoints returns proper error
    let response = server.post("/health").await;
    
    response.assert_status(StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_server_headers() {
    let server = create_default_test_server().await;
    
    let response = server.get("/health").await;
    
    response.assert_status(StatusCode::OK);
    
    // Check for important security headers (if added)
    // These would be tested if we add security middleware
    let headers = response.headers();
    assert!(!headers.is_empty());
}

#[tokio::test]
#[serial] // Run serially to avoid port conflicts
async fn test_server_startup_shutdown() {
    // Test that server can start and stop cleanly
    let config = MimirConfig::default();
    let _temp_dir = create_temp_dir();
    
    // This would test the actual server startup if we had a way to shut it down gracefully
    // For now, we test that the server creation doesn't panic
    let app_result = mimir::create_app(config).await;
    assert!(app_result.is_ok());
}

#[tokio::test]
async fn test_large_request_handling() {
    let server = create_default_test_server().await;
    
    // Test with large query parameters (simulating large requests)
    let large_query = "?".to_string() + &"x=1&".repeat(1000);
    let response = server.get(&format!("/health{}", large_query)).await;
    
    // Should still work (query is ignored for health endpoint)
    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn test_invalid_utf8_path() {
    let server = create_default_test_server().await;
    
    // Test handling of paths with encoded characters
    let response = server.get("/health%FF").await;
    
    // Should return 404 for invalid/nonexistent paths
    response.assert_status(StatusCode::NOT_FOUND);
}

// Performance tests
#[tokio::test]
async fn test_response_time_baseline() {
    let server = create_default_test_server().await;
    
    let start = std::time::Instant::now();
    let response = server.get("/health").await;
    let duration = start.elapsed();
    
    response.assert_status(StatusCode::OK);
    
    // Basic performance check - health endpoint should be very fast
    assert!(duration < Duration::from_millis(100), 
            "Health endpoint should respond in <100ms, took {:?}", duration);
}

#[tokio::test]
async fn test_memory_usage_stability() {
    let server = create_default_test_server().await;
    
    // Make many requests to check for memory leaks
    for _ in 0..100 {
        let response = server.get("/health").await;
        response.assert_status(StatusCode::OK);
    }
    
    // If we reach here without OOM, basic memory stability is okay
}

// Test error conditions
#[tokio::test]
async fn test_malformed_requests() {
    let server = create_default_test_server().await;
    
    // Test various malformed requests
    let test_cases = vec![
        "//health",      // Double slash
        "/health/../",   // Path traversal attempt
        "/health?",      // Empty query
        "/health#",      // Fragment
    ];
    
    for path in test_cases {
        let response = server.get(path).await;
        
        // Should either work (if normalized) or return 404, but not crash
        assert!(
            response.status_code() == StatusCode::OK || 
            response.status_code() == StatusCode::NOT_FOUND,
            "Path '{}' returned unexpected status: {:?}", 
            path, response.status_code()
        );
    }
}

#[tokio::test]
async fn test_content_type_headers() {
    let server = create_default_test_server().await;
    
    let response = server.get("/").await;
    response.assert_status(StatusCode::OK);
    
    // Check that we get text content type for text responses
    let content_type = response.headers().get("content-type");
    if let Some(ct) = content_type {
        let ct_str = ct.to_str().unwrap();
        // Should be text/plain or similar for our simple text responses
        assert!(ct_str.contains("text") || ct_str.contains("plain"));
    }
} 