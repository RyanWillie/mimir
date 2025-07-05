//! Mimir Crypto - Encryption layer for AI Memory Vault
//!
//! This module provides:
//! - Root Key (RK) management in OS keychain
//! - Per-class key derivation using HMAC-SHA256
//! - XChaCha20-Poly1305 encryption for memory content
//! - Keyset management and rotation

use crate::{error::MimirError, Result};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305, XNonce,
};
use ring::{hmac, rand::{SecureRandom, SystemRandom}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Length of a root key in bytes (32 bytes = 256 bits)
pub const ROOT_KEY_LEN: usize = 32;

/// Length of a class key in bytes (32 bytes = 256 bits) 
pub const CLASS_KEY_LEN: usize = 32;

/// Length of XChaCha20-Poly1305 nonce (24 bytes)
pub const NONCE_LEN: usize = 24;

/// Service name for OS keychain storage
pub const KEYCHAIN_SERVICE: &str = "com.mimir.memory-vault";

/// Root key identifier in keychain
pub const ROOT_KEY_ID: &str = "mimir-root-key";

/// Root Key for the device, stored in OS keychain
#[derive(ZeroizeOnDrop, Zeroize)]
pub struct RootKey {
    key: [u8; ROOT_KEY_LEN],
}

impl RootKey {
    /// Generate a new root key
    pub fn new() -> Result<Self> {
        let rng = SystemRandom::new();
        let mut key = [0u8; ROOT_KEY_LEN];
        rng.fill(&mut key)
            .map_err(|_| MimirError::Encryption("Failed to generate root key".to_string()))?;
        
        Ok(RootKey { key })
    }

    /// Load root key from OS keychain
    pub fn load() -> Result<Self> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, ROOT_KEY_ID)
            .map_err(|e| MimirError::Encryption(format!("Failed to access keychain: {}", e)))?;
        
        let key_hex = entry.get_password()
            .map_err(|e| MimirError::Encryption(format!("Failed to load root key: {}", e)))?;
        
        let key_bytes = hex::decode(key_hex)
            .map_err(|e| MimirError::Encryption(format!("Invalid root key format: {}", e)))?;
        
        if key_bytes.len() != ROOT_KEY_LEN {
            return Err(MimirError::Encryption("Invalid root key length".to_string()));
        }
        
        let mut key = [0u8; ROOT_KEY_LEN];
        key.copy_from_slice(&key_bytes);
        
        Ok(RootKey { key })
    }

    /// Save root key to OS keychain
    pub fn save(&self) -> Result<()> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, ROOT_KEY_ID)
            .map_err(|e| MimirError::Encryption(format!("Failed to access keychain: {}", e)))?;
        
        let key_hex = hex::encode(self.key);
        entry.set_password(&key_hex)
            .map_err(|e| MimirError::Encryption(format!("Failed to save root key: {}", e)))?;
        
        Ok(())
    }

    /// Check if root key exists in keychain
    pub fn exists() -> bool {
        keyring::Entry::new(KEYCHAIN_SERVICE, ROOT_KEY_ID)
            .and_then(|entry| entry.get_password())
            .is_ok()
    }

    /// Derive a class key from the root key using HMAC-SHA256
    pub fn derive_class_key(&self, class: &str) -> Result<ClassKey> {
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.key);
        let class_bytes = class.as_bytes();
        let signature = hmac::sign(&key, class_bytes);
        
        // Use first 32 bytes of HMAC output as class key
        let mut class_key = [0u8; CLASS_KEY_LEN];
        class_key.copy_from_slice(&signature.as_ref()[..CLASS_KEY_LEN]);
        
        Ok(ClassKey { key: class_key })
    }

    /// Derive SQLCipher database key from root key
    pub fn derive_db_key(&self) -> Result<String> {
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.key);
        let db_context = b"mimir-database";
        let signature = hmac::sign(&key, db_context);
        
        // Return hex-encoded key for SQLCipher
        Ok(hex::encode(signature.as_ref()))
    }

    /// Rotate root key - generates new key and returns old one for re-encryption
    pub fn rotate(&mut self) -> Result<RootKey> {
        let old_key = RootKey { key: self.key };
        
        // Generate new key
        let rng = SystemRandom::new();
        rng.fill(&mut self.key)
            .map_err(|_| MimirError::Encryption("Failed to generate new root key".to_string()))?;
        
        // Save new key to keychain
        self.save()?;
        
        Ok(old_key)
    }
}

