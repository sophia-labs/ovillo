//! Preflight planning for one track-owned physical body.

use mus_notation::{
    expand_pattern_repetitions, parse_token, split_attached_dynamics, tokenize_bar_content, Note,
};

use crate::layout::swung;
use crate::pack::{Voice, SYNTH_KEYS};
use crate::util::{with_context, ART_GAIN, ART_SHORTEN};
use crate::{RenderError, RenderStats};

use super::WalkContext;

#[derive(Debug)]
pub(super) struct TrackBodySession {
    pub(super) first_bar: u32,
    pub(super) onset_seconds: f64,
    pub(super) end_seconds: f64,
    pub(super) duration_seconds: f64,
    pub(super) member_event_count: usize,
    pub(super) note_onsets: Vec<f64>,
    pub(super) dynamic: String,
    pub(super) first_event: Note,
}

pub(super) fn ctx_error(bar: u32, abbr: &str, message: impl Into<String>) -> RenderError {
    with_context(
        &format!("b{bar} {abbr}"),
        RenderError::Invalid(message.into()),
    )
}

pub(super) fn ctx_existing(bar: u32, abbr: &str, error: RenderError) -> RenderError {
    with_context(&format!("b{bar} {abbr}"), error)
}

const SESSION_POST_SOURCE_PARAMS: &[&str] = &[
    "chop", "ring", "rwet", "glow", "ghold", "gwarble", "gharm", "pump", "str", "atk", "rel",
    "lpf", "hpf", "drive", "dist", "crush", "decim", "stut", "gain", "pan", "haas", "send",
];

fn is_trajectory_coordinate(key: &str) -> bool {
    matches!(
        key,
        "couple"
            | "chirality"
            | "orbit"
            | "orbit_depth"
            | "curvature"
            | "body"
            | "buzz"
            | "sing"
            | "bow"
            | "bow_speed"
            | "sus"
            | "damp"
            | "breath"
            | "pitch_ratio"
    )
}

fn is_trajectory_surface_key(key: &str) -> bool {
    matches!(key, "traj_curve" | "traj_hz" | "traj_strict")
        || is_trajectory_coordinate(key)
        || key
            .strip_suffix("_curve")
            .is_some_and(is_trajectory_coordinate)
}

fn validate_first_event_surface(event: &Note, bar: u32, abbr: &str) -> Result<(), RenderError> {
    for key in event.params.keys() {
        if !(SYNTH_KEYS.contains(&key.as_str())
            || is_trajectory_surface_key(key)
            || SESSION_POST_SOURCE_PARAMS.contains(&key.as_str()))
        {
            return Err(ctx_error(
                bar,
                abbr,
                format!(
                    "body_lifetime=track first-event param {key:?} belongs to source resolution or an unsupported event-local stage; move persistent controls to the trajectory/instrument surface"
                ),
            ));
        }
    }
    for flag in &event.flags {
        let transform_flag = ART_SHORTEN
            .iter()
            .any(|(name, _)| flag.as_str() == *name)
            || ART_GAIN
                .iter()
                .any(|(name, _)| flag.as_str() == *name)
            || matches!(flag.as_str(), "gate" | "fer");
        if !transform_flag {
            return Err(ctx_error(
                bar,
                abbr,
                format!(
                    "body_lifetime=track first-event flag {flag:?} is not a post-source session transform; it would otherwise be silently ignored"
                ),
            ));
        }
    }
    Ok(())
}

fn ensure_continuation_is_plain(
    event: &Note,
    bar: u32,
    abbr: &str,
) -> Result<(), RenderError> {
    if !event.params.is_empty() || !event.flags.is_empty() || event.gliss_midi.is_some() {
        return Err(ctx_error(
            bar,
            abbr,
            "body_lifetime=track v2 continuations schedule a fixed-topology unity-gain attack only; per-attack params, flags, and gliss require the next rearticulation payload contract",
        ));
    }
    Ok(())
}

