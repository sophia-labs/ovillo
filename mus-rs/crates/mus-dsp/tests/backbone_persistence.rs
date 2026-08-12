//! U3 persistence law 4: the backbone executes the committed unified
//! words across separate events and lands on the Python checkpoints —
//! `W_B W_A prepared` for AB, the reset counterfeit at its fixture
//! distance. Persistence semantics per theory §13.3: a gesture boundary
//! is not a state reset.

use mus_dsp::backbone::{ProtectedBackbone, BACKBONE_MODES};
use mus_watsn::{
    Complex64, ComplexGate, ComplexOperatorProgram, ComplexPhaseGate, ComplexTwoModeGate,
};
use serde_json::Value;

fn vectors() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../paper/ariadne-protected-nonabelian/unified_artifacts/unified_rust_test_vectors.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("vectors present")).expect("valid")
}

fn complex(entry: &Value) -> Complex64 {
    Complex64::new(entry[0].as_f64().unwrap(), entry[1].as_f64().unwrap())
}

fn state_16(entry: &Value) -> [Complex64; BACKBONE_MODES] {
    let mut out = [Complex64::ZERO; BACKBONE_MODES];
    for (slot, value) in out.iter_mut().zip(entry.as_array().unwrap()) {
        *slot = complex(value);
    }
    out
}

fn word(record: &Value) -> ComplexOperatorProgram {
    let gates = record["schedule"]
        .as_array()
        .unwrap()
        .iter()
        .map(|gate| match gate["kind"].as_str().unwrap() {
            "phase_shift" => ComplexGate::Phase(ComplexPhaseGate {
                mode: gate["i"].as_u64().unwrap() as usize,
                phase: gate["phase"].as_f64().unwrap(),
            }),
            _ => {
                let block = &gate["block"];
                ComplexGate::TwoMode(ComplexTwoModeGate {
                    left: gate["i"].as_u64().unwrap() as usize,
                    right: gate["j"].as_u64().unwrap() as usize,
                    matrix: [
                        complex(&block[0][0]),
                        complex(&block[0][1]),
                        complex(&block[1][0]),
                        complex(&block[1][1]),
                    ],
                })
            }
        })
        .collect();
    ComplexOperatorProgram::new(
        BACKBONE_MODES,
        gates,
        None,
        ComplexOperatorProgram::DEFAULT_UNITARITY_TOLERANCE,
    )
    .expect("committed word validates")
}

fn distance(a: &[Complex64; BACKBONE_MODES], b: &[Complex64; BACKBONE_MODES]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (*x - *y).abs_squared())
        .sum::<f64>()
        .sqrt()
}

#[test]
fn persistent_ab_matches_the_python_checkpoint_across_event_gaps() {
    let data = vectors();
    let word_a = word(&data["words"]["A"]);
    let word_b = word(&data["words"]["B"]);
    let prepared = state_16(&data["persistent"]["prepared"]);
    let expected_ab = state_16(&data["persistent"]["final_ab"]);
    let expected_ba = state_16(&data["persistent"]["final_ba"]);

    // AB: word A on event one, word B on event two, a silent gap between.
    let mut backbone = ProtectedBackbone::new(0, 0.0, 0.0).unwrap();
    backbone.prepare(&prepared);
    backbone.arm_word(&word_a, 100, 4800).unwrap();
    backbone.arm_word(&word_b, 9600, 4800).unwrap();
    backbone.advance_gap(20_000, &mut []);
    assert!(
        distance(backbone.state(), &expected_ab) < 1.0e-12,
        "AB persistent state must equal W_B W_A prepared"
    );

    // BA with the same machinery
    let mut backbone = ProtectedBackbone::new(0, 0.0, 0.0).unwrap();
    backbone.prepare(&prepared);
    backbone.arm_word(&word_b, 100, 4800).unwrap();
    backbone.arm_word(&word_a, 9600, 4800).unwrap();
    backbone.advance_gap(20_000, &mut []);
    assert!(distance(backbone.state(), &expected_ba) < 1.0e-12);

    // the AB/BA order defect replays the exported checkpoint distance
    let ab_ba = data["persistent"]["ab_ba_distance"].as_f64().unwrap();
    assert!((distance(&expected_ab, &expected_ba) - ab_ba).abs() < 1.0e-12);
    assert!(ab_ba > 1.0, "the order defect is macroscopic");
}

#[test]
fn reset_between_words_is_the_counterfeit_at_its_fixture_distance() {
    let data = vectors();
    let word_a = word(&data["words"]["A"]);
    let word_b = word(&data["words"]["B"]);
    let prepared = state_16(&data["persistent"]["prepared"]);
    let expected_ab = state_16(&data["persistent"]["final_ab"]);

    let mut backbone = ProtectedBackbone::new(0, 0.0, 0.0).unwrap();
    backbone.prepare(&prepared);
    backbone.arm_word(&word_a, 100, 4800).unwrap();
    backbone.advance_gap(9_600, &mut []);
    backbone.reset();
    backbone.prepare(&prepared);
    backbone.arm_word(&word_b, 9_700, 4800).unwrap();
    backbone.advance_gap(10_000, &mut []);

    let counterfeit_distance = distance(backbone.state(), &expected_ab);
    let expected = data["persistent"]["reset_counterfeit_distance"]
        .as_f64()
        .unwrap();
    assert!(
        (counterfeit_distance - expected).abs() < 1.0e-10,
        "reset counterfeit distance {counterfeit_distance} vs fixture {expected}"
    );
    // the fixture's own value, replayed to the digit the summary quotes
    assert!((expected - 0.889_660_680_171_708).abs() < 1.0e-9);
}
