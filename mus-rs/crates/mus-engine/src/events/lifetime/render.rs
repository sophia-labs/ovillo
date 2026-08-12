//! Allocation-conscious execution of one prevalidated track-body session.

use std::collections::BTreeMap;

use mus_dsp::pluck::{ControlFrame, StringNetworkVoice};
use mus_dsp::trajectory::{lower_string_trajectory, program_from_inline_string_params};

use crate::mixbus::{db_to_gain, pan_and_haas, truncate_to_render, MixBuffers, PlacedNote};
use crate::pack::Voice;
use crate::util::{midi_to_hz, param_pair, require_f64, require_f64_opt};
use crate::{EventTrajectoryReceipt, RenderError, RenderStats};

use super::plan::{ctx_error, ctx_existing, plan_session};
use super::WalkContext;

pub(super) const TRACK_LIFETIME_CLAIM: &str = "score-spanning-single-body";
pub(super) const TRACK_LIFETIME_EVIDENCE: &str =
    "validated-numerics+engine-execution+single-construction+monotone-no-trace";

/// Execute already-canonical, monotonically ordered control frames without
/// creating Observatory telemetry. This is the ordinary-render fast path; the
/// probe/analysis entry points retain `render_traced_with`.
pub(super) fn render_controlled_voice(
    mut voice: StringNetworkVoice,
    control_frames: &[ControlFrame],
) -> Vec<f32> {
    debug_assert!(control_frames
        .windows(2)
        .all(|pair| pair[0].at_sample <= pair[1].at_sample));
    let total_samples = voice.remaining_samples();
    let mut output = Vec::with_capacity(total_samples);
    let mut next_frame = 0_usize;
    for sample_index in 0..total_samples {
        while next_frame < control_frames.len()
            && control_frames[next_frame].at_sample <= sample_index
        {
            for &(target, value) in &control_frames[next_frame].sets {
                voice.apply_control(target, value);
            }
            next_frame += 1;
        }
        output.push(voice.next_sample());
    }
    output
}

