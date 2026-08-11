//! E15 — closed control-loop commutator (`.string-research/EXPERIMENTS.md`
//! §E15, `ARIADNE-MATHEMATICS.md` §5-6).
//!
//! Two control loops, both starting and ending at the same scalar control
//! point, differing only in the order of two coupling gestures `A`
//! (retarget `couple`) and `B` (retarget `chirality`):
//!
//!   gamma_AB = A,  B, -A, -B
//!   gamma_BA = B,  A, -B, -A
//!
//! The control endpoint is identical for both. The vibrating state need
//! not be: this is discrete ordered-scattering path memory inside one
//! note (`ARIADNE-MATHEMATICS.md` §6's `W[gamma]`), not a Berry phase —
//! there is no continuously-identified transported subspace and no
//! adiabatic separation here (see `.string-research/E15-REPORT.md`).
//!
//! Starting primitive: `closed_loop_commutator_is_path_dependent` in
//! `pluck_invariants.rs`. Its single (da=0.22, db=0.85) point is this
//! file's largest grid corner, reproduced below as a cross-check.

use std::collections::BTreeMap;

use mus_dsp::pluck::{weave_holonomy_defect, ControlFrame, ControlTarget, StringNetworkVoice};
use mus_dsp::SR_F64;

/// `couple` baseline used throughout (matches the starting-primitive
/// test): a mid-range scattering angle, away from both the `<=1e-12`
/// dead zone and the `0.45` clamp.
const COUPLE0: f64 = 0.12;
const DA_GRID: [f64; 4] = [0.02, 0.06, 0.12, 0.22];
const DB_GRID: [f64; 3] = [0.15, 0.4, 0.85];
/// The smallest two entries of each grid: the "tiny (da, db)" subset used
/// for the small-amplitude scaling fit, kept apart from the full-grid fit
/// so a possible large-amplitude departure from the asymptotic law is
/// visible rather than averaged away.
const TINY_DA: [f64; 2] = [0.02, 0.06];
const TINY_DB: [f64; 2] = [0.15, 0.4];

fn s(t: f64) -> usize {
    (t * SR_F64) as usize
}

/// Same synth/topology parameters as `closed_loop_commutator_is_path_dependent`
/// (9 courses, sus=2.0, orbit=0.4, orbit_depth=0.5), with `couple0` and
/// `body` exposed so the control condition (§3) can zero them out.
fn base_patch(couple0: f64, body: f64) -> BTreeMap<String, String> {
    let mut patch = BTreeMap::new();
    patch.insert("synth".to_string(), "weave".to_string());
    patch.insert("sus".to_string(), "2.0".to_string());
    patch.insert("courses".to_string(), "9".to_string());
    patch.insert("couple".to_string(), couple0.to_string());
    patch.insert("chirality".to_string(), "0.0".to_string());
    patch.insert("body".to_string(), body.to_string());
    patch.insert("orbit".to_string(), "0.4".to_string());
    patch.insert("orbit_depth".to_string(), "0.5".to_string());
    patch
}

/// The two order-reversed closed loops over (couple, chirality):
/// `A`: couple0 -> couple0+da -> couple0.
/// `B`: chirality 0 -> db -> 0.
/// Frame times match the starting primitive (0.20s, 0.45s, 0.70s, 0.95s).
fn loops(couple0: f64, da: f64, db: f64) -> (Vec<ControlFrame>, Vec<ControlFrame>) {
    let a_then_b = vec![
        ControlFrame {
            at_sample: s(0.20),
            sets: vec![(ControlTarget::Couple, couple0 + da)],
        },
        ControlFrame {
            at_sample: s(0.45),
            sets: vec![(ControlTarget::Chirality, db)],
        },
        ControlFrame {
            at_sample: s(0.70),
            sets: vec![(ControlTarget::Couple, couple0)],
        },
        ControlFrame {
            at_sample: s(0.95),
            sets: vec![(ControlTarget::Chirality, 0.0)],
        },
    ];
    let b_then_a = vec![
        ControlFrame {
            at_sample: s(0.20),
            sets: vec![(ControlTarget::Chirality, db)],
        },
        ControlFrame {
            at_sample: s(0.45),
            sets: vec![(ControlTarget::Couple, couple0 + da)],
        },
        ControlFrame {
            at_sample: s(0.70),
            sets: vec![(ControlTarget::Chirality, 0.0)],
        },
        ControlFrame {
            at_sample: s(0.95),
            sets: vec![(ControlTarget::Couple, couple0)],
        },
    ];
    (a_then_b, b_then_a)
}

