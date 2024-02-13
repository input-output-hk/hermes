//! `wasm::Module` benchmark
#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use criterion::{criterion_group, criterion_main, Criterion};

fn module_benches(c: &mut Criterion) {
    c.bench_function("module_bench", |_b| {});
}

criterion_group!(benches, module_benches);

criterion_main!(benches);
