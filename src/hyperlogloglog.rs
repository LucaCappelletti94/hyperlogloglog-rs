//! HyperLogLog-log: compressed `HyperLogLog` sketches.
//!
//! Compresses the `HyperLogLog` register array from `O(m * log log n)` bits down to
//! `m * kappa + |S| * (log m + 6)` bits by storing offsets from a base value `B`
//! in a dense packed array and overflowing registers in a sparse associative array.
//!
//! The compression parameter `kappa` controls the number of bits per dense register.
//! The paper recommends `kappa = 3` for practical use.

use crate::estimator::{compute_harmonic_sum, corrected_estimate};
use crate::sparse::SparseRegisters;
use core::fmt::Debug;
use core::hash::{Hash, Hasher as StdHasher};
#[allow(unused_imports)]
use sketching_core::prelude::{
    Bits3, FloatOps, Packed, Precision10, Precision4, Precision5, Precision6, Precision7,
    Precision8, Precision9,
};

/// Number of bits per dense register (kappa in the paper).
pub const KAPPA: u8 = 3;

/// Dense range width: `2^KAPPA`.
const DENSE_RANGE: u8 = 1 << KAPPA;

/// Trait for hasher types compatible with this crate.
pub trait HasherType: Default + StdHasher + Send + Sync + Clone {}

impl<T> HasherType for T where T: Default + StdHasher + Send + Sync + Clone {}

// ---------------------------------------------------------------------------
// Per-precision macro: generates the struct and impl block with concrete
// array sizes (avoids generic const expressions which are unstable).
// ---------------------------------------------------------------------------

