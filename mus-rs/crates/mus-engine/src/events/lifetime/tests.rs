//! Acceptance tests for the track-owned lifetime kernel.

use std::collections::BTreeMap;

use mus_dsp::pluck::{ControlFrame, ControlTarget, StringNetworkVoice};

use crate::gestures::Gesture;
use crate::layout::{BarMap, BarTiming};
use crate::mixbus::MixBuffers;
use crate::pack::{SampleCache, Voice};
use crate::RenderStats;

use super::render::{
    render_controlled_voice, walk_track, TRACK_LIFETIME_CLAIM, TRACK_LIFETIME_EVIDENCE,
};
use super::WalkContext;

fn context<'a>(
    bar_map: &'a BarMap,
    bar_content: &'a BTreeMap<u32, BTreeMap<String, String>>,
    gestures: &'a BTreeMap<String, Gesture>,
    cache: &'a SampleCache,
) -> WalkContext<'a> {
    WalkContext {
        bar_map,
        bar_content,
        n_bars: 2,
        swing_unit_ql: 0.0,
        swing_r: 0.5,
        a4: 440.0,
        gestures,
        tape_audio: None,
        cache,
        n_total: 48_000 * 8,
        eager: true,
        sc_track: None,
    }
}

fn two_bar_map() -> BarMap {
    BarMap {
        bars: BTreeMap::from([
            (
                1,
                BarTiming {
                    start_s: 0.0,
                    qlen: 4.0,
                    spq: 0.5,
                },
            ),
            (
                2,
                BarTiming {
                    start_s: 2.0,
                    qlen: 4.0,
                    spq: 0.5,
                },
            ),
        ]),
        total_s: 8.0,
    }
}

fn track_voice() -> Voice {
    Voice {
        abbrev: "w".to_string(),
        samples: Vec::new(),
        mode: "varispeed".to_string(),
        gain_db: 0.0,
        pan: 0.0,
        send: 0.0,
        defaults: BTreeMap::from([
            ("body_lifetime".to_string(), "track".to_string()),
            ("traj_curve".to_string(), "smootherstep".to_string()),
            ("traj_hz".to_string(), "200".to_string()),
        ]),
        synth: Some(BTreeMap::from([
            ("synth".to_string(), "weave".to_string()),
            ("courses".to_string(), "3".to_string()),
            ("sus".to_string(), "0.20".to_string()),
            (
                "couple".to_string(),
                "0.10@0->0.32@0.5->0.16@1".to_string(),
            ),
        ])),
        ducks: true,
    }
}

#[test]
fn monotone_executor_is_audio_identical_to_the_traced_executor() {
    let params = BTreeMap::from([
        ("synth".to_string(), "weave".to_string()),
        ("courses".to_string(), "3".to_string()),
        ("sus".to_string(), "0.12".to_string()),
    ]);
    let frames = vec![
        ControlFrame {
            at_sample: 0,
            sets: vec![(ControlTarget::Couple, 0.1)],
        },
        ControlFrame {
            at_sample: 2400,
            sets: vec![(ControlTarget::Couple, 0.3)],
        },
    ];
    let expected = StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0)
        .render_traced_with(128, &frames)
        .0;
    let actual = render_controlled_voice(
        StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0),
        &frames,
    );
    assert_eq!(actual, expected);
}

#[test]
fn one_body_spans_two_bars_and_collapses_three_note_boundaries_to_one_attack() {
    let bar_map = two_bar_map();
    let bar_content = BTreeMap::from([
        (
            1,
            BTreeMap::from([("w".to_string(), "A3q Rq A3q Rq".to_string())]),
        ),
        (
            2,
            BTreeMap::from([("w".to_string(), "A3h Rh".to_string())]),
        ),
    ]);
    let gestures = BTreeMap::new();
    let cache = SampleCache::new();
    let ctx = context(&bar_map, &bar_content, &gestures, &cache);
    let mut buffers = MixBuffers::new(ctx.n_total);
    let mut placed = Vec::new();
    let mut sc_onsets = Vec::new();
    let mut stats = RenderStats::default();

    walk_track(
        &ctx,
        &track_voice(),
        &mut buffers,
        &mut placed,
        &mut sc_onsets,
        &mut stats,
    )
    .unwrap();

    assert_eq!(stats.notes, 3);
    assert_eq!(stats.per_track_events.get("w").copied(), Some(3));
    assert_eq!(stats.trajectory_receipts.len(), 1);
    let event = &stats.trajectory_receipts[0];
    assert_eq!(event.bar, 1);
    assert_eq!(event.onset_seconds, 0.0);
    assert_eq!(event.slot_seconds, 4.0);
    assert_eq!(event.receipt.trajectory.claim_level, TRACK_LIFETIME_CLAIM);
    assert_eq!(
        event.receipt.trajectory.evidence_class,
        TRACK_LIFETIME_EVIDENCE
    );
}

