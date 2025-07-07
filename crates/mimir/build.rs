use std::env;
use std::path::Path;
use std::process::{Command, Stdio};
use std::fs;


// SHA-256 checksums for the model files
// TODO: Replace with actual checksums after first download
const SHA_ONNX: &str = "828e1496d7fabb79cfa4dcd84fa38625c0d3d21da474a00f08db0f559940cf35";
const SHA_TOKENIZER: &str = "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66";
const SHA_VOCAB: &str = "07eced375cec144d27c900241f3e339478dec958f92fddbc551f295c992038a3";

const MODEL_BASE_URL: &str = "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main";
const ASSETS_DIR: &str = "assets/bge-small-en-int8";

fn main() {
    // Check if model download should be skipped
    if env::var("MIMIR_SKIP_MODEL_DL").is_ok() {
        println!("cargo:warning=Model not present; skipping fetch");
        return;
    }

    let assets_path = Path::new(ASSETS_DIR);
    let model_path = assets_path.join("model-int8.onnx");
    let tokenizer_path = assets_path.join("tokenizer.json");
    let vocab_path = assets_path.join("vocab.txt");

    // Check if all required files already exist
    if model_path.exists() && tokenizer_path.exists() && vocab_path.exists() {
        println!("cargo:rerun-if-changed={}", model_path.display());
        println!("cargo:rerun-if-changed={}", tokenizer_path.display());
        println!("cargo:rerun-if-changed={}", vocab_path.display());
        println!("cargo:warning=Model files already exist, skipping download");
        return;
    }

    println!("cargo:warning=Downloading BGE-small-en model files...");

    // Create assets directory
    if !assets_path.exists() {
        fs::create_dir_all(assets_path).expect("Failed to create assets directory");
        println!("cargo:warning=Created assets directory: {}", assets_path.display());
    }

    // Download model files
    download_file(
        &format!("{}/onnx/model.onnx", MODEL_BASE_URL),
        &model_path,
        "model.onnx"
    );

    download_file(
        &format!("{}/tokenizer.json", MODEL_BASE_URL),
        &tokenizer_path,
        "tokenizer.json"
    );

    download_file(
        &format!("{}/vocab.txt", MODEL_BASE_URL),
        &vocab_path,
        "vocab.txt"
    );

    // Quantize model to int8 if needed
    quantize_model(&model_path);

    // Verify checksums
    verify_checksums(&model_path, &tokenizer_path, &vocab_path);

    println!("cargo:warning=Model files downloaded and verified successfully");
}

fn download_file(url: &str, output_path: &Path, filename: &str) {
    if output_path.exists() {
        println!("cargo:warning=File already exists: {}", filename);
        return;
    }

    println!("cargo:warning=Downloading {} from {}...", filename, url);

    let status = if cfg!(target_os = "windows") {
        // Try curl first, fallback to PowerShell
        let curl_result = Command::new("curl")
            .args(["-L", "-o", output_path.to_str().unwrap(), url])
            .status();

        match curl_result {
            Ok(exit_status) if exit_status.success() => Ok(exit_status),
            _ => {
                // PowerShell fallback
                let ps_script = format!(
                    "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                    url, output_path.to_str().unwrap()
                );
                Command::new("powershell")
                    .args(["-Command", &ps_script])
                    .status()
            }
        }
    } else {
        // Unix-like systems
        Command::new("curl")
            .args(["-L", "-o", output_path.to_str().unwrap(), url])
            .status()
    };

    match status {
        Ok(exit_status) if exit_status.success() => {
            println!("cargo:warning=Successfully downloaded {}", filename);
        }
        Ok(exit_status) => {
            panic!("Failed to download {}: exit code {}", filename, exit_status);
        }
        Err(e) => {
            panic!("Failed to download {}: {}", filename, e);
        }
    }
}

fn quantize_model(model_path: &Path) {
    // Check if model is already int8 quantized
    if model_path.to_str().unwrap().contains("int8") {
        return;
    }

    println!("cargo:warning=Quantizing model to int8...");

    // Check if optimum-cli is available
    let optimum_check = Command::new("optimum-cli")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if optimum_check.is_err() || !optimum_check.unwrap().success() {
        println!("cargo:warning=optimum-cli not found, skipping quantization");
        return;
    }

    let output_path = model_path.parent().unwrap().join("model-int8.onnx");
    
    let status = Command::new("optimum-cli")
        .args([
            "export", "onnx",
            "--quantization", "int8",
            "--model", model_path.to_str().unwrap(),
            "--output", output_path.to_str().unwrap()
        ])
        .status();

    match status {
        Ok(exit_status) if exit_status.success() => {
            // Replace original model with quantized version
            fs::rename(&output_path, model_path).expect("Failed to replace model with quantized version");
            println!("cargo:warning=Model quantized successfully");
        }
        Ok(exit_status) => {
            println!("cargo:warning=Quantization failed: exit code {}", exit_status);
        }
        Err(e) => {
            println!("cargo:warning=Failed to run quantization: {}", e);
        }
    }
}

fn verify_checksums(model_path: &Path, tokenizer_path: &Path, vocab_path: &Path) {
    println!("cargo:warning=Verifying checksums...");

    // Calculate SHA-256 checksums
    let model_sha = calculate_sha256(model_path);
    let tokenizer_sha = calculate_sha256(tokenizer_path);
    let vocab_sha = calculate_sha256(vocab_path);

    // Skip verification if using placeholder checksums
    if SHA_ONNX == "0000000000000000000000000000000000000000000000000000000000000000" {
        println!("cargo:warning=Skipping checksum verification (placeholder checksums)");
        println!("cargo:warning=Model SHA: {}", model_sha);
        println!("cargo:warning=Tokenizer SHA: {}", tokenizer_sha);
        println!("cargo:warning=Vocab SHA: {}", vocab_sha);
        return;
    }

    // Verify checksums
    if model_sha != SHA_ONNX {
        panic!("Model checksum mismatch: expected {}, got {}", SHA_ONNX, model_sha);
    }

    if tokenizer_sha != SHA_TOKENIZER {
        panic!("Tokenizer checksum mismatch: expected {}, got {}", SHA_TOKENIZER, tokenizer_sha);
    }

    if vocab_sha != SHA_VOCAB {
        panic!("Vocab checksum mismatch: expected {}, got {}", SHA_VOCAB, vocab_sha);
    }

    println!("cargo:warning=All checksums verified successfully");
}

fn calculate_sha256(file_path: &Path) -> String {
    use sha2::{Sha256, Digest};
    
    let content = fs::read(file_path).expect("Failed to read file for checksum calculation");
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    
    hex::encode(result)
} 