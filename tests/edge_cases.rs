#![allow(
    missing_docs,
    clippy::print_stdout,
    clippy::decimal_bitwise_operands,
    clippy::unwrap_used
)]

use core::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use hyperlogloglog::{estimate_cardinality, relative_standard_error, HyperLogLogLog8, Precision8};
use sketching_core::splitmix64;

#[test]
fn test_compress_heuristic() {
    let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();

    // Insert enough elements to populate registers across a range
    for i in 0..500u64 {
        sketch.update_with_hash(splitmix64(i));
    }

    let sparse_before = sketch.sparse_len();

    // Compress with the heuristic
    sketch.compress_heuristic();

    let sparse_after = sketch.sparse_len();

    // The heuristic should not increase the number of sparse entries
    assert!(
        sparse_after <= sparse_before,
        "compress_heuristic increased sparse entries: {} -> {}",
        sparse_before,
        sparse_after
    );

    // Estimate should remain stable after heuristic compression
    let est_after = sketch.estimate();
    assert!(
        est_after > 0.0,
        "estimate should be positive after compress_heuristic"
    );
}

#[test]
fn test_for_each_sparse() {
    use hyperlogloglog::sparse::SparseRegisters;

    let mut sparse = SparseRegisters::<16>::new();

    // Insert entries out of order
    sparse.insert(10, 5).unwrap();
    sparse.insert(2, 3).unwrap();
    sparse.insert(7, 8).unwrap();
    sparse.insert(0, 1).unwrap();

    let mut visited = Vec::new();
    sparse.for_each(|index, value| {
        visited.push((index, value));
    });

    // Should visit exactly the inserted entries
    assert_eq!(visited.len(), sparse.len());

    // Entries should be visited in sorted index order
    assert_eq!(visited, vec![(0, 1), (2, 3), (7, 8), (10, 5)]);
}

#[test]
fn test_partial_eq() {
    let mut sketch1: HyperLogLogLog8 = HyperLogLogLog8::new();
    let mut sketch2: HyperLogLogLog8 = HyperLogLogLog8::new();

    // Insert the same elements into both
    for i in 0..1000u64 {
        sketch1.update_with_hash(splitmix64(i));
        sketch2.update_with_hash(splitmix64(i));
    }

    // Compress both to ensure identical internal state
    sketch1.compress();
    sketch2.compress();

    assert_eq!(sketch1, sketch2, "sketches with same input should be equal");

    // Create a third sketch with different elements to verify inequality
    let mut sketch3: HyperLogLogLog8 = HyperLogLogLog8::new();
    for i in 50000..51000u64 {
        sketch3.update_with_hash(splitmix64(i));
    }
    sketch3.compress();

    assert_ne!(
        sketch1, sketch3,
        "sketches with different input should not be equal"
    );
}

#[test]
fn test_hash_consistency() {
    fn hash_sketch(sketch: &HyperLogLogLog8) -> u64 {
        let mut hasher = DefaultHasher::new();
        sketch.hash(&mut hasher);
        hasher.finish()
    }

    let mut sketch1: HyperLogLogLog8 = HyperLogLogLog8::new();
    let mut sketch2: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..1000u64 {
        sketch1.update_with_hash(splitmix64(i));
        sketch2.update_with_hash(splitmix64(i));
    }

    sketch1.compress();
    sketch2.compress();

    assert_eq!(
        hash_sketch(&sketch1),
        hash_sketch(&sketch2),
        "equal sketches should produce equal hashes"
    );

    // Different sketch should produce a different hash
    let mut sketch3: HyperLogLogLog8 = HyperLogLogLog8::new();
    sketch3.update_with_hash(splitmix64(99999));
    sketch3.compress();

    assert_ne!(
        hash_sketch(&sketch1),
        hash_sketch(&sketch3),
        "different sketches should produce different hashes"
    );
}

#[test]
fn test_estimate_cardinality() {
    // Build a sketch and use the standalone estimator function
    let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();

    let cardinality = 10_000u64;
    for i in 0..cardinality {
        sketch.update_with_hash(splitmix64(i));
    }

    sketch.compress();

    // Use the standalone estimate_cardinality function
    let estimate = estimate_cardinality::<Precision8, _>(|j| sketch.get_register(j));

    // Should be within 10% of actual cardinality
    let error = ((estimate - cardinality as f64) / cardinality as f64).abs();
    assert!(
        error < 0.10,
        "estimate {:.0} vs actual {} (error {:.2}%)",
        estimate,
        cardinality,
        error * 100.0
    );
}

#[test]
fn test_relative_standard_error() {
    // P8 has 2^8 = 256 registers
    let rse = relative_standard_error::<Precision8>();

    // Expected: 1.04 / sqrt(256) = 1.04 / 16 = 0.065
    let expected = 1.04 / 16.0;

    assert!(
        (rse - expected).abs() < 1e-10,
        "RSE for P8: got {}, expected {}",
        rse,
        expected
    );
}

#[test]
fn test_saturation_correction() {
    use hyperlogloglog::corrected_estimate;

    // harmonic_sum=5e-15 produces raw_estimate ~9e18 (below u64::MAX).
    let result = corrected_estimate(256, 5e-15, 0);
    assert!(result.is_finite(), "Result should be finite: {}", result);
    assert!(result > 1e17, "Saturation branch not triggered: {}", result);
}
