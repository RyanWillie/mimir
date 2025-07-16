# Mimir LLM

Large Language Model integration for Mimir AI Memory Vault using MistralRS.

## Features

- **Multiple Model Support**: Gemma3, Qwen, Llama, and custom models
- **Automatic Builder Selection**: VisionModelBuilder for Gemma3, TextModelBuilder for others
- **Format Support**: Both GGUF and SafeTensors formats
- **Memory Processing**: Extract, summarize, resolve conflicts, and classify memories
- **Easy Model Swapping**: Change models by simply updating the ModelType

## Quick Start

```rust
use mimir_llm::{LlmConfig, ModelType, QuantizationType, MistralRSService};

// Create configuration for Gemma3 (uses VisionModelBuilder automatically)
let config = LlmConfig::new()
    .with_model_path("/path/to/gemma-3-1b-it-standard")
    .with_model_type(ModelType::Gemma3_1bIt)
    .with_gguf(false)
    .with_quantization(QuantizationType::Q4_0)
    .with_temperature(0.7)
    .with_max_tokens(200);

// Create service and load model
let mut service = MistralRSService::new(config);
service.load_model().await?;

// Generate response
let response = service.generate_response("Hello, how are you?").await?;
println!("Response: {}", response);
```

## Model Swapping

Easily swap between different models by changing the ModelType:

```rust
// Gemma3 model (uses VisionModelBuilder)
let gemma_config = LlmConfig::new()
    .with_model_type(ModelType::Gemma3_1bIt)
    .with_model_path("/path/to/gemma-3-1b-it");

// Qwen model (uses TextModelBuilder)
let qwen_config = LlmConfig::new()
    .with_model_type(ModelType::Qwen06b)
    .with_model_path("/path/to/qwen-0.6b");

// Custom model ID
let custom_config = LlmConfig::new()
    .with_model_id("microsoft/DialoGPT-medium".to_string())
    .with_model_path("/path/to/custom-model");
```

## Memory Processing

The service provides specialized methods for memory processing tasks:

```rust
// Extract memories from conversation
let memories = service.extract_memories(conversation_text).await?;

// Summarize a memory
let summary = service.summarize_memory(long_memory, 50).await?;

// Classify a memory
let class = service.classify_memory("Meeting tomorrow at 2pm").await?;

// Resolve conflicts between similar memories
let resolution = service.resolve_conflict(existing_memory, new_memory, 0.85).await?;

// Summarize search results for relevance and reduced token output
let query = "What meetings do I have scheduled?";
let results = vec![
    "Meeting with John tomorrow at 3pm".to_string(),
    "Team standup every Monday at 9am".to_string(),
    "Dentist appointment next Friday".to_string(),
];
let summary = service.summarize_search_results(query, &results).await?;
```

## Supported Models

### Gemma Models (VisionModelBuilder)
- `ModelType::Gemma3_1bIt` - google/gemma-3-1b-it
- `ModelType::Gemma2_9bIt` - google/gemma-2-9b-it
- `ModelType::Gemma7bIt` - google/gemma-7b-it

### Qwen Models (TextModelBuilder)
- `ModelType::Qwen06b` - Qwen/Qwen2.5-0.6B-Instruct
- `ModelType::Qwen15b` - Qwen/Qwen2.5-1.5B-Instruct
- `ModelType::Qwen3b` - Qwen/Qwen2.5-3B-Instruct

### Llama Models (TextModelBuilder)
- `ModelType::Llama27b` - meta-llama/Llama-2-7b-chat-hf
- `ModelType::Llama38b` - meta-llama/Llama-3-8b-instruct

### Custom Models
Use `with_model_id()` to specify any HuggingFace model ID.

## Examples

Run the examples to see the service in action:

```bash
# Basic model integration
cargo run --example sample_integration -- --prompt "Hello, world!"

# Memory processing tasks
cargo run --example memory_processing

# Model swapping demonstration
cargo run --example model_swapping

# Simple Gemma3 test
cargo run --example gemma_integration

# Search result summarization
cargo run --example search_summarization
```

## Configuration

### LlmConfig Options

- `model_path`: Path to model file/directory
- `model_type`: Predefined model type (optional)
- `model_id`: Custom HuggingFace model ID (overrides model_type)
- `use_gguf`: Use GGUF format instead of SafeTensors
- `quantization`: Quantization type for SafeTensors models
- `temperature`: Sampling temperature (0.0-1.0)
- `top_p`: Top-p nucleus sampling
- `max_tokens`: Maximum tokens to generate
- `repeat_penalty`: Repetition penalty

### Model Type Helper Methods

```rust
let model_type = ModelType::Gemma3_1bIt;

println!("Default ID: {}", model_type.default_model_id());
println!("Requires Vision Builder: {}", model_type.requires_vision_builder());
println!("Is Gemma: {}", model_type.is_gemma());
println!("Is Qwen: {}", model_type.is_qwen());
println!("Is Llama: {}", model_type.is_llama());
```

## Error Handling

The service uses a unified error type `LlmError` that covers:
- Model loading errors
- Inference errors
- Configuration errors
- Serialization errors
- Invalid input errors

## Dependencies

- `mistralrs`: LLM inference engine
- `tokio`: Async runtime
- `serde`: Serialization
- `tracing`: Logging

## License

See the main Mimir project license. 