#[test]
fn breath_that_rises_from_zero_refuses_before_body_construction() {
    let bar_map = two_bar_map();
    let bar_content = BTreeMap::from([(
        1,
        BTreeMap::from([("w".to_string(), "A3w".to_string())]),
    )]);
    let gestures = BTreeMap::new();
    let cache = SampleCache::new();
    let ctx = context(&bar_map, &bar_content, &gestures, &cache);
    let mut voice = track_voice();
    voice
        .synth
        .as_mut()
        .expect("synth patch")
        .insert("breath".to_string(), "0@0->0.45@1".to_string());
    let mut buffers = MixBuffers::new(ctx.n_total);
    let mut placed = Vec::new();
    let mut sc_onsets = Vec::new();
    let mut stats = RenderStats::default();
    let error = walk_track(
        &ctx,
        &voice,
        &mut buffers,
        &mut placed,
        &mut sc_onsets,
        &mut stats,
    )
    .unwrap_err();
    assert!(error.to_string().contains("preallocated"), "{error}");
}

#[test]
fn a_pitch_set_change_refuses_instead_of_reconstructing_the_body() {
    let bar_map = two_bar_map();
    let bar_content = BTreeMap::from([(
        1,
        BTreeMap::from([("w".to_string(), "A3q B3q Rh".to_string())]),
    )]);
    let gestures = BTreeMap::new();
    let cache = SampleCache::new();
    let ctx = context(&bar_map, &bar_content, &gestures, &cache);
    let mut buffers = MixBuffers::new(ctx.n_total);
    let mut placed = Vec::new();
    let mut sc_onsets = Vec::new();
    let mut stats = RenderStats::default();
    let error = walk_track(
        &ctx,
        &track_voice(),
        &mut buffers,
        &mut placed,
        &mut sc_onsets,
        &mut stats,
    )
    .unwrap_err();
    assert!(error.to_string().contains("fixed course topology"), "{error}");
}

#[test]
fn source_stage_first_event_controls_refuse_instead_of_being_ignored() {
    let bar_map = two_bar_map();
    let gestures = BTreeMap::new();
    let cache = SampleCache::new();

    for token in ["A3w[off=10]", "A3w[reverse]"] {
        let bar_content = BTreeMap::from([(
            1,
            BTreeMap::from([("w".to_string(), token.to_string())]),
        )]);
        let ctx = context(&bar_map, &bar_content, &gestures, &cache);
        let mut buffers = MixBuffers::new(ctx.n_total);
        let mut placed = Vec::new();
        let mut sc_onsets = Vec::new();
        let mut stats = RenderStats::default();
        let error = walk_track(
            &ctx,
            &track_voice(),
            &mut buffers,
            &mut placed,
            &mut sc_onsets,
            &mut stats,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("silently ignored")
                || error.to_string().contains("source resolution"),
            "{token}: {error}"
        );
    }
}

#[test]
fn body_lifetime_cannot_be_overridden_by_an_event() {
    let bar_map = two_bar_map();
    let bar_content = BTreeMap::from([(
        1,
        BTreeMap::from([(
            "w".to_string(),
            "A3w[body_lifetime=event]".to_string(),
        )]),
    )]);
    let gestures = BTreeMap::new();
    let cache = SampleCache::new();
    let ctx = context(&bar_map, &bar_content, &gestures, &cache);
    let mut buffers = MixBuffers::new(ctx.n_total);
    let mut placed = Vec::new();
    let mut sc_onsets = Vec::new();
    let mut stats = RenderStats::default();
    let error = walk_track(
        &ctx,
        &track_voice(),
        &mut buffers,
        &mut placed,
        &mut sc_onsets,
        &mut stats,
    )
    .unwrap_err();
    assert!(error.to_string().contains("instrument-owned"), "{error}");
}
