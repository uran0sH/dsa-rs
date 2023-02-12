use criterion::{criterion_group, criterion_main, Criterion};
use dsa_rs::concurrent_lru::LRUCache;
use rand::prelude::*;

fn criterion_benchmark(c: &mut Criterion) {
    let cache: LRUCache<i32> = LRUCache::new(10);
    c.bench_function("insert", |b| {
        b.iter(|| {
            let mut rng = rand::thread_rng();
            let i: i32 = rng.gen::<i32>() % 10;
            cache.insert(&i.to_le_bytes(), i);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
