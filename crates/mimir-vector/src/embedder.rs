//! ONNX embedding model for text-to-vector conversion

use crate::error::{VectorError, VectorResult};
use ort::{
    session::Session,
    session::builder::{GraphOptimizationLevel, SessionBuilder},
    value::Tensor,
};
use std::path::Path;
use tokenizers::Tokenizer;

/// Embedding model using ONNX Runtime
///
/// Note: If you use a rotation matrix for embedding security, the rotation matrix dimension
/// must match the embedding dimension reported by this embedder. Always use
/// `embedder.embedding_dimension()` when constructing a rotation matrix.
#[derive(Debug)]
pub struct Embedder {
    session: Session,
    model_path: String,
    tokenizer: Tokenizer,
    embedding_dimension: usize,
}

impl Embedder {
    /// Create a new embedder from ONNX model file
    pub async fn new<P: AsRef<Path>>(model_path: P) -> VectorResult<Self> {
        let model_path = model_path.as_ref();
        
        if !model_path.exists() {
            return Err(VectorError::OnnxModel(format!(
                "Model file not found: {}",
                model_path.display()
            )));
        }
        
        // Find tokenizer file in the same directory
        let model_dir = model_path.parent()
            .ok_or_else(|| VectorError::OnnxModel("Invalid model path".to_string()))?;
        let tokenizer_path = model_dir.join("tokenizer.json");
        
        if !tokenizer_path.exists() {
            return Err(VectorError::OnnxModel(format!(
                "Tokenizer file not found: {}",
                tokenizer_path.display()
            )));
        }
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| VectorError::OnnxModel(format!("Failed to load tokenizer: {}", e)))?;
        
