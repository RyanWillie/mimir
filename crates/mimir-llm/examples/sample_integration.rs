//! Sample integration using MistralRSService for LLM inference
//! 
//! This example demonstrates how to use the MistralRSService for running Gemma models locally.
//! It supports both GGUF and SafeTensors formats and provides a simple CLI interface.

use anyhow::Result;
use clap::Parser;
use mimir_llm::{LlmConfig, ModelType, QuantizationType, MistralRSService};
use std::path::PathBuf;
use std::time::Instant;
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The prompt to generate from
    #[arg(long)]
    prompt: String,

    /// Path to the model directory or GGUF file
    #[arg(long)]
    model_path: Option<PathBuf>,

    /// Model type to use
    #[arg(long, default_value = "gemma-3-1b-it")]
    model_type: ModelType,

    /// Maximum number of tokens to generate
    #[arg(long, default_value = "100")]
    max_tokens: usize,

    /// Temperature for sampling (0.0 = deterministic, 1.0 = creative)
    #[arg(long, default_value = "0.7")]
    temperature: f64,

    /// Top-p nucleus sampling
    #[arg(long, default_value = "0.9")]
    top_p: f64,

    /// Use GGUF format instead of SafeTensors
    #[arg(long)]
    gguf: bool,

    /// Enable verbose logging
    #[arg(long)]
    verbose: bool,

    /// Quantization type (only for non-GGUF models)
    #[arg(long, default_value = "q4-0")]
    quantization: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging (only if not already initialized)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(if args.verbose {
            "debug,mistralrs=info"
        } else {
            "info,mistralrs=warn"
        })
        .try_init();

    info!("🚀 MistralRSService Sample Integration");
    info!("Model type: {:?}", args.model_type);
    info!("Model path: {:?}", args.model_path);
    info!("Prompt: {}", args.prompt);

    // Determine model path
    let model_path = args.model_path.unwrap_or_else(|| {
        get_default_model_path(&args.model_type, args.gguf)
    });

    info!("Using model path: {}", model_path.display());

    // Check if model path exists
    if !model_path.exists() {
        return Err(anyhow::anyhow!(
            "Model path does not exist: {}. Please provide a valid model path with --model-path",
            model_path.display()
        ));
    }

    // Parse quantization type
    let quantization = match args.quantization.as_str() {
        "q4-0" => QuantizationType::Q4_0,
        "q4-1" => QuantizationType::Q4_1,
        "q8-0" => QuantizationType::Q8_0,
        "q8-1" => QuantizationType::Q8_1,
        "q4k" => QuantizationType::Q4K,
        _ => {
            warn!("Unknown quantization type: {}, using Q4_0", args.quantization);
            QuantizationType::Q4_0
        }
    };

    // Create configuration
    let config = LlmConfig::new()
        .with_model_path(model_path)
        .with_model_type(args.model_type)
        .with_gguf(args.gguf)
        .with_quantization(quantization)
        .with_temperature(args.temperature)
        .with_top_p(args.top_p)
        .with_max_tokens(args.max_tokens);

    // Create service
    let mut service = MistralRSService::new(config);

    // Load model
    info!("Loading model...");
    service.load_model().await?;
    info!("✅ Model loaded successfully!");

    // Generate response
    info!("🤖 Generating response...");
    let start_time = Instant::now();
    let response = service.generate_response(&args.prompt).await?;
    let duration = start_time.elapsed();

    // Print response
    println!("\n🤖 Response (generated in {:.2?}):", duration);
    println!("{}", response);

    Ok(())
}

fn get_default_model_path(model_type: &ModelType, gguf: bool) -> PathBuf {
    let base_path = PathBuf::from("/Users/ryanwilliamson/Library/Application Support/Mimir/models/");
    
    match model_type {
        ModelType::Gemma2_2bIt => {
            if gguf {
                base_path.join("gemma-2-2b-it.gguf") // hypothetical
            } else {
                base_path.join("gemma-2-2b-it")
            }
        }
        ModelType::Gemma3_1bIt => {
            if gguf {
                base_path.join("gemma-3-1b-it-qat-q4_0.gguf")
            } else {
                base_path.join("gemma-3-1b-it-standard")
            }
        }
        ModelType::Gemma2_9bIt => {
            if gguf {
                base_path.join("gemma-2-9b-it.gguf") // hypothetical
            } else {
                base_path.join("gemma-2-9b-it")
            }
        }
        ModelType::Gemma7bIt => {
            if gguf {
                base_path.join("gemma-7b-it.gguf") // hypothetical
            } else {
                base_path.join("gemma-7b-it")
            }
        }
        ModelType::Qwen06b => {
            if gguf {
                base_path.join("qwen-0.6b.gguf")
            } else {
                base_path.join("qwen-0.6b")
            }
        }
        ModelType::Qwen15b => {
            if gguf {
                base_path.join("qwen-1.5b.gguf")
            } else {
                base_path.join("qwen-1.5b")
            }
        }
        ModelType::Qwen3b => {
            if gguf {
                base_path.join("qwen-3b.gguf")
            } else {
                base_path.join("qwen-3b")
            }
        }
        ModelType::Llama27b => {
            if gguf {
                base_path.join("llama-2-7b.gguf")
            } else {
                base_path.join("llama-2-7b")
            }
        }
        ModelType::Llama38b => {
            if gguf {
                base_path.join("llama-3-8b.gguf")
            } else {
                base_path.join("llama-3-8b")
            }
        }
        ModelType::Custom => {
            if gguf {
                base_path.join("custom-model.gguf")
            } else {
                base_path.join("custom-model")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model_paths() {
        let base_path = PathBuf::from("/Users/ryanwilliamson/Library/Application Support/Mimir/models/");
        
        assert_eq!(
            get_default_model_path(&ModelType::Gemma3_1bIt, false),
            base_path.join("gemma-3-1b-it-standard")
        );
        
        assert_eq!(
            get_default_model_path(&ModelType::Gemma3_1bIt, true),
            base_path.join("gemma-3-1b-it-qat-q4_0.gguf")
        );
    }
}