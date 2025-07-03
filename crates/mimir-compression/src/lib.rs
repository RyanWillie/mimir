//! Mimir Compression - Memory compression and deduplication

use mimir_core::{Memory, Result};

/// Compression statistics
#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f32,
}

/// Compression engine for memory content
pub struct Compressor {
    // TODO: Add compression algorithm (e.g., zstd, lz4)
}

impl Compressor {
    /// Create a new compressor
    pub fn new() -> Self {
        Self {}
    }

    /// Compress memory content
    pub async fn compress_memory(&self, memory: &Memory) -> Result<(Vec<u8>, CompressionStats)> {
        let content_bytes = memory.content.as_bytes();
        let original_size = content_bytes.len();

        // TODO: Implement actual compression
        let compressed_data = content_bytes.to_vec();
        let compressed_size = compressed_data.len();

        let compression_ratio = if original_size > 0 {
            compressed_size as f32 / original_size as f32
        } else {
            1.0
        };

        let stats = CompressionStats {
            original_size,
            compressed_size,
            compression_ratio,
        };

        Ok((compressed_data, stats))
    }

    /// Decompress memory content
    pub async fn decompress_memory(&self, compressed_data: &[u8]) -> Result<String> {
        // TODO: Implement actual decompression
        let content = String::from_utf8(compressed_data.to_vec()).map_err(|e| {
            mimir_core::MimirError::Compression(format!("UTF-8 decode error: {}", e))
        })?;

        Ok(content)
    }

