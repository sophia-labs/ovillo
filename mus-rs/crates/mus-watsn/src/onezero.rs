//! Exact storage and dissipation for the one-zero loop-damping filter
//! `y = (1-d)*u + d*x1`, `x1' = u`.
//!
//! The production string's frequency-dependent loop loss is this filter.
//! Unlike the allpass it is genuinely lossy, so a work ledger needs its
//! storage function and its exact per-sample dissipation, not a bound.
//!
//! Seek `V(x1) = c*x1^2/2` making the balance
//! `y^2/2 + V(x1') - u^2/2 - V(x1)` a negative semidefinite quadratic
//! form in `(u, x1)`. Writing the form out and requiring the `x1^2`
//! coefficient and the discriminant to close simultaneously forces
//! `c = d`, and then the form collapses to a perfect square:
//!
//! ```text
//! y^2/2 + d*x1'^2/2 = u^2/2 + d*x1^2/2 - d*(1-d)*(u - x1)^2/2.
//! ```
//!
//! So the declared storage is `E_z(x1) = d*x1^2/2` and the dissipation is
//! exactly `d*(1-d)*(u - x1)^2/2` — zero when `d` is 0 or 1, and zero
//! whenever the filter's input equals its memory (a DC-flat instant).
//! This is an identity, not an estimate: no residual is hidden in it.

/// Declared internal storage of the one-zero damping filter.
pub fn onezero_storage_energy(damp: f64, x1: f64) -> f64 {
    0.5 * damp * x1 * x1
}

/// Energy dissipated by one sample of the one-zero damping filter, given
/// the incoming sample `u` and the filter memory `x1` it is about to
/// replace. Non-negative for `damp` in `[0, 1]`.
pub fn onezero_dissipation(damp: f64, u: f64, x1: f64) -> f64 {
    let difference = u - x1;
    0.5 * damp * (1.0 - damp) * difference * difference
}

/// One sample of the filter, returning `(y, x1')`. Provided so the law
/// above can be tested against the realization it describes.
pub fn onezero_tick(damp: f64, u: f64, x1: f64) -> (f64, f64) {
    ((1.0 - damp) * u + damp * x1, u)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_balance_is_an_identity() {
        for damp in [0.0, 0.05, 0.35, 0.5, 0.9, 0.95, 1.0] {
            for u in [-1.7, -0.3, 0.0, 0.44, 2.1] {
                for x1 in [-2.2, -0.11, 0.0, 0.9, 1.3] {
                    let (y, x1_next) = onezero_tick(damp, u, x1);
                    let before = 0.5 * u * u + onezero_storage_energy(damp, x1);
                    let after = 0.5 * y * y + onezero_storage_energy(damp, x1_next);
                    let dissipated = onezero_dissipation(damp, u, x1);
                    assert!(
                        (before - after - dissipated).abs() <= before.max(1.0) * 4.0e-16,
                        "damp={damp} u={u} x1={x1}: before={before} after={after} loss={dissipated}"
                    );
                }
            }
        }
    }

    #[test]
    fn dissipation_is_non_negative_over_the_production_range() {
        // The production clamp is `damp` in 0..0.95.
        for step in 0..=95 {
            let damp = step as f64 / 100.0;
            for u in [-3.0, -0.5, 0.0, 0.75, 4.0] {
                for x1 in [-1.0, 0.0, 1.0] {
                    assert!(onezero_dissipation(damp, u, x1) >= 0.0);
                }
            }
        }
    }

    #[test]
    fn a_flat_instant_dissipates_nothing() {
        assert_eq!(onezero_dissipation(0.42, 0.31, 0.31), 0.0);
        // Both hard limits of `damp` are lossless for a different reason:
        // at 0 the filter is a wire, at 1 it is a pure unit delay.
        assert_eq!(onezero_dissipation(0.0, 1.0, -1.0), 0.0);
        assert_eq!(onezero_dissipation(1.0, 1.0, -1.0), 0.0);
    }
}
