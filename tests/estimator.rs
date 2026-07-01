#![allow(missing_docs)]
use hyperlogloglog::{alpha_m, corrected_estimate, raw_estimate};

#[test]
fn test_alpha_m_constants() {
    assert!((alpha_m(16) - 0.673).abs() < 1e-10);
    assert!((alpha_m(32) - 0.697).abs() < 1e-10);
    assert!((alpha_m(64) - 0.709).abs() < 1e-10);
    let alpha_128 = 0.7213 / (1.0 + 1.079 / 128.0);
    assert!((alpha_m(128) - alpha_128).abs() < 1e-10);
}

#[test]
fn test_empty_sketch() {
    let m = 1024;
    let result = corrected_estimate(m, m as f64, m);
    assert!((result - 0.0).abs() < 1e-6);
}

#[test]
fn test_raw_estimate() {
    let m = 64;
    let harmonic_sum = m as f64 * 0.5;
    let estimate = raw_estimate(m, harmonic_sum);
    let expected = alpha_m(m) * 2.0 * m as f64;

    assert!((estimate - expected).abs() < 1e-6);
}

#[test]
fn test_saturation_correction_branch() {
    // Trigger saturation correction with a large raw estimate.
    // harmonic_sum=5e-15 produces raw_estimate ~9e18 (below u64::MAX).
    let m = 256;
    let harmonic_sum = 5e-15;
    let zeros = 0;
    let result = corrected_estimate(m, harmonic_sum, zeros);
    assert!(result.is_finite(), "Result should be finite: {}", result);
    assert!(result > 1e17, "Saturation branch not triggered: {}", result);
}
