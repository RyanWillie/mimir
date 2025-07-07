use std::process::Command;


fn main() {
    println!("Testing Mimir build process...");
    
    // Test 1: Build with skip flag
    println!("\n1. Testing build with MIMIR_SKIP_MODEL_DL=1");
    let status = Command::new("cargo")
        .args(["check"])
        .env("MIMIR_SKIP_MODEL_DL", "1")
        .current_dir("../crates/mimir")
        .status()
        .expect("Failed to run cargo check");
    
    if status.success() {
        println!("✓ Build with skip flag successful");
    } else {
        println!("✗ Build with skip flag failed");
        std::process::exit(1);
    }
    
    // Test 2: Check if build script compiles
    println!("\n2. Testing build script compilation");
    let status = Command::new("cargo")
        .args(["check"])
        .current_dir("../crates/mimir")
        .status()
        .expect("Failed to run cargo check");
    
    if status.success() {
        println!("✓ Build script compilation successful");
    } else {
        println!("✗ Build script compilation failed");
        std::process::exit(1);
    }
    
    // Test 3: Check if checksum calculator compiles
    println!("\n3. Testing checksum calculator compilation");
    let status = Command::new("cargo")
        .args(["check", "--bin", "calculate_checksums"])
        .current_dir(".")
        .status()
        .expect("Failed to run cargo check for checksum calculator");
    
    if status.success() {
        println!("✓ Checksum calculator compilation successful");
    } else {
        println!("✗ Checksum calculator compilation failed");
        std::process::exit(1);
    }
    
    println!("\n✓ All build tests passed!");
    println!("\nTo test the full download process:");
    println!("1. cd ../crates/mimir");
    println!("2. cargo build  # This will download the model files");
    println!("3. cd ../../scripts");
    println!("4. cargo run --bin calculate_checksums  # To get real checksums");
} 