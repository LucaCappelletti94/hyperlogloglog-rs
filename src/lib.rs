#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]
#![deny(missing_docs)]
#![deny(unsafe_code)]
#![allow(dead_code)]
#![cfg_attr(
    test,
    allow(
        clippy::print_stdout,
        clippy::unwrap_used,
        clippy::decimal_bitwise_operands
    )
)]

mod estimator;
mod hyperlogloglog;
pub mod sparse;

pub use estimator::{
    alpha_m, corrected_estimate, estimate_cardinality, raw_estimate, relative_standard_error,
};
pub use hyperlogloglog::{
    HasherType, HyperLogLogLog10, HyperLogLogLog4, HyperLogLogLog5, HyperLogLogLog6,
    HyperLogLogLog7, HyperLogLogLog8, HyperLogLogLog9, KAPPA,
};
pub use sketching_core::prelude::{
    Bits, Bits3, Bits4, Bits5, Bits6, Bits7, Bits8, Precision, Precision10, Precision11,
    Precision12, Precision13, Precision14, Precision15, Precision16, Precision17, Precision18,
    Precision4, Precision5, Precision6, Precision7, Precision8, Precision9,
};
pub use sketching_core::CardinalityEstimator;
pub use sketching_core::{FloatOps, Number, One, Packed, PositiveInteger, VariableWord, Zero};

/// Re-exports of the most important traits and structs.
pub mod prelude {
    pub use crate::estimator::{corrected_estimate, estimate_cardinality, relative_standard_error};
    pub use crate::hyperlogloglog::{
        HasherType, HyperLogLogLog10, HyperLogLogLog4, HyperLogLogLog5, HyperLogLogLog6,
        HyperLogLogLog7, HyperLogLogLog8, HyperLogLogLog9, KAPPA,
    };
    pub use sketching_core::prelude::*;
}
