//! Sample-faithful DSP for the MUS render face.
//!
//! Every public function here is a port of the equivalently named function
//! in `mus_audio.py`, and the oracle is not prose: `mus-rs/tools/
//! gen_dsp_fixtures.py` dumps golden input/output vectors from the Python
//! implementation into `mus-rs/fixtures/dsp/`, and every port carries a
//! test that replays them through [`fixtures`]. A passing suite means
//! "same math", not "similar idea".
//!
//! Parity tiers (the manifest records the metric and tolerance per case):
//!
//! - **Pure-numpy ports** (`kernels`, `synth`, most of `filters`,
//!   `pitch::pitch_ramp`/`pitch_polyline`): `max_abs` at 1e-6..5e-5 —
//!   effectively exact in f32.
//! - **Same-C-library paths** (`pitch::varispeed` via libsoxr, the same
//!   library librosa's `soxr_hq` binds): `rel_rms` at 5e-4.
//! - **Reimplemented-algorithm paths** (`pitch::vocode`/`stretch`, a port
//!   of librosa's STFT phase vocoder over a different FFT): `rel_rms` at
//!   5e-3, with exact output lengths.
//!
//! Where the port cannot meet its tier, the correct move is to say so —
//! record the measured divergence and surface it, never to widen a
//! tolerance silently. Refusal is a result.
//!
//! The live Ariadne modules (`power_port`, `electrodynamic_radiator`, and
//! `powered_radiator`) are native physical contracts rather than Python ports.
//! Their oracles are power preservation, passivity, reciprocity, and receipt
//! closure.

#[cfg(feature = "full")]
pub mod backbone;
#[cfg(feature = "full")]
pub mod body_profiles;
#[cfg(feature = "full")]
pub mod bus;
/// Reciprocal electrical/mechanical/acoustic radiator for live physical graphs.
pub mod electrodynamic_radiator;
#[cfg(feature = "full")]
pub mod filters;
#[cfg(feature = "full")]
pub mod fixtures;
#[cfg(feature = "full")]
pub mod glow;
#[cfg(feature = "full")]
pub mod kernels;
#[cfg(feature = "full")]
pub mod pitch;
/// Domain-explicit effort/flow ports and power-preserving junctions.
pub mod power_port;
/// Sample-rate-native, allocation-free powered modal body for live audio input.
pub mod powered_radiator;

/// Physically informed guitar and Weave networks, plus the fixed-topology
/// rearticulation and attack-work extensions. The parity-pinned implementation
/// remains in `src/pluck.rs`; `build.rs` appends the child extensions without
/// widening its private state surface.
#[cfg(feature = "full")]
pub mod pluck {
    include!(concat!(env!("OUT_DIR"), "/pluck_generated.rs"));
}

#[cfg(feature = "full")]
pub mod scene;
#[cfg(feature = "full")]
pub mod synth;
/// Direct authored string trajectories. The committed adapter remains in
/// `src/trajectory.rs`; `build.rs` adds the optional rearticulation receipt at
/// the body-runtime boundary without creating a dependency cycle.
#[cfg(feature = "full")]
pub mod trajectory {
    include!(concat!(env!("OUT_DIR"), "/trajectory_generated.rs"));
}

/// Engine sample rate. `mus_audio.py` line 47: `SR = 48_000`.
pub const SR: usize = 48_000;
/// `SR` as f32, for rate math in kernel code.
pub const SR_F: f32 = 48_000.0;
/// `SR` as f64, for phase/frequency math done in double precision.
pub const SR_F64: f64 = 48_000.0;
