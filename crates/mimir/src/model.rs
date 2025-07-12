use std::fs;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use reqwest::Client;
use mimir_core::get_default_app_dir;

// BGE model constants
const MODEL_ONNX: &str = "model-int8.onnx";
const TOKENIZER: &str = "tokenizer.json";
const VOCAB: &str = "vocab.txt";

const SHA_ONNX: &str = "828e1496d7fabb79cfa4dcd84fa38625c0d3d21da474a00f08db0f559940cf35";
const SHA_TOKENIZER: &str = "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66";
const SHA_VOCAB: &str = "07eced375cec144d27c900241f3e339478dec958f92fddbc551f295c992038a3";

const MODEL_BASE_URL: &str = "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main";

// Gemma3 model constants
const GEMMA3_MODEL_FILE: &str = "gemma-3-1b-it-qat-q4_0.gguf";
const GEMMA3_BASE_URL: &str = "https://huggingface.co/google/gemma-3-1b-it-qat-q4_0-gguf/resolve/main";
// SHA256 checksum will be calculated and updated after first download
const GEMMA3_SHA: &str = ""; // Will be filled after first download

pub async fn ensure_model_files() -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let model_dir = get_default_app_dir().join("models");
    if !model_dir.exists() {
        fs::create_dir_all(&model_dir).map_err(|e| format!("Failed to create model dir: {}", e))?;
    }
    let model_path = model_dir.join(MODEL_ONNX);
    let tokenizer_path = model_dir.join(TOKENIZER);
    let vocab_path = model_dir.join(VOCAB);

    let client = Client::new();
    download_if_missing(&client, &model_path, &format!("{}/onnx/model.onnx", MODEL_BASE_URL), MODEL_ONNX).await?;
    download_if_missing(&client, &tokenizer_path, &format!("{}/tokenizer.json", MODEL_BASE_URL), TOKENIZER).await?;
    download_if_missing(&client, &vocab_path, &format!("{}/vocab.txt", MODEL_BASE_URL), VOCAB).await?;

    verify_sha256(&model_path, SHA_ONNX)?;
    verify_sha256(&tokenizer_path, SHA_TOKENIZER)?;
    verify_sha256(&vocab_path, SHA_VOCAB)?;

    Ok((model_path, tokenizer_path, vocab_path))
}

/// Ensure Gemma3 model is downloaded and available
pub async fn ensure_gemma3_model() -> Result<PathBuf, String> {
    let model_dir = get_default_app_dir().join("models");
    if !model_dir.exists() {
        fs::create_dir_all(&model_dir).map_err(|e| format!("Failed to create model dir: {}", e))?;
    }
    
    let gemma3_path = model_dir.join(GEMMA3_MODEL_FILE);
    
    let client = Client::new();
    download_if_missing(&client, &gemma3_path, &format!("{}/{}", GEMMA3_BASE_URL, GEMMA3_MODEL_FILE), GEMMA3_MODEL_FILE).await?;
    
    // Skip SHA verification for now since we don't have the checksum yet
    // TODO: Add SHA verification after first download
    if !GEMMA3_SHA.is_empty() {
        verify_sha256(&gemma3_path, GEMMA3_SHA)?;
    }
    
    Ok(gemma3_path)
}

async fn download_if_missing(client: &Client, path: &Path, url: &str, name: &str) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    eprintln!("Downloading {} from {}...", name, url);
    let resp = client.get(url).send().await.map_err(|e| format!("Failed to GET {}: {}", url, e))?;
    let bytes = resp.bytes().await.map_err(|e| format!("Failed to read bytes for {}: {}", url, e))?;
    fs::write(path, &bytes).map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let data = fs::read(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    let actual = hex::encode(result);
    if actual != expected {
        return Err(format!("Checksum mismatch for {}: expected {}, got {}", path.display(), expected, actual));
    }
    Ok(())
} 