/// Class-specific encryption key
#[derive(ZeroizeOnDrop, Zeroize)]
pub struct ClassKey {
    key: [u8; CLASS_KEY_LEN],
}

impl ClassKey {
    /// Create new random class key
    pub fn new() -> Result<Self> {
        let rng = SystemRandom::new();
        let mut key = [0u8; CLASS_KEY_LEN];
        rng.fill(&mut key)
            .map_err(|_| MimirError::Encryption("Failed to generate class key".to_string()))?;
        
        Ok(ClassKey { key })
    }

    /// Get key bytes for encryption
    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }

    /// Encrypt data with this class key using XChaCha20-Poly1305
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Ciphertext> {
        let cipher = XChaCha20Poly1305::new_from_slice(&self.key)
            .map_err(|_| MimirError::Encryption("Failed to create cipher".to_string()))?;
        
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, plaintext)
            .map_err(|_| MimirError::Encryption("Failed to encrypt data".to_string()))?;
        
        Ok(Ciphertext {
            data: ciphertext,
            nonce: nonce.to_vec(),
        })
    }

    /// Decrypt data with this class key using XChaCha20-Poly1305
    pub fn decrypt(&self, ciphertext: &Ciphertext) -> Result<Vec<u8>> {
        let cipher = XChaCha20Poly1305::new_from_slice(&self.key)
            .map_err(|_| MimirError::Encryption("Failed to create cipher".to_string()))?;
        
        if ciphertext.nonce.len() != NONCE_LEN {
            return Err(MimirError::Encryption("Invalid nonce length".to_string()));
        }
        
        let nonce = XNonce::from_slice(&ciphertext.nonce);
        let plaintext = cipher.decrypt(nonce, ciphertext.data.as_slice())
            .map_err(|_| MimirError::Encryption("Failed to decrypt data".to_string()))?;
        
        Ok(plaintext)
    }
}

/// Encrypted data with nonce
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ciphertext {
    /// Encrypted data
    pub data: Vec<u8>,
    /// Nonce used for encryption
    pub nonce: Vec<u8>,
}

/// Keyset storage format
#[derive(Debug, Serialize, Deserialize)]
pub struct Keyset {
    /// Version of the keyset format
    pub version: u32,
    /// Encrypted class keys (encrypted with root key)
    pub class_keys: HashMap<String, Ciphertext>,
    /// Timestamp of last update
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Crypto manager for the memory vault
pub struct CryptoManager {
    root_key: RootKey,
    class_keys: HashMap<String, ClassKey>,
    purged_classes: std::collections::HashSet<String>,
    keyset_path: std::path::PathBuf,
}

impl CryptoManager {
    /// Initialize crypto manager
    pub fn new<P: AsRef<Path>>(keyset_path: P) -> Result<Self> {
        let keyset_path = keyset_path.as_ref().to_path_buf();
        
        // Load or create root key
        let root_key = if RootKey::exists() {
            RootKey::load()?
        } else {
            let root_key = RootKey::new()?;
            root_key.save()?;
            root_key
        };
        
        let mut crypto_manager = CryptoManager {
            root_key,
            class_keys: HashMap::new(),
            purged_classes: std::collections::HashSet::new(),
            keyset_path,
        };
        
        // Load existing keyset or create new one
        if crypto_manager.keyset_path.exists() {
            crypto_manager.load_keyset()?;
        } else {
            crypto_manager.create_keyset()?;
        }
        
        Ok(crypto_manager)
    }

