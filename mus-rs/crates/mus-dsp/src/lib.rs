//! Sample-faithful DSP for the MUS render face.
//!
//! Every historical public function here is a port of the equivalently named
//! function in `mus_audio.py`, and the oracle is not prose: `mus-rs/tools/
//! gen_dsp_fixtures.py` dumps golden input/output vectors from the Python
//! implementation into `mus-rs/fixtures/dsp/`, and every port carries a test
//! that replays them. A passing suite means "same math", not "similar idea".
//!
//! The live Ariadne modules are native contracts rather than Python ports.
//! Their oracles are stored-state evolution, work/loss/radiation closure,
//! ordered-transport identities, exact zero limits, and host invariance.

/// Pure powered metric material and reciprocal spatial halo for live input.
#[cfg(feature = "ariadne-live")]
pub mod ariadne_terminal;
#[cfg(feature = "full")]
pub mod backbone;
#[cfg(feature = "full")]
pub mod body_profiles;
#[cfg(feature = "full")]
pub mod bus;
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
/// Transitional live processor retained as a scientific and listening control.
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
