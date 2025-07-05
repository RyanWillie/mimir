//! Integration tests for Mimir encryption layer
//!
//! These tests verify the complete encryption functionality including
//! round-trip encryption/decryption, key rotation, and class purging.

use mimir_core::crypto::{CryptoManager, RootKey};
use mimir_core::Result;
use tempfile::TempDir;

/// Helper to create a test crypto manager
fn create_test_crypto_manager() -> (CryptoManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let keyset_path = temp_dir.path().join("keyset.json");
    let crypto_manager = CryptoManager::new(&keyset_path).unwrap();
    (crypto_manager, temp_dir)
}

#[test]
fn test_round_trip() {
    let (mut crypto_manager, _temp_dir) = create_test_crypto_manager();
    
    let test_data = "This is sensitive information that needs encryption";
    let class = "personal";
    
    // Encrypt the data
    let ciphertext = crypto_manager.encrypt(class, test_data.as_bytes()).unwrap();
    
    // Verify ciphertext is different from plaintext
    assert_ne!(ciphertext.data, test_data.as_bytes());
    assert!(!ciphertext.nonce.is_empty());
    
    // Decrypt the data
    let decrypted = crypto_manager.decrypt(class, &ciphertext).unwrap();
    let decrypted_str = String::from_utf8(decrypted).unwrap();
    
    // Verify round-trip integrity
    assert_eq!(decrypted_str, test_data);
}

#[test]
fn test_round_trip_multiple_classes() {
    let (mut crypto_manager, _temp_dir) = create_test_crypto_manager();
    
    let test_cases = vec![
        ("personal", "Personal secret information"),
        ("work", "Confidential work data"),
        ("health", "Medical information"),
        ("financial", "Bank account details"),
        ("custom", "Custom class data"),
    ];
    
    // Encrypt all data
    let mut ciphertexts = Vec::new();
    for (class, data) in &test_cases {
        let ciphertext = crypto_manager.encrypt(class, data.as_bytes()).unwrap();
        ciphertexts.push((class, ciphertext));
    }
    
    // Decrypt and verify all data
    for ((class, original_data), (_, ciphertext)) in test_cases.iter().zip(ciphertexts.iter()) {
        let decrypted = crypto_manager.decrypt(class, ciphertext).unwrap();
        let decrypted_str = String::from_utf8(decrypted).unwrap();
        assert_eq!(decrypted_str, *original_data);
    }
}

#[test]
fn test_class_rotation_preserves_plaintext() {
    let (mut crypto_manager, _temp_dir) = create_test_crypto_manager();
    
    let test_data = "Data that should survive key rotation";
    let class = "personal";
    
    // Encrypt with original key
    let original_ciphertext = crypto_manager.encrypt(class, test_data.as_bytes()).unwrap();
    
    // Verify original decryption works
    let decrypted_original = crypto_manager.decrypt(class, &original_ciphertext).unwrap();
    assert_eq!(String::from_utf8(decrypted_original).unwrap(), test_data);
    
    // Rotate the class key
    crypto_manager.rotate_class_key(class).unwrap();
    
    // Original ciphertext should no longer decrypt (key has changed)
    let decrypt_result = crypto_manager.decrypt(class, &original_ciphertext);
    assert!(decrypt_result.is_err(), "Old ciphertext should not decrypt with new key");
    
    // New encryption should work with rotated key
    let new_ciphertext = crypto_manager.encrypt(class, test_data.as_bytes()).unwrap();
    let decrypted_new = crypto_manager.decrypt(class, &new_ciphertext).unwrap();
    assert_eq!(String::from_utf8(decrypted_new).unwrap(), test_data);
    
    // New and old ciphertexts should be different
    assert_ne!(original_ciphertext.data, new_ciphertext.data);
}

#[test]
fn test_root_rotation_breaks_old_key() {
    let temp_dir = TempDir::new().unwrap();
    let keyset_path = temp_dir.path().join("keyset.json");
    
    // Create first crypto manager and encrypt some data
    let test_data = "Data encrypted with original root key";
    let class = "personal";
    
    let original_ciphertext = {
        let mut crypto_manager = CryptoManager::new(&keyset_path).unwrap();
        crypto_manager.encrypt(class, test_data.as_bytes()).unwrap()
    };
    
    // Rotate root key
    {
        let mut crypto_manager = CryptoManager::new(&keyset_path).unwrap();
        crypto_manager.rotate_root_key().unwrap();
    }
    
    // Create new crypto manager with rotated root key
    let mut new_crypto_manager = CryptoManager::new(&keyset_path).unwrap();
    
    // Original data encrypted with old derived key should not decrypt
    let decrypt_result = new_crypto_manager.decrypt(class, &original_ciphertext);
    assert!(decrypt_result.is_err(), "Old ciphertext should not decrypt after root key rotation");
    
    // New encryption should work
    let new_ciphertext = new_crypto_manager.encrypt(class, test_data.as_bytes()).unwrap();
    let decrypted = new_crypto_manager.decrypt(class, &new_ciphertext).unwrap();
    assert_eq!(String::from_utf8(decrypted).unwrap(), test_data);
}

