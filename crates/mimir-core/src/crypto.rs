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
use ring::{
    hmac, pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
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

/// Length of salt for password derivation (32 bytes)
pub const SALT_LEN: usize = 32;

/// Number of PBKDF2 iterations for password derivation
pub const PBKDF2_ITERATIONS: u32 = 100_000;

/// Service name for OS keychain storage
pub const KEYCHAIN_SERVICE: &str = "com.mimir.memory-vault";

/// Root key identifier in keychain
pub const ROOT_KEY_ID: &str = "mimir-root-key";

/// Root Key for the device, stored in OS keychain or derived from password
#[derive(ZeroizeOnDrop, Zeroize)]
pub struct RootKey {
    key: [u8; ROOT_KEY_LEN],
}

impl RootKey {
    /// Generate a new random root key
    pub fn new() -> Result<Self> {
        let rng = SystemRandom::new();
        let mut key = [0u8; ROOT_KEY_LEN];
        rng.fill(&mut key)
            .map_err(|_| MimirError::Encryption("Failed to generate root key".to_string()))?;

        Ok(RootKey { key })
    }

    /// Create a new root key from a password
    pub fn from_password(password: &str, salt: &[u8; SALT_LEN]) -> Result<Self> {
        if password.is_empty() {
            return Err(MimirError::Encryption(
                "Password cannot be empty".to_string(),
            ));
        }

        let mut key = [0u8; ROOT_KEY_LEN];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            std::num::NonZeroU32::new(PBKDF2_ITERATIONS).unwrap(),
            salt,
            password.as_bytes(),
            &mut key,
        );

        Ok(RootKey { key })
    }

    /// Generate a new random salt for password derivation
    pub fn generate_salt() -> Result<[u8; SALT_LEN]> {
        let rng = SystemRandom::new();
        let mut salt = [0u8; SALT_LEN];
        rng.fill(&mut salt)
            .map_err(|_| MimirError::Encryption("Failed to generate salt".to_string()))?;

        Ok(salt)
    }

    /// Load root key from OS keychain
    pub fn load() -> Result<Self> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, ROOT_KEY_ID)
            .map_err(|e| MimirError::Encryption(format!("Failed to access keychain: {}", e)))?;

        let key_hex = entry
            .get_password()
            .map_err(|e| MimirError::Encryption(format!("Failed to load root key: {}", e)))?;

        let key_bytes = hex::decode(key_hex)
            .map_err(|e| MimirError::Encryption(format!("Invalid root key format: {}", e)))?;

        if key_bytes.len() != ROOT_KEY_LEN {
            return Err(MimirError::Encryption(
                "Invalid root key length".to_string(),
            ));
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
        entry
            .set_password(&key_hex)
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

        // SQLCipher expects a 32-byte key, so take the first 32 bytes of the HMAC signature
        let signature_bytes = signature.as_ref();
        let db_key_bytes = &signature_bytes[..32];

        // Return as a passphrase for SQLCipher (it will hash it internally)
        Ok(String::from_utf8_lossy(db_key_bytes).to_string())
    }

    /// Derive SQLCipher database key as raw bytes from root key
    pub fn derive_db_key_bytes(&self) -> [u8; 32] {
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.key);
        let db_context = b"mimir-database";
        let signature = hmac::sign(&key, db_context);
        let signature_bytes = signature.as_ref();
        let mut db_key_bytes = [0u8; 32];
        db_key_bytes.copy_from_slice(&signature_bytes[..32]);
        db_key_bytes
    }

    /// Get root key bytes for cryptographic operations
    ///
    /// This method provides safe access to the root key bytes for use in
    /// cryptographic operations like HKDF derivation. The bytes are returned
    /// as a reference to avoid unnecessary copying.
    pub fn as_bytes(&self) -> &[u8; ROOT_KEY_LEN] {
        &self.key
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
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
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
        let plaintext = cipher
            .decrypt(nonce, ciphertext.data.as_slice())
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
    /// Salt for password derivation (if using password-based encryption)
    pub salt: Option<[u8; SALT_LEN]>,
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
    /// Initialize crypto manager with OS keychain
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

    /// Initialize crypto manager with password-based encryption
    pub fn with_password<P: AsRef<Path>>(keyset_path: P, password: &str) -> Result<Self> {
        let keyset_path = keyset_path.as_ref().to_path_buf();

        let mut crypto_manager = CryptoManager {
            root_key: RootKey::new()?, // Will be set properly below
            class_keys: HashMap::new(),
            purged_classes: std::collections::HashSet::new(),
            keyset_path,
        };

        // Load existing keyset or create new one
        if crypto_manager.keyset_path.exists() {
            crypto_manager.load_keyset_with_password(password)?;
        } else {
            crypto_manager.create_keyset_with_password(password)?;
        }

        Ok(crypto_manager)
    }

    /// Get database key for SQLCipher
    pub fn get_db_key(&self) -> Result<String> {
        self.root_key.derive_db_key()
    }

    /// Get database key for SQLCipher as raw bytes
    pub fn get_db_key_bytes(&self) -> [u8; 32] {
        self.root_key.derive_db_key_bytes()
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
            return Err(MimirError::Encryption(format!(
                "Class '{}' has been purged",
                class
            )));
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
            let decrypted_key_bytes = self
                .root_key
                .derive_class_key(&class)?
                .decrypt(&encrypted_key)?;

            if decrypted_key_bytes.len() != CLASS_KEY_LEN {
                return Err(MimirError::Encryption(
                    "Invalid class key length".to_string(),
                ));
            }

            let mut class_key = [0u8; CLASS_KEY_LEN];
            class_key.copy_from_slice(&decrypted_key_bytes);

            self.class_keys.insert(class, ClassKey { key: class_key });
        }

        Ok(())
    }

    /// Save keyset to disk
    fn save_keyset(&self) -> Result<()> {
        // Check if this is a password-based keyset by reading the existing keyset
        let salt = if self.keyset_path.exists() {
            let keyset_data = fs::read(&self.keyset_path)
                .map_err(|e| MimirError::Encryption(format!("Failed to read keyset: {}", e)))?;

            let keyset: Keyset = serde_json::from_slice(&keyset_data)
                .map_err(|e| MimirError::Encryption(format!("Failed to parse keyset: {}", e)))?;

            keyset.salt
        } else {
            None
        };

        let mut encrypted_class_keys = HashMap::new();

        // For password-based keysets, we don't store encrypted class keys since they're derived
        // For keychain-based keysets, we encrypt and store the class keys
        if salt.is_none() {
            // Encrypt class keys with root key (keychain-based)
            for (class, class_key) in &self.class_keys {
                let root_derived_key = self.root_key.derive_class_key(class)?;
                let encrypted_key = root_derived_key.encrypt(class_key.as_bytes())?;
                encrypted_class_keys.insert(class.clone(), encrypted_key);
            }
        }
        // For password-based keysets, class_keys remains empty since keys are derived on-demand

        let keyset = Keyset {
            version: 1,
            salt,
            class_keys: encrypted_class_keys,
            updated_at: chrono::Utc::now(),
        };

        let keyset_data = serde_json::to_vec_pretty(&keyset)
            .map_err(|e| MimirError::Encryption(format!("Failed to serialize keyset: {}", e)))?;

        // Ensure the parent directory exists before writing
        if let Some(parent) = self.keyset_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                MimirError::Encryption(format!("Failed to create keyset directory: {}", e))
            })?;
        }

        fs::write(&self.keyset_path, keyset_data)
            .map_err(|e| MimirError::Encryption(format!("Failed to write keyset: {}", e)))?;

        Ok(())
    }

    /// Create initial keyset
    fn create_keyset(&mut self) -> Result<()> {
        let keyset = Keyset {
            version: 1,
            salt: None, // No salt for keychain-based encryption
            class_keys: HashMap::new(),
            updated_at: chrono::Utc::now(),
        };

        let keyset_data = serde_json::to_vec_pretty(&keyset)
            .map_err(|e| MimirError::Encryption(format!("Failed to serialize keyset: {}", e)))?;

        // Ensure the parent directory exists before writing
        if let Some(parent) = self.keyset_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                MimirError::Encryption(format!("Failed to create keyset directory: {}", e))
            })?;
        }

        fs::write(&self.keyset_path, keyset_data)
            .map_err(|e| MimirError::Encryption(format!("Failed to write keyset: {}", e)))?;

        Ok(())
    }

    /// Create initial keyset with password-based encryption
    fn create_keyset_with_password(&mut self, password: &str) -> Result<()> {
        // Generate salt and derive root key from password
        let salt = RootKey::generate_salt()?;
        self.root_key = RootKey::from_password(password, &salt)?;

        let keyset = Keyset {
            version: 1,
            salt: Some(salt),
            class_keys: HashMap::new(),
            updated_at: chrono::Utc::now(),
        };

        let keyset_data = serde_json::to_vec_pretty(&keyset)
            .map_err(|e| MimirError::Encryption(format!("Failed to serialize keyset: {}", e)))?;

        // Ensure the parent directory exists before writing
        if let Some(parent) = self.keyset_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                MimirError::Encryption(format!("Failed to create keyset directory: {}", e))
            })?;
        }

        fs::write(&self.keyset_path, keyset_data)
            .map_err(|e| MimirError::Encryption(format!("Failed to write keyset: {}", e)))?;

        Ok(())
    }

    /// Load keyset with password-based decryption
    fn load_keyset_with_password(&mut self, password: &str) -> Result<()> {
        let keyset_data = fs::read(&self.keyset_path)
            .map_err(|e| MimirError::Encryption(format!("Failed to read keyset: {}", e)))?;

        let keyset: Keyset = serde_json::from_slice(&keyset_data)
            .map_err(|e| MimirError::Encryption(format!("Failed to parse keyset: {}", e)))?;

        // Check if this is a password-based keyset
        let salt = keyset.salt.ok_or_else(|| {
            MimirError::Encryption(
                "Keyset is not password-based. Use keychain-based initialization.".to_string(),
            )
        })?;

        // Derive root key from password and salt
        self.root_key = RootKey::from_password(password, &salt)?;

        // For password-based keysets, class keys are derived from the root key, not stored encrypted
        // So we don't need to decrypt them - they'll be derived on-demand when needed
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
    fn test_db_key_length() {
        let root_key = RootKey::new().unwrap();
        let db_key = root_key.derive_db_key().unwrap();
        println!("DB key length: {}, key: {:?}", db_key.len(), db_key);
        // Should be reasonable length for a passphrase
        assert!(db_key.len() > 0);
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

        let password = "test-password-for-ci";
        let mut crypto_manager = CryptoManager::with_password(&keyset_path, password).unwrap();

        let plaintext = b"Hello, secure world!";
        let ciphertext = crypto_manager.encrypt("personal", plaintext).unwrap();
        let decrypted = crypto_manager.decrypt("personal", &ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_class_key_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let keyset_path = temp_dir.path().join("keyset.json");

        let password = "test-password-for-ci";
        let mut crypto_manager = CryptoManager::with_password(&keyset_path, password).unwrap();

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

        let password = "test-password-for-ci";
        let mut crypto_manager = CryptoManager::with_password(&keyset_path, password).unwrap();

        // Create some encrypted data
        let plaintext = b"Sensitive data";
        let _ciphertext = crypto_manager.encrypt("personal", plaintext).unwrap();

        // Purge the class
        crypto_manager.purge_class("personal").unwrap();

        // Should not be able to decrypt anymore
        assert!(!crypto_manager.class_keys.contains_key("personal"));
    }

    #[test]
    fn test_password_based_encryption() {
        let temp_dir = TempDir::new().unwrap();
        let keyset_path = temp_dir.path().join("keyset.json");

        let password = "my-secure-password-123";
        let test_data = "Sensitive information encrypted with password";

        // Create crypto manager with password
        let mut crypto_manager = CryptoManager::with_password(&keyset_path, password).unwrap();

        // Encrypt data
        let ciphertext = crypto_manager
            .encrypt("personal", test_data.as_bytes())
            .unwrap();

        // Verify ciphertext is different from plaintext
        assert_ne!(ciphertext.data, test_data.as_bytes());

        // Decrypt data
        let decrypted = crypto_manager.decrypt("personal", &ciphertext).unwrap();
        let decrypted_str = String::from_utf8(decrypted).unwrap();

        // Verify round-trip integrity
        assert_eq!(decrypted_str, test_data);
    }

    #[test]
    fn test_password_based_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let keyset_path = temp_dir.path().join("keyset.json");

        let password = "another-secure-password";
        let test_data = "Data that should persist across sessions";

        // Create first manager and encrypt data
        let ciphertext = {
            let mut crypto_manager = CryptoManager::with_password(&keyset_path, password).unwrap();
            crypto_manager
                .encrypt("work", test_data.as_bytes())
                .unwrap()
        };

        // Create second manager and decrypt data
        {
            let mut crypto_manager = CryptoManager::with_password(&keyset_path, password).unwrap();
            let decrypted = crypto_manager.decrypt("work", &ciphertext).unwrap();
            assert_eq!(String::from_utf8(decrypted).unwrap(), test_data);
        }

        // Verify keyset file exists and contains salt
        assert!(keyset_path.exists());
        let keyset_data = fs::read(&keyset_path).unwrap();
        let keyset: Keyset = serde_json::from_slice(&keyset_data).unwrap();
        assert!(keyset.salt.is_some());
    }

    #[test]
    fn test_password_validation() {
        let temp_dir = TempDir::new().unwrap();
        let keyset_path = temp_dir.path().join("keyset.json");

        // Empty password should fail
        let result = CryptoManager::with_password(&keyset_path, "");
        assert!(result.is_err());

        // Valid password should work
        let result = CryptoManager::with_password(&keyset_path, "valid-password");
        assert!(result.is_ok());
    }

    #[test]
    fn test_salt_generation() {
        let salt1 = RootKey::generate_salt().unwrap();
        let salt2 = RootKey::generate_salt().unwrap();

        // Salts should be different
        assert_ne!(salt1, salt2);

        // Salts should be correct length
        assert_eq!(salt1.len(), SALT_LEN);
        assert_eq!(salt2.len(), SALT_LEN);
    }

    #[test]
    fn test_password_key_derivation() {
        let password = "test-password";
        let salt = RootKey::generate_salt().unwrap();

        let key1 = RootKey::from_password(password, &salt).unwrap();
        let key2 = RootKey::from_password(password, &salt).unwrap();

        // Same password and salt should produce same key
        assert_eq!(key1.key, key2.key);

        // Different password should produce different key
        let key3 = RootKey::from_password("different-password", &salt).unwrap();
        assert_ne!(key1.key, key3.key);

        // Different salt should produce different key
        let salt2 = RootKey::generate_salt().unwrap();
        let key4 = RootKey::from_password(password, &salt2).unwrap();
        assert_ne!(key1.key, key4.key);
    }
}
