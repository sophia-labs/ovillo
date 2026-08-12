//! Exact sparse Clifford gesture compiler for the Extension-v1 reference.
//!
//! This deliberately small crate lowers the four basis gestures of `Cl(0, 4)`
//! to the existing `mus_watsn::OperatorProgram` representation. It is staged
//! under the experiment rather than registered in the workspace so the first
//! landing does not alter production crate membership.

use std::f64::consts::FRAC_PI_2;

use mus_watsn::{OperatorProgram, PlaneRotation, ProgramError};

pub const LATENT_DIMENSION: usize = 4;
pub const STATE_DIMENSION: usize = 1 << LATENT_DIMENSION;
pub const FACTORS_PER_BASIS_GESTURE: usize = STATE_DIMENSION / 2;
pub const VACUUM_BLADE: usize = 0;
pub const PSEUDOSCALAR_BLADE: usize = STATE_DIMENSION - 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionError {
    AxisOutOfRange { axis: usize },
    BladeOutOfRange { blade: usize },
    Program(ProgramError),
}

impl From<ProgramError> for ExtensionError {
    fn from(value: ProgramError) -> Self {
        Self::Program(value)
    }
}

/// Exterior grade of a basis blade encoded as a bit mask.
pub fn blade_grade(mask: usize) -> u32 {
    mask.count_ones()
}

/// Sign of right exterior multiplication `e_mask wedge e_axis`.
///
/// The blade basis is ordered by ascending axis. Appending `axis` on the right
/// crosses every existing basis vector with index greater than `axis`.
pub fn right_wedge_sign(mask_without_axis: usize, axis: usize) -> i8 {
    let greater = (mask_without_axis >> (axis + 1)).count_ones();
    if greater % 2 == 0 { 1 } else { -1 }
}

/// The eight disjoint signed quarter-turns implementing one basis gesture.
pub fn basis_gesture_factors(axis: usize) -> Result<Vec<PlaneRotation>, ExtensionError> {
    if axis >= LATENT_DIMENSION {
        return Err(ExtensionError::AxisOutOfRange { axis });
    }

    let bit = 1 << axis;
    let mut factors = Vec::with_capacity(FACTORS_PER_BASIS_GESTURE);
    for mask in 0..STATE_DIMENSION {
        if mask & bit != 0 {
            continue;
        }
        let paired = mask | bit;
        let sign = f64::from(right_wedge_sign(mask, axis));
        factors.push(PlaneRotation {
            left: mask,
            right: paired,
            angle: sign * FRAC_PI_2,
        });
    }
    debug_assert_eq!(factors.len(), FACTORS_PER_BASIS_GESTURE);
    Ok(factors)
}

/// Compile one latent basis gesture directly into the WATSN sparse executor IR.
pub fn basis_gesture_program(axis: usize) -> Result<OperatorProgram, ExtensionError> {
    Ok(OperatorProgram::new(
        STATE_DIMENSION,
        basis_gesture_factors(axis)?,
    )?)
}

pub fn vacuum_state() -> [f64; STATE_DIMENSION] {
    let mut state = [0.0; STATE_DIMENSION];
    state[VACUUM_BLADE] = 1.0;
    state
}

pub fn basis_blade_state(blade: usize) -> Result<[f64; STATE_DIMENSION], ExtensionError> {
    if blade >= STATE_DIMENSION {
        return Err(ExtensionError::BladeOutOfRange { blade });
    }
    let mut state = [0.0; STATE_DIMENSION];
    state[blade] = 1.0;
    Ok(state)
}

/// Apply a word of basis gestures in authored sequence order.
pub fn apply_basis_word(word: &[usize]) -> Result<[f64; STATE_DIMENSION], ExtensionError> {
    let mut state = vacuum_state();
    for &axis in word {
        basis_gesture_program(axis)?.apply(&mut state)?;
    }
    Ok(state)
}

pub fn euclidean_energy(state: &[f64]) -> f64 {
    state.iter().map(|value| value * value).sum()
}

