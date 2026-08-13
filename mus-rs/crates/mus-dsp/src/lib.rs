//! Sample-faithful DSP for the historical MUS render face.
//!
//! Canonical Ariadne anatomy, laws, persistent bodies, spaces, trajectories,
//! execution, and receipts now live in `mus-ariadne`. This crate retains the
//! older renderer kernels and one temporary import shim for saved plug-in and
//! test code; it must not grow a second Ariadne implementation.

/// Temporary source-compatibility seam for the currently shipped terminal.
/// The implementation is owned once, under `mus_ariadne::compat`.
#[cfg(feature = "ariadne-live")]
#[deprecated(note = "import mus_ariadne::compat::terminal directly")]
pub mod ariadne_terminal {
    pub use mus_ariadne::compat::terminal::*;
}

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
/// Transitional noncanonical processor retained as a scientific control.
pub mod powered_radiator;

/// Historical physically informed guitar and Weave networks. New Ariadne
/// bodies are declarations lowered into `mus-ariadne::World`, not additions to
/// this module.
#[cfg(feature = "full")]
pub mod pluck {
    include!(concat!(env!("OUT_DIR"), "/pluck_generated.rs"));
}

#[cfg(feature = "full")]
pub mod scene;
#[cfg(feature = "full")]
pub mod synth;
#[cfg(feature = "full")]
pub mod trajectory {
    include!(concat!(env!("OUT_DIR"), "/trajectory_generated.rs"));
}

pub const SR: usize = 48_000;
pub const SR_F: f32 = 48_000.0;
pub const SR_F64: f64 = 48_000.0;
