use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Unique identifier for a memory
pub type MemoryId = Uuid;

/// Unique identifier for an application
pub type AppId = String;

/// Memory classification types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{MemoryBuilder, MemoryIngestionBuilder, MemoryQueryBuilder};
    use crate::test_utils::assertions::*;
    use crate::test_utils::generators::*;
    use proptest::prelude::*;

    #[test]
    fn test_memory_class_serialization() {
        // Test standard classes
        let personal = MemoryClass::Personal;
        let serialized = serde_json::to_string(&personal).unwrap();
        assert_eq!(serialized, "\"personal\"");
        
        let work = MemoryClass::Work;
        let serialized = serde_json::to_string(&work).unwrap();
        assert_eq!(serialized, "\"work\"");
        
        // Test custom class
        let custom = MemoryClass::Other("custom".to_string());
        let serialized = serde_json::to_string(&custom).unwrap();
        assert!(serialized.contains("custom"));
    }

    #[test]
    fn test_memory_class_deserialization() {
        let personal: MemoryClass = serde_json::from_str("\"personal\"").unwrap();
        assert_eq!(personal, MemoryClass::Personal);
        
        let work: MemoryClass = serde_json::from_str("\"work\"").unwrap();
        assert_eq!(work, MemoryClass::Work);
        
        let health: MemoryClass = serde_json::from_str("\"health\"").unwrap();
        assert_eq!(health, MemoryClass::Health);
        
        let financial: MemoryClass = serde_json::from_str("\"financial\"").unwrap();
        assert_eq!(financial, MemoryClass::Financial);
    }

    #[test]
    fn test_memory_builder() {
        let memory = MemoryBuilder::new()
            .with_content("Test content")
            .with_class(MemoryClass::Work)
            .with_tags(vec!["tag1".to_string(), "tag2".to_string()])
            .build();

        assert_eq!(memory.content, "Test content");
        assert_eq!(memory.class, MemoryClass::Work);
        assert_eq!(memory.tags, vec!["tag1", "tag2"]);
        assert_memory_valid(&memory);
    }

    #[test]
    fn test_memory_ingestion_builder() {
        let ingestion = MemoryIngestionBuilder::new()
            .with_content("Ingestion test")
            .with_class(MemoryClass::Health)
            .with_app_id("health-app")
            .build();

        assert_eq!(ingestion.content, "Ingestion test");
        assert_eq!(ingestion.class, Some(MemoryClass::Health));
        assert_eq!(ingestion.app_id, "health-app");
    }

    #[test]
    fn test_memory_query_builder() {
        let query = MemoryQueryBuilder::new()
            .with_query("search term")
            .with_class_filter(vec![MemoryClass::Personal, MemoryClass::Work])
            .with_top_k(5)
            .build();

        assert_eq!(query.query, "search term");
        assert_eq!(query.class_filter, Some(vec![MemoryClass::Personal, MemoryClass::Work]));
        assert_eq!(query.top_k, 5);
    }

    #[test]
    fn test_memory_serialization_roundtrip() {
        let original = MemoryBuilder::new()
            .with_content("Test serialization")
            .with_class(MemoryClass::Personal)
            .with_tags(vec!["test".to_string()])
            .build();

        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: Memory = serde_json::from_str(&serialized).unwrap();

        assert_memories_equivalent(&original, &deserialized);
    }

    #[test]
    fn test_auth_token_validation() {
        let now = Utc::now();
        let token = AuthToken {
            app_id: "test-app".to_string(),
            permissions: vec![MemoryClass::Personal, MemoryClass::Work],
            issued_at: now,
            expires_at: Some(now + chrono::Duration::hours(1)),
        };

        assert_eq!(token.app_id, "test-app");
        assert_eq!(token.permissions.len(), 2);
        assert!(token.expires_at.unwrap() > token.issued_at);
    }

    #[test]
    fn test_generate_test_memories() {
        let memories = generate_test_memories(5);
        assert_eq!(memories.len(), 5);
        
        for memory in &memories {
            assert_memory_valid(memory);
        }
        
        // Check that we have variety in classes
        let classes: std::collections::HashSet<_> = memories.iter()
            .map(|m| &m.class)
            .collect();
        assert!(classes.len() > 1);
    }

    #[test]
    fn test_generate_test_embedding() {
        let embedding = generate_test_embedding(128);
        assert_eq!(embedding.len(), 128);
        
        // Check that values are normalized between 0 and 1
        for &value in &embedding {
            assert!(value >= 0.0 && value <= 1.0);
        }
    }

    // Property-based tests using proptest
    proptest! {
        #[test]
        fn test_memory_content_never_empty(content in ".+") {
            let memory = MemoryBuilder::new()
                .with_content(content.clone())
                .build();
            
            assert_eq!(memory.content, content);
            assert!(!memory.content.is_empty());
        }

        #[test]
        fn test_memory_id_uniqueness(content1 in ".+", content2 in ".+") {
            let memory1 = MemoryBuilder::new().with_content(content1).build();
            let memory2 = MemoryBuilder::new().with_content(content2).build();
            
            // UUIDs should be unique (extremely high probability)
            assert_ne!(memory1.id, memory2.id);
        }

        #[test]
        fn test_top_k_positive(k in 1usize..1000) {
            let query = MemoryQueryBuilder::new()
                .with_top_k(k)
                .build();
            
            assert_eq!(query.top_k, k);
            assert!(query.top_k > 0);
        }

        #[test]
        fn test_embedding_dimensions(dims in 1usize..2048) {
            let embedding = generate_test_embedding(dims);
            assert_eq!(embedding.len(), dims);
        }
    }
} 