macro_rules! impl_hlll {
    (
        $(
            $N:tt = $Prec:ident {
                exponent = $exp:expr,
                words = $words:expr,
                sparse_cap = $scap:expr
            }
        ),* $(,)?
    ) => {
        $(
            paste::paste! {
                /// HyperLogLog-log compressed cardinality estimator.
                ///
                /// Stores the `HyperLogLog` register array as a compressed 3-tuple `(S, M, B)`:
                /// - `B`: base value (the floor of the dense range)
                /// - `M`: dense packed array of offsets `M[j] = M'[j] - B` for registers in `[B, B + 2^KAPPA)`
                /// - `S`: sparse associative array of `(index, value)` pairs for registers outside the dense range
                ///
                /// The sketch is a drop-in replacement for `HyperLogLog`: it produces the same
                /// estimates, supports merge in the compressed domain, and can be converted
                /// to/from the uncompressed representation.
                #[derive(Clone, Debug)]
                pub struct [<HyperLogLogLog $N>]<H: HasherType = twox_hash::XxHash64> {
                    sparse: SparseRegisters<$scap>,
                    dense: Packed<[u64; $words], Bits3>,
                    base: u8,
                    min_register: u8,
                    _hasher: core::marker::PhantomData<H>,
                }

                impl<H: HasherType> Default for [<HyperLogLogLog $N>]<H> {
                    fn default() -> Self {
                        Self {
                            sparse: SparseRegisters::new(),
                            dense: Packed::default(),
                            base: 0,
                            min_register: 0,
                            _hasher: core::marker::PhantomData,
                        }
                    }
                }

                impl<H: HasherType> [<HyperLogLogLog $N>]<H> {
                    /// Creates a new empty sketch.
                    #[must_use]
                    pub fn new() -> Self {
                        Self::default()
                    }

                    /// Returns the number of registers (`2^$exp`).
                    #[inline]
                    #[must_use]
                    pub const fn num_registers(&self) -> u64 {
                        1 << $exp
                    }

                    /// Returns the number of registers as `usize`.
                    #[inline]
                    #[must_use]
                    pub const fn num_registers_usize(&self) -> usize {
                        self.num_registers() as usize
                    }

                    /// Returns the base value.
                    #[inline]
                    #[must_use]
                    pub const fn base(&self) -> u8 {
                        self.base
                    }

                    /// Returns the minimum register value.
                    #[inline]
                    #[must_use]
                    pub const fn min_register(&self) -> u8 {
                        self.min_register
                    }

                    /// Returns the number of sparse entries.
                    #[inline]
                    #[must_use]
                    pub fn sparse_len(&self) -> usize {
                        self.sparse.len()
                    }

                    /// Retrieves the raw register value at the given index.
                    #[inline]
                    #[must_use]
                    pub fn get_register(&self, index: usize) -> u8 {
                        if let Some(value) = self.sparse.get(index) {
                            value
                        } else {
                            self.base.saturating_add(self.dense.get(index))
                        }
                    }

                    /// Sets a register to a raw value, handling sparse/dense dispatch.
                    #[inline]
                    fn set_register(&mut self, index: usize, value: u8) {
                        if self.base <= value && value < self.base.saturating_add(DENSE_RANGE) {
                            let offset = value - self.base;
                            self.dense.set(index, offset);
                            self.sparse.remove(index);
                        } else {
                            let _ = self.sparse.insert(index, value);
                        }
                    }

                    /// Updates the minimum register value.
                    #[inline]
                    fn update_min_register(&mut self) {
                        let m = self.num_registers_usize();
                        let mut min_val = u8::MAX;
                        for j in 0..m {
                            let val = self.get_register(j);
                            if val < min_val {
                                min_val = val;
                            }
                        }
                        self.min_register = min_val;
                    }

                    /// Computes the total bit size of the compressed sketch.
                    #[inline]
                    #[must_use]
                    pub fn bitsize(&self) -> usize {
                        let m = self.num_registers_usize();
                        let log_m = (m as f64).log2() as usize;
                        let log_w = 6;
                        m * KAPPA as usize + self.sparse.len() * (log_m + log_w)
                    }

                    /// Finds the optimal base value that minimizes the total sketch size.
                    #[inline]
                    #[must_use]
                    fn find_optimal_base(&self) -> u8 {
                        let m = self.num_registers_usize();
                        let mut candidates: [u8; 64] = [0; 64];
                        let mut num_candidates = 0;

                        for j in 0..m {
                            let val = self.get_register(j);
                            let mut found = false;
                            for i in 0..num_candidates {
                                if candidates[i] == val {
                                    found = true;
                                    break;
                                }
                            }
                            if !found && num_candidates < candidates.len() {
                                candidates[num_candidates] = val;
                                num_candidates += 1;
                            }
                        }

                        for i in 1..num_candidates {
                            let key = candidates[i];
                            let mut j = i;
                            while j > 0 && candidates[j - 1] > key {
                                candidates[j] = candidates[j - 1];
                                j -= 1;
                            }
                            candidates[j] = key;
                        }

                        let mut best_base = self.base;
                        let mut best_sparse_count = self.sparse.len();

                        for ci in 0..num_candidates {
                            let candidate = candidates[ci];
                            let mut sparse_count = 0;

                            for j in 0..m {
                                let val = self.get_register(j);
                                if val < candidate || val >= candidate.saturating_add(DENSE_RANGE) {
                                    sparse_count += 1;
                                }
                                if sparse_count >= best_sparse_count {
                                    break;
                                }
                            }

                            if sparse_count < best_sparse_count {
                                best_sparse_count = sparse_count;
                                best_base = candidate;
                            }
                        }

                        best_base
                    }

                    /// Rebase: reassign all registers to the new base value.
                    /// Follows C++ reference: read values using old base,
                    /// check against new base, set base last.
                    #[inline]
                    fn rebase(&mut self, new_base: u8) {
                        if new_base == self.base {
                            return;
                        }
                        let m = self.num_registers_usize();
                        let max_offset = (1 << KAPPA) - 1;

                        // 1. Read all values using OLD base
                        let mut values: [u8; 1024] = [0; 1024];
                        for j in 0..m {
                            values[j] = self.get_register(j);
                        }

                        // 2. Clear storage
                        self.sparse.clear();
                        self.dense.clear();

                        // 3. Write values using NEW base
                        for j in 0..m {
                            let v = values[j];
                            if v >= new_base && v <= new_base + max_offset {
                                self.dense.set(j, v - new_base);
                            } else {
                                let _ = self.sparse.insert(j, v);
                            }
                        }

                        // 4. Set base LAST (after all writes)
                        self.base = new_base;
                    }

                    /// Compress: find and apply the optimal base value.
                    #[inline]
                    pub fn compress(&mut self) {
                        let optimal = self.find_optimal_base();
                        if optimal != self.base {
                            self.rebase(optimal);
                        }
                    }

                    /// Compress with heuristic: only try the next larger register value.
                    #[inline]
                    pub fn compress_heuristic(&mut self) {
                        let m = self.num_registers_usize();
                        let mut next_candidate = None;
                        for j in 0..m {
                            let val = self.get_register(j);
                            if val > self.base {
                                match next_candidate {
                                    None => next_candidate = Some(val),
                                    Some(current) if val < current => next_candidate = Some(val),
                                    _ => {}
                                }
                            }
                        }

                        if let Some(candidate) = next_candidate {
                            let mut candidate_sparse = 0;
                            let mut current_sparse = 0;
                            for j in 0..m {
                                let val = self.get_register(j);
                                if val < self.base || val >= self.base.saturating_add(DENSE_RANGE) {
                                    current_sparse += 1;
                                }
                                if val < candidate || val >= candidate.saturating_add(DENSE_RANGE) {
                                    candidate_sparse += 1;
                                }
                            }
                            if candidate_sparse < current_sparse {
                                self.rebase(candidate);
                            }
                        }
                    }

                    /// Updates the sketch with a raw 64-bit hash value.
                    #[inline]
                    pub fn update_with_hash(&mut self, hash: u64) {
                        let m = self.num_registers_usize();
                        let log_m = $exp as u32;

                        let index = (hash >> (64 - log_m)) as usize;
                        debug_assert!(index < m);

                        let remaining = hash << log_m;
                        let rho = if remaining == 0 {
                            64u8
                        } else {
                            remaining.leading_zeros() as u8 + 1
                        };

                        if rho <= self.min_register {
                            return;
                        }

                        let current = self.get_register(index);
                        if rho > current {
                            self.set_register(index, rho);
                            if current == self.min_register {
                                self.update_min_register();
                            }
                        }
                    }

                    /// Estimates the cardinality.
                    #[inline]
                    #[must_use]
                    pub fn estimate(&mut self) -> f64 {
                        self.compress();
                        let m = self.num_registers();
                        let (harmonic_sum, zeros) = compute_harmonic_sum(
                            self.num_registers_usize(),
                            |j| self.get_register(j),
                        );
                        corrected_estimate(m, harmonic_sum, zeros)
                    }

                    /// Merges another sketch into this one.
                    #[inline]
                    pub fn merge(&mut self, other: &Self) {
                        let m = self.num_registers_usize();
                        let mut merged = Self::new();
                        merged.base = if self.base > other.base {
                            self.base
                        } else {
                            other.base
                        };
                        for j in 0..m {
                            let r1 = self.get_register(j);
                            let r2 = other.get_register(j);
                            let r = if r1 > r2 { r1 } else { r2 };
                            merged.set_register(j, r);
                        }
                        merged.update_min_register();
                        merged.compress();
                        self.sparse = merged.sparse;
                        self.dense = merged.dense;
                        self.base = merged.base;
                        self.min_register = merged.min_register;
                    }

                    /// Clears the sketch.
                    #[inline]
                    pub fn clear(&mut self) {
                        self.sparse.clear();
                        self.dense.clear();
                        self.base = 0;
                        self.min_register = 0;
                    }

                    /// Converts to an uncompressed register array.
                    #[must_use]
                    pub fn to_registers(&self) -> [u8; 1 << $exp] {
                        let mut registers = [0u8; 1 << $exp];
                        for j in 0..(1 << $exp) {
                            registers[j] = self.get_register(j);
                        }
                        registers
                    }

                    /// Creates a compressed sketch from an uncompressed register array.
                    #[must_use]
                    pub fn from_registers(registers: &[u8]) -> Self {
                        debug_assert_eq!(registers.len(), 1 << $exp);
                        let mut sketch = Self::new();
                        let m = registers.len();
                        for j in 0..m {
                            sketch.set_register(j, registers[j]);
                        }
                        sketch.update_min_register();
                        sketch.compress();
                        sketch
                    }
                }

                impl<H: HasherType> PartialEq for [<HyperLogLogLog $N>]<H> {
                    fn eq(&self, other: &Self) -> bool {
                        self.sparse == other.sparse
                            && self.dense == other.dense
                            && self.base == other.base
                            && self.min_register == other.min_register
                    }
                }

                impl<H: HasherType> Eq for [<HyperLogLogLog $N>]<H> {}

                impl<H: HasherType> Hash for [<HyperLogLogLog $N>]<H> {
                    fn hash<HASHER: StdHasher>(&self, state: &mut HASHER) {
                        self.sparse.hash(state);
                        self.dense.hash(state);
                        self.base.hash(state);
                        self.min_register.hash(state);
                    }
                }

                impl<H: HasherType> sketching_core::CardinalityEstimator
                    for [<HyperLogLogLog $N>]<H>
                {
                    fn estimate_cardinality(&self) -> f64 {
                        let mut clone = self.clone();
                        clone.estimate()
                    }

                    fn estimate_union_cardinality(&self, other: &Self) -> f64 {
                        let mut clone = self.clone();
                        clone.merge(other);
                        clone.estimate()
                    }
                }

                impl<H: HasherType> sketching_core::HyperSpheresSketch
                    for [<HyperLogLogLog $N>]<H>
                {
                }
            }
        )+
    };
}

// Instantiate for P4-P10 (stack-allocated sparse arrays).
// words = ceil(m * 3 / 64), sparse_cap = m.
impl_hlll! {
    4   = Precision4  { exponent = 4,  words = 1,  sparse_cap = 16    },
    5   = Precision5  { exponent = 5,  words = 2,  sparse_cap = 32    },
    6   = Precision6  { exponent = 6,  words = 3,  sparse_cap = 64    },
    7   = Precision7  { exponent = 7,  words = 6,  sparse_cap = 128   },
    8   = Precision8  { exponent = 8,  words = 12, sparse_cap = 256   },
    9   = Precision9  { exponent = 9,  words = 24, sparse_cap = 512   },
    10  = Precision10 { exponent = 10, words = 48, sparse_cap = 1024  },
}
