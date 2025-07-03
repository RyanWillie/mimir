//! Mimir - Local-First AI Memory Vault
//!
//! This crate provides the main daemon process that serves the Mimir API
//! and manages the AI memory vault functionality.

pub mod mcp;
pub mod server;

// Re-export commonly used functions for external use (e.g., testing)
pub use mimir_core::{config::MimirConfig, MimirError, Result};
pub use server::{create_app, start};
