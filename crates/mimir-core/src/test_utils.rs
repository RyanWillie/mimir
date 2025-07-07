//! Test utilities for Mimir components
//!
//! This module provides common testing utilities, fixtures, and helpers
//! that can be used across all Mimir crates for consistent testing.

use crate::{AppId, Memory, MemoryClass, MemoryId, MemoryIngestion, MemoryQuery};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Test fixture builder for creating test memories
pub struct MemoryBuilder {
    id: MemoryId,
    content: String,
    embedding: Option<Vec<f32>>,
    class: MemoryClass,
    scope: Option<String>,
    tags: Vec<String>,
    app_acl: Vec<AppId>,
    key_id: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Default for MemoryBuilder {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content: "Test memory content".to_string(),
            embedding: None,
            class: MemoryClass::Personal,
            scope: None,
            tags: vec![],
            app_acl: vec!["test-app".to_string()],
            key_id: "test-key".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl MemoryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_id(mut self, id: MemoryId) -> Self {
        self.id = id;
        self
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    pub fn with_class(mut self, class: MemoryClass) -> Self {
        self.class = class;
        self
    }

    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_app_acl(mut self, app_acl: Vec<AppId>) -> Self {
        self.app_acl = app_acl;
        self
    }

    pub fn build(self) -> Memory {
        Memory {
            id: self.id,
            content: self.content,
            embedding: self.embedding,
            class: self.class,
            scope: self.scope,
            tags: self.tags,
            app_acl: self.app_acl,
            key_id: self.key_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Test fixture for creating memory ingestion requests
pub struct MemoryIngestionBuilder {
    content: String,
    class: Option<MemoryClass>,
    scope: Option<String>,
    tags: Vec<String>,
    app_id: AppId,
}

impl Default for MemoryIngestionBuilder {
    fn default() -> Self {
        Self {
            content: "Test ingestion content".to_string(),
            class: Some(MemoryClass::Personal),
            scope: None,
            tags: vec![],
            app_id: "test-app".to_string(),
        }
    }
}

impl MemoryIngestionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    pub fn with_class(mut self, class: MemoryClass) -> Self {
        self.class = Some(class);
        self
    }

    pub fn with_app_id(mut self, app_id: impl Into<AppId>) -> Self {
        self.app_id = app_id.into();
        self
    }

    pub fn build(self) -> MemoryIngestion {
        MemoryIngestion {
            content: self.content,
            class: self.class,
            scope: self.scope,
            tags: self.tags,
            app_id: self.app_id,
        }
    }
}

/// Test fixture for creating memory queries
pub struct MemoryQueryBuilder {
    query: String,
    class_filter: Option<Vec<MemoryClass>>,
    scope_filter: Option<String>,
    app_id: AppId,
    top_k: usize,
}

impl Default for MemoryQueryBuilder {
    fn default() -> Self {
        Self {
            query: "test query".to_string(),
            class_filter: None,
            scope_filter: None,
            app_id: "test-app".to_string(),
            top_k: 10,
        }
    }
}

impl MemoryQueryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    pub fn with_class_filter(mut self, classes: Vec<MemoryClass>) -> Self {
        self.class_filter = Some(classes);
        self
    }

    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }

    pub fn build(self) -> MemoryQuery {
        MemoryQuery {
            query: self.query,
            class_filter: self.class_filter,
            scope_filter: self.scope_filter,
            app_id: self.app_id,
            top_k: self.top_k,
        }
    }
}

/// Common test assertions and utilities
pub mod assertions {
    use super::*;

    /// Assert that a memory has the expected basic properties
    pub fn assert_memory_valid(memory: &Memory) {
        assert!(
            !memory.content.is_empty(),
            "Memory content should not be empty"
        );
        assert!(
            !memory.key_id.is_empty(),
            "Memory key_id should not be empty"
        );
        assert!(
            !memory.app_acl.is_empty(),
            "Memory should have at least one app in ACL"
        );
        assert!(
            memory.created_at <= memory.updated_at,
            "Created time should be <= updated time"
        );
    }

    /// Assert that two memories are equivalent for testing purposes
    pub fn assert_memories_equivalent(expected: &Memory, actual: &Memory) {
        assert_eq!(expected.id, actual.id);
        assert_eq!(expected.content, actual.content);
        assert_eq!(expected.class, actual.class);
        assert_eq!(expected.scope, actual.scope);
        assert_eq!(expected.tags, actual.tags);
        assert_eq!(expected.app_acl, actual.app_acl);
    }
}

/// Mock data generators for testing
pub mod generators {
    use super::*;

    /// Generate a vector of test memories with different characteristics
    pub fn generate_test_memories(count: usize) -> Vec<Memory> {
        (0..count)
            .map(|i| {
                MemoryBuilder::new()
                    .with_content(format!("Test memory content {}", i))
                    .with_class(match i % 4 {
                        0 => MemoryClass::Personal,
                        1 => MemoryClass::Work,
                        2 => MemoryClass::Health,
                        _ => MemoryClass::Financial,
                    })
                    .with_tags(vec![format!("tag_{}", i), "test".to_string()])
                    .build()
            })
            .collect()
    }

    /// Generate test embeddings with specified dimensions
    pub fn generate_test_embedding(dimensions: usize) -> Vec<f32> {
        (0..dimensions)
            .map(|i| (i as f32) / (dimensions as f32))
            .collect()
    }

    /// Generate test app IDs
    pub fn generate_app_ids(count: usize) -> Vec<AppId> {
        (0..count).map(|i| format!("test-app-{}", i)).collect()
    }
}

// Only include env module when test-utils feature is explicitly enabled
#[cfg(feature = "test-utils")]
pub mod env {
    use std::path::PathBuf;
    use tempfile;
    use crate::crypto::CryptoManager;
    
    /// Create a temporary directory for test data
    pub fn create_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("Failed to create temp directory")
    }

    /// Get a test database path in a temporary directory
    pub fn get_test_db_path(temp_dir: &tempfile::TempDir) -> PathBuf {
        temp_dir.path().join("test.db")
    }

    /// Get a test vault path in a temporary directory
    pub fn get_test_vault_path(temp_dir: &tempfile::TempDir) -> PathBuf {
        temp_dir.path().join("test_vault.db")
    }

    /// Create a test crypto manager with OS keychain
    pub fn create_test_crypto_manager() -> (CryptoManager, tempfile::TempDir) {
        let temp_dir = create_temp_dir();
        let keyset_path = temp_dir.path().join("keyset.json");
        let crypto_manager = CryptoManager::new(&keyset_path).expect("Failed to create crypto manager");
        (crypto_manager, temp_dir)
    }

    /// Create a test crypto manager with password-based encryption
    pub fn create_test_crypto_manager_with_password(password: &str) -> (CryptoManager, tempfile::TempDir) {
        let temp_dir = create_temp_dir();
        let keyset_path = temp_dir.path().join("keyset.json");
        let crypto_manager = CryptoManager::with_password(&keyset_path, password)
            .expect("Failed to create password-based crypto manager");
        (crypto_manager, temp_dir)
    }
}