fn render(patch: &BTreeMap<String, String>, trajectory: &[ControlFrame]) -> Vec<f32> {
    StringNetworkVoice::weave(patch, &[110.0, 165.0], 1.2, 1.0)
        .render_traced_with(256, trajectory)
        .0
}

fn rms(values: &[f32]) -> f64 {
    (values
        .iter()
        .map(|&value| (value as f64) * (value as f64))
        .sum::<f64>()
        / values.len().max(1) as f64)
        .sqrt()
}

fn l2_norm(values: &[f32]) -> f64 {
    values
        .iter()
        .map(|&value| (value as f64) * (value as f64))
        .sum::<f64>()
        .sqrt()
}

fn peak(values: &[f32]) -> f32 {
    values.iter().fold(0.0_f32, |m, v| m.max(v.abs()))
}

/// `(absolute L2 distance, distance relative to the mean of the two
/// trajectories' norms)`. The relative form is the dimensionless
/// quantity comparable to the closed-form operator-norm defect.
fn l2_distance(ab: &[f32], ba: &[f32]) -> (f64, f64) {
    let abs_dist = ab
        .iter()
        .zip(ba.iter())
        .map(|(x, y)| {
            let d = (*x - *y) as f64;
            d * d
        })
        .sum::<f64>()
        .sqrt();
    let scale = 0.5 * (l2_norm(ab) + l2_norm(ba));
    (abs_dist, abs_dist / scale.max(1e-12))
}

/// Ordinary least squares slope+intercept of `y = slope*x + intercept`.
fn fit_line(points: &[(f64, f64)]) -> (f64, f64) {
    let n = points.len() as f64;
    let mean_x = points.iter().map(|(x, _)| x).sum::<f64>() / n;
    let mean_y = points.iter().map(|(_, y)| y).sum::<f64>() / n;
    let cov: f64 = points
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum();
    let var: f64 = points.iter().map(|(x, _)| (x - mean_x).powi(2)).sum();
    let slope = cov / var;
    let intercept = mean_y - slope * mean_x;
    (slope, intercept)
}

/// §1 + §4: the amplitude sweep table (with the closed-form local defect
/// printed alongside for correspondence, not identity) and the fitted
/// small-amplitude scaling exponent.
#[test]
fn e15_amplitude_sweep_and_scaling() {
    println!("\n=== E15 — closed control-loop commutator: amplitude sweep ===");
    println!("couple0={COUPLE0:.4}  body=0.45  courses=9  sus=2.0  orbit=0.4  orbit_depth=0.5");
    println!(
        "{:>5} {:>5} | {:>8} {:>8} | {:>10} {:>10} | {:>9} {:>9} {:>8} | {:>11}",
        "da",
        "db",
        "da_ang",
        "db_ang",
        "rel_dist",
        "abs_dist",
        "rms_ab",
        "rms_ba",
        "dB_diff",
        "closed_form"
    );
    let patch = base_patch(COUPLE0, 0.45);
    let mut full_grid_fit: Vec<(f64, f64)> = Vec::new();
    let mut tiny_fit: Vec<(f64, f64)> = Vec::new();
    for &da in &DA_GRID {
        for &db in &DB_GRID {
            let (a_then_b, b_then_a) = loops(COUPLE0, da, db);
            let ab = render(&patch, &a_then_b);
            let ba = render(&patch, &b_then_a);

            // Sanity: finite, bounded.
            assert!(
                ab.iter().chain(ba.iter()).all(|v| v.is_finite()),
                "da={da} db={db}: non-finite sample"
            );
            let pk = peak(&ab).max(peak(&ba));
            assert!(pk <= 8.0, "da={da} db={db}: unbounded peak {pk}");

            // Sanity: determinism — identical trajectory, identical render.
            let ab_again = render(&patch, &a_then_b);
            assert_eq!(
                ab, ab_again,
                "da={da} db={db}: render_traced_with is not deterministic"
            );

            let (abs_dist, rel_dist) = l2_distance(&ab, &ba);
            let rms_ab = rms(&ab);
            let rms_ba = rms(&ba);
            let db_diff = 20.0 * (rms_ab / rms_ba.max(1e-12)).log10();

            // da is already a rotation angle in radians (couple's own
            // unit); db is a dimensionless bias, not an angle, so it is
            // converted via the induced forward/backward angle asymmetry
            // at baseline couple: Delta(angle_fwd - angle_back) = couple0
            // * db. Both are proxies for the §5 toy model's (a, b), not
            // identical to it — see the report for the honest caveat.
            let da_angle = da;
            let db_angle = COUPLE0 * db;
            let closed_form = weave_holonomy_defect(da_angle, db_angle);

            println!(
                "{da:>5.2} {db:>5.2} | {da_angle:>8.4} {db_angle:>8.4} | {rel_dist:>10.6} {abs_dist:>10.4} | {rms_ab:>9.5} {rms_ba:>9.5} {db_diff:>8.4} | {closed_form:>11.6}"
            );

            let x = (da * db).ln();
            full_grid_fit.push((x, rel_dist.ln()));
            if TINY_DA.contains(&da) && TINY_DB.contains(&db) {
                tiny_fit.push((x, rel_dist.ln()));
            }
        }
    }

    let (full_exponent, full_intercept) = fit_line(&full_grid_fit);
    let (tiny_exponent, tiny_intercept) = fit_line(&tiny_fit);
    println!("\n--- small-amplitude scaling fit: rel_dist ~ C * (da*db)^p, log-log OLS ---");
    println!(
        "full grid  (n={:>2}): p={:.4}  C={:.4}",
        full_grid_fit.len(),
        full_exponent,
        full_intercept.exp()
    );
    println!(
        "tiny subset (n={:>2}, da<=0.06, db<=0.4): p={:.4}  C={:.4}",
        tiny_fit.len(),
        tiny_exponent,
        tiny_intercept.exp()
    );
    println!("(exponent reported, not asserted — see report for discussion)");

    assert!(full_exponent.is_finite() && tiny_exponent.is_finite());
}

