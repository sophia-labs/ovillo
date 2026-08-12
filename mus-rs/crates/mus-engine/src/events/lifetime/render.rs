//! Allocation-conscious execution of one prevalidated track-body session.

use std::collections::BTreeMap;

use mus_dsp::pluck::{
    AttackFrame, AttackSchedule, RearticulatedStringVoice, StringNetworkVoice,
};
use mus_dsp::trajectory::{lower_string_trajectory, program_from_inline_string_params};

use crate::mixbus::{db_to_gain, pan_and_haas, truncate_to_render, MixBuffers, PlacedNote};
use crate::pack::Voice;
use crate::util::{midi_to_hz, param_pair, require_f64, require_f64_opt};
use crate::{EventTrajectoryReceipt, RenderError, RenderStats};

use super::plan::{ctx_error, ctx_existing, plan_session};
use super::WalkContext;

pub(super) const TRACK_LIFETIME_CLAIM: &str =
    "score-spanning-preallocated-rearticulation";
pub(super) const TRACK_LIFETIME_EVIDENCE: &str =
    "validated-numerics+engine-execution+single-construction+preallocated-attacks+network-observer-off-specialization";

const REARTICULATION_WORK_OBLIGATION: &str = "rearticulation-excitation-work-ledger";

fn attack_frames(
    session: &super::plan::TrackBodySession,
    abbr: &str,
) -> Result<Vec<AttackFrame>, RenderError> {
    let mut frames = Vec::with_capacity(session.note_onsets.len());
    for &onset in &session.note_onsets {
        let relative_seconds = onset - session.onset_seconds;
        if !relative_seconds.is_finite() || relative_seconds < 0.0 {
            return Err(ctx_error(
                session.first_bar,
                abbr,
                format!(
                    "rearticulation onset {onset} is before/non-finite relative to session start {}",
                    session.onset_seconds
                ),
            ));
        }
        let sample = relative_seconds * mus_dsp::SR_F64;
        if !sample.is_finite() || sample > usize::MAX as f64 {
            return Err(ctx_error(
                session.first_bar,
                abbr,
                format!("rearticulation onset {relative_seconds}s does not fit a sample index"),
            ));
        }
        frames.push(AttackFrame {
            at_sample: sample.round() as usize,
            gain: 1.0,
        });
    }
    Ok(frames)
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

    let attacks = attack_frames(&session, abbr)?;
    let attack_samples = attacks
        .iter()
        .map(|attack| attack.at_sample.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let notated_event_count = session.note_onsets.len();
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
        notated_event_count.to_string(),
    );
    program.metadata.insert(
        "physical_attack_count".to_string(),
        attacks.len().to_string(),
    );
    program
        .metadata
        .insert("attack_samples".to_string(), attack_samples);
    program
        .metadata
        .insert("attack_gain_policy".to_string(), "unity-v2".to_string());
    program.metadata.insert(
        "attack_excitation_identity".to_string(),
        "constructor-content-keyed-template".to_string(),
    );
    program
        .metadata
        .insert("attack_overlap_policy".to_string(), "refuse-v1".to_string());
    program.metadata.insert(
        "body_construction_count".to_string(),
        "1".to_string(),
    );
    program.metadata.insert(
        "event_local_body_construction_baseline".to_string(),
        notated_event_count.to_string(),
    );
    program.metadata.insert(
        "avoided_body_constructions".to_string(),
        notated_event_count.saturating_sub(1).to_string(),
    );
    program
        .metadata
        .insert("session_tail_count".to_string(), "1".to_string());
    program.metadata.insert(
        "event_local_tail_baseline".to_string(),
        notated_event_count.to_string(),
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
        "prevalidated-monotone-attack-cursor+network-observer-off-v2".to_string(),
    );

    if attacks.len() > 1
        && program
            .metadata
            .get("strict_transport")
            .is_some_and(|value| value == "true" || value == "1")
    {
        return Err(ctx_error(
            session.first_bar,
            abbr,
            "traj_strict=1 refuses multi-attack track lifetime until rearticulation excitation work is carried by the energy ledger",
        ));
    }

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
                "string.breath cannot rise from zero in track-lifetime v2 because its content-keyed noise channels must be preallocated at body construction",
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

    // This is the final authoring-level refusal gate and still occurs before a
    // StringNetworkVoice exists. It derives the exact constructor excitation
    // span, validates monotone non-overlapping attacks, and fixes the attack
    // topology against the pitch-scaled course set.
    let drive_samples = usize::try_from(program.duration_samples).map_err(|_| {
        ctx_error(
            session.first_bar,
            abbr,
            "track lifetime duration does not fit this platform's sample index",
        )
    })?;
    let attack_schedule = AttackSchedule::compile(
        &static_patch,
        &lowered.frequencies_hz,
        drive_samples,
        &attacks,
    )
    .map_err(|error| {
        ctx_error(
            session.first_bar,
            abbr,
            format!("track rearticulation: {error}"),
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
    let scheduled = RearticulatedStringVoice::new(body, attack_schedule).map_err(|error| {
        ctx_error(
            session.first_bar,
            abbr,
            format!("track rearticulation invariant: {error}"),
        )
    })?;
    debug_assert_eq!(scheduled.receipt().attack_count, attacks.len());
    let audio = scheduled
        .render_controlled(&lowered.control_frames)
        .map_err(|error| {
            ctx_error(
                session.first_bar,
                abbr,
                format!("track rearticulation execution: {error}"),
            )
        })?;

    let mut receipt = lowered.receipt;
    receipt.trajectory.claim_level = TRACK_LIFETIME_CLAIM.to_string();
    receipt.trajectory.evidence_class = TRACK_LIFETIME_EVIDENCE.to_string();
    if attacks.len() > 1 {
        receipt
            .unresolved_obligations
            .push(REARTICULATION_WORK_OBLIGATION.to_string());
        receipt.unresolved_obligations.sort_unstable();
        receipt.unresolved_obligations.dedup();
    }

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

    let notated_events = u32::try_from(notated_event_count).map_err(|_| {
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
