// Print the SQLCipher passphrase for a Mimir-encrypted database
// Usage: cargo run --bin print_sqlcipher_passphrase -- <keyset_path> <password>

use mimir_core::crypto::CryptoManager;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <keyset_path> <password>", args[0]);
        std::process::exit(1);
    }
    let keyset_path = &args[1];
    let password = &args[2];
    match CryptoManager::with_password(keyset_path, password) {
        Ok(crypto) => {
            let db_key_bytes = crypto.get_db_key_bytes();
            println!("SQLCipher raw key (hex): 0x{}", hex::encode(db_key_bytes));
            println!("\nIn DB Browser for SQLite, use this hex string as the passphrase, including the '0x' prefix, and select the 'Raw Key' option.");
        }
        Err(e) => {
            eprintln!("Failed to initialize CryptoManager: {}", e);
            std::process::exit(1);
        }
    }
} 