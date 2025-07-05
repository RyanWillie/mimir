//! Mimir Core - Common types and utilities for the AI Memory Vault
//!
//! This crate provides shared types, error handling, and utilities used across
//! all Mimir components.

pub mod config;
pub mod crypto;
pub mod error;
pub mod types;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use error::{MimirError, Result};
pub use types::*;
pub use config::{get_default_app_dir, get_default_vault_path, get_default_keyset_path};
