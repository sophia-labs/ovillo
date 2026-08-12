//! Track rendering with an explicit lifetime policy for physical bodies.
//!
//! The parity-locked event renderer remains byte-for-byte source-identical in
//! [`original`]. This wrapper selects it by structural absence (the default
//! `body_lifetime=event`) and routes the opt-in `body_lifetime=track` surface
//! to the score-spanning single-body executor in [`lifetime`].

mod original;
mod lifetime;

pub use original::WalkContext;

use crate::mixbus::{MixBuffers, PlacedNote};
use crate::pack::Voice;
use crate::{RenderError, RenderStats};

/// Walk one track under its declared physical-body lifetime.
///
/// `event` and absence retain the historical per-event renderer exactly. The
/// first score-spanning implementation is deliberately explicit and strict:
/// `track` owns one string-network body from the first attack through the end
/// of the notated score and refuses unsupported topology or articulation
/// changes rather than reconstructing hidden event bodies. Notated pitched
/// events retain score statistics and sidechain onsets; the physical attack and
/// body-construction counts are separate trajectory-lifetime evidence.
pub fn walk_track(
    ctx: &WalkContext,
    voice: &Voice,
    buffers: &mut MixBuffers,
    placed: &mut Vec<PlacedNote>,
    sc_onsets: &mut Vec<f64>,
    stats: &mut RenderStats,
) -> Result<(), RenderError> {
    match voice
        .defaults
        .get("body_lifetime")
        .map(String::as_str)
        .unwrap_or("event")
    {
        "event" => original::walk_track(ctx, voice, buffers, placed, sc_onsets, stats),
        "track" => lifetime::walk_track(ctx, voice, buffers, placed, sc_onsets, stats),
        other => Err(RenderError::Invalid(format!(
            "instrument {:?} has body_lifetime={other:?}; expected event or track",
            voice.abbrev
        ))),
    }
}