pub(super) fn walk_track(
    ctx: &WalkContext,
    voice: &Voice,
    buffers: &mut MixBuffers,
    placed: &mut Vec<PlacedNote>,
    sc_onsets: &mut Vec<f64>,
    stats: &mut RenderStats,
) -> Result<(), RenderError> {
    let abbr = voice.abbrev.as_str();
    let Some(session) = plan_session(ctx, voice, stats)? else {
        return Ok(());
    };
    let synth_defaults = voice.synth.as_ref().ok_or_else(|| {
        ctx_error(
            session.first_bar,
            abbr,
            "body_lifetime=track is currently implemented only for synth=pluck or synth=weave",
        )
    })?;

    let mut authored: BTreeMap<String, String> = synth_defaults.clone();
    authored.extend(session.first_event.params.clone());
    let (static_patch, maybe_program) =
        program_from_inline_string_params(&authored, session.duration_seconds).map_err(|error| {
            ctx_error(
                session.first_bar,
                abbr,
                format!("track trajectory: {error}"),
            )
        })?;
    let mut program = maybe_program.ok_or_else(|| {
        ctx_error(
            session.first_bar,
            abbr,
            "body_lifetime=track requires at least one registered arrow-valued physical trajectory",
        )
    })?;
    program
        .metadata
        .insert("lifetime_scope".to_string(), "track".to_string());
    program
        .metadata
        .insert("lifetime_track".to_string(), abbr.to_string());
    program.metadata.insert(
        "member_event_count".to_string(),
        session.member_event_count.to_string(),
    );
    program.metadata.insert(
        "notated_event_count".to_string(),
        session.note_onsets.len().to_string(),
    );
    program.metadata.insert(
        "physical_attack_count".to_string(),
        "1".to_string(),
    );
    program.metadata.insert(
        "body_construction_count".to_string(),
        "1".to_string(),
    );
    let event_local_baseline = session.note_onsets.len();
    program.metadata.insert(
        "event_local_body_construction_baseline".to_string(),
        event_local_baseline.to_string(),
    );
    program.metadata.insert(
        "avoided_body_constructions".to_string(),
        event_local_baseline.saturating_sub(1).to_string(),
    );
    program
        .metadata
        .insert("session_tail_count".to_string(), "1".to_string());
    program.metadata.insert(
        "event_local_tail_baseline".to_string(),
        event_local_baseline.to_string(),
    );
    program.metadata.insert(
        "ordinary_trace_allocation".to_string(),
        "none".to_string(),
    );
    program.metadata.insert(
        "lifetime_end_seconds".to_string(),
        session.end_seconds.to_string(),
    );
    program.metadata.insert(
        "execution_strategy".to_string(),
        "single-construction-monotone-no-trace-v1".to_string(),
    );

    let has_sing_lane = program
        .tracks
        .iter()
        .any(|track| track.coordinate == "string.sing");
    let scalar_sing = static_patch
        .get("sing")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    if has_sing_lane || scalar_sing > 0.0 {
        return Err(ctx_error(
            session.first_bar,
            abbr,
            "track-lifetime execution with an external sing source or string.sing lane is not yet supply/work-accounted",
        ));
    }

    if let Some(breath_track) = program
        .tracks
        .iter()
        .find(|track| track.coordinate == "string.breath")
    {
        let starts_silent = breath_track
            .knots
            .first()
            .is_some_and(|knot| knot.value <= 1e-12);
        let later_breathes = breath_track
            .knots
            .iter()
            .any(|knot| knot.value > 1e-12);
        if starts_silent && later_breathes {
            return Err(ctx_error(
                session.first_bar,
                abbr,
                "string.breath cannot rise from zero in track-lifetime v1 because its content-keyed noise channels must be preallocated at body construction",
            ));
        }
    }

    let frequencies_hz: Vec<f64> = session
        .first_event
        .midis
        .iter()
        .map(|&midi| midi_to_hz(midi, ctx.a4))
        .collect();
    let score_gliss_ratio = session
        .first_event
        .gliss_midi
        .map(|target| midi_to_hz(target, ctx.a4) / frequencies_hz[0])
        .unwrap_or(1.0);
    let lowered = lower_string_trajectory(
        &program,
        &frequencies_hz,
        session.duration_seconds,
        score_gliss_ratio,
    )
    .map_err(|error| {
        ctx_error(
            session.first_bar,
            abbr,
            format!("track trajectory: {error}"),
        )
    })?;

    let body = match static_patch.get("synth").map(String::as_str) {
        Some("pluck") => StringNetworkVoice::guitar(
            &static_patch,
            &lowered.frequencies_hz,
            session.duration_seconds,
            lowered.native_gliss_ratio,
        ),
        Some("weave") => StringNetworkVoice::weave(
            &static_patch,
            &lowered.frequencies_hz,
            session.duration_seconds,
            lowered.native_gliss_ratio,
        ),
        other => {
            return Err(ctx_error(
                session.first_bar,
                abbr,
                format!(
                    "body_lifetime=track requires synth=pluck or synth=weave, got {other:?}"
                ),
            ));
        }
    };
    let audio = render_controlled_voice(body, &lowered.control_frames);

    let mut receipt = lowered.receipt;
    receipt.trajectory.claim_level = TRACK_LIFETIME_CLAIM.to_string();
    receipt.trajectory.evidence_class = TRACK_LIFETIME_EVIDENCE.to_string();

    let audio = crate::transforms::apply_chain(
        audio,
        &session.first_event,
        session.duration_seconds,
    )
    .map_err(|error| ctx_existing(session.first_bar, abbr, error))?;
    let db = crate::transforms::level_db(&session.dynamic, voice, &session.first_event)
        .map_err(|error| ctx_existing(session.first_bar, abbr, error))?;
    let Some((start, audio)) = truncate_to_render(audio, session.onset_seconds, ctx.n_total) else {
        return Ok(());
    };

    let (pan_start, pan_end) = param_pair(&session.first_event.params, "pan", voice.pan);
    let haas_ms = require_f64_opt(&session.first_event.params, "haas")
        .map_err(|error| ctx_existing(session.first_bar, abbr, error))?;
    let (left, right) = pan_and_haas(&audio, pan_start, pan_end, haas_ms);
    let send = require_f64(&session.first_event.params, "send", voice.send)
        .map_err(|error| ctx_existing(session.first_bar, abbr, error))?;
    let gain = db_to_gain(db);

    if ctx.sc_track == Some(abbr) {
        sc_onsets.extend(session.note_onsets.iter().copied());
    }
    if ctx.eager {
        buffers.mix_one(start, &left, &right, send, gain, None);
    } else {
        placed.push(PlacedNote {
            track: abbr.to_string(),
            ducks: voice.ducks,
            start,
            l: left,
            r: right,
            send,
            gain,
        });
    }

    let notated_events = u32::try_from(session.note_onsets.len()).map_err(|_| {
        ctx_error(
            session.first_bar,
            abbr,
            "track lifetime contains more notated events than fit in the render statistics counter",
        )
    })?;
    stats.notes += notated_events;
    *stats.per_track_events.entry(abbr.to_string()).or_default() += notated_events;
    stats.trajectory_receipts.push(EventTrajectoryReceipt {
        bar: session.first_bar,
        track: abbr.to_string(),
        onset_seconds: session.onset_seconds,
        slot_seconds: session.duration_seconds,
        receipt,
    });

    Ok(())
}
