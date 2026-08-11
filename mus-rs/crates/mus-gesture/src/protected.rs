//! Protected family/path IR (unified-instrument stage U2).
//!
//! Closed-form Wilson–Dirac family evaluation, smooth torus loops, the
//! Jansen–Ruskai–Seiler finite-time receipt, typed refusals, and the claim
//! evaluator — per `RUST-API-CONTRACT.md` and `UNIFIED-U2-protected-path-ir.md`.
//!
//! No eigensolver appears in this module: `H(k)^2 = r(k)^2 I` by Clifford
//! anticommutation, so every gap and operator norm reduces to the Euclidean
//! norm of the five-component `d`-vector.

use mus_watsn::Complex64;
use serde::Serialize;

pub const COMPLEX_DIMENSION: usize = 16;
pub const ACTIVE_DIMENSION: usize = 4;
pub const JRS_INTEGRATION_GRID: usize = 4097;
pub const PROTECTED_RANK: f64 = 2.0;

/// The five Hermitian, pairwise-anticommuting 4x4 Gammas (theory §4):
/// `sx⊗sx, sx⊗sy, sx⊗sz, sy⊗I, sz⊗I`.
pub fn gammas() -> [[[Complex64; 4]; 4]; 5] {
    let zero = Complex64::ZERO;
    let one = Complex64::ONE;
    let i = Complex64::new(0.0, 1.0);
    let ni = Complex64::new(0.0, -1.0);
    let none = Complex64::new(-1.0, 0.0);
    [
        // sigma_x ⊗ sigma_x
        [
            [zero, zero, zero, one],
            [zero, zero, one, zero],
            [zero, one, zero, zero],
            [one, zero, zero, zero],
        ],
        // sigma_x ⊗ sigma_y
        [
            [zero, zero, zero, ni],
            [zero, zero, i, zero],
            [zero, ni, zero, zero],
            [i, zero, zero, zero],
        ],
        // sigma_x ⊗ sigma_z
        [
            [zero, zero, one, zero],
            [zero, zero, zero, none],
            [one, zero, zero, zero],
            [zero, none, zero, zero],
        ],
        // sigma_y ⊗ I
        [
            [zero, zero, ni, zero],
            [zero, zero, zero, ni],
            [i, zero, zero, zero],
            [zero, i, zero, zero],
        ],
        // sigma_z ⊗ I
        [
            [one, zero, zero, zero],
            [zero, one, zero, zero],
            [zero, zero, none, zero],
            [zero, zero, zero, none],
        ],
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FamilyParameters {
    pub mass: f64,
    pub carrier_hz: f64,
    pub dirac_scale_hz: f64,
}

impl Default for FamilyParameters {
    fn default() -> Self {
        Self {
            mass: -3.0,
            carrier_hz: 440.0,
            dirac_scale_hz: 20.0,
        }
    }
}

/// Wilson–Dirac coefficient vector `d(k)` (theory §4).
pub fn dirac_d(k: &[f64; 4], mass: f64) -> [f64; 5] {
    [
        k[0].sin(),
        k[1].sin(),
        k[2].sin(),
        k[3].sin(),
        mass + k.iter().map(|value| value.cos()).sum::<f64>(),
    ]
}

pub fn dirac_radius(k: &[f64; 4], mass: f64) -> f64 {
    dirac_d(k, mass)
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt()
}

/// Dense active-block Hamiltonian `H_D(k) = sum_a d_a Gamma_a`.
pub fn dirac_hamiltonian(k: &[f64; 4], mass: f64) -> [[Complex64; 4]; 4] {
    let coefficients = dirac_d(k, mass);
    let gamma = gammas();
    let mut out = [[Complex64::ZERO; 4]; 4];
    for (a, coefficient) in coefficients.iter().enumerate() {
        for (out_row, gamma_row) in out.iter_mut().zip(gamma[a].iter()) {
            for (out_entry, gamma_entry) in out_row.iter_mut().zip(gamma_row.iter()) {
                *out_entry = *out_entry + gamma_entry.scale(*coefficient);
            }
        }
    }
    out
}

fn wrap_torus(value: f64) -> f64 {
    let period = 2.0 * std::f64::consts::PI;
    let wrapped = value.rem_euclid(period);
    if wrapped > std::f64::consts::PI {
        wrapped - period
    } else {
        wrapped
    }
}

/// Smooth closed control loop on `T^4` — the exact port of the reference
/// `SmoothLoopSpec` (quintic smoothstep phase; first axis `r1 (1 - cos θ)`,
/// second axis `r2 sin θ`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SmoothTorusLoop {
    pub name: String,
    pub base: [f64; 4],
    pub first_axis: usize,
    pub second_axis: usize,
    pub first_radius: f64,
    pub second_radius: f64,
    /// Number of full phase turns over `s in [0,1]`. The reference loops
    /// use exactly `1.0`; any non-integer value leaves the path open and
    /// must be refused by `verify_path`, never silently wrapped closed.
    pub turns: f64,
}

pub const DEFAULT_BASE: [f64; 4] = [-0.4, 0.0, 0.9, -0.7];

pub fn loop_a() -> SmoothTorusLoop {
    SmoothTorusLoop {
        name: "A".to_owned(),
        base: DEFAULT_BASE,
        first_axis: 2,
        second_axis: 3,
        first_radius: 1.50,
        second_radius: 1.20,
        turns: 1.0,
    }
}

pub fn loop_b() -> SmoothTorusLoop {
    SmoothTorusLoop {
        name: "B".to_owned(),
        base: DEFAULT_BASE,
        first_axis: 0,
        second_axis: 3,
        first_radius: 1.30,
        second_radius: 1.00,
        turns: 1.0,
    }
}

impl SmoothTorusLoop {
    pub fn validate(&self) -> Result<(), ProtectedRefusalDetail> {
        let axes_valid = self.first_axis != self.second_axis
            && self.first_axis < 4
            && self.second_axis < 4;
        let radii_valid = self.first_radius.is_finite()
            && self.second_radius.is_finite()
            && self.first_radius != 0.0
            && self.second_radius != 0.0;
        let base_valid = self.base.iter().all(|value| value.is_finite());
        if !(axes_valid && radii_valid && base_valid) {
            return Err(ProtectedRefusalDetail {
                refusal: ProtectedRefusal::NonFiniteInput,
                field_path: "path.axes/radii/base".to_owned(),
                observed: f64::NAN,
                required: 0.0,
                strongest_available_claim: None,
            });
        }
        Ok(())
    }

    /// `(theta, dtheta/ds, d2theta/ds2)` under the quintic smoothstep.
    fn phase(&self, s: f64) -> (f64, f64, f64) {
        let scale = 2.0 * std::f64::consts::PI * self.turns;
        let u = 10.0 * s.powi(3) - 15.0 * s.powi(4) + 6.0 * s.powi(5);
        let du = 30.0 * s.powi(2) - 60.0 * s.powi(3) + 30.0 * s.powi(4);
        let ddu = 60.0 * s - 180.0 * s.powi(2) + 120.0 * s.powi(3);
        (scale * u, scale * du, scale * ddu)
    }

    pub fn point(&self, s: f64) -> [f64; 4] {
        let (theta, _, _) = self.phase(s);
        let mut value = self.base;
        value[self.first_axis] += self.first_radius * (1.0 - theta.cos());
        value[self.second_axis] += self.second_radius * theta.sin();
        [
            wrap_torus(value[0]),
            wrap_torus(value[1]),
            wrap_torus(value[2]),
            wrap_torus(value[3]),
        ]
    }

    /// Analytic first and second `k`-derivatives with respect to `s`.
    pub fn derivatives(&self, s: f64) -> ([f64; 4], [f64; 4]) {
        let (theta, velocity, acceleration) = self.phase(s);
        let mut first = [0.0; 4];
        let mut second = [0.0; 4];
        first[self.first_axis] = self.first_radius * theta.sin() * velocity;
        first[self.second_axis] = self.second_radius * theta.cos() * velocity;
        second[self.first_axis] = self.first_radius
            * (theta.cos() * velocity * velocity + theta.sin() * acceleration);
        second[self.second_axis] = self.second_radius
            * (-theta.sin() * velocity * velocity + theta.cos() * acceleration);
        (first, second)
    }

    /// Wrap-aware endpoint closure distance.
    pub fn closure_distance(&self) -> f64 {
        let start = self.point(0.0);
        let end = self.point(1.0);
        start
            .iter()
            .zip(end.iter())
            .map(|(a, b)| wrap_torus(a - b).abs())
            .fold(0.0_f64, f64::max)
    }
}

/// `(||d||, ||d'||, ||d''||)` at `s`, chain-ruled through the loop — the
/// operator norms of `H`, `H'`, `H''` by the Clifford identity.
fn dirac_derivative_norms(spec: &SmoothTorusLoop, s: f64, mass: f64) -> (f64, f64, f64) {
    let k = spec.point(s);
    let (k_first, k_second) = spec.derivatives(s);
    let d = dirac_d(&k, mass);
    let mut first = [0.0_f64; 5];
    let mut second = [0.0_f64; 5];
    for axis in 0..4 {
        first[axis] = k[axis].cos() * k_first[axis];
        second[axis] =
            -k[axis].sin() * k_first[axis] * k_first[axis] + k[axis].cos() * k_second[axis];
    }
    first[4] = -(0..4).map(|axis| k[axis].sin() * k_first[axis]).sum::<f64>();
    second[4] = -(0..4)
        .map(|axis| {
            k[axis].cos() * k_first[axis] * k_first[axis] + k[axis].sin() * k_second[axis]
        })
        .sum::<f64>();
    let norm = |values: &[f64]| values.iter().map(|v| v * v).sum::<f64>().sqrt();
    (norm(&d), norm(&first), norm(&second))
}

/// Jansen–Ruskai–Seiler Theorem 3 receipt for the rank-two band.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AdiabaticBoundCertificate {
    pub loop_name: String,
    pub dimensionless_duration: f64,
    pub physical_duration_seconds: f64,
    pub projector_error_upper_bound: f64,
    pub minimum_gap: f64,
    pub maximum_h_prime: f64,
    pub maximum_h_second: f64,
    pub jrs_numerator: f64,
    pub accepted: bool,
    pub requested_budget: f64,
}

