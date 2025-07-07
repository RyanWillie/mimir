use std::path::Path;
use std::fs;
use sha2::{Sha256, Digest};
use hex;

fn main() {
    let assets_dir = Path::new("crates/mimir/assets/bge-small-en-int8");
    
    let files = [
        ("model-int8.onnx", "SHA_ONNX"),
        ("tokenizer.json", "SHA_TOKENIZER"),
        ("vocab.txt", "SHA_VOCAB"),
    ];

    println!("Calculating SHA-256 checksums for model files:");
    println!();

    for (filename, const_name) in files.iter() {
        let file_path = assets_dir.join(filename);
        
        if !file_path.exists() {
            println!("{}: File not found: {}", const_name, file_path.display());
            continue;
        }

        let content = match fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                println!("{}: Failed to read file: {}", const_name, e);
                continue;
            }
        };

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let result = hasher.finalize();
        let checksum = hex::encode(result);

        println!("const {}: &str = \"{}\";", const_name, checksum);
    }

    println!();
    println!("Copy these constants to crates/mimir/build.rs");
} 