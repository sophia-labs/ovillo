//! Non-loading programs over one physical Ariadne World.
//!
//! This module contains no persistent physical state. Programs bind body,
//! shared-field, and coupling names to numeric indices at construction time and
//! can then observe without allocation or back-action on the material clock.

mod listener;
mod program;
mod runtime;

pub use listener::{
    ListenerError, StereoBodyTap, StereoFrame, StereoListener, StereoListenerSpec,
    STEREO_LISTENER_VERSION,
};
pub use program::{
    ObservationProgramError, ObservationSource, WorldObservationChannel, WorldObservationProgram,
    WORLD_OBSERVATION_PROGRAM_VERSION,
};
pub use runtime::{BoundObservationProgram, ObservationRuntimeError};
