//! Mimir Core - Common types and utilities for the AI Memory Vault
//!
//! This crate provides shared types, error handling, and utilities used across
//! all Mimir components.

pub mod types;
pub mod error;
pub mod config;

pub use error::{MimirError, Result};
pub use types::*; 