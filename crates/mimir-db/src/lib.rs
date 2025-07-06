//! Mimir Database - Encrypted storage for memory entries

use mimir_core::{Memory, MemoryClass, MemoryId, Result, crypto::CryptoManager};
use rusqlite::{Connection, params, Transaction};
use std::path::Path;

/// Encrypted database for storing memories
pub struct Database {
    conn: Connection,
    crypto_manager: CryptoManager,
}

impl Database {
    /// Create a new encrypted database with an existing crypto manager
    pub fn with_crypto_manager<P: AsRef<Path>>(db_path: P, crypto_manager: CryptoManager) -> Result<Self> {
        let db_path = db_path.as_ref();
        
        // Validate path is not empty
        if db_path.to_string_lossy().is_empty() {
            return Err(mimir_core::MimirError::Database(anyhow::anyhow!("Database path cannot be empty")));
        }
        
        // Ensure the database directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to create database directory: {}", e)))?;
        }
        
        // Create an encrypted database with SQLCipher
        let conn = Connection::open_with_flags(
            db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to open database: {}", e)))?;
        
        // Set the SQLCipher key using PRAGMA - SQLCipher returns results from PRAGMA commands
        let db_key_bytes = crypto_manager.get_db_key_bytes();
        let db_key_hex = hex::encode(db_key_bytes);
        let pragma_sql = format!("PRAGMA key = \"x'{}'\"", db_key_hex);
        conn.execute_batch(&pragma_sql)
            .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to set SQLCipher key: {}", e)))?;
        
        // Test that the key is correct by executing a simple query
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
            let count: i64 = row.get(0)?;
            Ok(count)
        }).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to verify SQLCipher key: {}", e)))?;
        
        // Test write access by creating a simple table
        conn.execute("CREATE TABLE IF NOT EXISTS test_write (id INTEGER PRIMARY KEY)", [])
            .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Database is read-only: {}", e)))?;
        
        // Test inserting into the test table
        conn.execute("INSERT OR REPLACE INTO test_write (id) VALUES (1)", [])
            .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Cannot write to database: {}", e)))?;
        
        // Test inserting into memory table immediately after creation
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory (
                id        TEXT PRIMARY KEY,
                user_id   TEXT NOT NULL,
                class_id  TEXT NOT NULL,
                text_enc  BLOB NOT NULL,
                vec_id    INTEGER NOT NULL,
                ts        INTEGER NOT NULL
            )",
            [],
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to create memory table: {}", e)))?;
        
        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memory_class_id ON memory(class_id)",
            [],
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to create class_id index: {}", e)))?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memory_user_ts ON memory(user_id, ts DESC)",
            [],
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to create user_ts index: {}", e)))?;
        
        Ok(Database {
            conn,
            crypto_manager,
        })
    }

    /// Create a new encrypted database (backward compatibility - uses keychain-based crypto)
    pub fn new<P: AsRef<Path>>(db_path: P, keyset_path: P) -> Result<Self> {
        let db_path = db_path.as_ref();
        let keyset_path = keyset_path.as_ref();
        
        // Validate paths are not empty
        if db_path.to_string_lossy().is_empty() {
            return Err(mimir_core::MimirError::Database(anyhow::anyhow!("Database path cannot be empty")));
        }
        if keyset_path.to_string_lossy().is_empty() {
            return Err(mimir_core::MimirError::Database(anyhow::anyhow!("Keyset path cannot be empty")));
        }
        
        // Validate keyset file exists
        if !keyset_path.exists() {
            return Err(mimir_core::MimirError::Database(anyhow::anyhow!("Keyset file does not exist: {:?}", keyset_path)));
        }
        
        // Initialize crypto manager (keychain-based)
        let crypto_manager = CryptoManager::new(keyset_path)?;
        
        Self::with_crypto_manager(db_path, crypto_manager)
    }

    /// Store a memory in the database
    pub async fn store_memory(&mut self, memory: &Memory) -> Result<()> {
        // Determine class_id string
        let class_id = match &memory.class {
            MemoryClass::Personal => "personal",
            MemoryClass::Work => "work",
            MemoryClass::Health => "health",
            MemoryClass::Financial => "financial",
            MemoryClass::Other(s) => s,
        };
        
        // Encrypt memory content with class-specific key
        let content_bytes = memory.content.as_bytes();
        let ciphertext = self.crypto_manager.encrypt(class_id, content_bytes)?;
        
        // Serialize the ciphertext (including nonce) for storage
        let ciphertext_data = serde_json::to_vec(&ciphertext)
            .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to serialize ciphertext: {}", e)))?;
        
        // Use a default user_id for now (can be made configurable later)
        let user_id = "default_user";
        
        // Use vec_id as 0 for now (can be updated when vector storage is implemented)
        let vec_id = 0;
        
        // Convert timestamp to Unix timestamp (seconds since epoch)
        let ts = memory.created_at.timestamp();
        
        // Insert into database
        let result = self.conn.execute(
            "INSERT OR REPLACE INTO memory (id, user_id, class_id, text_enc, vec_id, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                memory.id.to_string(),
                user_id,
                class_id,
                ciphertext_data,
                vec_id,
                ts,
            ],
        );
        
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                // Try to get more information about the error
                let error_msg = format!("Failed to store memory: {} (Error code: {:?})", e, e);
                Err(mimir_core::MimirError::Database(anyhow::anyhow!(error_msg)))
            }
        }
    }

    /// Get memories by classification
    pub async fn get_memories_by_class(&mut self, class: &MemoryClass) -> Result<Vec<Memory>> {
        let class_id = match class {
            MemoryClass::Personal => "personal",
            MemoryClass::Work => "work",
            MemoryClass::Health => "health", 
            MemoryClass::Financial => "financial",
            MemoryClass::Other(s) => s,
        };
        
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, class_id, text_enc, vec_id, ts
             FROM memory WHERE class_id = ?1
             ORDER BY ts DESC"
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to prepare query: {}", e)))?;
        
        let memory_iter = stmt.query_map([class_id], |row| {
            Ok((
                row.get::<_, String>(0)?,        // id
                row.get::<_, String>(1)?,        // user_id
                row.get::<_, String>(2)?,        // class_id
                row.get::<_, Vec<u8>>(3)?,       // text_enc
                row.get::<_, i64>(4)?,           // vec_id
                row.get::<_, i64>(5)?,           // ts
            ))
        }).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to execute query: {}", e)))?;
        
        let mut memories = Vec::new();
        
        for memory_result in memory_iter {
            let (id_str, user_id, class_id, text_enc, _vec_id, ts) = 
                memory_result.map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to read row: {}", e)))?;
            
            // Parse ID
            let id = uuid::Uuid::parse_str(&id_str)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid UUID: {}", e)))?;
            
            // Deserialize and decrypt content
            let ciphertext: mimir_core::crypto::Ciphertext = serde_json::from_slice(&text_enc)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to deserialize ciphertext: {}", e)))?;
            let plaintext_bytes = self.crypto_manager.decrypt(&class_id, &ciphertext)?;
            let content = String::from_utf8(plaintext_bytes)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid UTF-8: {}", e)))?;
            
            // Parse class
            let memory_class = match class_id.as_str() {
                "personal" => MemoryClass::Personal,
                "work" => MemoryClass::Work,
                "health" => MemoryClass::Health,
                "financial" => MemoryClass::Financial,
                other => MemoryClass::Other(other.to_string()),
            };
            
            // Convert timestamp back to DateTime
            let created_at = chrono::DateTime::from_timestamp(ts, 0)
                .ok_or_else(|| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid timestamp: {}", ts)))?
                .with_timezone(&chrono::Utc);
            
            // For now, use created_at as updated_at since we don't store it separately
            let updated_at = created_at;
            
            let memory = Memory {
                id,
                content,
                embedding: None, // TODO: Add embedding support when vec_id is implemented
                class: memory_class,
                scope: None, // Not stored in new schema
                tags: vec![], // Not stored in new schema
                app_acl: vec![user_id], // Use user_id as app_acl for now
                key_id: class_id.to_string(), // Use class_id as key_id
                created_at,
                updated_at,
            };
            
            memories.push(memory);
        }
        
        Ok(memories)
    }

    /// Get the last N memories for a user
    pub async fn get_last_memories(&mut self, user_id: &str, limit: usize) -> Result<Vec<Memory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, class_id, text_enc, vec_id, ts
             FROM memory WHERE user_id = ?1
             ORDER BY ts DESC
             LIMIT ?2"
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to prepare query: {}", e)))?;
        
        let memory_iter = stmt.query_map(params![user_id, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,        // id
                row.get::<_, String>(1)?,        // user_id
                row.get::<_, String>(2)?,        // class_id
                row.get::<_, Vec<u8>>(3)?,       // text_enc
                row.get::<_, i64>(4)?,           // vec_id
                row.get::<_, i64>(5)?,           // ts
            ))
        }).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to execute query: {}", e)))?;
        
        let mut memories = Vec::new();
        
        for memory_result in memory_iter {
            let (id_str, user_id, class_id, text_enc, _vec_id, ts) = 
                memory_result.map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to read row: {}", e)))?;
            
            // Parse ID
            let id = uuid::Uuid::parse_str(&id_str)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid UUID: {}", e)))?;
            
            // Deserialize and decrypt content
            let ciphertext: mimir_core::crypto::Ciphertext = serde_json::from_slice(&text_enc)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to deserialize ciphertext: {}", e)))?;
            let plaintext_bytes = self.crypto_manager.decrypt(&class_id, &ciphertext)?;
            let content = String::from_utf8(plaintext_bytes)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid UTF-8: {}", e)))?;
            
            // Parse class
            let memory_class = match class_id.as_str() {
                "personal" => MemoryClass::Personal,
                "work" => MemoryClass::Work,
                "health" => MemoryClass::Health,
                "financial" => MemoryClass::Financial,
                other => MemoryClass::Other(other.to_string()),
            };
            
            // Convert timestamp back to DateTime
            let created_at = chrono::DateTime::from_timestamp(ts, 0)
                .ok_or_else(|| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid timestamp: {}", ts)))?
                .with_timezone(&chrono::Utc);
            
            // For now, use created_at as updated_at since we don't store it separately
            let updated_at = created_at;
            
            let memory = Memory {
                id,
                content,
                embedding: None, // TODO: Add embedding support when vec_id is implemented
                class: memory_class,
                scope: None, // Not stored in new schema
                tags: vec![], // Not stored in new schema
                app_acl: vec![user_id], // Use user_id as app_acl for now
                key_id: class_id.to_string(), // Use class_id as key_id
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
            "DELETE FROM memory WHERE id = ?1",
            params![id.to_string()],
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to delete memory: {}", e)))?;
        
        Ok(())
    }

    /// Get memory by ID
    pub async fn get_memory(&mut self, id: MemoryId) -> Result<Option<Memory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, class_id, text_enc, vec_id, ts
             FROM memory WHERE id = ?1"
        ).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to prepare query: {}", e)))?;
        
        let mut rows = stmt.query_map([id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,        // id
                row.get::<_, String>(1)?,        // user_id
                row.get::<_, String>(2)?,        // class_id
                row.get::<_, Vec<u8>>(3)?,       // text_enc
                row.get::<_, i64>(4)?,           // vec_id
                row.get::<_, i64>(5)?,           // ts
            ))
        }).map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to execute query: {}", e)))?;
        
        if let Some(memory_result) = rows.next() {
            let (id_str, user_id, class_id, text_enc, _vec_id, ts) = 
                memory_result.map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to read row: {}", e)))?;
            
            // Parse ID
            let id = uuid::Uuid::parse_str(&id_str)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid UUID: {}", e)))?;
            
            // Deserialize and decrypt content
            let ciphertext: mimir_core::crypto::Ciphertext = serde_json::from_slice(&text_enc)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to deserialize ciphertext: {}", e)))?;
            let plaintext_bytes = self.crypto_manager.decrypt(&class_id, &ciphertext)?;
            let content = String::from_utf8(plaintext_bytes)
                .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid UTF-8: {}", e)))?;
            
            // Parse class
            let memory_class = match class_id.as_str() {
                "personal" => MemoryClass::Personal,
                "work" => MemoryClass::Work,
                "health" => MemoryClass::Health,
                "financial" => MemoryClass::Financial,
                other => MemoryClass::Other(other.to_string()),
            };
            
            // Convert timestamp back to DateTime
            let created_at = chrono::DateTime::from_timestamp(ts, 0)
                .ok_or_else(|| mimir_core::MimirError::Database(anyhow::anyhow!("Invalid timestamp: {}", ts)))?
                .with_timezone(&chrono::Utc);
            
            // For now, use created_at as updated_at since we don't store it separately
            let updated_at = created_at;
            
            let memory = Memory {
                id,
                content,
                embedding: None, // TODO: Add embedding support when vec_id is implemented
                class: memory_class,
                scope: None, // Not stored in new schema
                tags: vec![], // Not stored in new schema
                app_acl: vec![user_id], // Use user_id as app_acl for now
                key_id: class_id.to_string(), // Use class_id as key_id
                created_at,
                updated_at,
            };
            
            Ok(Some(memory))
        } else {
            Ok(None)
        }
    }

    /// Begin a transaction
    pub fn transaction(&mut self) -> Result<Transaction<'_>> {
        self.conn.transaction()
            .map_err(|e| mimir_core::MimirError::Database(anyhow::anyhow!("Failed to begin transaction: {}", e)))
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

    fn create_test_database() -> (Database, tempfile::TempDir) {
        let temp_dir = create_temp_dir();
        let db_path = get_test_db_path(&temp_dir);
        let keyset_path = temp_dir.path().join("keyset.json");
        
        // Ensure the parent directory exists
        if let Some(parent) = keyset_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create keyset directory");
        }
        
        // Use password-based CryptoManager to create a proper keyset file
        let _crypto_manager = mimir_core::crypto::CryptoManager::with_password(&keyset_path, "test-password")
            .expect("Failed to create test crypto manager");

        let db = Database::new(db_path, keyset_path)
            .expect("Failed to create test database");
        (db, temp_dir)
    }

    #[test]
    fn test_database_creation() {
        let temp_dir = create_temp_dir();
        let db_path = get_test_db_path(&temp_dir);
        let keyset_path = temp_dir.path().join("keyset.json");
        
        // Ensure the parent directory exists
        if let Some(parent) = keyset_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create keyset directory");
        }
        
        // Use password-based CryptoManager to create a proper keyset file
        let _crypto_manager = mimir_core::crypto::CryptoManager::with_password(&keyset_path, "test-password")
            .expect("Failed to create test crypto manager");

        let result = Database::new(db_path, keyset_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_database_creation_with_different_paths() {
        let temp_dir = create_temp_dir();
        let keyset_path = temp_dir.path().join("keyset.json");
        
        // Ensure the parent directory exists
        if let Some(parent) = keyset_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create keyset directory");
        }
        
        // Use password-based CryptoManager to create a proper keyset file
        let _crypto_manager = mimir_core::crypto::CryptoManager::with_password(&keyset_path, "test-password")
            .expect("Failed to create test crypto manager");

        let test_cases = vec!["test1.db", "subdir/test2.db", "memory_vault.sqlite"];

        for case in test_cases {
            let db_path = temp_dir.path().join(case);
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            let result = Database::new(&db_path, &keyset_path);
            assert!(
                result.is_ok(),
                "Failed to create database at path: {:?}",
                db_path
            );
        }
    }

    #[tokio::test]
    async fn test_store_and_retrieve_memory() {
        let (mut db, _temp_dir) = create_test_database();

        let memory = MemoryBuilder::new()
            .with_content("Test memory content")
            .with_class(MemoryClass::Personal)
            .build();

        // Store memory
        let result = db.store_memory(&memory).await;
        assert!(result.is_ok());

        // Retrieve by class
        let retrieved = db.get_memories_by_class(&MemoryClass::Personal).await;
        assert!(retrieved.is_ok());

        let memories = retrieved.unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].content, "Test memory content");
        assert_eq!(memories[0].class, MemoryClass::Personal);
    }

    #[tokio::test]
    async fn test_store_multiple_memories() {
        let (mut db, _temp_dir) = create_test_database();

        let memories = generate_test_memories(5);

        for memory in &memories {
            let result = db.store_memory(memory).await;
            assert!(result.is_ok());
        }

        // Verify we can retrieve them
        for memory in &memories {
            let retrieved = db.get_memory(memory.id).await;
            assert!(retrieved.is_ok());
            let retrieved_memory = retrieved.unwrap();
            assert!(retrieved_memory.is_some());
            assert_eq!(retrieved_memory.unwrap().content, memory.content);
        }
    }

    #[tokio::test]
    async fn test_get_memories_by_class() {
        let (mut db, _temp_dir) = create_test_database();

        // Store memories in different classes
        let personal_memory = MemoryBuilder::new()
            .with_content("Personal content")
            .with_class(MemoryClass::Personal)
            .build();

        let work_memory = MemoryBuilder::new()
            .with_content("Work content")
            .with_class(MemoryClass::Work)
            .build();

        db.store_memory(&personal_memory).await.unwrap();
        db.store_memory(&work_memory).await.unwrap();

        // Test retrieving by class
        let personal_memories = db.get_memories_by_class(&MemoryClass::Personal).await.unwrap();
        assert_eq!(personal_memories.len(), 1);
        assert_eq!(personal_memories[0].content, "Personal content");

        let work_memories = db.get_memories_by_class(&MemoryClass::Work).await.unwrap();
        assert_eq!(work_memories.len(), 1);
        assert_eq!(work_memories[0].content, "Work content");
    }

    #[tokio::test]
    async fn test_get_last_memories() {
        let (mut db, _temp_dir) = create_test_database();

        // Store multiple memories
        for i in 0..10 {
            let memory = MemoryBuilder::new()
                .with_content(format!("Memory {}", i))
                .with_class(MemoryClass::Personal)
                .build();
            db.store_memory(&memory).await.unwrap();
        }

        // Get last 5 memories
        let last_memories = db.get_last_memories("default_user", 5).await.unwrap();
        assert_eq!(last_memories.len(), 5);
    }

    #[tokio::test]
    async fn test_delete_memory() {
        let (mut db, _temp_dir) = create_test_database();

        let memory = MemoryBuilder::new()
            .with_content("Memory to delete")
            .with_class(MemoryClass::Personal)
            .build();

        // Store memory
        db.store_memory(&memory).await.unwrap();

        // Verify it exists
        let retrieved = db.get_memory(memory.id).await.unwrap();
        assert!(retrieved.is_some());

        // Delete memory
        let result = db.delete_memory(memory.id).await;
        assert!(result.is_ok());

        // Verify it's gone
        let retrieved = db.get_memory(memory.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let (mut db, _temp_dir) = create_test_database();

        let memory1 = MemoryBuilder::new()
            .with_content("Concurrent memory 1")
            .build();

        let memory2 = MemoryBuilder::new()
            .with_content("Concurrent memory 2")
            .build();

        // Test operations sequentially since we can't borrow mutably and immutably at the same time
        let store_result = db.store_memory(&memory1).await;
        let query_result = db.get_memories_by_class(&MemoryClass::Personal).await;
        let delete_result = db.delete_memory(memory2.id).await;

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
        
        // Ensure the parent directory exists
        if let Some(parent) = keyset_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create keyset directory");
        }
        
        // Use password-based CryptoManager to create a proper keyset file
        let _crypto_manager = mimir_core::crypto::CryptoManager::with_password(&keyset_path, "test-password")
            .expect("Failed to create test crypto manager");

        let mut db1 = Database::new(&db_path1, &keyset_path).unwrap();
        let mut db2 = Database::new(&db_path2, &keyset_path).unwrap();

        let memory1 = MemoryBuilder::new().with_content("DB1 memory").build();
        let memory2 = MemoryBuilder::new().with_content("DB2 memory").build();

        // Store in different databases
        assert!(db1.store_memory(&memory1).await.is_ok());
        assert!(db2.store_memory(&memory2).await.is_ok());
    }

    #[test]
    fn test_database_error_handling_invalid_db_path() {
        use mimir_core::test_utils::env::create_temp_dir;
        let temp_dir = create_temp_dir();
        let db_path = get_test_db_path(&temp_dir);
        let db_path_str = db_path.to_str().unwrap();
        let keyset_path = temp_dir.path().join("keyset.json");
        let keyset_path_str = keyset_path.to_str().unwrap();
        let _crypto_manager = mimir_core::crypto::CryptoManager::with_password(&keyset_path, "test-password")
            .expect("Failed to create test crypto manager");
        let invalid_paths = vec![
            "",                       // Empty path
            "/root/no_permission.db", // Permission denied path
        ];
        for path in invalid_paths {
            let result = Database::new(path, keyset_path_str);
            assert!(result.is_err(), "Should fail for invalid db path: {}", path);
        }
        // Also test with a valid db path but invalid keyset path
        let result = Database::new(db_path_str, "");
        assert!(result.is_err(), "Should fail for invalid keyset path");
    }

    #[test]
    fn test_database_error_handling_invalid_keyset_path() {
        use mimir_core::test_utils::env::create_temp_dir;
        let temp_dir = create_temp_dir();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();
        let invalid_keyset_paths = vec![
            "/non/existent/keyset.json",  // Absolute path that doesn't exist
            "/tmp/non_existent_keyset.json", // Another absolute path
            "",  // Empty path
        ];
        for keyset_path in invalid_keyset_paths {
            let result = Database::new(db_path_str, keyset_path);
            assert!(result.is_err(), "Should fail for invalid keyset path: {}", keyset_path);
        }
    }

    #[test]
    fn test_crypto_manager_integration() {
        let temp_dir = create_temp_dir();
        let db_path = get_test_db_path(&temp_dir);
        let keyset_path = temp_dir.path().join("keyset.json");

        // Ensure the parent directory exists
        if let Some(parent) = keyset_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create keyset directory");
        }

        // Test that the CryptoManager can be created and used properly
        let crypto_manager = mimir_core::crypto::CryptoManager::with_password(&keyset_path, "test-password");
        assert!(crypto_manager.is_ok(), "Failed to create crypto manager");

        // Test that we can create a database with the crypto manager
        let result = Database::new(&db_path, &keyset_path);
        assert!(result.is_ok(), "Failed to create database with crypto manager");
    }

    #[tokio::test]
    async fn test_memory_data_integrity() {
        let (mut db, _temp_dir) = create_test_database();

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

        // Retrieve and verify content integrity
        let retrieved = db.get_memory(original_memory.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_memory = retrieved.unwrap();
        assert_eq!(retrieved_memory.content, original_memory.content);
        assert_eq!(retrieved_memory.class, original_memory.class);
    }

    #[tokio::test]
    async fn test_large_content_handling() {
        let (mut db, _temp_dir) = create_test_database();

        // Test with large content (simulating edge cases)
        let large_content = "x".repeat(10_000); // 10KB content
        let memory = MemoryBuilder::new().with_content(large_content).build();

        let result = db.store_memory(&memory).await;
        assert!(result.is_ok());

        // Verify we can retrieve it
        let retrieved = db.get_memory(memory.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content.len(), 10_000);
    }

    #[tokio::test]
    async fn test_special_characters_in_content() {
        let (mut db, _temp_dir) = create_test_database();

        let special_contents = vec![
            "🧠 Memory with emojis 🔒",
            "Memory with\nnewlines\nand\ttabs",
            "Memory with 'quotes' and \"double quotes\"",
            "Memory with JSON: {\"key\": \"value\", \"number\": 42}",
            "Memory with SQL: SELECT * FROM memory WHERE id = 1; DROP TABLE memory;",
        ];

        for content in special_contents {
            let memory = MemoryBuilder::new().with_content(content).build();

            let result = db.store_memory(&memory).await;
            assert!(result.is_ok(), "Failed to store content: {}", content);

            // Verify we can retrieve it
            let retrieved = db.get_memory(memory.id).await.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().content, content);
        }
    }

    #[tokio::test]
    async fn test_transaction_support() {
        let (mut db, _temp_dir) = create_test_database();

        // Test transaction support
        let transaction = db.transaction();
        assert!(transaction.is_ok());

        // Note: In a real implementation, you would use the transaction
        // to perform multiple operations atomically
    }
}
