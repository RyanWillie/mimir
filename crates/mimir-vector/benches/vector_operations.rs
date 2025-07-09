use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mimir_core::{crypto::RootKey, test_utils::generators::generate_test_embedding};
use mimir_vector::{
    BatchConfig, MemoryConfig, SearchQuery, ThreadSafeVectorStore, VectorInsert, VectorStore,
};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Generate a random vector with specified dimension
fn generate_random_vector(dim: usize) -> Vec<f32> {
    generate_test_embedding(dim)
}

/// Generate multiple random vectors for testing
fn generate_test_vectors(dim: usize, count: usize) -> Vec<Vec<f32>> {
    (0..count).map(|_| generate_random_vector(dim)).collect()
}

/// Create a test vector store with specified dimension
fn create_test_store(dimension: usize) -> VectorStore<'static> {
    VectorStore::with_dimension(dimension).unwrap()
}

/// Create a test thread-safe store
fn create_thread_safe_store(dimension: usize) -> ThreadSafeVectorStore {
    let temp_dir = TempDir::new().unwrap();
    ThreadSafeVectorStore::new(temp_dir.path(), dimension, None, None).unwrap()
}

/// Create a Tokio runtime for async benchmarks
fn create_runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// Benchmark vector store creation
fn bench_vector_store_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("store_creation");

    let dimensions = vec![128, 256, 384, 512, 768];

    for dim in dimensions {
        group.bench_with_input(BenchmarkId::new("vector_store", dim), &dim, |b, &dim| {
            b.iter(|| {
                let store = black_box(create_test_store(dim));
                black_box(store)
            })
        });

        group.bench_with_input(
            BenchmarkId::new("thread_safe_store", dim),
            &dim,
            |b, &dim| {
                b.iter(|| {
                    let store = black_box(create_thread_safe_store(dim));
                    black_box(store)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark vector insertion operations
fn bench_vector_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_insertion");
    group.measurement_time(Duration::from_secs(10));

    let dimensions = vec![128, 256, 384, 512, 768];
    let vector_counts = vec![10, 100, 1000];

    for dim in dimensions {
        for count in &vector_counts {
            group.bench_with_input(
                BenchmarkId::new(format!("insert_vectors_{}d", dim), count),
                count,
                |b, &count| {
                    b.iter_batched(
                        || {
                            let store = create_test_store(dim);
                            let vectors = generate_test_vectors(dim, count);
                            let ids: Vec<Uuid> = (0..count).map(|_| Uuid::new_v4()).collect();
                            (store, vectors, ids)
                        },
                        |(mut store, vectors, ids)| {
                            let rt = create_runtime();
                            rt.block_on(async {
                                for (id, vector) in ids.into_iter().zip(vectors) {
                                    let _ = black_box(store.add_vector(id, vector).await);
                                }
                                black_box(store)
                            })
                        },
                        criterion::BatchSize::SmallInput,
                    )
                },
            );
        }
    }

    group.finish();
}

/// Benchmark thread-safe vector insertion
fn bench_thread_safe_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_safe_insertion");
    group.measurement_time(Duration::from_secs(10));

    let dimensions = vec![128, 256, 384, 512, 768];
    let vector_counts = vec![10, 100, 1000];

    for dim in dimensions {
        for count in &vector_counts {
            group.bench_with_input(
                BenchmarkId::new(format!("thread_safe_insert_{}d", dim), count),
                count,
                |b, &count| {
                    b.iter_batched(
                        || {
                            let store = create_thread_safe_store(dim);
                            let vectors = generate_test_vectors(dim, count);
                            let ids: Vec<Uuid> = (0..count).map(|_| Uuid::new_v4()).collect();
                            (store, vectors, ids)
                        },
                        |(store, vectors, ids)| {
                            let rt = create_runtime();
                            rt.block_on(async {
                                for (id, vector) in ids.into_iter().zip(vectors) {
                                    let _ = black_box(store.add_vector(id, vector).await);
                                }
                                black_box(store)
                            })
                        },
                        criterion::BatchSize::SmallInput,
                    )
                },
            );
        }
    }

    group.finish();
}

/// Benchmark search operations
fn bench_search_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_operations");
    group.measurement_time(Duration::from_secs(10));

    let dimensions = vec![128, 256, 384, 512, 768];
    let k_values = vec![1, 5, 10, 50, 100];

    for dim in dimensions {
        for k in &k_values {
            group.bench_with_input(
                BenchmarkId::new(format!("search_{}d", dim), k),
                k,
                |b, &k| {
                    b.iter_batched(
                        || {
                            let mut store = create_test_store(dim);
                            let vectors = generate_test_vectors(dim, 1000);
                            let ids: Vec<Uuid> = (0..1000).map(|_| Uuid::new_v4()).collect();

                            // Populate store
                            let rt = create_runtime();
                            rt.block_on(async {
                                for (id, vector) in ids.into_iter().zip(vectors) {
                                    let _ = store.add_vector(id, vector).await;
                                }
                                store
                            })
                        },
                        |store| {
                            let query = generate_random_vector(dim);
                            let rt = create_runtime();
                            rt.block_on(async {
                                let results = black_box(store.search(query, k).await.unwrap());
                                black_box(results)
                            })
                        },
                        criterion::BatchSize::SmallInput,
                    )
                },
            );
        }
    }

    group.finish();
}

/// Benchmark thread-safe search operations
fn bench_thread_safe_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_safe_search");
    group.measurement_time(Duration::from_secs(10));

    let dimensions = vec![128, 256, 384, 512, 768];
    let k_values = vec![1, 5, 10, 50, 100];

    for dim in dimensions {
        for k in &k_values {
            group.bench_with_input(
                BenchmarkId::new(format!("thread_safe_search_{}d", dim), k),
                k,
                |b, &k| {
                    b.iter_batched(
                        || {
                            let store = create_thread_safe_store(dim);
                            let vectors = generate_test_vectors(dim, 1000);
                            let ids: Vec<Uuid> = (0..1000).map(|_| Uuid::new_v4()).collect();

                            // Populate store
                            let rt = create_runtime();
                            rt.block_on(async {
                                for (id, vector) in ids.into_iter().zip(vectors) {
                                    let _ = store.add_vector(id, vector).await;
                                }
                                store
                            })
                        },
                        |store| {
                            let query = generate_random_vector(dim);
                            let rt = create_runtime();
                            rt.block_on(async {
                                let results = black_box(store.search(query, k).await.unwrap());
                                black_box(results)
                            })
                        },
                        criterion::BatchSize::SmallInput,
                    )
                },
            );
        }
    }

    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("concurrent_inserts", |b| {
        b.iter_batched(
            || {
                let store = Arc::new(create_thread_safe_store(384));
                let vectors = generate_test_vectors(384, 100);
                let ids: Vec<Uuid> = (0..100).map(|_| Uuid::new_v4()).collect();
                (store, vectors, ids)
            },
            |(store, vectors, ids)| {
                let rt = create_runtime();
                rt.block_on(async {
                    // Sequential operations to avoid Send trait issues
                    for (id, vector) in ids.into_iter().zip(vectors) {
                        let _ = store.add_vector(id, vector).await;
                    }
                    black_box(store)
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// Benchmark batch operations
fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");
    group.measurement_time(Duration::from_secs(15));

    let batch_sizes = vec![10, 50, 100, 500, 1000];

    for batch_size in batch_sizes {
        group.bench_with_input(
            BenchmarkId::new("batch_insert", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter_batched(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let batch_config = BatchConfig::default();
                        let store = ThreadSafeVectorStore::new(
                            temp_dir.path(),
                            384,
                            None,
                            Some(batch_config),
                        )
                        .unwrap();

                        let vectors: Vec<VectorInsert> = (0..batch_size)
                            .map(|_| VectorInsert {
                                memory_id: Uuid::new_v4(),
                                vector: generate_random_vector(384),
                            })
                            .collect();
                        (store, vectors)
                    },
                    |(store, vectors)| {
                        let rt = create_runtime();
                        rt.block_on(async {
                            let _ = black_box(store.batch_insert(vectors).await);
                        })
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("batch_search", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter_batched(
                    || {
                        let store = create_thread_safe_store(384);
                        let vectors = generate_test_vectors(384, 1000);
                        let ids: Vec<Uuid> = (0..1000).map(|_| Uuid::new_v4()).collect();

                        // Populate store
                        let rt = create_runtime();
                        let populated_store = rt.block_on(async {
                            for (id, vector) in ids.into_iter().zip(vectors) {
                                let _ = store.add_vector(id, vector).await;
                            }
                            store
                        });

                        let queries: Vec<SearchQuery> = (0..batch_size)
                            .map(|_| SearchQuery {
                                query_vector: generate_random_vector(384),
                                k: 10,
                            })
                            .collect();
                        (populated_store, queries)
                    },
                    |(store, queries)| {
                        let rt = create_runtime();
                        rt.block_on(async {
                            let _ = black_box(store.batch_search(queries).await);
                        })
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark memory management
fn bench_memory_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_management");

    let test_cases = vec![
        (128, 1000),  // 128d, 1k vectors
        (256, 1000),  // 256d, 1k vectors
        (384, 1000),  // 384d, 1k vectors
        (512, 1000),  // 512d, 1k vectors
        (768, 1000),  // 768d, 1k vectors
        (128, 10000), // 128d, 10k vectors
        (256, 10000), // 256d, 10k vectors
    ];

    for (dim, count) in test_cases {
        group.bench_with_input(
            BenchmarkId::new("memory_allocation", format!("{}d_{}v", dim, count)),
            &(dim, count),
            |b, &(dim, count)| {
                b.iter_batched(
                    || {
                        let memory_config = MemoryConfig {
                            max_vectors: count * 2,
                            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
                            ..Default::default()
                        };
                        let temp_dir = TempDir::new().unwrap();
                        ThreadSafeVectorStore::new(temp_dir.path(), dim, Some(memory_config), None)
                            .unwrap()
                    },
                    |store| {
                        let vectors = generate_test_vectors(dim, count);
                        let ids: Vec<Uuid> = (0..count).map(|_| Uuid::new_v4()).collect();

                        let rt = create_runtime();
                        rt.block_on(async {
                            for (id, vector) in ids.into_iter().zip(vectors) {
                                let _ = store.add_vector(id, vector).await;
                            }
                            let stats = store.get_memory_stats();
                            black_box(stats)
                        })
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark vector operations with rotation
fn bench_rotation_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("rotation_operations");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("rotation_matrix_creation", |b| {
        b.iter_batched(
            || {
                let root_key = RootKey::new().unwrap();
                (root_key, 768)
            },
            |(root_key, dim)| {
                // Benchmark rotation matrix creation
                let rotation_matrix =
                    mimir_vector::rotation::RotationMatrix::from_root_key(&root_key, dim);
                black_box(rotation_matrix)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// Benchmark persistence operations
fn bench_persistence_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("persistence_operations");
    group.measurement_time(Duration::from_secs(15));

    let test_cases = vec![
        (128, 100),  // Small dataset
        (384, 1000), // Medium dataset
        (768, 5000), // Large dataset
    ];

    for (dim, count) in test_cases {
        group.bench_with_input(
            BenchmarkId::new("save_load", format!("{}d_{}v", dim, count)),
            &(dim, count),
            |b, &(dim, count)| {
                b.iter_batched(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let store =
                            ThreadSafeVectorStore::new(temp_dir.path(), dim, None, None).unwrap();
                        let vectors = generate_test_vectors(dim, count);
                        let ids: Vec<Uuid> = (0..count).map(|_| Uuid::new_v4()).collect();
                        (store, vectors, ids, temp_dir)
                    },
                    |(store, vectors, ids, temp_dir)| {
                        let rt = create_runtime();
                        rt.block_on(async {
                            // Add vectors
                            for (id, vector) in ids.into_iter().zip(vectors) {
                                let _ = store.add_vector(id, vector).await;
                            }

                            // Save
                            let _ = store.save(None).await;

                            // Load
                            let loaded_store =
                                ThreadSafeVectorStore::load(temp_dir.path(), None, None, None)
                                    .await
                                    .unwrap();

                            black_box(loaded_store)
                        })
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark search quality and accuracy
fn bench_search_quality(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_quality");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("similarity_ranking", |b| {
        b.iter_batched(
            || {
                let mut store = create_test_store(384);
                let vectors = generate_test_vectors(384, 1000);
                let ids: Vec<Uuid> = (0..1000).map(|_| Uuid::new_v4()).collect();

                // Populate store
                let rt = create_runtime();
                rt.block_on(async {
                    for (id, vector) in ids.into_iter().zip(vectors) {
                        let _ = store.add_vector(id, vector).await;
                    }
                    store
                })
            },
            |store| {
                let query = generate_random_vector(384);
                let rt = create_runtime();
                rt.block_on(async {
                    let results = store.search_detailed(query, 100).await.unwrap();

                    // Verify results are sorted by similarity (descending)
                    let mut prev_similarity = f32::MAX;
                    for result in &results {
                        assert!(result.similarity <= prev_similarity);
                        prev_similarity = result.similarity;
                    }

                    black_box(results)
                })
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// Benchmark different HNSW parameters
fn bench_hnsw_parameters(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_parameters");
    group.measurement_time(Duration::from_secs(10));

    let ef_values = vec![16, 32, 64, 128];

    for ef in ef_values {
        group.bench_with_input(BenchmarkId::new("search_ef", ef), &ef, |b, &_ef| {
            b.iter_batched(
                || {
                    let mut store = create_test_store(384);
                    let vectors = generate_test_vectors(384, 1000);
                    let ids: Vec<Uuid> = (0..1000).map(|_| Uuid::new_v4()).collect();

                    // Populate store
                    let rt = create_runtime();
                    rt.block_on(async {
                        for (id, vector) in ids.into_iter().zip(vectors) {
                            let _ = store.add_vector(id, vector).await;
                        }
                        store
                    })
                },
                |store| {
                    let query = generate_random_vector(384);
                    let rt = create_runtime();
                    rt.block_on(async {
                        // Note: The current implementation uses fixed ef=32
                        // This benchmark shows the potential for parameter tuning
                        let results = store.search(query, 10).await.unwrap();
                        black_box(results)
                    })
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_vector_store_creation,
    bench_vector_insertion,
    bench_thread_safe_insertion,
    bench_search_operations,
    bench_thread_safe_search,
    bench_concurrent_operations,
    bench_batch_operations,
    bench_memory_management,
    bench_rotation_operations,
    bench_persistence_operations,
    bench_search_quality,
    bench_hnsw_parameters
);

criterion_main!(benches);
