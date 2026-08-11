//! Cross-runtime replay of the unified protected instrument's compiled
//! words (RUST-API-CONTRACT.md §16, acceptance tests 2-6 and 11).
//!
//! Reads the committed NPZ-derived vectors exported by
//! `mus-rs/tools/unified_export_test_vectors.py` and independently replays
//! every gate word through the `mus-watsn` complex executor. Nothing here
//! copies a Python claim bit: dense matrices are reconstructed gate by gate,
//! occupied restrictions and eigenphases are recomputed from scratch, and
//! the non-Abelian witness is measured, not read.

use mus_watsn::{
    real_givens_symplectic_residual, Complex64, ComplexGate, ComplexOperatorProgram,
    ComplexPhaseGate, ComplexTwoModeGate,
};
use serde_json::Value;

const DENSE_TOLERANCE: f64 = 1.0e-12;
const WITNESS_TOLERANCE: f64 = 1.0e-9;

fn vectors() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../paper/ariadne-protected-nonabelian/unified_artifacts/unified_rust_test_vectors.json"
    );
    let raw = std::fs::read_to_string(path).expect("committed test vectors present");
    serde_json::from_str(&raw).expect("valid JSON")
}

fn complex(entry: &Value) -> Complex64 {
    Complex64::new(entry[0].as_f64().unwrap(), entry[1].as_f64().unwrap())
}

fn complex_row(entry: &Value) -> Vec<Complex64> {
    entry.as_array().unwrap().iter().map(complex).collect()
}

fn complex_matrix(entry: &Value) -> Vec<Vec<Complex64>> {
    entry.as_array().unwrap().iter().map(complex_row).collect()
}

fn parse_schedule(entry: &Value) -> Vec<ComplexGate> {
    entry
        .as_array()
        .unwrap()
        .iter()
        .map(|gate| match gate["kind"].as_str().unwrap() {
            "phase_shift" => ComplexGate::Phase(ComplexPhaseGate {
                mode: gate["i"].as_u64().unwrap() as usize,
                phase: gate["phase"].as_f64().unwrap(),
            }),
            "two_mode_unitary" => {
                let block = complex_matrix(&gate["block"]);
                ComplexGate::TwoMode(ComplexTwoModeGate {
                    left: gate["i"].as_u64().unwrap() as usize,
                    right: gate["j"].as_u64().unwrap() as usize,
                    matrix: [block[0][0], block[0][1], block[1][0], block[1][1]],
                })
            }
            other => panic!("unknown gate kind {other}"),
        })
        .collect()
}

fn parse_edges(entry: &Value) -> Vec<(usize, usize)> {
    entry
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            (
                edge[0].as_u64().unwrap() as usize,
                edge[1].as_u64().unwrap() as usize,
            )
        })
        .collect()
}

fn program_from(record: &Value) -> (usize, ComplexOperatorProgram) {
    let n = record["n"].as_u64().unwrap() as usize;
    let edges = parse_edges(&record["graph_edges"]);
    let program = ComplexOperatorProgram::new(
        n,
        parse_schedule(&record["schedule"]),
        Some(&edges),
        ComplexOperatorProgram::DEFAULT_UNITARITY_TOLERANCE,
    )
    .expect("committed word validates");
    (n, program)
}

/// Reconstruct the dense matrix by applying the word to every basis vector.
fn reconstruct(n: usize, program: &ComplexOperatorProgram) -> Vec<Vec<Complex64>> {
    let mut columns = Vec::with_capacity(n);
    for basis in 0..n {
        let mut state = vec![Complex64::ZERO; n];
        state[basis] = Complex64::ONE;
        program.apply(&mut state).unwrap();
        columns.push(state);
    }
    // columns[j][i] = matrix[i][j]
    (0..n)
        .map(|row| (0..n).map(|column| columns[column][row]).collect())
        .collect()
}

fn max_entry_distance(actual: &[Vec<Complex64>], expected: &[Vec<Complex64>]) -> f64 {
    let mut worst = 0.0_f64;
    for (actual_row, expected_row) in actual.iter().zip(expected) {
        for (a, e) in actual_row.iter().zip(expected_row) {
            worst = worst.max((*a - *e).abs());
        }
    }
    worst
}

fn matmul(a: &[Vec<Complex64>], b: &[Vec<Complex64>]) -> Vec<Vec<Complex64>> {
    let n = a.len();
    let m = b[0].len();
    let k_dim = b.len();
    (0..n)
        .map(|row| {
            (0..m)
                .map(|column| {
                    let mut sum = Complex64::ZERO;
                    for k in 0..k_dim {
                        sum = sum + a[row][k] * b[k][column];
                    }
                    sum
                })
                .collect()
        })
        .collect()
}

fn frobenius(a: &[Vec<Complex64>]) -> f64 {
    a.iter()
        .flat_map(|row| row.iter())
        .map(|value| value.abs_squared())
        .sum::<f64>()
        .sqrt()
}

/// Principal-branch eigenphases of a 2x2 unitary, ascending.
fn eigenphases_2x2(matrix: &[Vec<Complex64>]) -> [f64; 2] {
    let trace = matrix[0][0] + matrix[1][1];
    let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
    // lambda = (tr +/- sqrt(tr^2 - 4 det)) / 2, complex square root by hand
    let discriminant = trace * trace - determinant.scale(4.0);
    let radius = discriminant.abs().sqrt();
    let half_angle = discriminant.im.atan2(discriminant.re) / 2.0;
    let root = Complex64::new(radius * half_angle.cos(), radius * half_angle.sin());
    let first = (trace + root).scale(0.5);
    let second = (trace - root).scale(0.5);
    let mut phases = [first.im.atan2(first.re), second.im.atan2(second.re)];
    phases.sort_by(f64::total_cmp);
    phases
}