    /// Get database key for SQLCipher
    pub fn get_db_key(&self) -> Result<String> {
        self.root_key.derive_db_key()
    }

    /// Encrypt plaintext for a specific class
    pub fn encrypt(&mut self, class: &str, plaintext: &[u8]) -> Result<Ciphertext> {
        // Ensure we have the class key
        if !self.class_keys.contains_key(class) {
            let class_key = self.root_key.derive_class_key(class)?;
            self.class_keys.insert(class.to_string(), class_key);
            // Remove from purged classes if it was purged before
            self.purged_classes.remove(class);
            self.save_keyset()?;
        }
        
        let class_key = self.class_keys.get(class).unwrap();
        class_key.encrypt(plaintext)
    }

    /// Decrypt ciphertext for a specific class
    pub fn decrypt(&mut self, class: &str, ciphertext: &Ciphertext) -> Result<Vec<u8>> {
        // Check if class was purged
        if self.purged_classes.contains(class) {
            return Err(MimirError::Encryption(format!("Class '{}' has been purged", class)));
        }
        
        // Ensure we have the class key
        if !self.class_keys.contains_key(class) {
            let class_key = self.root_key.derive_class_key(class)?;
            self.class_keys.insert(class.to_string(), class_key);
        }
        
        let class_key = self.class_keys.get(class).unwrap();
        class_key.decrypt(ciphertext)
    }

    /// Rotate root key and re-encrypt all class keys
    pub fn rotate_root_key(&mut self) -> Result<()> {
        let _old_root_key = self.root_key.rotate()?;
        
        // Re-derive all class keys with new root key
        let class_names: Vec<String> = self.class_keys.keys().cloned().collect();
        for class in class_names {
            let new_class_key = self.root_key.derive_class_key(&class)?;
            self.class_keys.insert(class, new_class_key);
        }
        
        self.save_keyset()?;
        Ok(())
    }

    /// Rotate a specific class key
    pub fn rotate_class_key(&mut self, class: &str) -> Result<()> {
        let new_class_key = ClassKey::new()?;
        self.class_keys.insert(class.to_string(), new_class_key);
        // Remove from purged classes if it was purged before
        self.purged_classes.remove(class);
        self.save_keyset()?;
        Ok(())
    }

    /// Purge a class (remove its key and mark as purged)
    pub fn purge_class(&mut self, class: &str) -> Result<()> {
        self.class_keys.remove(class);
        self.purged_classes.insert(class.to_string());
        self.save_keyset()?;
        Ok(())
    }

    /// Load keyset from disk
    fn load_keyset(&mut self) -> Result<()> {
        let keyset_data = fs::read(&self.keyset_path)
            .map_err(|e| MimirError::Encryption(format!("Failed to read keyset: {}", e)))?;
        
        let keyset: Keyset = serde_json::from_slice(&keyset_data)
            .map_err(|e| MimirError::Encryption(format!("Failed to parse keyset: {}", e)))?;
        
        // Decrypt class keys using root key
        for (class, encrypted_key) in keyset.class_keys {
            let decrypted_key_bytes = self.root_key.derive_class_key(&class)?
                .decrypt(&encrypted_key)?;
            
            if decrypted_key_bytes.len() != CLASS_KEY_LEN {
                return Err(MimirError::Encryption("Invalid class key length".to_string()));
            }
            
            let mut class_key = [0u8; CLASS_KEY_LEN];
            class_key.copy_from_slice(&decrypted_key_bytes);
            
            self.class_keys.insert(class, ClassKey { key: class_key });
        }
        
        Ok(())
    }

