//! Mimir - Local-First AI Memory Vault
//!
//! This crate provides the main daemon process that serves the Mimir API
//! and manages the AI memory vault functionality.

pub mod server;
pub mod mcp;

// Re-export commonly used functions for external use (e.g., testing)
pub use server::{create_app, start};
pub use mimir_core::{Result, MimirError, config::MimirConfig}; 