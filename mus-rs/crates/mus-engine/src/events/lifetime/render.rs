//! Allocation-conscious execution of one prevalidated track-body session.

use std::collections::BTreeMap;

use mus_dsp::pluck::{
    AttackFrame, AttackSchedule, RearticulatedStringVoice, StringNetworkVoice,
    WorkReceiptedStringVoice,
};
use mus_dsp::trajectory::{lower_string_trajectory, program_from_inline_string_params};

use crate::mixbus::{db_to_gain, pan_and_haas, truncate_to_render, MixBuffers, PlacedNote};
use crate::pack::Voice;
use crate::util::{dyn_db, midi_to_hz, param_pair, require_f64, require_f64_opt};
use crate::{EventTrajectoryReceipt, RenderError, RenderStats};

use super::plan::{ctx_error, ctx_existing, plan_session, TrackBodySession};
use super::WalkContext;

pub(super) const TRACK_LIFETIME_CLAIM: &str =
    "score-spanning-work-receipted-rearticulation";
pub(super) const TRACK_LIFETIME_EVIDENCE: &str =
    "validated-numerics+engine-execution+single-construction+physical-gain-lowering+exact-attack-work+fully-unobserved-course-kernel";

struct LoweredAttacks {
    frames: Vec<AttackFrame>,
    dynamics: Vec<String>,
    articulation_db: Vec<f64>,
    gain_db: Vec<f64>,
}

impl LoweredAttacks {
    fn gains(&self) -> impl Iterator<Item = f64> + '_ {
        self.frames.iter().map(|frame| frame.gain)
    }
}

fn join_values<T: ToString>(values: impl IntoIterator<Item = T>) -> String {
    values
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn lower_attacks(
    session: &TrackBodySession,
    abbr: &str,
) -> Result<LoweredAttacks, RenderError> {
    let mut frames = Vec::with_capacity(session.attacks.len());
    let mut dynamics = Vec::with_capacity(session.attacks.len());
    let mut articulation_db = Vec::with_capacity(session.attacks.len());
    let mut gain_db = Vec::with_capacity(session.attacks.len());

    for attack in &session.attacks {
        let relative_seconds = attack.onset_seconds - session.onset_seconds;
        if !relative_seconds.is_finite() || relative_seconds < 0.0 {
            return Err(ctx_error(
                session.first_bar,
                abbr,
                format!(
                    "rearticulation onset {} is before/non-finite relative to session start {}",
                    attack.onset_seconds, session.onset_seconds
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

        let physical_db = dyn_db(&attack.dynamic) + attack.articulation_db;
        let gain = db_to_gain(physical_db);
        if !gain.is_finite() || gain <= 0.0 {
            return Err(ctx_error(
                session.first_bar,
                abbr,
                format!(
                    "dynamic {:?} plus articulation {} dB produced invalid physical attack gain {gain}",
                    attack.dynamic, attack.articulation_db
                ),
            ));
        }
        frames.push(AttackFrame {
            at_sample: sample.round() as usize,
            gain,
        });
        dynamics.push(attack.dynamic.clone());
        articulation_db.push(attack.articulation_db);
        gain_db.push(physical_db);
    }

    Ok(LoweredAttacks {
        frames,
        dynamics,
        articulation_db,
        gain_db,
    })
}

pub(crate) fn walk_track(
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

    let attacks = lower_attacks(&session, abbr)?;
    let notated_event_count = session.attacks.len();
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
        attacks.frames.len().to_string(),
    );
    program.metadata.insert(
        "attack_samples".to_string(),
        join_values(attacks.frames.iter().map(|attack| attack.at_sample)),
    );
    program.metadata.insert(
        "attack_dynamics".to_string(),
        attacks.dynamics.join(","),
    );
    program.metadata.insert(
        "attack_articulation_db".to_string(),
        join_values(attacks.articulation_db.iter().copied()),
    );
    program.metadata.insert(
        "attack_gain_db".to_string(),
        join_values(attacks.gain_db.iter().copied()),
    );
    program.metadata.insert(
        "attack_gains".to_string(),
        join_values(attacks.gains()),
    );
    program.metadata.insert(
        "attack_gain_law".to_string(),
        "10^((dynamic_db+gain_articulation_db)/20)".to_string(),
    );
    program.metadata.insert(
        "attack_excitation_identity".to_string(),
        "constructor-content-keyed-template".to_string(),
    );
    program
        .metadata
        .insert("attack_overlap_policy".to_string(), "refuse-v1".to_string());
    program
        .metadata
        .insert("body_construction_count".to_string(), "1".to_string());
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
        "prevalidated-monotone-attack-cursor+fully-unobserved-course-kernel-v3".to_string(),
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
                "string.breath cannot rise from zero in track lifetime because its content-keyed noise channels must be preallocated at body construction",
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

    // This remains the final authoring-level refusal gate and still occurs
    // before a StringNetworkVoice exists.
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
        &attacks.frames,
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
    debug_assert_eq!(scheduled.receipt().attack_count, attacks.frames.len());
    let receipted = WorkReceiptedStringVoice::new(scheduled).map_err(|error| {
        ctx_error(
            session.first_bar,
            abbr,
            format!("track attack-work allocation: {error}"),
        )
    })?;
    let rendered = receipted
        .render_controlled_with_receipt(&lowered.control_frames)
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
    receipt.rearticulation = Some(rendered.receipt);

    let audio = crate::transforms::apply_chain(
        rendered.audio,
        &session.first_event,
        session.duration_seconds,
    )
    .map_err(|error| ctx_existing(session.first_bar, abbr, error))?;

    // Dynamics and gain articulations now own physical excitation. Voice trim
    // and explicit `gain=` remain session-placement controls.
    let db = voice.gain_db
        + require_f64(&session.first_event.params, "gain", 0.0)
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
        sc_onsets.extend(session.attacks.iter().map(|attack| attack.onset_seconds));
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
