#![allow(dead_code, missing_docs, clippy::print_stdout, clippy::unwrap_used)]

use hyperlogloglog::{CardinalityEstimator, HyperLogLogLog8};
use sketching_core::{splitmix64, HyperSpheresSketch};

#[test]
fn test_cardinality_estimator_trait() {
    let mut sketch1: HyperLogLogLog8 = HyperLogLogLog8::new();
    let mut sketch2: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..10_000u64 {
        sketch1.update_with_hash(splitmix64(i));
    }
    for i in 5_000..15_000u64 {
        sketch2.update_with_hash(splitmix64(i));
    }

    // Use trait methods
    let card1 = sketch1.estimate_cardinality();
    let card2 = sketch2.estimate_cardinality();
    let union_card = sketch1.estimate_union_cardinality(&sketch2);
    let intersection_card = sketch1.estimate_intersection_cardinality(&sketch2);

    println!(
        "Card1: {}, Card2: {}, Union: {}, Intersection: {}",
        card1, card2, union_card, intersection_card
    );

    assert!(card1 > 7_000.0 && card1 < 13_000.0);
    assert!(card2 > 9_000.0 && card2 < 15_000.0);
    assert!(union_card > 14_000.0 && union_card < 20_000.0);
    assert!(intersection_card > 2_000.0 && intersection_card < 8_000.0);
}

#[test]
fn test_hyper_spheres_trait() {
    let mut sketch1: HyperLogLogLog8 = HyperLogLogLog8::new();
    let mut sketch2: HyperLogLogLog8 = HyperLogLogLog8::new();

    for i in 0..10_000u64 {
        sketch1.update_with_hash(splitmix64(i));
    }
    for i in 5_000..15_000u64 {
        sketch2.update_with_hash(splitmix64(i));
    }

    // Use HyperSpheresSketch trait
    let lefts = [sketch1.clone()];
    let rights = [sketch2.clone()];
    let joint = HyperLogLogLog8::joint_sketch(&lefts, &rights);

    println!("Joint overlap: {:?}", joint.overlap);

    assert!(joint.overlap[0][0] > 3_000.0);
}
