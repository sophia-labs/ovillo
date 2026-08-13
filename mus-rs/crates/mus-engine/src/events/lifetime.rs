//! Score-spanning physical-body execution for `body_lifetime=track`.
//!
//! V3 gives one fixed-topology `StringNetworkVoice` to a track, compiles every
//! pitched onset into a prevalidated physical `AttackFrame`, maps MUS dynamics
//! and gain articulations into attack amplitude, and receipts exact source work
//! against the body's live wave state. Delay lines, allpass memories, damping
//! state, modal body, Weave waves, orbit phase, contact, stochastic source
//! state, and continuous controls remain alive across notes, rests, and bars.
//!
//! Later params, glissandi, duration/contact articulations, pitch-set changes,
//! and score hairpins still refuse: V3 owns attack time and gain, not yet a full
//! per-attack articulation or state-transport payload.
//!
//! The ordinary executor is performance-specialized: one construction and tail,
//! monotone attack/control cursors, preallocated excitation/work storage,
//! hoisted phase/normalization invariants, and explicit observer-free network
//! and per-course kernels.

mod plan;
mod render;
#[cfg(test)]
mod tests;

pub(super) use render::walk_track;

use super::WalkContext;
