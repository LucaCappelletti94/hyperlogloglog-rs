//! Cardinality estimation for `HyperLogLog-log`.
//!
//! Implements the standard `HyperLogLog` estimator with bias correction:
//! linear counting for small cardinalities, the raw harmonic mean for
//! the dense regime, and the saturation correction for large ranges.
//! The correction constants match the `HyperLogLog` paper (Flajolet et al.).

use sketching_core::{FloatOps, Precision};

/// Returns the alpha\_m bias-correction constant for a given number of registers.
///
/// # Examples
///
/// ```
/// use hyperlogloglog::alpha_m;
/// assert!((alpha_m(16) - 0.673).abs() < 1e-6);
/// assert!((alpha_m(64) - 0.709).abs() < 1e-6);
/// ```
#[inline]
#[must_use]
pub fn alpha_m(m: u64) -> f64 {
    match m {
        16 => 0.673,
        32 => 0.697,
        64 => 0.709,
        _ => 0.7213 / (1.0 + 1.079 / m as f64),
    }
}

/// Computes the raw `HyperLogLog` estimate from a harmonic sum.
///
/// The harmonic sum is `sum(2^(-M[j]))` over all registers.
///
/// # Examples
///
/// ```
/// use hyperlogloglog::raw_estimate;
/// // All registers at value 1: harmonic_sum = m * 0.5
/// let est = raw_estimate(256, 128.0);
/// assert!(est > 0.0);
/// ```
#[inline]
#[must_use]
pub fn raw_estimate(m: u64, harmonic_sum: f64) -> f64 {
    alpha_m(m) * m as f64 * m as f64 / harmonic_sum
}

/// Applies bias correction to a raw `HyperLogLog` estimate.
///
/// Uses linear counting for small cardinalities (E <= 5m/32),
/// the saturation correction for large ranges (E > 2^32 / 30),
/// and returns the raw estimate otherwise.
///
/// # Examples
///
/// ```
/// use hyperlogloglog::corrected_estimate;
/// // Empty sketch: all registers zero
/// assert_eq!(corrected_estimate(256, 256.0, 256), 0.0);
/// ```
#[inline]
#[must_use]
pub fn corrected_estimate(m: u64, harmonic_sum: f64, zeros: u64) -> f64 {
    // Empty sketch: all registers are zero.
    if zeros == m {
        return 0.0;
    }

    let e = raw_estimate(m, harmonic_sum);

    // Linear counting for small cardinalities.
    if e <= (5.0 / 32.0) * m as f64 && zeros > 0 {
        return m as f64 * FloatOps::natural_log(m as f64 / zeros as f64);
    }

    // Saturation correction for large cardinalities.
    if e > (u64::MAX as f64) / 30.0 {
        return -(u64::MAX as f64) * FloatOps::natural_log(1.0 - e / (u64::MAX as f64));
    }

    e
}

/// Computes the harmonic sum and zero count from register values.
///
/// Takes a closure that yields each register's raw value.
#[inline]
#[must_use]
pub fn compute_harmonic_sum<F>(m: usize, mut get_register: F) -> (f64, u64)
where
    F: FnMut(usize) -> u8,
{
    let mut harmonic_sum = 0.0f64;
    let mut zeros = 0u64;

    for j in 0..m {
        let r = get_register(j);
        if r == 0 {
            zeros += 1;
        }
        harmonic_sum += 2.0f64.powi(-(r as i32));
    }

    (harmonic_sum, zeros)
}

/// Theoretical relative standard error for a given precision.
#[inline]
#[must_use]
pub fn relative_standard_error<P: Precision>() -> f64 {
    1.04 / FloatOps::sqrt((1 << P::EXPONENT) as f64)
}

/// Estimates cardinality from the compressed HyperLogLog-log representation.
///
/// This is the core estimation function. It computes the harmonic sum over
/// all registers (retrieved via the `get` function which handles sparse/dense
/// dispatch), then applies the standard `HyperLogLog` bias correction.
#[inline]
#[must_use]
pub fn estimate_cardinality<P: Precision, F>(mut get_register: F) -> f64
where
    F: FnMut(usize) -> u8,
{
    let m = 1 << P::EXPONENT;
    let m_usize = m as usize;

    let (harmonic_sum, zeros) = compute_harmonic_sum(m_usize, &mut get_register);
    corrected_estimate(m, harmonic_sum, zeros)
}
