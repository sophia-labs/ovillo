use std::collections::BTreeMap;

use mus_dsp::pluck::StringNetworkVoice;
use mus_dsp::trajectory::{
    program_from_inline_string_params, render_weave_program, StringTrajectoryErrorKind,
};

const NOTES: [f64; 2] = [110.0, 165.0];

fn patch() -> BTreeMap<String, String> {
    [
        ("synth", "weave"),
        ("sus", "1.2"),
        ("courses", "9"),
        ("couple", "0.17"),
        ("chirality", "0.4"),
        ("curvature", "0.5"),
        ("body", "0.4"),
        ("orbit", "0.5"),
        ("orbit_depth", "0.7"),
        ("retune", "neutral_filters"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect()
}

#[test]
fn constant_direct_path_is_the_historical_render() {
    let source = BTreeMap::from([
        ("couple".to_string(), "0.17->0.17".to_string()),
        ("body".to_string(), "0.4->0.4".to_string()),
    ]);
    let (static_params, program) =
        program_from_inline_string_params(&source, 0.5).expect("inline program");
    let mut full_patch = patch();
    full_patch.extend(static_params);
    let direct = StringNetworkVoice::weave(&full_patch, &NOTES, 0.5, 1.0).render();
    let rendered = render_weave_program(
        &full_patch,
        &NOTES,
        0.5,
        1.0,
        &program.expect("program"),
        256,
    )
    .expect("trajectory render");
    assert_eq!(direct, rendered.audio);
}

#[test]
fn moving_body_path_is_deterministic_and_nontrivial() {
    let source = BTreeMap::from([
        ("couple".to_string(), "0.17->0.42->0.17".to_string()),
        ("body".to_string(), "0.4->0.8->0.4".to_string()),
        ("traj_curve".to_string(), "cosine".to_string()),
        ("traj_hz".to_string(), "200".to_string()),
    ]);
    let (static_params, program) =
        program_from_inline_string_params(&source, 0.5).expect("inline program");
    let mut full_patch = patch();
    full_patch.extend(static_params);
    let program = program.expect("program");
    let first = render_weave_program(&full_patch, &NOTES, 0.5, 1.0, &program, 256)
        .expect("first render");
    let second = render_weave_program(&full_patch, &NOTES, 0.5, 1.0, &program, 256)
        .expect("second render");
    assert_eq!(first.audio, second.audio);
    assert_ne!(
        first.audio,
        StringNetworkVoice::weave(&patch(), &NOTES, 0.5, 1.0).render()
    );
    assert!(first.audio.iter().all(|sample| sample.is_finite()));
    assert_eq!(
        first.receipt.trajectory.program_digest,
        second.receipt.trajectory.program_digest
    );
    assert!(first.receipt.control_frame_count > 20);
}

#[test]
fn pitch_path_uses_native_persistent_retune_and_keeps_its_debt_visible() {
    let source = BTreeMap::from([(
        "pitch_ratio".to_string(),
        "1->1.0594630943592953".to_string(),
    )]);
    let (static_params, program) =
        program_from_inline_string_params(&source, 0.5).expect("inline program");
    let mut full_patch = patch();
    full_patch.extend(static_params);
    let rendered = render_weave_program(
        &full_patch,
        &NOTES,
        0.5,
        1.0,
        &program.expect("program"),
        256,
    )
    .expect("pitch render");
    assert_eq!(
        rendered.receipt.native_pitch_transport,
        "persistent-linear-f0-to-f1"
    );
    assert!(rendered
        .receipt
        .unresolved_obligations
        .contains(&"integer-delay-state-transport".to_string()));
    assert!(rendered.audio.iter().all(|sample| sample.is_finite()));
}

#[test]
fn strict_mode_refuses_before_audio() {
    let source = BTreeMap::from([
        ("pitch_ratio".to_string(), "1->1.02".to_string()),
        ("traj_strict".to_string(), "1".to_string()),
    ]);
    let (static_params, program) =
        program_from_inline_string_params(&source, 0.5).expect("inline program");
    let mut full_patch = patch();
    full_patch.extend(static_params);
    let error = render_weave_program(
        &full_patch,
        &NOTES,
        0.5,
        1.0,
        &program.expect("program"),
        256,
    )
    .err()
    .expect("strict refusal");
    assert_eq!(error.kind, StringTrajectoryErrorKind::TransportUncertified);
}
