//! Configuration for Gemma3 model and inference settings

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for Gemma3 model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemmaConfig {
    /// Path to the Gemma3 model file (GGUF format)
    pub model_path: PathBuf,
    
    /// Number of threads for inference
    pub n_threads: Option<usize>,
    
    /// Context length (number of tokens)
    pub context_length: usize,
    
    /// Inference parameters
    pub inference: InferenceConfig,
    
    /// Memory management settings
    pub memory: MemoryConfig,
}

/// Inference configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Temperature for sampling (0.0 = deterministic, 1.0 = creative)
    pub temperature: f32,
    
    /// Top-p nucleus sampling
    pub top_p: f32,
    
    /// Top-k sampling
    pub top_k: i32,
    
    /// Repetition penalty
    pub repeat_penalty: f32,
    
    /// Maximum tokens to generate
    pub max_tokens: usize,
    
    /// Stop sequences
    pub stop_sequences: Vec<String>,
}

/// Memory management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Use memory mapping for model loading
    pub use_mmap: bool,
    
    /// Lock memory pages (prevents swapping)
    pub use_mlock: bool,
    
    /// Number of GPU layers to offload
    pub n_gpu_layers: i32,
    
    /// Main GPU device
    pub main_gpu: i32,
}

impl Default for GemmaConfig {
    fn default() -> Self {
        Self {
            model_path: get_default_gemma3_path(),
            n_threads: None, // Use system default
            context_length: 8192,
            inference: InferenceConfig::default(),
            memory: MemoryConfig::default(),
        }
    }
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            repeat_penalty: 1.1,
            max_tokens: 1024,
            stop_sequences: vec![
                "<|end_of_text|>".to_string(),
                "<|eot_id|>".to_string(),
            ],
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            use_mmap: true,
            use_mlock: false,
            n_gpu_layers: 0, // CPU-only by default
            main_gpu: 0,
        }
    }
}

impl GemmaConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the model path
    pub fn with_model_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.model_path = path.into();
        self
    }
    
    /// Set the number of threads
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.n_threads = Some(threads);
        self
    }
    
    /// Set the context length
    pub fn with_context_length(mut self, length: usize) -> Self {
        self.context_length = length;
        self
    }
    
    /// Set the temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.inference.temperature = temp;
        self
    }
    
    /// Set the top-p value
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.inference.top_p = top_p;
        self
    }
    
    /// Set the top-k value
    pub fn with_top_k(mut self, top_k: i32) -> Self {
        self.inference.top_k = top_k;
        self
    }
    
    /// Set the maximum tokens to generate
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.inference.max_tokens = max_tokens;
        self
    }
    
    /// Set the number of GPU layers
    pub fn with_gpu_layers(mut self, layers: i32) -> Self {
        self.memory.n_gpu_layers = layers;
        self
    }
    
    /// Enable memory mapping
    pub fn with_mmap(mut self, enable: bool) -> Self {
        self.memory.use_mmap = enable;
        self
    }
    
    /// Enable memory locking
    pub fn with_mlock(mut self, enable: bool) -> Self {
        self.memory.use_mlock = enable;
        self
    }
}

/// Get the default Gemma3 model path
fn get_default_gemma3_path() -> PathBuf {
    mimir_core::get_default_app_dir()
        .join("models")
        .join("gemma-3-1b-it-qat-q4_0.gguf")
} 