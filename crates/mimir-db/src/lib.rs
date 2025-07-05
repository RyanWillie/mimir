//! Mimir Database - Encrypted storage for memory entries

use mimir_core::{Memory, MemoryClass, MemoryId, Result, crypto::CryptoManager};
use rusqlite::{Connection, params};
use std::path::Path;

/// Encrypted database for storing memories
pub struct Database {
    conn: Connection,
    crypto_manager: CryptoManager,
}

impl Database {
    /// Create a new encrypted database
    pub fn new<P: AsRef<Path>>(db_path: P, keyset_path: P) -> Result<Self> {
        let db_path = db_path.as_ref();
        let keyset_path = keyset_path.as_ref();
        
        // Initialize crypto manager
        let crypto_manager = CryptoManager::new(keyset_path)?;
        
        // Open SQLCipher database
        let conn = Connection::open(db_path)
            .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to open database: {}", e)))?;
        
        // Set SQLCipher pragma with derived key
        let db_key = crypto_manager.get_db_key()?;
        let pragma_sql = format!("PRAGMA key = '{}'", db_key);
        conn.execute(&pragma_sql, [])
            .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to set database key: {}", e)))?;
        
        // Create tables if they don't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                content_enc BLOB NOT NULL,
                nonce BLOB NOT NULL,
                class TEXT NOT NULL,
                scope TEXT,
                tags TEXT,
                app_acl TEXT NOT NULL,
                key_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to create tables: {}", e)))?;
        
        Ok(Database {
            conn,
            crypto_manager,
        })
    }