/// Build one strict session plan while retaining the ordinary lexical timing
/// rules. The planner performs no DSP allocation and rejects every ambiguity
/// before a body exists.
pub(super) fn plan_session(
    ctx: &WalkContext,
    voice: &Voice,
    stats: &mut RenderStats,
) -> Result<Option<TrackBodySession>, RenderError> {
    let abbr = voice.abbrev.as_str();
    let mut current_dynamic = "mf".to_string();
    let mut first: Option<(u32, f64, Note, String)> = None;
    let mut fixed_midis: Option<Vec<f64>> = None;
    let mut member_event_count = 0_usize;
    let mut note_onsets = Vec::new();

    for bar in 1..=ctx.n_bars {
        let Some(content) = ctx
            .bar_content
            .get(&bar)
            .and_then(|tracks| tracks.get(abbr))
        else {
            continue;
        };
        if content.trim().is_empty() || content.trim() == "-" {
            continue;
        }
        let tokens =
            split_attached_dynamics(&expand_pattern_repetitions(&tokenize_bar_content(content)));
        let mut quarter_offset = 0.0_f64;

        for token in &tokens {
            let Some(event) = parse_token(token) else {
                let trimmed = token.trim();
                if trimmed != "-" && !trimmed.is_empty() {
                    stats.bad.push(format!("b{bar} {abbr}: {token}"));
                }
                continue;
            };
            if event.ql <= 0.0
                && token.trim() != "R"
                && matches!(
                    event.kind.as_str(),
                    "note" | "chord" | "rest" | "unpitched"
                )
            {
                stats
                    .bad
                    .push(format!("b{bar} {abbr}: {token} (zero duration)"));
            }

            match event.kind.as_str() {
                "dynamic" => {
                    if first.is_some() {
                        return Err(ctx_error(
                            bar,
                            abbr,
                            "body_lifetime=track v2 does not yet map later score dynamics onto AttackFrame gain; author a physical trajectory or split the body lifetime",
                        ));
                    }
                    if let Some(value) = &event.dyn_value {
                        current_dynamic = value.clone();
                    }
                    continue;
                }
                "hairpin" => {
                    return Err(ctx_error(
                        bar,
                        abbr,
                        "body_lifetime=track v2 does not yet lower a score hairpin into attack gain or a physical control trajectory",
                    ));
                }
                _ => {}
            }

            let bar_timing = *ctx
                .bar_map
                .bars
                .get(&bar)
                .ok_or_else(|| RenderError::Invalid(format!("event uses missing bar {bar}")))?;
            let onset = bar_timing.start_s
                + swung(quarter_offset, ctx.swing_unit_ql, ctx.swing_r) * bar_timing.spq;
            quarter_offset += event.ql;

            match event.kind.as_str() {
                "rest" => {
                    if first.is_some() {
                        ensure_continuation_is_plain(&event, bar, abbr)?;
                        member_event_count += 1;
                    }
                }
                "unpitched" => {
                    return Err(ctx_error(
                        bar,
                        abbr,
                        "body_lifetime=track currently requires a pitched note or chord because the persistent string topology is frequency-addressed",
                    ));
                }
                "note" | "chord" => {
                    if event.midis.is_empty() {
                        return Err(ctx_error(
                            bar,
                            abbr,
                            "body_lifetime=track encountered an empty pitched topology",
                        ));
                    }
                    match &fixed_midis {
                        None => {
                            fixed_midis = Some(event.midis.clone());
                            note_onsets.push(onset);
                            first = Some((bar, onset, event, current_dynamic.clone()));
                            member_event_count += 1;
                        }
                        Some(expected) => {
                            ensure_continuation_is_plain(&event, bar, abbr)?;
                            if event.midis.as_slice() != expected.as_slice() {
                                return Err(ctx_error(
                                    bar,
                                    abbr,
                                    format!(
                                        "body_lifetime=track requires fixed course topology; expected MIDI pitches {expected:?}, found {:?}",
                                        event.midis
                                    ),
                                ));
                            }
                            note_onsets.push(onset);
                            member_event_count += 1;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let Some((first_bar, onset_seconds, mut first_event, dynamic)) = first else {
        return Ok(None);
    };
    let last_bar = ctx
        .bar_map
        .bars
        .get(&ctx.n_bars)
        .ok_or_else(|| RenderError::Invalid("track lifetime has no final bar".to_string()))?;
    let end_seconds = last_bar.start_s + last_bar.qlen * last_bar.spq;
    let duration_seconds = end_seconds - onset_seconds;
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err(ctx_error(
            first_bar,
            abbr,
            format!(
                "track lifetime [{onset_seconds}, {end_seconds}] has no positive finite duration"
            ),
        ));
    }

    if first_event.params.contains_key("body_lifetime") {
        return Err(ctx_error(
            first_bar,
            abbr,
            "body_lifetime is instrument-owned and may not be overridden by the first event",
        ));
    }
    let mut merged = voice.defaults.clone();
    merged.extend(std::mem::take(&mut first_event.params));
    merged.remove("body_lifetime");
    first_event.params = merged;
    validate_first_event_surface(&first_event, first_bar, abbr)?;

    Ok(Some(TrackBodySession {
        first_bar,
        onset_seconds,
        end_seconds,
        duration_seconds,
        member_event_count,
        note_onsets,
        dynamic,
        first_event,
    }))
}