#[test]
fn test_different_classes_different_keys() {
    let (mut crypto_manager, _temp_dir) = create_test_crypto_manager();
    
    let test_data = "Same data, different classes";
    
    // Encrypt same data with different classes
    let personal_ciphertext = crypto_manager.encrypt("personal", test_data.as_bytes()).unwrap();
    let work_ciphertext = crypto_manager.encrypt("work", test_data.as_bytes()).unwrap();
    
    // Ciphertexts should be different despite same plaintext
    assert_ne!(personal_ciphertext.data, work_ciphertext.data);
    
    // Each should decrypt correctly with its own class
    let personal_decrypted = crypto_manager.decrypt("personal", &personal_ciphertext).unwrap();
    let work_decrypted = crypto_manager.decrypt("work", &work_ciphertext).unwrap();
    
    assert_eq!(String::from_utf8(personal_decrypted).unwrap(), test_data);
    assert_eq!(String::from_utf8(work_decrypted).unwrap(), test_data);
    
    // Cross-class decryption should fail
    let cross_decrypt = crypto_manager.decrypt("work", &personal_ciphertext);
    assert!(cross_decrypt.is_err(), "Should not be able to decrypt with wrong class key");
}

#[test]
fn test_nonce_uniqueness() {
    let (mut crypto_manager, _temp_dir) = create_test_crypto_manager();
    
    let test_data = "Same data for nonce test";
    let class = "personal";
    
    // Encrypt same data multiple times
    let ciphertext1 = crypto_manager.encrypt(class, test_data.as_bytes()).unwrap();
    let ciphertext2 = crypto_manager.encrypt(class, test_data.as_bytes()).unwrap();
    let ciphertext3 = crypto_manager.encrypt(class, test_data.as_bytes()).unwrap();
    
    // Nonces should be unique
    assert_ne!(ciphertext1.nonce, ciphertext2.nonce);
    assert_ne!(ciphertext2.nonce, ciphertext3.nonce);
    assert_ne!(ciphertext1.nonce, ciphertext3.nonce);
    
    // All should decrypt to same plaintext
    let decrypted1 = crypto_manager.decrypt(class, &ciphertext1).unwrap();
    let decrypted2 = crypto_manager.decrypt(class, &ciphertext2).unwrap();
    let decrypted3 = crypto_manager.decrypt(class, &ciphertext3).unwrap();
    
    assert_eq!(decrypted1, decrypted2);
    assert_eq!(decrypted2, decrypted3);
    assert_eq!(String::from_utf8(decrypted1).unwrap(), test_data);
}

#[test]
fn test_empty_data_encryption() {
    let (mut crypto_manager, _temp_dir) = create_test_crypto_manager();
    
    let empty_data = b"";
    let class = "personal";
    
    // Should be able to encrypt empty data
    let ciphertext = crypto_manager.encrypt(class, empty_data).unwrap();
    assert!(!ciphertext.nonce.is_empty());
    
    // Should decrypt back to empty data
    let decrypted = crypto_manager.decrypt(class, &ciphertext).unwrap();
    assert_eq!(decrypted, empty_data);
}

#[test]
fn test_large_data_encryption() {
    let (mut crypto_manager, _temp_dir) = create_test_crypto_manager();
    
    // Create 1MB of test data
    let large_data = "A".repeat(1024 * 1024);
    let class = "personal";
    
    // Should handle large data
    let ciphertext = crypto_manager.encrypt(class, large_data.as_bytes()).unwrap();
    let decrypted = crypto_manager.decrypt(class, &ciphertext).unwrap();
    
    assert_eq!(String::from_utf8(decrypted).unwrap(), large_data);
}

#[test]
fn test_unicode_data_encryption() {
    let (mut crypto_manager, _temp_dir) = create_test_crypto_manager();
    
    let unicode_data = "🔒 Encrypted: 你好世界 🌍 Здравствуй мир 🇺🇳";
    let class = "personal";
    
    let ciphertext = crypto_manager.encrypt(class, unicode_data.as_bytes()).unwrap();
    let decrypted = crypto_manager.decrypt(class, &ciphertext).unwrap();
    
    assert_eq!(String::from_utf8(decrypted).unwrap(), unicode_data);
}