pub fn permutation_parity(permutation: &[usize]) -> usize {
    let mut inversions = 0;
    for left in 0..permutation.len() {
        for right in (left + 1)..permutation.len() {
            if permutation[left] > permutation[right] {
                inversions += 1;
            }
        }
    }
    inversions % 2
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 2.0e-14;

    fn assert_state_close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
            assert!(
                (actual - expected).abs() <= TOLERANCE,
                "coordinate {index}: actual={actual}, expected={expected}"
            );
        }
    }

    fn apply_axis(state: &mut [f64; STATE_DIMENSION], axis: usize) {
        basis_gesture_program(axis).unwrap().apply(state).unwrap();
    }

    #[test]
    fn grade_layout_is_one_four_six_four_one() {
        let mut counts = [0_usize; LATENT_DIMENSION + 1];
        for blade in 0..STATE_DIMENSION {
            counts[blade_grade(blade) as usize] += 1;
        }
        assert_eq!(counts, [1, 4, 6, 4, 1]);
    }

    #[test]
    fn each_axis_compiles_to_eight_disjoint_local_factors() {
        for axis in 0..LATENT_DIMENSION {
            let factors = basis_gesture_factors(axis).unwrap();
            assert_eq!(factors.len(), FACTORS_PER_BASIS_GESTURE);
            let mut seen = [false; STATE_DIMENSION];
            for factor in factors {
                assert!(!seen[factor.left]);
                assert!(!seen[factor.right]);
                seen[factor.left] = true;
                seen[factor.right] = true;
                assert!((factor.angle.abs() - FRAC_PI_2).abs() <= f64::EPSILON);
            }
            assert!(seen.into_iter().all(|value| value));
        }
    }

    #[test]
    fn every_basis_generator_squares_to_minus_identity() {
        for axis in 0..LATENT_DIMENSION {
            for blade in 0..STATE_DIMENSION {
                let original = basis_blade_state(blade).unwrap();
                let mut state = original;
                apply_axis(&mut state, axis);
                apply_axis(&mut state, axis);
                let expected = original.map(|value| -value);
                assert_state_close(&state, &expected);
            }
        }
    }

    #[test]
    fn distinct_basis_generators_anticommute() {
        for first in 0..LATENT_DIMENSION {
            for second in 0..LATENT_DIMENSION {
                if first == second {
                    continue;
                }
                for blade in 0..STATE_DIMENSION {
                    let mut forward = basis_blade_state(blade).unwrap();
                    apply_axis(&mut forward, first);
                    apply_axis(&mut forward, second);

                    let mut reverse = basis_blade_state(blade).unwrap();
                    apply_axis(&mut reverse, second);
                    apply_axis(&mut reverse, first);

                    for (left, right) in forward.iter().zip(reverse.iter()) {
                        assert!((left + right).abs() <= TOLERANCE);
                    }
                }
            }
        }
    }

    #[test]
    fn all_basis_permutations_encode_orientation_in_the_top_grade() {
        let mut count = 0;
        for a in 0..LATENT_DIMENSION {
            for b in 0..LATENT_DIMENSION {
                for c in 0..LATENT_DIMENSION {
                    for d in 0..LATENT_DIMENSION {
                        let permutation = [a, b, c, d];
                        let mut sorted = permutation;
                        sorted.sort_unstable();
                        if sorted != [0, 1, 2, 3] {
                            continue;
                        }
                        count += 1;
                        let state = apply_basis_word(&permutation).unwrap();
                        let expected = if permutation_parity(&permutation) == 0 {
                            1.0
                        } else {
                            -1.0
                        };
                        assert!((state[PSEUDOSCALAR_BLADE] - expected).abs() <= TOLERANCE);
                        assert!((euclidean_energy(&state) - 1.0).abs() <= TOLERANCE);
                    }
                }
            }
        }
        assert_eq!(count, 24);
    }

    #[test]
    fn inverse_program_recovers_every_basis_state() {
        for axis in 0..LATENT_DIMENSION {
            let program = basis_gesture_program(axis).unwrap();
            for blade in 0..STATE_DIMENSION {
                let original = basis_blade_state(blade).unwrap();
                let mut state = original;
                program.apply(&mut state).unwrap();
                program.inverse().apply(&mut state).unwrap();
                assert_state_close(&state, &original);
            }
        }
    }
}
