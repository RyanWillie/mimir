//! Mimir - Local-First AI Memory Vault
//!
//! This crate provides the main daemon process that serves the Mimir API
//! and manages the AI memory vault functionality.

pub mod mcp;
pub mod storage;

// Re-export commonly used functions for external use (e.g., testing)
pub use mimir_core::{Config, MimirError, Result};
pub use storage::{IntegratedStorage, MemoryAddResult, MemorySearchResult, StorageStats};