#[test]
fn committed_words_replay_to_npz_dense_matrices() {
    let data = vectors();
    for word in ["A", "B", "preparation", "readout"] {
        let record = &data["words"][word];
        let (n, program) = program_from(record);
        assert_eq!(n, 16);
        let dense = reconstruct(n, &program);
        let expected = complex_matrix(&record["expected_dense"]);
        let distance = max_entry_distance(&dense, &expected);
        assert!(distance < DENSE_TOLERANCE, "{word}: dense distance {distance}");

        for pair in record["state_pairs"].as_array().unwrap() {
            let mut state = complex_row(&pair["input"]);
            let receipt = program.apply_with_receipt(&mut state).unwrap();
            let expected_out = complex_row(&pair["output"]);
            for (actual, expected) in state.iter().zip(&expected_out) {
                assert!((*actual - *expected).abs() < DENSE_TOLERANCE);
            }
            assert!(receipt.measured_acoustic_state_work.abs() < 1.0e-13);
            if word == "A" {
                assert_eq!(receipt.two_mode_gate_count, 120);
                assert_eq!(receipt.phase_gate_count, 16);
            }
        }

        // inverse word returns the protected state (acceptance test 6)
        let mut state: Vec<Complex64> = (0..n)
            .map(|index| Complex64::new(1.0 / (index as f64 + 2.0), 0.1 * index as f64))
            .collect();
        let norm = mus_watsn::state_energy(&state).sqrt();
        for value in &mut state {
            *value = value.scale(1.0 / norm);
        }
        let original = state.clone();
        program.apply(&mut state).unwrap();
        program.inverse().apply(&mut state).unwrap();
        for (actual, expected) in state.iter().zip(&original) {
            assert!((*actual - *expected).abs() < DENSE_TOLERANCE);
        }
    }
}

#[test]
fn occupied_holonomies_and_nonabelian_witness_are_measured() {
    let data = vectors();
    let frame = complex_matrix(&data["occupied"]["base_frame"]);
    let n = frame.len();
    // frame_dagger[a][i] = conj(frame[i][a])
    let frame_dagger: Vec<Vec<Complex64>> = (0..2)
        .map(|a| (0..n).map(|i| frame[i][a].conj()).collect())
        .collect();

    let mut occupied = Vec::new();
    for word in ["A", "B"] {
        let (dim, program) = program_from(&data["words"][word]);
        let dense = reconstruct(dim, &program);
        let restricted = matmul(&frame_dagger, &matmul(&dense, &frame));
        let expected = complex_matrix(&data["occupied"][word]["matrix"]);
        assert!(max_entry_distance(&restricted, &expected) < 1.0e-10);

        let phases = eigenphases_2x2(&restricted);
        let expected_phases: Vec<f64> = data["occupied"][word]["eigenphases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_f64().unwrap())
            .collect();
        for (actual, expected) in phases.iter().zip(&expected_phases) {
            assert!(
                (actual - expected).abs() < WITNESS_TOLERANCE,
                "{word} eigenphase {actual} vs {expected}"
            );
        }
        occupied.push(restricted);
    }

    let forward = matmul(&occupied[0], &occupied[1]);
    let backward = matmul(&occupied[1], &occupied[0]);
    let commutator: Vec<Vec<Complex64>> = forward
        .iter()
        .zip(&backward)
        .map(|(f_row, b_row)| f_row.iter().zip(b_row).map(|(f, b)| *f - *b).collect())
        .collect();
    let witness = frobenius(&commutator);
    let expected = data["occupied"]["commutator_fro"].as_f64().unwrap();
    assert!(
        (witness - expected).abs() < WITNESS_TOLERANCE,
        "commutator {witness} vs {expected}"
    );
    // and it is genuinely non-Abelian, not merely nonzero
    assert!(witness > 1.5);
}

#[test]
fn random_unitary_fixtures_replay_across_dimensions() {
    let data = vectors();
    for record in data["random_unitaries"].as_array().unwrap() {
        let (n, program) = program_from(record);
        let dense = reconstruct(n, &program);
        let expected = complex_matrix(&record["expected_dense"]);
        let distance = max_entry_distance(&dense, &expected);
        // compile tolerance is 1e-9 upstream; allow the same order here
        assert!(distance < 1.0e-8, "n={n}: dense distance {distance}");
        for pair in record["state_pairs"].as_array().unwrap() {
            let mut state = complex_row(&pair["input"]);
            program.apply(&mut state).unwrap();
            for (actual, expected) in state.iter().zip(complex_row(&pair["output"]).iter()) {
                assert!((*actual - *expected).abs() < 1.0e-8);
            }
        }
    }
}

#[test]
fn illegal_real_factor_is_structurally_excluded() {
    // The complex IR cannot express a lone-quadrature rotation at all; the
    // executable counterexample shows why the real compiler must not be
    // promoted: same Euclidean-energy law, broken complex structure.
    let unpaired = real_givens_symplectic_residual(16, 3, 7, 0.9, false);
    let paired = real_givens_symplectic_residual(16, 3, 7, 0.9, true);
    assert!(unpaired > 1.0);
    assert!(paired < 1.0e-13);
}
