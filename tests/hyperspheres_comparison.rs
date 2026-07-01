#![allow(
    dead_code,
    missing_docs,
    clippy::print_stdout,
    clippy::unwrap_used,
    clippy::decimal_bitwise_operands,
    clippy::similar_names,
    clippy::clone_on_copy
)]

use hyperlogloglog::HyperLogLogLog8;
use sketching_core::{splitmix64, CardinalityEstimator, HyperSpheresSketch};

/// Verify `CardinalityEstimator` trait produces correct results.
#[test]
fn test_cardinality_estimator_trait() {
    let n = 100_000;
    let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..n {
        sketch.update_with_hash(splitmix64(i));
    }

    let card = sketch.estimate_cardinality();
    println!("HLL-log cardinality: {}", card);

    assert!(card > n as f64 * 0.9 && card < n as f64 * 1.1);
}

/// Verify union/intersection estimates against ground truth.
#[test]
fn test_union_intersection_ground_truth() {
    let mut s1: HyperLogLogLog8 = HyperLogLogLog8::new();
    let mut s2: HyperLogLogLog8 = HyperLogLogLog8::new();

    // Set 1: 0..100k, Set 2: 50k..150k (50% overlap)
    for i in 0..100_000 {
        s1.update_with_hash(splitmix64(i));
    }
    for i in 50_000..150_000 {
        s2.update_with_hash(splitmix64(i));
    }

    let union_card = s1.estimate_union_cardinality(&s2);
    let intersect_card = s1.estimate_intersection_cardinality(&s2);
    let jaccard = s1.estimate_jaccard_index(&s2);

    println!(
        "Union: {}, Intersect: {}, Jaccard: {}",
        union_card, intersect_card, jaccard
    );

    // True: union=150k, intersect=50k, jaccard=0.333
    let true_union = 150_000.0;
    let true_intersect = 50_000.0;
    let true_jaccard = 1.0 / 3.0;

    assert!(
        union_card > true_union * 0.8 && union_card < true_union * 1.2,
        "Union out of range"
    );
    assert!(
        intersect_card > true_intersect * 0.5 && intersect_card < true_intersect * 1.5,
        "Intersection out of range"
    );
    assert!(
        (jaccard - true_jaccard).abs() < 0.15,
        "Jaccard too far from expected"
    );
}

/// Verify joint sketch overlap matches intersection estimate.
#[test]
fn test_joint_sketch_consistency() {
    let mut s1: HyperLogLogLog8 = HyperLogLogLog8::new();
    let mut s2: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..100_000 {
        s1.update_with_hash(splitmix64(i));
    }
    for i in 50_000..150_000 {
        s2.update_with_hash(splitmix64(i));
    }

    let joint = HyperLogLogLog8::joint_sketch(&[s1.clone()], &[s2.clone()]);
    let intersect = s1.estimate_intersection_cardinality(&s2);

    println!(
        "Joint overlap: {}, Intersection: {}",
        joint.overlap[0][0], intersect
    );

    // Joint overlap[0][0] should equal intersection estimate
    assert!(
        (joint.overlap[0][0] - intersect).abs() < intersect * 0.01,
        "Joint overlap differs from intersection"
    );

    // Verify left_diff and right_diff are reasonable
    // |A \ B| = |A| - |AnB| = 100k - 50k = 50k
    // |B \ A| = |B| - |AnB| = 100k - 50k = 50k
    let true_diff = 50_000.0;
    assert!(
        joint.left_diff[0] > true_diff * 0.5 && joint.left_diff[0] < true_diff * 1.5,
        "Left diff out of range"
    );
    assert!(
        joint.right_diff[0] > true_diff * 0.5 && joint.right_diff[0] < true_diff * 1.5,
        "Right diff out of range"
    );
}
