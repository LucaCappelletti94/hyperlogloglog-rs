#![allow(missing_docs, clippy::print_stdout, clippy::decimal_bitwise_operands)]

use hyperlogloglog::{HyperLogLogLog4, HyperLogLogLog8};
use sketching_core::splitmix64;

#[test]
fn test_new_empty() {
    let sketch: HyperLogLogLog4 = HyperLogLogLog4::new();
    assert_eq!(sketch.base(), 0);
    assert_eq!(sketch.min_register(), 0);
    assert_eq!(sketch.sparse_len(), 0);
    assert_eq!(sketch.num_registers(), 16);
}

#[test]
fn test_single_insert() {
    let mut sketch: HyperLogLogLog4 = HyperLogLogLog4::new();
    sketch.update_with_hash(0x1234_5678_9ABC_DEF0u64);

    let estimate = sketch.estimate();
    assert!(
        estimate >= 1.0,
        "Estimate should be at least 1, got {estimate}"
    );
}

#[test]
fn test_idempotence() {
    let mut sketch: HyperLogLogLog4 = HyperLogLogLog4::new();
    let hash = 0xDEAD_BEEF_CAFE_BABEu64;

    sketch.update_with_hash(hash);
    let estimate1 = sketch.estimate();

    sketch.update_with_hash(hash);
    let estimate2 = sketch.estimate();

    assert!((estimate1 - estimate2).abs() < 1e-6);
}

#[test]
fn test_small_cardinality_accuracy() {
    let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..1000u64 {
        sketch.update_with_hash(splitmix64(i));
    }

    let estimate = sketch.estimate();
    let error = (estimate - 1000.0).abs() / 1000.0;

    assert!(
        error < 0.25,
        "Relative error {error:.3} too large for 1000 elements; estimate {estimate:.1}"
    );
}

#[test]
fn test_clear() {
    let mut sketch: HyperLogLogLog4 = HyperLogLogLog4::new();
    sketch.update_with_hash(42);
    sketch.clear();

    assert_eq!(sketch.base(), 0);
    assert_eq!(sketch.min_register(), 0);
    assert_eq!(sketch.sparse_len(), 0);
    assert!((sketch.estimate() - 0.0).abs() < 1.0);
}

#[test]
fn test_merge_increases_cardinality() {
    let mut sketch1: HyperLogLogLog8 = HyperLogLogLog8::new();
    let mut sketch2: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..5000u64 {
        sketch1.update_with_hash(splitmix64(i));
    }
    for i in 5000..10000u64 {
        sketch2.update_with_hash(splitmix64(i));
    }

    let est1 = sketch1.estimate();
    let est2 = sketch2.estimate();

    sketch1.merge(&sketch2);
    let est_merged = sketch1.estimate();

    assert!(est_merged >= est1, "Merged {est_merged} < est1 {est1}");
    assert!(est_merged >= est2, "Merged {est_merged} < est2 {est2}");

    let error = (est_merged - 10000.0).abs() / 10000.0;
    assert!(
        error < 0.25,
        "Merge error {error:.3}; estimate {est_merged:.1}"
    );
}

#[test]
fn test_bitsize() {
    let sketch: HyperLogLogLog4 = HyperLogLogLog4::new();
    let expected = 16 * 3;
    assert_eq!(sketch.bitsize(), expected);
}

#[test]
fn test_compression_reduces_sparse() {
    let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..100_000u64 {
        sketch.update_with_hash(
            i.wrapping_mul(6_364_136_223_846_793_005) ^ 1_442_695_040_888_963_407,
        );
    }

    let sparse_before = sketch.sparse_len();
    sketch.compress();
    let sparse_after = sketch.sparse_len();

    assert!(
        sparse_after < sparse_before,
        "Compression did not reduce sparse entries: {sparse_before} -> {sparse_after}"
    );
}

#[test]
fn test_splitmix_accuracy() {
    let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..1000u64 {
        sketch.update_with_hash(splitmix64(i));
    }

    let est = sketch.estimate();
    println!(
        "1000 splitmix: base={} sparse={} est={}",
        sketch.base(),
        sketch.sparse_len(),
        est
    );

    let error = (est - 1000.0).abs() / 1000.0;
    assert!(
        error < 0.25,
        "Relative error {:.3} too large; est={:.1}",
        error,
        est
    );
}

#[test]
fn test_splitmix_merge() {
    let mut s1: HyperLogLogLog8 = HyperLogLogLog8::new();
    let mut s2: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..5000u64 {
        s1.update_with_hash(splitmix64(i));
    }
    for i in 5000..10000u64 {
        s2.update_with_hash(splitmix64(i));
    }

    s1.merge(&s2);
    let est = s1.estimate();
    println!("merge 10000: est={:.1}", est);

    let error = (est - 10000.0).abs() / 10000.0;
    assert!(error < 0.25, "Merge error {:.3}; est={:.1}", error, est);
}

#[test]
fn test_10k_elements() {
    let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..10_000u64 {
        sketch.update_with_hash(splitmix64(i));
    }

    let est = sketch.estimate();
    println!(
        "10k: base={} sparse={} est={:.1}",
        sketch.base(),
        sketch.sparse_len(),
        est
    );

    let error = (est - 10_000.0).abs() / 10_000.0;
    assert!(error < 0.15, "Error {:.3}; est={:.1}", error, est);
}

#[test]
fn test_100k_elements() {
    let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..100_000u64 {
        sketch.update_with_hash(splitmix64(i));
    }

    let est = sketch.estimate();
    println!(
        "100k: base={} sparse={} est={:.1}",
        sketch.base(),
        sketch.sparse_len(),
        est
    );

    let error = (est - 100_000.0).abs() / 100_000.0;
    assert!(error < 0.10, "Error {:.3}; est={:.1}", error, est);
}

#[test]
fn test_1m_elements() {
    let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..1_000_000u64 {
        sketch.update_with_hash(splitmix64(i));
    }

    let est = sketch.estimate();
    let error = (est - 1_000_000.0).abs() / 1_000_000.0;
    println!(
        "1M: base={} sparse={} est={:.0} error={:.3}",
        sketch.base(),
        sketch.sparse_len(),
        est,
        error
    );
    assert!(error < 0.15, "Error {:.3} too large", error);
}

#[test]
fn test_to_and_from_registers() {
    let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..50_000u64 {
        sketch.update_with_hash(
            i.wrapping_mul(6_364_136_223_846_793_005) ^ 1_442_695_040_888_963_407,
        );
    }

    let estimate_before = sketch.estimate();
    let registers = sketch.to_registers();
    let mut recompressed: HyperLogLogLog8 = HyperLogLogLog8::from_registers(&registers);
    let estimate_after = recompressed.estimate();

    assert!(
        (estimate_before - estimate_after).abs() < 1e-10,
        "Round-trip estimate mismatch: {estimate_before} vs {estimate_after}"
    );
}

#[test]
fn test_compress_only() {
    let mut sketch: HyperLogLogLog4 = HyperLogLogLog4::new();
    sketch.update_with_hash(42u64);
    // Just compress, don't estimate
    sketch.compress();
}

#[test]
fn test_update_only() {
    let mut sketch: HyperLogLogLog4 = HyperLogLogLog4::new();
    sketch.update_with_hash(42u64);
}
