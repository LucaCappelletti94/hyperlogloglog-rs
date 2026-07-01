#![allow(missing_docs)]
use hyperlogloglog::HyperLogLogLog8;
use proptest::prelude::*;
use sketching_core::splitmix64;

proptest! {
    #[test]
    fn test_idempotence(hash in any::<u64>()) {
        let mut sketch_once: HyperLogLogLog8 = HyperLogLogLog8::new();
        sketch_once.update_with_hash(hash);
        let est_once = sketch_once.estimate();

        let mut sketch_twice: HyperLogLogLog8 = HyperLogLogLog8::new();
        sketch_twice.update_with_hash(hash);
        sketch_twice.update_with_hash(hash);
        let est_twice = sketch_twice.estimate();

        prop_assert_eq!(est_once, est_twice);
    }

    #[test]
    fn test_monotonicity(cardinality in 1..=10000u64, extra in any::<u64>()) {
        let mut sketch: HyperLogLogLog8 = HyperLogLogLog8::new();

        for i in 0..cardinality {
            sketch.update_with_hash(splitmix64(i));
        }
        let est_before = sketch.estimate();

        sketch.update_with_hash(extra);
        let est_after = sketch.estimate();

        prop_assert!(
            est_after >= est_before,
            "estimate decreased from {est_before} to {est_after} after adding element"
        );
    }
}
