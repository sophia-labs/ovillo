//! Score-spanning physical-body execution for `body_lifetime=track`.
//!
//! V2 gives one fixed-topology `StringNetworkVoice` to a track and compiles
//! every repeated note/chord onset into a prevalidated physical
//! `mus_dsp::pluck::AttackFrame` schedule. The attacks reuse the constructor's
//! content-keyed excitation template while delay lines, allpass memories,
//! damping state, modal body, Weave waves, orbit phase, and control state remain
//! alive across notes, rests, and bar boundaries.
//!
//! Later event-local parameter payloads, pitch-set changes, post-first dynamics,
//! and score hairpins still refuse: V2 owns attack time and unity gain, not yet
//! per-attack articulation, retune transport, or dynamic mapping.
//!
//! The executor is also a production performance specialization: one body
//! construction replaces N event constructions and tails; attack lookup is a
//! monotone cursor over preallocated templates; invariant phase offsets and
//! normalization divisors are hoisted; and the network-level ordinary kernel
//! omits trace, gesture, backbone, external-drive, edge-telemetry, and body
//! observer bookkeeping.

mod plan;
mod render;
#[cfg(test)]
mod tests;

pub(super) use render::walk_track;

use super::WalkContext;