    /// Calculate similarity between memories for deduplication
    pub fn calculate_similarity(&self, memory1: &Memory, memory2: &Memory) -> f32 {
        if memory1.content == memory2.content {
            return 1.0;
        }

        // Simple similarity based on common words (placeholder)
        let words1: std::collections::HashSet<&str> = memory1.content.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = memory2.content.split_whitespace().collect();

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union > 0 {
            intersection as f32 / union as f32
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mimir_core::test_utils::MemoryBuilder;
    use proptest::prelude::*;

    fn create_test_compressor() -> Compressor {
        Compressor::new()
    }

    #[test]
    fn test_compressor_creation() {
        let compressor = Compressor::new();
        // Just verify it can be created without panicking
        drop(compressor);
    }

    #[tokio::test]
    async fn test_compress_memory_basic() {
        let compressor = create_test_compressor();
        let memory = MemoryBuilder::new()
            .with_content("Test content for compression")
            .build();

        let result = compressor.compress_memory(&memory).await;
        assert!(result.is_ok());

        let (compressed_data, stats) = result.unwrap();
        assert!(!compressed_data.is_empty());
        assert_eq!(stats.original_size, "Test content for compression".len());
        assert!(stats.compression_ratio > 0.0);
    }

    #[tokio::test]
    async fn test_compress_empty_content() {
        let compressor = create_test_compressor();
        let memory = MemoryBuilder::new().with_content("").build();

        let result = compressor.compress_memory(&memory).await;
        assert!(result.is_ok());

        let (compressed_data, stats) = result.unwrap();
        assert!(compressed_data.is_empty());
        assert_eq!(stats.original_size, 0);
        assert_eq!(stats.compression_ratio, 1.0);
    }

    #[tokio::test]
    async fn test_compress_large_content() {
        let compressor = create_test_compressor();
        let large_content = "Lorem ipsum ".repeat(1000); // ~12KB
        let memory = MemoryBuilder::new()
            .with_content(large_content.clone())
            .build();

        let result = compressor.compress_memory(&memory).await;
        assert!(result.is_ok());

        let (compressed_data, stats) = result.unwrap();
        assert!(!compressed_data.is_empty());
        assert_eq!(stats.original_size, large_content.len());
    }

    #[tokio::test]
    async fn test_decompress_memory_basic() {
        let compressor = create_test_compressor();
        let original_content = "Test content for decompression";
        let compressed_data = original_content.as_bytes();

        let result = compressor.decompress_memory(compressed_data).await;
        assert!(result.is_ok());

        let decompressed_content = result.unwrap();
        assert_eq!(decompressed_content, original_content);
    }

    #[tokio::test]
    async fn test_decompress_empty_data() {
        let compressor = create_test_compressor();
        let empty_data: &[u8] = &[];

        let result = compressor.decompress_memory(empty_data).await;
        assert!(result.is_ok());

        let decompressed_content = result.unwrap();
        assert!(decompressed_content.is_empty());
    }

    #[tokio::test]
    async fn test_compress_decompress_roundtrip() {
        let compressor = create_test_compressor();
        let original_content = "This is a test of round-trip compression and decompression";
        let memory = MemoryBuilder::new().with_content(original_content).build();

        // Compress
        let (compressed_data, _stats) = compressor.compress_memory(&memory).await.unwrap();

        // Decompress
        let decompressed_content = compressor
            .decompress_memory(&compressed_data)
            .await
            .unwrap();

        assert_eq!(decompressed_content, original_content);
    }

    #[tokio::test]
    async fn test_compression_stats() {
        let compressor = create_test_compressor();
        let content = "Test content with stats";
        let memory = MemoryBuilder::new().with_content(content).build();

        let (_compressed_data, stats) = compressor.compress_memory(&memory).await.unwrap();

        assert_eq!(stats.original_size, content.len());
        assert_eq!(stats.compressed_size, content.len()); // Stub implementation
        assert_eq!(stats.compression_ratio, 1.0); // No actual compression in stub
    }

    #[test]
    fn test_calculate_similarity_identical() {
        let compressor = create_test_compressor();
        let memory1 = MemoryBuilder::new()
            .with_content("Identical content")
            .build();
        let memory2 = MemoryBuilder::new()
            .with_content("Identical content")
            .build();

        let similarity = compressor.calculate_similarity(&memory1, &memory2);
        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn test_calculate_similarity_different() {
        let compressor = create_test_compressor();
        let memory1 = MemoryBuilder::new()
            .with_content("Completely different")
            .build();
        let memory2 = MemoryBuilder::new()
            .with_content("Totally unrelated")
            .build();

        let similarity = compressor.calculate_similarity(&memory1, &memory2);
        assert!(similarity < 1.0);
    }

    #[test]
    fn test_calculate_similarity_partial() {
        let compressor = create_test_compressor();
        let memory1 = MemoryBuilder::new().with_content("This is a test").build();
        let memory2 = MemoryBuilder::new()
            .with_content("This is different")
            .build();

        let similarity = compressor.calculate_similarity(&memory1, &memory2);
        assert!(similarity > 0.0 && similarity < 1.0);
    }

    #[test]
    fn test_calculate_similarity_empty() {
        let compressor = create_test_compressor();
        let memory1 = MemoryBuilder::new().with_content("").build();
        let memory2 = MemoryBuilder::new().with_content("").build();

        let similarity = compressor.calculate_similarity(&memory1, &memory2);
        // Both empty should be considered similar in some way
        assert!(similarity >= 0.0);
    }

    #[test]
    fn test_calculate_similarity_one_empty() {
        let compressor = create_test_compressor();
        let memory1 = MemoryBuilder::new().with_content("Some content").build();
        let memory2 = MemoryBuilder::new().with_content("").build();

        let similarity = compressor.calculate_similarity(&memory1, &memory2);
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_similarity_word_overlap() {
        let compressor = create_test_compressor();
        let memory1 = MemoryBuilder::new()
            .with_content("The quick brown fox jumps")
            .build();
        let memory2 = MemoryBuilder::new()
            .with_content("The brown fox runs quickly")
            .build();

        let similarity = compressor.calculate_similarity(&memory1, &memory2);

        // Should have some overlap (The, brown, fox)
        assert!(similarity > 0.0);
        assert!(similarity < 1.0);
    }

    #[tokio::test]
    async fn test_compression_with_special_characters() {
        let compressor = create_test_compressor();
        let content = "Special chars: 🧠 Memory with émojis and ñoñ-ASCII chars";
        let memory = MemoryBuilder::new().with_content(content).build();

        let result = compressor.compress_memory(&memory).await;
        assert!(result.is_ok());

        let (compressed_data, _stats) = result.unwrap();

        // Test round-trip
        let decompressed = compressor
            .decompress_memory(&compressed_data)
            .await
            .unwrap();
        assert_eq!(decompressed, content);
    }

    #[tokio::test]
    async fn test_compression_with_unicode() {
        let compressor = create_test_compressor();
        let content = "Unicode test: 你好 مرحبا Здравствуйте";
        let memory = MemoryBuilder::new().with_content(content).build();

        let (compressed_data, _stats) = compressor.compress_memory(&memory).await.unwrap();
        let decompressed = compressor
            .decompress_memory(&compressed_data)
            .await
            .unwrap();

        assert_eq!(decompressed, content);
    }

    #[tokio::test]
    async fn test_concurrent_compression() {
        let compressor = create_test_compressor();

        let memory1 = MemoryBuilder::new().with_content("First memory").build();
        let memory2 = MemoryBuilder::new().with_content("Second memory").build();
        let memory3 = MemoryBuilder::new().with_content("Third memory").build();

        // Test concurrent compression operations
        let (result1, result2, result3) = tokio::join!(
            compressor.compress_memory(&memory1),
            compressor.compress_memory(&memory2),
            compressor.compress_memory(&memory3)
        );

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_utf8_decompression() {
        let compressor = create_test_compressor();

        // Create invalid UTF-8 bytes
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];

        let result = compressor.decompress_memory(&invalid_utf8).await;
        assert!(result.is_err());

        // Should be a compression error
        match result.unwrap_err() {
            mimir_core::MimirError::Compression(_) => {
                // Expected error type
            }
            _ => panic!("Expected Compression error"),
        }
    }

    #[test]
    fn test_compression_stats_calculations() {
        let stats = CompressionStats {
            original_size: 1000,
            compressed_size: 500,
            compression_ratio: 0.5,
        };

        assert_eq!(stats.original_size, 1000);
        assert_eq!(stats.compressed_size, 500);
        assert_eq!(stats.compression_ratio, 0.5);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn test_compression_preserves_data(content in ".*") {
            let compressor = create_test_compressor();
            let memory = MemoryBuilder::new().with_content(content.clone()).build();

            tokio_test::block_on(async {
                if let Ok((compressed_data, _stats)) = compressor.compress_memory(&memory).await {
                    if let Ok(decompressed) = compressor.decompress_memory(&compressed_data).await {
                        assert_eq!(decompressed, content);
                    }
                }
            });
        }

        #[test]
        fn test_similarity_bounds(
            content1 in "[a-zA-Z0-9 ]{0,100}",
            content2 in "[a-zA-Z0-9 ]{0,100}"
        ) {
            let compressor = create_test_compressor();
            let memory1 = MemoryBuilder::new().with_content(content1).build();
            let memory2 = MemoryBuilder::new().with_content(content2).build();

            let similarity = compressor.calculate_similarity(&memory1, &memory2);
            assert!(similarity >= 0.0 && similarity <= 1.0);
        }

        #[test]
        fn test_compression_ratio_positive(content in ".*") {
            let compressor = create_test_compressor();
            let memory = MemoryBuilder::new().with_content(content).build();

            tokio_test::block_on(async {
                if let Ok((_compressed_data, stats)) = compressor.compress_memory(&memory).await {
                    assert!(stats.compression_ratio >= 0.0);
                }
            });
        }
    }

    #[test]
    fn test_multiple_compressor_instances() {
        // Test that multiple compressor instances can coexist
        let compressor1 = Compressor::new();
        let compressor2 = Compressor::new();
        let compressor3 = Compressor::new();

        // All should be independently usable
        drop(compressor1);
        drop(compressor2);
        drop(compressor3);
    }

    #[test]
    fn test_similarity_symmetry() {
        let compressor = create_test_compressor();
        let memory1 = MemoryBuilder::new().with_content("First memory").build();
        let memory2 = MemoryBuilder::new().with_content("Second memory").build();

        let similarity1 = compressor.calculate_similarity(&memory1, &memory2);
        let similarity2 = compressor.calculate_similarity(&memory2, &memory1);

        // Similarity should be symmetric
        assert_eq!(similarity1, similarity2);
    }

    #[test]
    fn test_similarity_reflexivity() {
        let compressor = create_test_compressor();
        let memory = MemoryBuilder::new().with_content("Test memory").build();

        let similarity = compressor.calculate_similarity(&memory, &memory);

        // A memory should be 100% similar to itself
        assert_eq!(similarity, 1.0);
    }
}
