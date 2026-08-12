//! Acceptance tests for the track-owned multi-attack lifetime kernel.

use std::collections::BTreeMap;

use crate::gestures::Gesture;
use crate::layout::{BarMap, BarTiming};
use crate::mixbus::MixBuffers;
use crate::pack::{SampleCache, Voice};
use crate::RenderStats;

use super::render::{walk_track, TRACK_LIFETIME_CLAIM, TRACK_LIFETIME_EVIDENCE};
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
            ("strum".to_string(), "0".to_string()),
            ("sus".to_string(), "0.20".to_string()),
            (
                "couple".to_string(),
                "0.10@0->0.32@0.5->0.16@1".to_string(),
            ),
        ])),
        ducks: true,
    }
}

fn rms(values: &[f32]) -> f64 {
    (values
        .iter()
        .map(|&value| f64::from(value) * f64::from(value))
        .sum::<f64>()
        / values.len().max(1) as f64)
        .sqrt()
}

#[test]
fn one_body_receives_three_physical_attacks_across_two_bars() {
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
    assert!(
        event
            .receipt
            .unresolved_obligations
            .iter()
            .any(|item| item == "rearticulation-excitation-work-ledger")
    );

    // At sus=0.20 the first attack is effectively gone by the second and
    // third score onsets. Nontrivial energy around both later onsets is the
    // audible evidence that they are now physical reattacks, not notation-only
    // lifetime markers.
    let left = &buffers.dry[0];
    assert!(rms(&left[48_000..50_400]) > 1e-6);
    assert!(rms(&left[96_000..98_400]) > 1e-6);
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

#[test]
fn strict_transport_refuses_multi_attack_work_without_a_ledger() {
    let bar_map = two_bar_map();
    let bar_content = BTreeMap::from([(
        1,
        BTreeMap::from([("w".to_string(), "A3q Rq A3q Rq".to_string())]),
    )]);
    let gestures = BTreeMap::new();
    let cache = SampleCache::new();
    let ctx = context(&bar_map, &bar_content, &gestures, &cache);
    let mut voice = track_voice();
    voice
        .defaults
        .insert("traj_strict".to_string(), "1".to_string());
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
    assert!(
        error
            .to_string()
            .contains("rearticulation excitation work"),
        "{error}"
    );
}
