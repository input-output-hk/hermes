//! `wasm::Module` benchmark
#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use criterion::{criterion_group, criterion_main, Criterion};

fn module_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("WASM module execution");
    group.bench_function("module_hermes_component_bench", |b| {
        hermes::module_hermes_component_bench(b)
    });

    group.bench_function("module_small_component_bench", |b| {
        hermes::module_small_component_bench(b)
    });

    group.bench_function("module_small_component_full_pre_load_bench", |b| {
        hermes::module_small_component_full_pre_load_bench(b)
    });
    group.finish();
}

criterion_group!(benches, module_benches);

criterion_main!(benches);
