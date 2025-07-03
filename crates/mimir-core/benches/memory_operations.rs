use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use mimir_core::{Memory, MemoryClass};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde_json;

fn create_test_memory(content_size: usize) -> Memory {
    let content = "a".repeat(content_size);
    Memory {
        id: Uuid::new_v4(),
        content,
        embedding: None,
        class: MemoryClass::Personal,
        scope: None,
        tags: vec!["test".to_string(), "benchmark".to_string()],
        app_acl: vec!["benchmark_app".to_string()],
        key_id: "default_key".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn bench_memory_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_creation");
    
    let sizes = vec![100, 1000, 10000, 100000];
    
    for size in sizes {
        group.bench_with_input(
            BenchmarkId::new("create_memory", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let content = black_box("x".repeat(size));
                    black_box(Memory {
                        id: Uuid::new_v4(),
                        content,
                        embedding: None,
                        class: MemoryClass::Personal,
                        scope: None,
                        tags: vec!["benchmark".to_string()],
                        app_acl: vec!["test_app".to_string()],
                        key_id: "default_key".to_string(),
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    })
                })
            },
        );
    }
    
    group.finish();
}

fn bench_memory_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_serialization");
    
    let sizes = vec![100, 1000, 10000, 100000];
    
    for size in sizes {
        let memory = create_test_memory(size);
        
        group.bench_with_input(
            BenchmarkId::new("serialize_json", size),
            &memory,
            |b, memory| {
                b.iter(|| {
                    let json = black_box(serde_json::to_string(memory).unwrap());
                    black_box(json)
                })
            },
        );
        
        let json = serde_json::to_string(&memory).unwrap();
        group.bench_with_input(
            BenchmarkId::new("deserialize_json", size),
            &json,
            |b, json| {
                b.iter(|| {
                    let memory: Memory = black_box(serde_json::from_str(json).unwrap());
                    black_box(memory)
                })
            },
        );
    }
    
    group.finish();
}

fn bench_memory_cloning(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_cloning");
    
    let sizes = vec![100, 1000, 10000, 100000];
    
    for size in sizes {
        let memory = create_test_memory(size);
        
        group.bench_with_input(
            BenchmarkId::new("clone_memory", size),
            &memory,
            |b, memory| {
                b.iter(|| {
                    let cloned = black_box(memory.clone());
                    black_box(cloned)
                })
            },
        );
    }
    
    group.finish();
}

fn bench_memory_class_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_class_operations");
    
    let classes = vec![
        MemoryClass::Personal,
        MemoryClass::Work,
        MemoryClass::Health,
        MemoryClass::Financial,
        MemoryClass::Other("custom_class".to_string()),
    ];
    
    group.bench_function("create_memory_classes", |b| {
        b.iter(|| {
            for class in &classes {
                let _memory = black_box(Memory {
                    id: Uuid::new_v4(),
                    content: "test content".to_string(),
                    embedding: None,
                    class: class.clone(),
                    scope: None,
                    tags: vec![],
                    app_acl: vec!["test".to_string()],
                    key_id: "default_key".to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                });
            }
        })
    });
    
    group.bench_function("serialize_memory_classes", |b| {
        b.iter(|| {
            for class in &classes {
                let memory = Memory {
                    id: Uuid::new_v4(),
                    content: "test".to_string(),
                    embedding: None,
                    class: class.clone(),
                    scope: None,
                    tags: vec![],
                    app_acl: vec!["test".to_string()],
                    key_id: "default_key".to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                let _json = black_box(serde_json::to_string(&memory).unwrap());
            }
        })
    });
    
    group.finish();
}

fn bench_uuid_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("uuid_generation");
    
    group.bench_function("uuid_v4_generation", |b| {
        b.iter(|| {
            let id = black_box(Uuid::new_v4());
            black_box(id)
        })
    });
    
    group.bench_function("uuid_to_string", |b| {
        let id = Uuid::new_v4();
        b.iter(|| {
            let id_str = black_box(id.to_string());
            black_box(id_str)
        })
    });
    
    group.bench_function("uuid_from_string", |b| {
        let id_str = Uuid::new_v4().to_string();
        b.iter(|| {
            let id = black_box(Uuid::parse_str(&id_str).unwrap());
            black_box(id)
        })
    });
    
    group.finish();
}

fn bench_memory_collections(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_collections");
    
    let sizes = vec![10, 100, 1000, 10000];
    
    for size in sizes {
        let memories: Vec<Memory> = (0..size).map(|_| create_test_memory(1000)).collect();
        
        group.bench_with_input(
            BenchmarkId::new("serialize_memory_vec", size),
            &memories,
            |b, memories| {
                b.iter(|| {
                    let json = black_box(serde_json::to_string(memories).unwrap());
                    black_box(json)
                })
            },
        );
        
        let json = serde_json::to_string(&memories).unwrap();
        group.bench_with_input(
            BenchmarkId::new("deserialize_memory_vec", size),
            &json,
            |b, json| {
                b.iter(|| {
                    let memories: Vec<Memory> = black_box(serde_json::from_str(json).unwrap());
                    black_box(memories)
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("filter_by_class", size),
            &memories,
            |b, memories| {
                b.iter(|| {
                    let filtered: Vec<&Memory> = black_box(
                        memories
                            .iter()
                            .filter(|m| m.class == MemoryClass::Personal)
                            .collect()
                    );
                    black_box(filtered)
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("find_by_id", size),
            &memories,
            |b, memories| {
                let target_id = memories[size / 2].id;
                b.iter(|| {
                    let found = black_box(
                        memories
                            .iter()
                            .find(|m| m.id == target_id)
                    );
                    black_box(found)
                })
            },
        );
    }
    
    group.finish();
}

fn bench_date_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("date_operations");
    
    group.bench_function("utc_now", |b| {
        b.iter(|| {
            let now = black_box(Utc::now());
            black_box(now)
        })
    });
    
    group.bench_function("date_to_rfc3339", |b| {
        let now = Utc::now();
        b.iter(|| {
            let date_str = black_box(now.to_rfc3339());
            black_box(date_str)
        })
    });
    
    group.bench_function("date_from_rfc3339", |b| {
        let date_str = Utc::now().to_rfc3339();
        b.iter(|| {
            let date: DateTime<Utc> = black_box(date_str.parse().unwrap());
            black_box(date)
        })
    });
    
    group.finish();
}

fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");
    
    let sizes = vec![100, 1000, 10000, 100000];
    
    for size in sizes {
        let content = "a".repeat(size);
        
        group.bench_with_input(
            BenchmarkId::new("string_clone", size),
            &content,
            |b, content| {
                b.iter(|| {
                    let cloned = black_box(content.clone());
                    black_box(cloned)
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("string_len", size),
            &content,
            |b, content| {
                b.iter(|| {
                    let len = black_box(content.len());
                    black_box(len)
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("string_contains", size),
            &content,
            |b, content| {
                b.iter(|| {
                    let contains = black_box(content.contains("test"));
                    black_box(contains)
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_memory_creation,
    bench_memory_serialization,
    bench_memory_cloning,
    bench_memory_class_operations,
    bench_uuid_generation,
    bench_memory_collections,
    bench_date_operations,
    bench_string_operations
);

criterion_main!(benches); 