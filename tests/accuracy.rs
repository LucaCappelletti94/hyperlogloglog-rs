#![allow(
    dead_code,
    missing_docs,
    clippy::print_stdout,
    clippy::unwrap_used,
    clippy::decimal_bitwise_operands
)]

use hyperlogloglog::HyperLogLogLog8;
use sketching_core::splitmix64;

fn measure_accuracy(n: u64, trials: usize) -> (f64, f64, f64) {
    let mut errors = Vec::new();

    for trial in 0..trials {
        let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();
        for i in 0..n {
            let seed = (trial as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let hash = splitmix64(i.wrapping_add(seed));
            sketch.update_with_hash(hash);
        }
        let estimate = sketch.estimate();
        let error_pct = (estimate - n as f64) / n as f64 * 100.0;
        errors.push(error_pct);
    }

    let mean = errors.iter().sum::<f64>() / errors.len() as f64;
    let variance = errors.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / errors.len() as f64;
    let std = variance.sqrt();
    let max_abs = errors.iter().map(|e| e.abs()).fold(0.0_f64, f64::max);

    (mean, std, max_abs)
}

#[test]
fn test_accuracy_1k() {
    let (mean, std, max_abs) = measure_accuracy(1_000, 30);
    println!(
        "n=1000: mean={:.2}, std={:.2}, max_err={:.2}",
        mean, std, max_abs
    );
    assert!(mean.abs() < 5.0, "Mean error too large for n=1k: {}", mean);
}

#[test]
fn test_accuracy_10k() {
    let (mean, std, max_abs) = measure_accuracy(10_000, 30);
    println!(
        "n=10000: mean={:.2}, std={:.2}, max_err={:.2}",
        mean, std, max_abs
    );
    assert!(mean.abs() < 3.0, "Mean error too large for n=10k: {}", mean);
}

#[test]
fn test_accuracy_100k() {
    let (mean, std, max_abs) = measure_accuracy(100_000, 30);
    println!(
        "n=100000: mean={:.2}, std={:.2}, max_err={:.2}",
        mean, std, max_abs
    );
    assert!(
        mean.abs() < 3.0,
        "Mean error too large for n=100k: {}",
        mean
    );
}

#[test]
fn test_accuracy_1m() {
    let (mean, std, max_abs) = measure_accuracy(1_000_000, 20);
    println!(
        "n=1000000: mean={:.2}, std={:.2}, max_err={:.2}",
        mean, std, max_abs
    );
    assert!(mean.abs() < 5.0, "Mean error too large for n=1M: {}", mean);
}

#[test]
fn test_accuracy_10m() {
    let (mean, std, max_abs) = measure_accuracy(10_000_000, 10);
    println!(
        "n=10000000: mean={:.2}, std={:.2}, max_err={:.2}",
        mean, std, max_abs
    );
    assert!(mean.abs() < 5.0, "Mean error too large for n=10M: {}", mean);
}