pub fn jrs_adiabatic_bound(
    spec: &SmoothTorusLoop,
    parameters: &FamilyParameters,
    physical_duration_seconds: f64,
    requested_projector_error: f64,
) -> AdiabaticBoundCertificate {
    let grid = JRS_INTEGRATION_GRID;
    let mut radii = vec![0.0_f64; grid];
    let mut first = vec![0.0_f64; grid];
    let mut second = vec![0.0_f64; grid];
    for (index, ((radius_slot, first_slot), second_slot)) in radii
        .iter_mut()
        .zip(first.iter_mut())
        .zip(second.iter_mut())
        .enumerate()
    {
        let s = index as f64 / (grid - 1) as f64;
        let (radius, d_first, d_second) = dirac_derivative_norms(spec, s, parameters.mass);
        *radius_slot = radius;
        *first_slot = d_first;
        *second_slot = d_second;
    }
    let gap = |index: usize| 2.0 * radii[index];
    let endpoint = PROTECTED_RANK
        * (first[0] / gap(0).powi(2) + first[grid - 1] / gap(grid - 1).powi(2));
    let integrand = |index: usize| {
        PROTECTED_RANK * second[index] / gap(index).powi(2)
            + 7.0 * PROTECTED_RANK * PROTECTED_RANK.sqrt() * first[index] * first[index]
                / gap(index).powi(3)
    };
    let step = 1.0 / (grid - 1) as f64;
    let mut integral = 0.0;
    for index in 0..grid - 1 {
        integral += 0.5 * (integrand(index) + integrand(index + 1)) * step;
    }
    let numerator = endpoint + integral;
    let dimensionless =
        2.0 * std::f64::consts::PI * parameters.dirac_scale_hz * physical_duration_seconds;
    let bound = numerator / dimensionless;
    let minimum_gap = (0..grid).map(gap).fold(f64::INFINITY, f64::min);
    AdiabaticBoundCertificate {
        loop_name: spec.name.clone(),
        dimensionless_duration: dimensionless,
        physical_duration_seconds,
        projector_error_upper_bound: bound,
        minimum_gap,
        maximum_h_prime: first.iter().copied().fold(0.0, f64::max),
        maximum_h_second: second.iter().copied().fold(0.0, f64::max),
        jrs_numerator: numerator,
        accepted: bound <= requested_projector_error,
        requested_budget: requested_projector_error,
    }
}