/// §2: both orders retarget every scalar control back to its exact
/// starting value; only the intervening order differs.
#[test]
fn e15_endpoint_identity() {
    println!("\n=== E15 — endpoint identity check (da=0.12, db=0.4) ===");
    let da = 0.12;
    let db = 0.4;
    let (a_then_b, b_then_a) = loops(COUPLE0, da, db);
    for (name, trajectory) in [
        ("gamma_AB = A,B,-A,-B", &a_then_b),
        ("gamma_BA = B,A,-B,-A", &b_then_a),
    ] {
        println!("-- {name} --");
        for frame in trajectory {
            for &(target, value) in &frame.sets {
                println!(
                    "  t={:>6.4}s (sample {:>6}) {:?} -> {:.4}",
                    frame.at_sample as f64 / SR_F64,
                    frame.at_sample,
                    target,
                    value
                );
            }
        }
        let last = |wanted: ControlTarget| -> Option<f64> {
            trajectory
                .iter()
                .rev()
                .flat_map(|frame| frame.sets.iter())
                .find(|(target, _)| *target == wanted)
                .map(|(_, value)| *value)
        };
        assert_eq!(
            last(ControlTarget::Couple),
            Some(COUPLE0),
            "{name}: couple must end at its initial value"
        );
        assert_eq!(
            last(ControlTarget::Chirality),
            Some(0.0),
            "{name}: chirality must end at its initial value"
        );
    }
}

/// §3: with couple=0 and body=0 (and da=0, so couple never leaves 0),
/// `chirality` is read only inside `scatter_weave`'s and `scatter_body`'s
/// early-return-guarded branches (`couple <= 1e-12`, `body <= 1e-12`), so
/// it has no causal path to the output. Order-dependence should vanish —
/// not just approximately, but bit-for-bit, since nothing downstream of
/// `chirality` executes.
#[test]
fn e15_control_couple_and_body_zero_is_inert() {
    println!("\n=== E15 — control condition: couple=0, body=0, da=0 ===");
    let patch = base_patch(0.0, 0.0);
    for &db in &DB_GRID {
        let (a_then_b, b_then_a) = loops(0.0, 0.0, db);
        let ab = render(&patch, &a_then_b);
        let ba = render(&patch, &b_then_a);
        assert!(ab.iter().chain(ba.iter()).all(|v| v.is_finite()));
        let (abs_dist, rel_dist) = l2_distance(&ab, &ba);
        println!("db={db:.2}: abs_dist={abs_dist:.3e} rel_dist={rel_dist:.3e}");
        assert_eq!(
            ab, ba,
            "control condition must be exactly path-independent when couple=0 and body=0: db={db}"
        );
    }
}
