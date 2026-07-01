#![allow(
    dead_code,
    missing_docs,
    clippy::print_stdout,
    clippy::decimal_bitwise_operands,
    clippy::semicolon_if_nothing_returned
)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use hyperlogloglog::HyperLogLogLog8;
use sketching_core::splitmix64;

fn populate_sketch(n: u64) -> HyperLogLogLog8 {
    let mut sketch = HyperLogLogLog8::new();
    for i in 0..n {
        sketch.update_with_hash(splitmix64(i));
    }
    sketch
}

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");
    group.throughput(criterion::Throughput::Elements(1));

    for &n in &[1_000, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();
                for i in 0..n {
                    sketch.update_with_hash(splitmix64(i));
                }
                sketch
            })
        });
    }

    group.finish();
}

fn bench_estimate(c: &mut Criterion) {
    let mut group = c.benchmark_group("estimate");
    group.throughput(criterion::Throughput::Elements(1));

    for &n in &[1_000, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let sketch = populate_sketch(n);
            b.iter_batched(
                || sketch.clone(),
                |mut s| s.estimate(),
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge");
    group.throughput(criterion::Throughput::Elements(1));

    for &n in &[1_000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let s1 = populate_sketch(n);
            let s2 = populate_sketch(n);
            b.iter_batched(
                || (s1.clone(), s2.clone()),
                |(mut a, b)| a.merge(&b),
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_compress(c: &mut Criterion) {
    let mut group = c.benchmark_group("compress");
    group.throughput(criterion::Throughput::Elements(1));

    for &n in &[10_000, 100_000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let sketch = populate_sketch(n);
            b.iter_batched(
                || sketch.clone(),
                |mut s| s.compress(),
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_insert,
    bench_estimate,
    bench_merge,
    bench_compress
);
criterion_main!(benches);
