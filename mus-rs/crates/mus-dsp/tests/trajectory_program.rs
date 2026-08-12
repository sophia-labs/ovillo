use std::collections::BTreeMap;

use mus_dsp::pluck::{weave_note, StringNetworkVoice};
use mus_dsp::trajectory::{
    compile_string_trajectory, StringTrajectoryError, StringTrajectoryVoice,
};
use mus_trajectory::TrajectoryProgram;

fn patch(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn working_patch() -> BTreeMap<String, String> {
    patch(&[
        ("synth", "weave"),
        ("sus", "0.8"),
        ("courses", "7"),
        ("couple", "0.12"),
        ("chirality", "0.3"),
        ("curvature", "0.4"),
        ("body", "0.35"),
        ("orbit", "0.7"),
        ("orbit_depth", "0.5"),
    ])
}

#[test]
fn empty_program_is_byte_identical_to_the_unwrapped_voice() {
    let patch = working_patch();
    let expected = weave_note(&patch, &[110.0, 165.0], 0.24, 1.0);
    let voice = StringNetworkVoice::weave(&patch, &[110.0, 165.0], 0.24, 1.0);
    let program = TrajectoryProgram::empty(48_000, 11_520);
    let actual = StringTrajectoryVoice::new(voice, program)
        .unwrap()
        .render();
    assert_eq!(expected, actual);
}

#[test]
fn zero_motion_arrow_is_byte_identical_to_a_scalar_patch() {
    let scalar = working_patch();
    let mut moving = scalar.clone();
    moving.insert("couple".into(), "0.12->0.12".into());
    moving.insert("trajectory_curve".into(), "smootherstep".into());
    let expected = weave_note(&scalar, &[110.0, 165.0], 0.24, 1.0);
    let actual = StringTrajectoryVoice::weave(&moving, &[110.0, 165.0], 0.24, 1.0)
        .unwrap()
        .render();
    assert_eq!(expected, actual);
}

#[test]
fn host_block_partition_cannot_change_the_path() {
    let mut patch = working_patch();
    patch.insert("couple".into(), "0.03->0.38->0.11".into());
    patch.insert("body".into(), "0.2->0.75->0.3".into());
    patch.insert("trajectory_curve".into(), "smoothstep".into());

    let whole = StringTrajectoryVoice::weave(&patch, &[110.0, 165.0], 0.30, 1.0)
        .unwrap()
        .render();
    let mut chunked = StringTrajectoryVoice::weave(&patch, &[110.0, 165.0], 0.30, 1.0)
        .unwrap();
    let block_sizes = [1_usize, 7, 64, 257, 31, 1024, 3];
    let mut cursor = 0;
    let mut actual = Vec::with_capacity(whole.len());
    while !chunked.is_finished() {
        let size = block_sizes[cursor % block_sizes.len()].min(chunked.remaining_samples());
        let mut block = vec![0.0; size];
        let written = chunked.render_block(&mut block);
        assert_eq!(written, size);
        actual.extend(block);
        cursor += 1;
    }
    assert_eq!(whole, actual);
}

#[test]
fn a_real_trajectory_is_not_a_static_endpoint_counterfeit() {
    let mut moving = working_patch();
    moving.insert("couple".into(), "0.02->0.42->0.08".into());
    moving.insert("curvature".into(), "0.1->0.9".into());
    moving.insert("trajectory_curve".into(), "smootherstep".into());

    let mut start = working_patch();
    start.insert("couple".into(), "0.02".into());
    start.insert("curvature".into(), "0.1".into());
    let mut end = working_patch();
    end.insert("couple".into(), "0.08".into());
    end.insert("curvature".into(), "0.9".into());

    let moving_audio = StringTrajectoryVoice::weave(&moving, &[110.0, 165.0], 0.30, 1.0)
        .unwrap()
        .render();
    assert_ne!(moving_audio, weave_note(&start, &[110.0, 165.0], 0.30, 1.0));
    assert_ne!(moving_audio, weave_note(&end, &[110.0, 165.0], 0.30, 1.0));
}

#[test]
fn pitch_ratio_is_refused_instead_of_resampled() {
    let mut patch = working_patch();
    patch.insert("pitch_ratio".into(), "1.0->1.5".into());
    let error = compile_string_trajectory(&patch, 4_800).unwrap_err();
    assert_eq!(
        error,
        StringTrajectoryError::PitchTransportPending,
        "the open mathematical seam has a typed boundary"
    );
    assert_eq!(error.code(), "string-trajectory-pitch-transport-pending");
}

#[test]
fn out_of_domain_authorship_refuses_before_rendering() {
    let mut patch = working_patch();
    patch.insert("couple".into(), "0.1->0.8".into());
    let error = compile_string_trajectory(&patch, 4_800).unwrap_err();
    assert_eq!(error.code(), "trajectory-above-maximum");
}