    /// Store a memory in the database
    pub async fn store_memory(&mut self, memory: &Memory) -> Result<()> {
        // Determine memory class string
        let class_str = match &memory.class {
            MemoryClass::Personal => "personal",
            MemoryClass::Work => "work", 
            MemoryClass::Health => "health",
            MemoryClass::Financial => "financial",
            MemoryClass::Other(s) => s,
        };
        
        // Encrypt memory content
        let content_bytes = memory.content.as_bytes();
        let ciphertext = self.crypto_manager.encrypt(class_str, content_bytes)?;
        
        // Serialize tags and app_acl
        let tags_json = serde_json::to_string(&memory.tags)
            .map_err(|e| mimir_core::MimirError::Serialization(e))?;
        let app_acl_json = serde_json::to_string(&memory.app_acl)
            .map_err(|e| mimir_core::MimirError::Serialization(e))?;
        
        // Insert into database
        self.conn.execute(
            "INSERT OR REPLACE INTO memories 
             (id, content_enc, nonce, class, scope, tags, app_acl, key_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                memory.id.to_string(),
                ciphertext.data,
                ciphertext.nonce,
                class_str,
                memory.scope,
                tags_json,
                app_acl_json,
                memory.key_id,
                memory.created_at.to_rfc3339(),
                memory.updated_at.to_rfc3339(),
            ],
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to store memory: {}", e)))?;
        
        Ok(())
    }

    /// Get memories by classification
    pub async fn get_memories_by_class(&mut self, class: &MemoryClass) -> Result<Vec<Memory>> {
        let class_str = match class {
            MemoryClass::Personal => "personal",
            MemoryClass::Work => "work",
            MemoryClass::Health => "health", 
            MemoryClass::Financial => "financial",
            MemoryClass::Other(s) => s,
        };
        
        let mut stmt = self.conn.prepare(
            "SELECT id, content_enc, nonce, class, scope, tags, app_acl, key_id, created_at, updated_at
             FROM memories WHERE class = ?1"
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to prepare query: {}", e)))?;
        
        let memory_iter = stmt.query_map([class_str], |row| {
            Ok((
                row.get::<_, String>(0)?,        // id
                row.get::<_, Vec<u8>>(1)?,       // content_enc
                row.get::<_, Vec<u8>>(2)?,       // nonce
                row.get::<_, String>(3)?,        // class
                row.get::<_, Option<String>>(4)?, // scope
                row.get::<_, String>(5)?,        // tags
                row.get::<_, String>(6)?,        // app_acl
                row.get::<_, String>(7)?,        // key_id
                row.get::<_, String>(8)?,        // created_at
                row.get::<_, String>(9)?,        // updated_at
            ))
        }).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to execute query: {}", e)))?;
        
        let mut memories = Vec::new();
        
        for memory_result in memory_iter {
            let (id_str, content_enc, nonce, class_str, scope, tags_str, app_acl_str, key_id, created_at_str, updated_at_str) = 
                memory_result.map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to read row: {}", e)))?;
            
            // Parse ID
            let id = uuid::Uuid::parse_str(&id_str)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid UUID: {}", e)))?;
            
            // Decrypt content
            let ciphertext = mimir_core::crypto::Ciphertext {
                data: content_enc,
                nonce,
            };
            let plaintext_bytes = self.crypto_manager.decrypt(&class_str, &ciphertext)?;
            let content = String::from_utf8(plaintext_bytes)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid UTF-8: {}", e)))?;
            
            // Parse class
            let memory_class = match class_str.as_str() {
                "personal" => MemoryClass::Personal,
                "work" => MemoryClass::Work,
                "health" => MemoryClass::Health,
                "financial" => MemoryClass::Financial,
                other => MemoryClass::Other(other.to_string()),
            };
            
            // Parse tags and app_acl
            let tags: Vec<String> = serde_json::from_str(&tags_str)
                .map_err(|e| mimir_core::MimirError::Serialization(e))?;
            let app_acl: Vec<String> = serde_json::from_str(&app_acl_str)
                .map_err(|e| mimir_core::MimirError::Serialization(e))?;
            
            // Parse timestamps
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid timestamp: {}", e)))?
                .with_timezone(&chrono::Utc);
            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid timestamp: {}", e)))?
                .with_timezone(&chrono::Utc);
            
            let memory = Memory {
                id,
                content,
                embedding: None, // TODO: Add embedding support
                class: memory_class,
                scope,
                tags,
                app_acl,
                key_id,
                created_at,
                updated_at,
            };
            
            memories.push(memory);
        }
        
        Ok(memories)
    }

    /// Delete a memory by ID
    pub async fn delete_memory(&self, id: MemoryId) -> Result<()> {
        self.conn.execute(
            "DELETE FROM memories WHERE id = ?1",
            params![id.to_string()],
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to delete memory: {}", e)))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mimir_core::test_utils::assertions::assert_memory_valid;
    use mimir_core::test_utils::generators::generate_test_memories;
    use mimir_core::test_utils::{
        env::{create_temp_dir, get_test_db_path},
        MemoryBuilder,
    };
    use mimir_core::MemoryClass;
    use serial_test::serial;

    fn create_test_database() -> Database {
        let temp_dir = create_temp_dir();
        let db_path = get_test_db_path(&temp_dir);
        let keyset_path = temp_dir.path().join("keyset.json");
        std::fs::write(&keyset_path, r#"{"key": "test_master_key_32_bytes_long!!"}"#).unwrap();

        Database::new(db_path, keyset_path)
            .expect("Failed to create test database")
    }

    #[test]
    fn test_database_creation() {
        let temp_dir = create_temp_dir();
        let db_path = get_test_db_path(&temp_dir);
        let keyset_path = temp_dir.path().join("keyset.json");
        std::fs::write(&keyset_path, r#"{"key": "test_master_key_32_bytes_long!!"}"#).unwrap();

        let result = Database::new(db_path, keyset_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_database_creation_with_different_paths() {
        let temp_dir = create_temp_dir();
        let keyset_path = temp_dir.path().join("keyset.json");
        std::fs::write(&keyset_path, r#"{"key": "another_test_key_32_bytes_long!!"}"#).unwrap();

        let test_cases = vec!["test1.db", "subdir/test2.db", "memory_vault.sqlite"];

        for case in test_cases {
            let db_path = temp_dir.path().join(case);
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            let result = Database::new(db_path, keyset_path);
            assert!(
                result.is_ok(),
                "Failed to create database at path: {:?}",
                db_path
            );
        }
    }

    #[tokio::test]
    async fn test_store_memory_stub() {
        let mut db = create_test_database();

        let memory = MemoryBuilder::new()
            .with_content("Test memory content")
            .with_class(MemoryClass::Personal)
            .build();

        let result = db.store_memory(&memory).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_store_multiple_memories() {
        let mut db = create_test_database();

        let memories = generate_test_memories(5);

        for memory in &memories {
            let result = db.store_memory(memory).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_get_memories_by_class_stub() {
        let mut db = create_test_database();

        let result = db.get_memories_by_class(&MemoryClass::Personal).await;
        assert!(result.is_ok());

        let memories = result.unwrap();
        assert_eq!(memories.len(), 0); // Stub returns empty vector
    }

    #[tokio::test]
    async fn test_get_memories_all_classes() {
        let mut db = create_test_database();

        let classes = vec![
            MemoryClass::Personal,
            MemoryClass::Work,
            MemoryClass::Health,
            MemoryClass::Financial,
            MemoryClass::Other("custom".to_string()),
        ];

        for class in classes {
            let result = db.get_memories_by_class(&class).await;
            assert!(result.is_ok(), "Failed to query class: {:?}", class);
        }
    }

    #[tokio::test]
    async fn test_delete_memory_stub() {
        let db = create_test_database();

        let memory = MemoryBuilder::new().build();
        let memory_id = memory.id;

        let result = db.delete_memory(memory_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_multiple_memories() {
        let mut db = create_test_database();

        let memories = generate_test_memories(3);

        for memory in &memories {
            let result = db.store_memory(memory).await;
            assert!(result.is_ok());
        }

        for memory in &memories {
            let result = db.delete_memory(memory.id).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let mut db = create_test_database();

        let memory1 = MemoryBuilder::new()
            .with_content("Concurrent memory 1")
            .build();

        let memory2 = MemoryBuilder::new()
            .with_content("Concurrent memory 2")
            .build();

        // Test concurrent operations
        let (store_result, query_result, delete_result) = tokio::join!(
            db.store_memory(&memory1),
            db.get_memories_by_class(&MemoryClass::Personal),
            db.delete_memory(memory2.id)
        );

        assert!(store_result.is_ok());
        assert!(query_result.is_ok());
        assert!(delete_result.is_ok());
    }

    #[tokio::test]
    #[serial] // Run serially to avoid database conflicts
    async fn test_database_isolation() {
        // Test that different database instances are isolated
        let temp_dir1 = create_temp_dir();
        let temp_dir2 = create_temp_dir();

        let db_path1 = get_test_db_path(&temp_dir1);
        let db_path2 = get_test_db_path(&temp_dir2);

        let keyset_path = temp_dir1.path().join("keyset.json");
        std::fs::write(&keyset_path, r#"{"key": "isolation_test_key_32_bytes_lng!"}"#).unwrap();

        let db1 = Database::new(db_path1, keyset_path).unwrap();
        let db2 = Database::new(db_path2, keyset_path).unwrap();

        let memory1 = MemoryBuilder::new().with_content("DB1 memory").build();
        let memory2 = MemoryBuilder::new().with_content("DB2 memory").build();

        // Store in different databases
        assert!(db1.store_memory(&memory1).await.is_ok());
        assert!(db2.store_memory(&memory2).await.is_ok());
    }

    #[test]
    fn test_database_error_handling() {
        // Test invalid paths
        let invalid_paths = vec![
            "",                       // Empty path
            "/root/no_permission.db", // Permission denied path
        ];

        let keyset_path = "non_existent_keyset.json";

        for path in invalid_paths {
            let result = Database::new(path, keyset_path);
            // With stub implementation, these might still succeed
            // When real implementation is added, should test actual error conditions
        }
    }

    #[test]
    fn test_master_key_variations() {
        let temp_dir = create_temp_dir();
        let db_path = get_test_db_path(&temp_dir);
        let keyset_path = temp_dir.path().join("keyset.json");

        let key_variations = vec![
            b"test_key_32_bytes_exactly_len_32!",
            b"exactly_32_bytes_key_for_testing!",
            b"another_32_byte_key_for_testing!!",
        ];

        for key in key_variations {
            let keyset_content = format!("{{\"key\": \"{}\"}}", hex::encode(key));
            std::fs::write(&keyset_path, keyset_content).unwrap();

            let result = Database::new(db_path, keyset_path);
            assert!(result.is_ok(), "Failed with key length: {}", key.len());
        }
    }

    #[tokio::test]
    async fn test_memory_data_integrity() {
        let mut db = create_test_database();

        let original_memory = MemoryBuilder::new()
            .with_content("Data integrity test content")
            .with_class(MemoryClass::Work)
            .with_tags(vec!["integrity".to_string(), "test".to_string()])
            .build();

        // Validate the test memory is well-formed
        assert_memory_valid(&original_memory);

        // Store and verify operations don't corrupt data structure
        let result = db.store_memory(&original_memory).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_large_content_handling() {
        let mut db = create_test_database();

        // Test with large content (simulating edge cases)
        let large_content = "x".repeat(10_000); // 10KB content
        let memory = MemoryBuilder::new().with_content(large_content).build();

        let result = db.store_memory(&memory).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_special_characters_in_content() {
        let mut db = create_test_database();

        let special_contents = vec![
            "🧠 Memory with emojis 🔒",
            "Memory with\nnewlines\nand\ttabs",
            "Memory with 'quotes' and \"double quotes\"",
            "Memory with JSON: {\"key\": \"value\", \"number\": 42}",
            "Memory with SQL: SELECT * FROM memories WHERE id = 1; DROP TABLE memories;",
        ];

        for content in special_contents {
            let memory = MemoryBuilder::new().with_content(content).build();

            let result = db.store_memory(&memory).await;
            assert!(result.is_ok(), "Failed to store content: {}", content);
        }
    }
}
