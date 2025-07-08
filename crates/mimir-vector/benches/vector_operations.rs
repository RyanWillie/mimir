use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mimir_vector::VectorStore;
use std::time::Duration;
use uuid::Uuid;

fn generate_random_vector(dim: usize) -> Vec<f32> {
    (0..dim).map(|i| (i as f32 * 0.1) % 1.0).collect()
}

fn create_test_store() -> VectorStore<'static> {
    VectorStore::new()
}

fn bench_vector_store_creation(c: &mut Criterion) {
    c.bench_function("vector_store_creation", |b| {
        b.iter(|| {
            let store = black_box(VectorStore::new());
            black_box(store)
        })
    });
}

fn bench_vector_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_generation");

    let dimensions = vec![64, 128, 256, 384, 512]; // Remove 768, 1024, 1536

    for dim in dimensions {
        group.bench_with_input(BenchmarkId::new("generate_vector", dim), &dim, |b, &dim| {
            b.iter(|| {
                let vector = black_box(generate_random_vector(dim));
                black_box(vector)
            })
        });
    }

    group.finish();
}

fn bench_add_vectors(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_vectors");
    group.measurement_time(Duration::from_secs(10));

    let dimensions = vec![128, 256, 384, 512]; // Remove 768, 1536
    let vector_counts = vec![10, 100, 1000];

    for dim in dimensions {
        for count in &vector_counts {
            group.bench_with_input(
                BenchmarkId::new(format!("add_vectors_{}d", dim), count),
                count,
                |b, &count| {
                    b.iter_batched(
                        || {
                            let mut store = create_test_store();
                            let vectors: Vec<(Uuid, Vec<f32>)> = (0..count)
                                .map(|_| (Uuid::new_v4(), generate_random_vector(dim)))
                                .collect();
                            (store, vectors)
                        },
                        |(mut store, vectors)| {
                            for (id, vector) in vectors {
                                tokio_test::block_on(async {
                                    let _ = black_box(store.add_vector(id, vector).await);
                                });
                            }
                            black_box(store)
                        },
                        criterion::BatchSize::SmallInput,
                    )
                },
            );
        }
    }

    group.finish();
}

fn bench_search_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_operations");
    group.measurement_time(Duration::from_secs(10));

    let dimensions = vec![128, 256, 384, 512]; // Remove 768
    let k_values = vec![1, 5, 10, 50];

    for dim in dimensions {
        for k in &k_values {
            group.bench_with_input(
                BenchmarkId::new(format!("search_{}d", dim), k),
                k,
                |b, &k| {
                    let store = create_test_store();
                    let query_vector = generate_random_vector(dim);

                    b.iter(|| {
                        tokio_test::block_on(async {
                            let results =
                                black_box(store.search(query_vector.clone(), k).await.unwrap());
                            black_box(results)
                        })
                    })
                },
            );
        }
    }

    group.finish();
}

fn bench_vector_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_memory");

    let test_cases = vec![
        (128, 1000),  // 128d, 1k vectors
        (256, 1000),  // 256d, 1k vectors
        (384, 1000),  // 384d, 1k vectors
        (128, 10000), // 128d, 10k vectors
        (256, 10000), // 256d, 10k vectors
    ]; // Remove 768 dimensions

    for (dim, count) in test_cases {
        group.bench_with_input(
            BenchmarkId::new("memory_allocation", format!("{}d_{}v", dim, count)),
            &(dim, count),
            |b, &(dim, count)| {
                b.iter(|| {
                    let vectors: Vec<Vec<f32>> =
                        black_box((0..count).map(|_| generate_random_vector(dim)).collect());
                    black_box(vectors)
                })
            },
        );
    }

    group.finish();
}

fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("concurrent_searches", |b| {
        let store = create_test_store();
        let queries: Vec<Vec<f32>> = (0..10).map(|_| generate_random_vector(128)).collect();

        b.iter(|| {
            tokio_test::block_on(async {
                let mut handles = vec![];

                for query in &queries {
                    let query = query.clone();
                    let handle = tokio::spawn(async move {
                        let store = create_test_store();
                        store.search(query, 10).await
                    });
                    handles.push(handle);
                }

                let results = futures::future::join_all(handles).await;
                black_box(results)
            })
        })
    });

    group.finish();
}

fn bench_vector_normalization(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_normalization");

    let dimensions = vec![128, 256, 384, 512]; // Remove 768, 1536

    for dim in dimensions {
        group.bench_with_input(
            BenchmarkId::new("normalize_vector", dim),
            &dim,
            |b, &dim| {
                let vector = generate_random_vector(dim);
                b.iter(|| {
                    let magnitude: f32 =
                        black_box(vector.iter().map(|x| x * x).sum::<f32>().sqrt());

                    let normalized: Vec<f32> =
                        black_box(vector.iter().map(|x| x / magnitude).collect());

                    black_box(normalized)
                })
            },
        );
    }

    group.finish();
}

fn bench_distance_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_calculations");

    let dimensions = vec![128, 256, 384, 512]; // Remove 768, 1536

    for dim in dimensions {
        let vector1 = generate_random_vector(dim);
        let vector2 = generate_random_vector(dim);

        group.bench_with_input(
            BenchmarkId::new("cosine_similarity", dim),
            &dim,
            |b, _dim| {
                b.iter(|| {
                    let dot_product: f32 =
                        black_box(vector1.iter().zip(&vector2).map(|(a, b)| a * b).sum());

                    let norm1: f32 = black_box(vector1.iter().map(|x| x * x).sum::<f32>().sqrt());

                    let norm2: f32 = black_box(vector2.iter().map(|x| x * x).sum::<f32>().sqrt());

                    let similarity = black_box(dot_product / (norm1 * norm2));
                    black_box(similarity)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("euclidean_distance", dim),
            &dim,
            |b, _dim| {
                b.iter(|| {
                    let distance: f32 = black_box(
                        vector1
                            .iter()
                            .zip(&vector2)
                            .map(|(a, b)| (a - b).powi(2))
                            .sum::<f32>()
                            .sqrt(),
                    );
                    black_box(distance)
                })
            },
        );
    }

    group.finish();
}

fn bench_vector_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");
    group.measurement_time(Duration::from_secs(10));

    let batch_sizes = vec![10, 50, 100, 500];

    for batch_size in batch_sizes {
        group.bench_with_input(
            BenchmarkId::new("batch_add", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter_batched(
                    || {
                        let mut store = create_test_store();
                        let vectors: Vec<(Uuid, Vec<f32>)> = (0..batch_size)
                            .map(|_| (Uuid::new_v4(), generate_random_vector(384)))
                            .collect();
                        (store, vectors)
                    },
                    |(mut store, vectors)| {
                        tokio_test::block_on(async {
                            for (id, vector) in vectors {
                                let _ = black_box(store.add_vector(id, vector).await);
                            }
                            black_box(store)
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
                let store = create_test_store();
                let queries: Vec<Vec<f32>> = (0..batch_size)
                    .map(|_| generate_random_vector(384))
                    .collect();

                b.iter(|| {
                    tokio_test::block_on(async {
                        for query in &queries {
                            let _ = black_box(store.search(query.clone(), 10).await);
                        }
                    })
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_vector_store_creation,
    bench_vector_generation,
    bench_add_vectors,
    bench_search_operations,
    bench_vector_memory_usage,
    bench_concurrent_operations,
    bench_vector_normalization,
    bench_distance_calculations,
    bench_vector_batch_operations
);

criterion_main!(benches);