#[test]
fn test_purge_class() {
    let (mut crypto_manager, _temp_dir) = create_test_crypto_manager();
    
    let test_data = "Data to be purged";
    let class = "personal";
    
    // Encrypt some data
    let ciphertext = crypto_manager.encrypt(class, test_data.as_bytes()).unwrap();
    
    // Verify it decrypts
    let decrypted = crypto_manager.decrypt(class, &ciphertext).unwrap();
    assert_eq!(String::from_utf8(decrypted).unwrap(), test_data);
    
    // Purge the class
    crypto_manager.purge_class(class).unwrap();
    
    // Should not be able to decrypt anymore
    let decrypt_result = crypto_manager.decrypt(class, &ciphertext);
    assert!(decrypt_result.is_err(), "Should not be able to decrypt after class purge");
    
    // Should be able to encrypt new data (will create new key)
    let new_ciphertext = crypto_manager.encrypt(class, test_data.as_bytes()).unwrap();
    let new_decrypted = crypto_manager.decrypt(class, &new_ciphertext).unwrap();
    assert_eq!(String::from_utf8(new_decrypted).unwrap(), test_data);
}

#[test]
fn test_keyset_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let keyset_path = temp_dir.path().join("keyset.json");
    
    let test_data = "Data that should persist across manager instances";
    let class = "personal";
    
    // Create first manager and encrypt data
    let ciphertext = {
        let mut crypto_manager = CryptoManager::new(&keyset_path).unwrap();
        crypto_manager.encrypt(class, test_data.as_bytes()).unwrap()
    };
    
    // Create second manager and decrypt data
    {
        let mut crypto_manager = CryptoManager::new(&keyset_path).unwrap();
        let decrypted = crypto_manager.decrypt(class, &ciphertext).unwrap();
        assert_eq!(String::from_utf8(decrypted).unwrap(), test_data);
    }
    
    // Verify keyset file exists
    assert!(keyset_path.exists());
}

#[test]
fn test_root_key_generation() {
    let root_key1 = RootKey::new().unwrap();
    let root_key2 = RootKey::new().unwrap();
    
    // Different root keys should generate different class keys
    let class_key1 = root_key1.derive_class_key("personal").unwrap();
    let class_key2 = root_key2.derive_class_key("personal").unwrap();
    
    assert_ne!(class_key1.as_bytes(), class_key2.as_bytes());
}

#[test]
fn test_deterministic_class_key_derivation() {
    let root_key = RootKey::new().unwrap();
    
    // Same root key and class should always produce same class key
    let class_key1 = root_key.derive_class_key("personal").unwrap();
    let class_key2 = root_key.derive_class_key("personal").unwrap();
    
    assert_eq!(class_key1.as_bytes(), class_key2.as_bytes());
    
    // Different classes should produce different keys
    let work_key = root_key.derive_class_key("work").unwrap();
    assert_ne!(class_key1.as_bytes(), work_key.as_bytes());
}

#[test]
fn test_malformed_ciphertext() {
    let (mut crypto_manager, _temp_dir) = create_test_crypto_manager();
    
    let class = "personal";
    
    // Test with invalid nonce length
    let bad_ciphertext = mimir_core::crypto::Ciphertext {
        data: vec![1, 2, 3, 4],
        nonce: vec![1, 2, 3], // Wrong length
    };
    
    let result = crypto_manager.decrypt(class, &bad_ciphertext);
    assert!(result.is_err());
    
    // Test with corrupted data
    let original_data = "Valid data";
    let mut ciphertext = crypto_manager.encrypt(class, original_data.as_bytes()).unwrap();
    
    // Corrupt the ciphertext
    if !ciphertext.data.is_empty() {
        ciphertext.data[0] = ciphertext.data[0].wrapping_add(1);
    }
    
    let result = crypto_manager.decrypt(class, &ciphertext);
    assert!(result.is_err(), "Corrupted ciphertext should not decrypt");
}

#[test]
fn test_concurrent_encryption() {
    use std::sync::{Arc, Mutex};
    use std::thread;
    
    let (crypto_manager, _temp_dir) = create_test_crypto_manager();
    let crypto_manager = Arc::new(Mutex::new(crypto_manager));
    
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let crypto_manager = Arc::clone(&crypto_manager);
            thread::spawn(move || {
                let test_data = format!("Thread {} data", i);
                let class = "personal";
                
                let mut manager = crypto_manager.lock().unwrap();
                let ciphertext = manager.encrypt(class, test_data.as_bytes()).unwrap();
                let decrypted = manager.decrypt(class, &ciphertext).unwrap();
                
                assert_eq!(String::from_utf8(decrypted).unwrap(), test_data);
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn test_encryption_performance() {
        let (mut crypto_manager, _temp_dir) = create_test_crypto_manager();
        
        let test_data = "Performance test data";
        let class = "personal";
        let iterations = 1000;
        
        let start = Instant::now();
        
        for _i in 0..iterations {
            let ciphertext = crypto_manager.encrypt(class, test_data.as_bytes()).unwrap();
            let _decrypted = crypto_manager.decrypt(class, &ciphertext).unwrap();
        }
        
        let duration = start.elapsed();
        println!("Encrypted/decrypted {} times in {:?}", iterations, duration);
        
        // Should complete reasonably quickly (adjust threshold as needed)
        assert!(duration.as_millis() < 5000, "Encryption should be reasonably fast");
    }
} 