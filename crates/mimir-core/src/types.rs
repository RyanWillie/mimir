use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Unique identifier for a memory
pub type MemoryId = Uuid;

/// Unique identifier for an application
pub type AppId = String;

/// Memory classification types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryClass {
    Personal,
    Work,
    Health,
    Financial,
    Other(String),
}

/// A memory entry in the vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: MemoryId,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub class: MemoryClass,
    pub scope: Option<String>,
    pub tags: Vec<String>,
    pub app_acl: Vec<AppId>,
    pub key_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Memory ingestion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryIngestion {
    pub content: String,
    pub class: Option<MemoryClass>,
    pub scope: Option<String>,
    pub tags: Vec<String>,
    pub app_id: AppId,
}

/// Memory retrieval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub query: String,
    pub class_filter: Option<Vec<MemoryClass>>,
    pub scope_filter: Option<String>,
    pub app_id: AppId,
    pub top_k: usize,
}

/// Memory search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResult {
    pub memory: Memory,
    pub score: f32,
}

/// Application authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub app_id: AppId,
    pub permissions: Vec<MemoryClass>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
} 