//! Configuration for LLM model and inference settings using MistralRS

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for LLM model using MistralRS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Path to the model (GGUF file or SafeTensors directory)
    pub model_path: PathBuf,
    
    /// Model type for automatic configuration
    pub model_type: Option<ModelType>,
    
    /// Model ID for HuggingFace models (overrides model_type if set)
    pub model_id: Option<String>,
    
    /// Use GGUF format instead of SafeTensors
    pub use_gguf: bool,
    
    /// Quantization type for SafeTensors models
    pub quantization: QuantizationType,
    
    /// Inference parameters
    pub inference: InferenceConfig,
    
    /// Device configuration
    pub device: DeviceConfig,
}

/// Supported model types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum ModelType {
    #[value(name = "gemma-2-2b-it")]
    Gemma2_2bIt,
    #[value(name = "gemma-3-1b-it")]
    Gemma3_1bIt,
    #[value(name = "gemma-2-9b-it")]
    Gemma2_9bIt,
    #[value(name = "gemma-7b-it")]
    Gemma7bIt,
    #[value(name = "qwen-0.6b")]
    Qwen06b,
    #[value(name = "qwen-1.5b")]
    Qwen15b,
    #[value(name = "qwen-3b")]
    Qwen3b,
    #[value(name = "llama-2-7b")]
    Llama27b,
    #[value(name = "llama-3-8b")]
    Llama38b,
    #[value(name = "custom")]
    Custom,
}

impl ModelType {
    /// Get the default model ID for this model type
    pub fn default_model_id(&self) -> &'static str {
        match self {
            ModelType::Gemma2_2bIt => "google/gemma-2-2b-it",
            ModelType::Gemma3_1bIt => "google/gemma-3-1b-it",
            ModelType::Gemma2_9bIt => "google/gemma-2-9b-it",
            ModelType::Gemma7bIt => "google/gemma-7b-it",
            ModelType::Qwen06b => "Qwen/Qwen2.5-0.6B-Instruct",
            ModelType::Qwen15b => "Qwen/Qwen2.5-1.5B-Instruct",
            ModelType::Qwen3b => "Qwen/Qwen2.5-3B-Instruct",
            ModelType::Llama27b => "meta-llama/Llama-2-7b-chat-hf",
            ModelType::Llama38b => "meta-llama/Llama-3-8b-instruct",
            ModelType::Custom => "custom",
        }
    }

    /// Check if this model type requires VisionModelBuilder (Gemma3 models)
    pub fn requires_vision_builder(&self) -> bool {
        matches!(self, ModelType::Gemma3_1bIt | ModelType::Gemma2_9bIt | ModelType::Gemma7bIt)
    }

    /// Check if this model type is a Gemma model
    pub fn is_gemma(&self) -> bool {
        matches!(self, ModelType::Gemma2_2bIt | ModelType::Gemma3_1bIt | ModelType::Gemma2_9bIt | ModelType::Gemma7bIt)
    }

    /// Check if this model type is a Qwen model
    pub fn is_qwen(&self) -> bool {
        matches!(self, ModelType::Qwen06b | ModelType::Qwen15b | ModelType::Qwen3b)
    }

    /// Check if this model type is a Llama model
    pub fn is_llama(&self) -> bool {
        matches!(self, ModelType::Llama27b | ModelType::Llama38b)
    }
}

/// Quantization types supported by MistralRS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantizationType {
    Q4_0,
    Q4_1,
    Q8_0,
    Q8_1,
    Q4K,
    None,
}

/// Inference configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Temperature for sampling (0.0 = deterministic, 1.0 = creative)
    pub temperature: f64,
    
    /// Top-p nucleus sampling
    pub top_p: f64,
    
    /// Maximum tokens to generate
    pub max_tokens: usize,
    
    /// Repetition penalty
    pub repeat_penalty: f32,
    
    /// Stop sequences
    pub stop_sequences: Vec<String>,
}

/// Device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Enable Metal acceleration (macOS)
    pub use_metal: bool,
    
    /// Enable CUDA acceleration
    pub use_cuda: bool,
    
    /// Number of CPU threads
    pub cpu_threads: Option<usize>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model_path: get_default_model_path(),
            model_type: Some(ModelType::Gemma3_1bIt),
            model_id: None,
            use_gguf: true,
            quantization: QuantizationType::Q4_0,
            inference: InferenceConfig::default(),
            device: DeviceConfig::default(),
        }
    }
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 1024,
            repeat_penalty: 1.1,
            stop_sequences: vec![
                "<|end_of_text|>".to_string(),
                "<|eot_id|>".to_string(),
            ],
        }
    }
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            use_metal: cfg!(target_os = "macos"),
            use_cuda: false,
            cpu_threads: None,
        }
    }
}

impl LlmConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the model path
    pub fn with_model_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.model_path = path.into();
        self
    }
    
    /// Set the model type
    pub fn with_model_type(mut self, model_type: ModelType) -> Self {
        self.model_type = Some(model_type);
        self
    }
    
    /// Set the model ID
    pub fn with_model_id(mut self, model_id: String) -> Self {
        self.model_id = Some(model_id);
        self
    }
    
    /// Set to use GGUF format
    pub fn with_gguf(mut self, use_gguf: bool) -> Self {
        self.use_gguf = use_gguf;
        self
    }
    
    /// Set quantization type
    pub fn with_quantization(mut self, quantization: QuantizationType) -> Self {
        self.quantization = quantization;
        self
    }
    
    /// Set the temperature
    pub fn with_temperature(mut self, temp: f64) -> Self {
        self.inference.temperature = temp;
        self
    }
    
    /// Set the top-p value
    pub fn with_top_p(mut self, top_p: f64) -> Self {
        self.inference.top_p = top_p;
        self
    }
    
    /// Set the maximum tokens to generate
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.inference.max_tokens = max_tokens;
        self
    }
    
    /// Set repetition penalty
    pub fn with_repeat_penalty(mut self, penalty: f32) -> Self {
        self.inference.repeat_penalty = penalty;
        self
    }
    
    /// Enable Metal acceleration
    pub fn with_metal(mut self, enable: bool) -> Self {
        self.device.use_metal = enable;
        self
    }
    
    /// Enable CUDA acceleration
    pub fn with_cuda(mut self, enable: bool) -> Self {
        self.device.use_cuda = enable;
        self
    }
    
    /// Set CPU threads
    pub fn with_cpu_threads(mut self, threads: usize) -> Self {
        self.device.cpu_threads = Some(threads);
        self
    }
}

/// Get the default model path
fn get_default_model_path() -> PathBuf {
    mimir_core::get_default_app_dir()
        .join("models")
        .join("gemma-3-1b-it-qat-q4_0.gguf")
} 