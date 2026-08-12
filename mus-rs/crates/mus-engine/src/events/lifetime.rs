//! Score-spanning physical-body execution for `body_lifetime=track`.
//!
//! V1 is intentionally narrower than a general note scheduler. One track owns
//! one fixed-topology `StringNetworkVoice`, receives one attack at the first
//! note/chord, and then traverses one trajectory timebase across every later
//! score event and rest until the end of the notated score. Repeated pitches
//! are lifetime boundaries, not hidden re-attacks. Pitch-set changes, later
//! event overrides, dynamic changes after the first attack, and every score
//! hairpin refuse until the persistent voice exposes an explicit allocation-free
//! rearticulation schedule.
//!
//! The executor is also the first production performance specialization for
//! MUS-T: one body construction replaces N event constructions and N event
//! tails, and canonical control frames are consumed by a monotone cursor
//! without allocating `WeaveTrace` telemetry on the ordinary render path.

mod plan;
mod render;
#[cfg(test)]
mod tests;

pub(super) use render::walk_track;

use super::WalkContext;