pub fn minimum_duration_for_jrs_budget(
    spec: &SmoothTorusLoop,
    parameters: &FamilyParameters,
    requested_projector_error: f64,
) -> f64 {
    let probe = jrs_adiabatic_bound(spec, parameters, 1.0, requested_projector_error);
    probe.jrs_numerator
        / (2.0 * std::f64::consts::PI * parameters.dirac_scale_hz * requested_projector_error)
}

// ---- refusals, modes, claims ------------------------------------------

/// Contract §9, complete. Variants not yet reachable in Rust are
/// constructible but never silently mapped onto a neighbor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProtectedRefusal {
    UnknownSchema,
    DigestMismatch,
    CoordinateMapMismatch,
    NonFiniteInput,
    NonPeriodicFamily,
    PathNotClosed,
    ActuatorStateNotClosed,
    RankMismatch,
    GapUnverified,
    GapBudgetViolated,
    ProjectorTableMismatch,
    AdjacentOverlapTooSmall,
    GaugeConventionMissing,
    IllegalRealFactor,
    EdgeNotAuthored,
    ComplexStructureViolation,
    LoweringResidualTooLarge,
    DurationUnsupported,
    AdiabaticBoundFailed,
    LeakageBudgetFailed,
    SampleRateUnsupported,
    BlockScheduleAmbiguous,
    MissingRadiationPort,
    MissingDisposalPolicy,
    PersistentStateUnavailable,
    ProductionReplayMissing,
    ClaimTooStrong,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProtectedRefusalDetail {
    pub refusal: ProtectedRefusal,
    pub field_path: String,
    pub observed: f64,
    pub required: f64,
    pub strongest_available_claim: Option<ClaimLevel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProtectedExecutionMode {
    CompiledGeometric,
    PhysicalAdiabatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ProtectedBudgets {
    pub minimum_gap: f64,
    pub maximum_projector_error: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum ClaimLevel {
    OrderDependent,
    ClosedControl,
    Geometric,
    GappedFamily,
    TopologicalBundle,
    AdiabaticRealization,
    ProtectedAcoustic,
    FaultTolerant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EvidenceClass {
    Theorem,
    ValidatedNumerics,
    ReferenceOracle,
    ProductionReplay,
    ListenerEvidence,
}

/// The evidence actually in hand for one compilation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EvidenceInHand {
    /// Full continuous path with per-sample projector transports, not just
    /// an endpoint operator.
    pub full_path_execution: bool,
    /// Sampled gap floor along the executed path.
    pub verified_gap_floor: Option<f64>,
    /// Accepted JRS certificate at the executed duration.
    pub adiabatic_theorem_accepted: bool,
    /// Persistent in-instrument state with radiation ledger (U3+).
    pub persistent_acoustic_state: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClaimDecision {
    pub promoted: Vec<ClaimLevel>,
    pub withheld: Vec<(ClaimLevel, String)>,
}

/// Contract §15: return every promoted claim and every withheld stronger
/// claim with a reason. Never one boolean.
pub fn evaluate_claims(evidence: &EvidenceInHand) -> ClaimDecision {
    let mut promoted = vec![ClaimLevel::OrderDependent, ClaimLevel::ClosedControl];
    let mut withheld = Vec::new();

    if evidence.full_path_execution {
        promoted.push(ClaimLevel::Geometric);
    } else {
        withheld.push((
            ClaimLevel::Geometric,
            "endpoint-only evidence: an endpoint matrix is not a path; supply \
             the full projector transport"
                .to_owned(),
        ));
    }
    match evidence.verified_gap_floor {
        Some(floor) if floor > 0.0 => promoted.push(ClaimLevel::GappedFamily),
        _ => withheld.push((
            ClaimLevel::GappedFamily,
            "no verified gap floor along the executed path".to_owned(),
        )),
    }
    // Topology is carried by the analytic corner formula + independent
    // estimators in the reference packet; Rust has no independent invariant
    // implementation yet (deferred per keystone), so the bundle claim stays
    // at reference evidence and is withheld from production promotion here.
    withheld.push((
        ClaimLevel::TopologicalBundle,
        "no production-independent second-Chern estimator in Rust yet".to_owned(),
    ));
    if evidence.adiabatic_theorem_accepted {
        promoted.push(ClaimLevel::AdiabaticRealization);
    } else {
        withheld.push((
            ClaimLevel::AdiabaticRealization,
            "JRS theorem receipt not accepted at the requested budget; a small \
             numerical replay error does not override a failed theorem bound"
                .to_owned(),
        ));
    }
    if evidence.persistent_acoustic_state {
        promoted.push(ClaimLevel::ProtectedAcoustic);
    } else {
        withheld.push((
            ClaimLevel::ProtectedAcoustic,
            "no persistent in-instrument backbone with radiation ledger \
             (stage U3/U4/U5 pending)"
                .to_owned(),
        ));
    }
    withheld.push((
        ClaimLevel::FaultTolerant,
        "no error-correcting mechanism exists or is claimed".to_owned(),
    ));
    ClaimDecision { promoted, withheld }
}

/// Path admission: closure and gap-floor verification for one loop.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PathGapCertificate {
    pub loop_name: String,
    pub samples: usize,
    pub minimum_gap: f64,
    pub closure_distance: f64,
}

pub const PATH_CLOSURE_TOLERANCE: f64 = 1.0e-12;

pub fn verify_path(
    spec: &SmoothTorusLoop,
    parameters: &FamilyParameters,
    budgets: &ProtectedBudgets,
    samples: usize,
) -> Result<PathGapCertificate, ProtectedRefusalDetail> {
    spec.validate()?;
    let closure = spec.closure_distance();
    if closure > PATH_CLOSURE_TOLERANCE {
        return Err(ProtectedRefusalDetail {
            refusal: ProtectedRefusal::PathNotClosed,
            field_path: "path.closure_distance".to_owned(),
            observed: closure,
            required: PATH_CLOSURE_TOLERANCE,
            strongest_available_claim: Some(ClaimLevel::OrderDependent),
        });
    }
    let mut minimum_gap = f64::INFINITY;
    for index in 0..samples {
        let s = index as f64 / (samples - 1) as f64;
        minimum_gap = minimum_gap.min(2.0 * dirac_radius(&spec.point(s), parameters.mass));
    }
    if minimum_gap < budgets.minimum_gap {
        return Err(ProtectedRefusalDetail {
            refusal: ProtectedRefusal::GapBudgetViolated,
            field_path: "path.minimum_sampled_gap".to_owned(),
            observed: minimum_gap,
            required: budgets.minimum_gap,
            strongest_available_claim: Some(ClaimLevel::ClosedControl),
        });
    }
    Ok(PathGapCertificate {
        loop_name: spec.name.clone(),
        samples,
        minimum_gap,
        closure_distance: closure,
    })
}

/// The PhysicalAdiabatic admission gate (their N9 made executable).
pub fn admit_physical_adiabatic(
    spec: &SmoothTorusLoop,
    parameters: &FamilyParameters,
    budgets: &ProtectedBudgets,
    duration_seconds: f64,
) -> Result<AdiabaticBoundCertificate, ProtectedRefusalDetail> {
    verify_path(spec, parameters, budgets, 1025)?;
    let certificate = jrs_adiabatic_bound(
        spec,
        parameters,
        duration_seconds,
        budgets.maximum_projector_error,
    );
    if !certificate.accepted {
        let sufficient =
            minimum_duration_for_jrs_budget(spec, parameters, budgets.maximum_projector_error);
        return Err(ProtectedRefusalDetail {
            refusal: ProtectedRefusal::AdiabaticBoundFailed,
            field_path: format!(
                "finite_time.jrs.projector_error_upper_bound (sufficient duration {sufficient:.9} s)"
            ),
            observed: certificate.projector_error_upper_bound,
            required: budgets.maximum_projector_error,
            strongest_available_claim: Some(ClaimLevel::Geometric),
        });
    }
    Ok(certificate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn unit(rng: &mut SplitMix64) -> f64 {
        (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn random_k(rng: &mut SplitMix64) -> [f64; 4] {
        let tau = 2.0 * std::f64::consts::PI;
        [
            (unit(rng) - 0.5) * tau,
            (unit(rng) - 0.5) * tau,
            (unit(rng) - 0.5) * tau,
            (unit(rng) - 0.5) * tau,
        ]
    }

    #[test]
    fn gammas_are_hermitian_and_anticommute() {
        let gamma = gammas();
        for a in 0..5 {
            for (row, gamma_row) in gamma[a].iter().enumerate() {
                for (column, direct) in gamma_row.iter().enumerate() {
                    let adjoint = gamma[a][column][row].conj();
                    assert!((*direct - adjoint).abs() < 1.0e-15);
                }
            }
            for b in 0..5 {
                // {Ga, Gb} = 2 delta_ab I
                for row in 0..4 {
                    for column in 0..4 {
                        let sum = gamma[a][row]
                            .iter()
                            .enumerate()
                            .map(|(middle, left)| {
                                *left * gamma[b][middle][column]
                                    + gamma[b][row][middle] * gamma[a][middle][column]
                            })
                            .fold(Complex64::ZERO, |accumulator, term| accumulator + term);
                        let expected = if a == b && row == column { 2.0 } else { 0.0 };
                        assert!(
                            (sum.re - expected).abs() < 1.0e-14 && sum.im.abs() < 1.0e-14,
                            "a={a} b={b}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn hamiltonian_squares_to_radius_squared_identity() {
        let mut rng = SplitMix64::new(0x5EED_0001);
        for _ in 0..8 {
            let k = random_k(&mut rng);
            let hamiltonian = dirac_hamiltonian(&k, -3.0);
            let radius_squared = dirac_radius(&k, -3.0).powi(2);
            for row in 0..4 {
                for column in 0..4 {
                    let sum = hamiltonian[row]
                        .iter()
                        .enumerate()
                        .map(|(middle, left)| *left * hamiltonian[middle][column])
                        .fold(Complex64::ZERO, |accumulator, term| accumulator + term);
                    let expected = if row == column { radius_squared } else { 0.0 };
                    assert!((sum.re - expected).abs() < 1.0e-12 && sum.im.abs() < 1.0e-12);
                }
            }
        }
    }

    #[test]
    fn exact_gap_identity_holds_at_reference_mass() {
        let mut rng = SplitMix64::new(0x5EED_0002);
        for _ in 0..32 {
            let k = random_k(&mut rng);
            let radius_squared = dirac_radius(&k, -3.0).powi(2);
            let t: Vec<f64> = k.iter().map(|value| 1.0 - value.cos()).collect();
            let mut cross = 0.0;
            for i in 0..4 {
                for j in (i + 1)..4 {
                    cross += t[i] * t[j];
                }
            }
            assert!((radius_squared - (1.0 + 2.0 * cross)).abs() < 1.0e-12);
            assert!(2.0 * radius_squared.sqrt() >= 2.0 - 1.0e-12);
        }
    }

    #[test]
    fn reference_loops_close_and_stay_gapped() {
        let parameters = FamilyParameters::default();
        let budgets = ProtectedBudgets {
            minimum_gap: 2.0 - 1.0e-9,
            maximum_projector_error: 0.05,
        };
        for spec in [loop_a(), loop_b()] {
            let certificate = verify_path(&spec, &parameters, &budgets, 4097).unwrap();
            assert!(certificate.closure_distance <= PATH_CLOSURE_TOLERANCE);
            assert!(certificate.minimum_gap >= 2.0 - 1.0e-9, "{certificate:?}");
        }
    }

    #[test]
    fn jrs_minimum_durations_match_the_committed_fixture() {
        let parameters = FamilyParameters::default();
        let expected_a = 22.362855480397428;
        let expected_b = 15.941498262892383;
        let duration_a = minimum_duration_for_jrs_budget(&loop_a(), &parameters, 0.05);
        let duration_b = minimum_duration_for_jrs_budget(&loop_b(), &parameters, 0.05);
        assert!(
            ((duration_a - expected_a) / expected_a).abs() < 1.0e-9,
            "A: {duration_a}"
        );
        assert!(
            ((duration_b - expected_b) / expected_b).abs() < 1.0e-9,
            "B: {duration_b}"
        );
    }

    #[test]
    fn one_second_physical_adiabatic_refuses_at_five_percent() {
        let parameters = FamilyParameters::default();
        let budgets = ProtectedBudgets {
            minimum_gap: 2.0 - 1.0e-9,
            maximum_projector_error: 0.05,
        };
        let refusal = admit_physical_adiabatic(&loop_a(), &parameters, &budgets, 1.0)
            .expect_err("one second must refuse");
        assert_eq!(refusal.refusal, ProtectedRefusal::AdiabaticBoundFailed);
        assert!(refusal.observed > 0.05);
        assert_eq!(refusal.strongest_available_claim, Some(ClaimLevel::Geometric));
        // and the theorem-sufficient duration passes
        let accepted = admit_physical_adiabatic(&loop_a(), &parameters, &budgets, 23.0).unwrap();
        assert!(accepted.accepted);
    }

    #[test]
    fn open_path_refuses_as_path_not_closed() {
        // A non-integer number of turns leaves the endpoint displaced.
        let mut spec = loop_a();
        spec.name = "A-open".to_owned();
        spec.turns = 0.75;
        let parameters = FamilyParameters::default();
        let budgets = ProtectedBudgets {
            minimum_gap: 2.0 - 1.0e-9,
            maximum_projector_error: 0.05,
        };
        let refusal = verify_path(&spec, &parameters, &budgets, 257)
            .expect_err("open path must refuse");
        assert_eq!(refusal.refusal, ProtectedRefusal::PathNotClosed);
        assert!(refusal.observed > PATH_CLOSURE_TOLERANCE);
        assert_eq!(
            refusal.strongest_available_claim,
            Some(ClaimLevel::OrderDependent)
        );

        // and a spec with a non-finite base refuses before any sampling
        let mut invalid = loop_a();
        invalid.base[0] = f64::NAN;
        let early = invalid.validate().expect_err("non-finite base refuses");
        assert_eq!(early.refusal, ProtectedRefusal::NonFiniteInput);
    }

    #[test]
    fn critical_mass_violates_the_gap_floor() {
        // Gap verification is loop-local: at m = -4 the reference LOOP_A
        // never approaches the critical corner and keeps its gap above 2
        // (verified below), so the refusal needs a loop that actually
        // crosses k = 0 — where the m = -4 family closes exactly.
        let parameters = FamilyParameters {
            mass: -4.0,
            ..FamilyParameters::default()
        };
        let budgets = ProtectedBudgets {
            minimum_gap: 1.999,
            maximum_projector_error: 0.05,
        };
        assert!(verify_path(&loop_a(), &parameters, &budgets, 4097).is_ok());

        let corner_loop = SmoothTorusLoop {
            name: "corner".to_owned(),
            base: [0.0; 4],
            first_axis: 2,
            second_axis: 3,
            first_radius: 0.3,
            second_radius: 0.3,
            turns: 1.0,
        };
        let refusal = verify_path(&corner_loop, &parameters, &budgets, 4097)
            .expect_err("critical family on a corner loop must refuse");
        assert_eq!(refusal.refusal, ProtectedRefusal::GapBudgetViolated);
        assert!(refusal.observed < 0.1, "observed {}", refusal.observed);
    }

    #[test]
    fn claim_evaluator_enforces_the_hard_rules() {
        // endpoint-only cannot reach Geometric
        let endpoint_only = EvidenceInHand {
            full_path_execution: false,
            verified_gap_floor: Some(2.0),
            adiabatic_theorem_accepted: false,
            persistent_acoustic_state: false,
        };
        let decision = evaluate_claims(&endpoint_only);
        assert!(!decision.promoted.contains(&ClaimLevel::Geometric));
        assert!(decision
            .withheld
            .iter()
            .any(|(level, reason)| *level == ClaimLevel::Geometric
                && reason.contains("endpoint-only")));

        // good numerics without theorem acceptance never reach
        // AdiabaticRealization
        let compiled = EvidenceInHand {
            full_path_execution: true,
            verified_gap_floor: Some(2.0),
            adiabatic_theorem_accepted: false,
            persistent_acoustic_state: false,
        };
        let decision = evaluate_claims(&compiled);
        assert!(decision.promoted.contains(&ClaimLevel::Geometric));
        assert!(!decision.promoted.contains(&ClaimLevel::AdiabaticRealization));
        assert!(!decision.promoted.contains(&ClaimLevel::ProtectedAcoustic));
        assert!(decision
            .withheld
            .iter()
            .any(|(level, reason)| *level == ClaimLevel::ProtectedAcoustic
                && reason.contains("U3")));

        // the evaluator never collapses to one boolean
        assert!(!decision.promoted.is_empty());
        assert!(!decision.withheld.is_empty());
    }
}