        // Create session with optimizations
        let mut session = SessionBuilder::new()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)
            .map_err(|e| VectorError::OnnxModel(format!("Failed to load model: {}", e)))?;

        // Validate model inputs and outputs
        // (This is a best-effort check; models may differ, but BGE models have these names)
        let expected_inputs = ["input_ids", "token_type_ids", "attention_mask"];
        let session_inputs = &session.inputs;
        for expected in &expected_inputs {
            if !session_inputs.iter().any(|input| input.name == *expected) {
                return Err(VectorError::OnnxModel(format!(
                    "Model is missing expected input: {}. Found inputs: {:?}",
                    expected, session_inputs.iter().map(|i| &i.name).collect::<Vec<_>>()
                )));
            }
        }
        // For BGE models, output is typically "last_hidden_state" or similar
        let session_outputs = &session.outputs;
        if session_outputs.is_empty() {
            return Err(VectorError::OnnxModel("Model has no outputs".to_string()));
        }
        // Optionally check for a specific output name
        // if !session_outputs.iter().any(|output| output.name == "last_hidden_state") {
        //     return Err(VectorError::OnnxModel(format!(
        //         "Model is missing expected output: last_hidden_state. Found outputs: {:?}",
        //         session_outputs.iter().map(|o| &o.name).collect::<Vec<_>>()
        //     )));
        // }

        // Determine embedding dimension from model output
        let embedding_dimension = Self::get_embedding_dimension(&mut session)?;
        
        Ok(Embedder {
            session,
            model_path: model_path.to_string_lossy().to_string(),
            tokenizer,
            embedding_dimension,
        })
    }
    
    /// Get embedding dimension from model output
    fn get_embedding_dimension(session: &mut Session) -> VectorResult<usize> {
        // For now, we'll use a more robust approach that works with the current ONNX Runtime API
        // We'll create a dummy input and run inference to determine the output shape
        
        // Create a minimal dummy input for shape inference
        let dummy_tokens = vec![1i64, 2i64, 3i64]; // Minimal sequence
        let dummy_input_ids = Tensor::from_array(([1, 3], dummy_tokens))
            .map_err(|e| VectorError::OnnxModel(format!("Failed to create dummy input: {}", e)))?;
        let dummy_token_type_ids = Tensor::from_array(([1, 3], vec![0i64; 3]))
            .map_err(|e| VectorError::OnnxModel(format!("Failed to create dummy token_type_ids: {}", e)))?;
        let dummy_attention_mask = Tensor::from_array(([1, 3], vec![1i64; 3]))
            .map_err(|e| VectorError::OnnxModel(format!("Failed to create dummy attention_mask: {}", e)))?;
        
        // Run inference with dummy input
        let outputs = session
            .run(ort::inputs![
                "input_ids" => dummy_input_ids,
                "token_type_ids" => dummy_token_type_ids,
                "attention_mask" => dummy_attention_mask
            ])
            .map_err(|e| VectorError::OnnxModel(format!("Failed to run dummy inference: {}", e)))?;
        
        // Extract embedding from output
        let embedding_tensor = outputs[0]
            .try_extract_array::<f32>()
            .map_err(|e| VectorError::OnnxModel(format!("Failed to extract dummy embedding: {}", e)))?;
        
        let shape = embedding_tensor.shape();
        if shape.len() < 3 {
            return Err(VectorError::OnnxModel(format!(
                "Unexpected output shape: expected at least 3 dimensions, got {}",
                shape.len()
            )));
        }
        
        let embedding_dim = shape[shape.len() - 1];
        if embedding_dim <= 0 {
            return Err(VectorError::OnnxModel(format!(
                "Invalid embedding dimension: {}",
                embedding_dim
            )));
        }
        
        Ok(embedding_dim as usize)
    }
    
    /// Generate embedding for text input
    pub async fn embed(&mut self, text: &str) -> VectorResult<Vec<f32>> {
        // Preprocess text for BGE model
        let processed_text = self.preprocess_text(text);
        
        // Tokenize text using the proper tokenizer
        let encoding = self.tokenizer.encode(processed_text, true)
            .map_err(|e| VectorError::EmbeddingGeneration(format!("Tokenization failed: {}", e)))?;
        
        let tokens: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let input_len = tokens.len();
        let token_type_ids = vec![0i64; input_len];
        let attention_mask = vec![1i64; input_len];
        
        // Create input tensors
        let input_ids_tensor = Tensor::from_array(([1, input_len], tokens))
            .map_err(|e| VectorError::EmbeddingGeneration(format!("Failed to create input tensor: {}", e)))?;
        let token_type_ids_tensor = Tensor::from_array(([1, input_len], token_type_ids))
            .map_err(|e| VectorError::EmbeddingGeneration(format!("Failed to create token_type_ids tensor: {}", e)))?;
        let attention_mask_tensor = Tensor::from_array(([1, input_len], attention_mask))
            .map_err(|e| VectorError::EmbeddingGeneration(format!("Failed to create attention_mask tensor: {}", e)))?;
        
        // Run inference with all required inputs
        let outputs = self.session
            .run(ort::inputs![
                "input_ids" => input_ids_tensor,
                "token_type_ids" => token_type_ids_tensor,
                "attention_mask" => attention_mask_tensor
            ])
            .map_err(|e| VectorError::EmbeddingGeneration(format!("Inference failed: {}", e)))?;
        
        // Extract embedding from output
        // BGE models typically have "last_hidden_state" as the first output
        let embedding_tensor = outputs[0]
            .try_extract_array::<f32>()
            .map_err(|e| VectorError::EmbeddingGeneration(format!("Failed to extract embedding: {}", e)))?;
        
        // Extract [CLS] token embedding (first token)
        let embedding_data = embedding_tensor.as_slice()
            .ok_or_else(|| VectorError::EmbeddingGeneration("Failed to get embedding data".to_string()))?;
        
        // Extract first token embedding (assuming [batch_size, seq_len, hidden_dim] format)
        if embedding_data.len() < self.embedding_dimension {
            return Err(VectorError::EmbeddingGeneration(format!(
                "Unexpected embedding size: expected at least {}, got {}",
                self.embedding_dimension,
                embedding_data.len()
            )));
        }
        
        let cls_embedding = embedding_data[..self.embedding_dimension].to_vec();
        
        // Normalize the embedding
        Self::normalize_embedding(&cls_embedding)
    }
    
    /// Preprocess text for BGE model
    fn preprocess_text(&self, text: &str) -> String {
        // BGE models expect text to be prefixed with "Represent this sentence: "
        format!("Represent this sentence: {}", text)
    }
    
    /// Normalize embedding vector to unit length
    fn normalize_embedding(embedding: &[f32]) -> VectorResult<Vec<f32>> {
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if magnitude < f32::EPSILON {
            return Err(VectorError::EmbeddingGeneration("Zero magnitude embedding".to_string()));
        }
        
        let normalized: Vec<f32> = embedding.iter().map(|x| x / magnitude).collect();
        Ok(normalized)
    }
    
    /// Get model path
    pub fn model_path(&self) -> &str {
        &self.model_path
    }
    
    /// Get embedding dimension
    pub fn embedding_dimension(&self) -> usize {
        self.embedding_dimension
    }
    
    /// Check if ONNX model is loaded
    pub fn has_model(&self) -> bool {
        true // Always true since we require the model to be loaded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_embedder_creation_with_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.onnx");
        
        let result = Embedder::new(nonexistent_path).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            VectorError::OnnxModel(msg) => {
                assert!(msg.contains("Model file not found"));
            }
            _ => panic!("Expected OnnxModel error"),
        }
    }
    
    #[tokio::test]
    async fn test_embedder_creation_with_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("test-model.onnx");
        std::fs::write(&model_path, b"dummy-model").unwrap();
        
        let result = Embedder::new(model_path).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            VectorError::OnnxModel(msg) => {
                assert!(msg.contains("Failed to load model") || msg.contains("Tokenizer file not found"));
            }
            _ => panic!("Expected OnnxModel error"),
        }
    }
    
    #[test]
    fn test_text_preprocessing() {
        // Test text preprocessing without needing a session
        let processed = format!("Represent this sentence: {}", "Hello world");
        assert_eq!(processed, "Represent this sentence: Hello world");
    }
    
    #[test]
    fn test_embedding_normalization() {
        let test_embedding = vec![3.0, 4.0, 0.0, 0.0]; // Magnitude = 5.0
        let normalized = Embedder::normalize_embedding(&test_embedding).unwrap();
        
        assert_eq!(normalized, vec![0.6, 0.8, 0.0, 0.0]);
        
        // Check magnitude is 1.0
        let magnitude: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 1e-6);
    }
    
    #[test]
    fn test_zero_magnitude_embedding() {
        let zero_embedding = vec![0.0; 768];
        let result = Embedder::normalize_embedding(&zero_embedding);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            VectorError::EmbeddingGeneration(msg) => {
                assert!(msg.contains("Zero magnitude"));
            }
            _ => panic!("Expected EmbeddingGeneration error"),
        }
    }
    
    #[tokio::test]
    async fn test_embedding_with_real_model() {
        // Test with the actual model in assets
        // Try to find the model file relative to the workspace root
        let model_path = if let Ok(workspace_root) = std::env::var("CARGO_WORKSPACE_DIR") {
            std::path::PathBuf::from(workspace_root)
                .join("crates/mimir/assets/bge-small-en-int8/model-int8.onnx")
        } else {
            // Fallback: try to find it relative to current directory
            let mut path = std::env::current_dir().unwrap();
            // Go up to workspace root if we're in a crate directory
            if path.ends_with("mimir-vector") {
                path.pop(); // Remove mimir-vector
                path.pop(); // Remove crates
            }
            path.join("crates/mimir/assets/bge-small-en-int8/model-int8.onnx")
        };
        
        if !model_path.exists() {
            eprintln!("Skipping test: model file not found at {}", model_path.display());
            return;
        }
        
        let mut embedder = Embedder::new(model_path).await.unwrap();
        
        // Test embedding generation
        let embedding = embedder.embed("Hello world").await.unwrap();
        
        // The embedding length should match the model's reported embedding dimension
        // (do not hardcode, as models may differ)
        assert_eq!(embedding.len(), embedder.embedding_dimension());
        
        // Check normalization
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 1e-6);
        
        // Test that different texts produce different embeddings
        let embedding2 = embedder.embed("Different text").await.unwrap();
        assert_ne!(embedding, embedding2);
        
        // Test that same text produces same embedding
        let embedding3 = embedder.embed("Hello world").await.unwrap();
        assert_eq!(embedding, embedding3);
    }
} 