    /// Save keyset to disk
    fn save_keyset(&self) -> Result<()> {
        let mut encrypted_class_keys = HashMap::new();
        
        // Encrypt class keys with root key
        for (class, class_key) in &self.class_keys {
            let root_derived_key = self.root_key.derive_class_key(class)?;
            let encrypted_key = root_derived_key.encrypt(class_key.as_bytes())?;
            encrypted_class_keys.insert(class.clone(), encrypted_key);
        }
        
        let keyset = Keyset {
            version: 1,
            class_keys: encrypted_class_keys,
            updated_at: chrono::Utc::now(),
        };
        
        let keyset_data = serde_json::to_vec_pretty(&keyset)
            .map_err(|e| MimirError::Encryption(format!("Failed to serialize keyset: {}", e)))?;
        
        fs::write(&self.keyset_path, keyset_data)
            .map_err(|e| MimirError::Encryption(format!("Failed to write keyset: {}", e)))?;
        
        Ok(())
    }

    /// Create initial keyset
    fn create_keyset(&mut self) -> Result<()> {
        let keyset = Keyset {
            version: 1,
            class_keys: HashMap::new(),
            updated_at: chrono::Utc::now(),
        };
        
        let keyset_data = serde_json::to_vec_pretty(&keyset)
            .map_err(|e| MimirError::Encryption(format!("Failed to serialize keyset: {}", e)))?;
        
        fs::write(&self.keyset_path, keyset_data)
            .map_err(|e| MimirError::Encryption(format!("Failed to write keyset: {}", e)))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_root_key_generation() {
        let root_key = RootKey::new().unwrap();
        // Just verify it can be created
        drop(root_key);
    }

    #[test]
    fn test_class_key_derivation() {
        let root_key = RootKey::new().unwrap();
        let class_key1 = root_key.derive_class_key("personal").unwrap();
        let class_key2 = root_key.derive_class_key("work").unwrap();
        
        // Keys should be different for different classes
        assert_ne!(class_key1.as_bytes(), class_key2.as_bytes());
        
        // Same class should produce same key
        let class_key3 = root_key.derive_class_key("personal").unwrap();
        assert_eq!(class_key1.as_bytes(), class_key3.as_bytes());
    }

    #[test]
    fn test_encryption_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let keyset_path = temp_dir.path().join("keyset.json");
        
        let mut crypto_manager = CryptoManager::new(keyset_path).unwrap();
        
        let plaintext = b"Hello, secure world!";
        let ciphertext = crypto_manager.encrypt("personal", plaintext).unwrap();
        let decrypted = crypto_manager.decrypt("personal", &ciphertext).unwrap();
        
        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_class_key_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let keyset_path = temp_dir.path().join("keyset.json");
        
        let mut crypto_manager = CryptoManager::new(keyset_path).unwrap();
        
        // Encrypt some data
        let plaintext = b"Test data";
        let ciphertext = crypto_manager.encrypt("personal", plaintext).unwrap();
        
        // Rotate class key
        crypto_manager.rotate_class_key("personal").unwrap();
        
        // Old ciphertext should not decrypt with new key
        let decrypt_result = crypto_manager.decrypt("personal", &ciphertext);
        assert!(decrypt_result.is_err());
        
        // New encryption should work
        let new_ciphertext = crypto_manager.encrypt("personal", plaintext).unwrap();
        let new_decrypted = crypto_manager.decrypt("personal", &new_ciphertext).unwrap();
        assert_eq!(plaintext, new_decrypted.as_slice());
    }

    #[test]
    fn test_purge_class() {
        let temp_dir = TempDir::new().unwrap();
        let keyset_path = temp_dir.path().join("keyset.json");
        
        let mut crypto_manager = CryptoManager::new(keyset_path).unwrap();
        
        // Create some encrypted data
        let plaintext = b"Sensitive data";
        let _ciphertext = crypto_manager.encrypt("personal", plaintext).unwrap();
        
        // Purge the class
        crypto_manager.purge_class("personal").unwrap();
        
        // Should not be able to decrypt anymore
        assert!(!crypto_manager.class_keys.contains_key("personal"));
    }
} 