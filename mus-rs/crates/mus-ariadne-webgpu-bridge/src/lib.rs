//! Typed bridge from one physical Ariadne World to non-loading WebGPU fields.
//!
//! The bridge owns semantic binding only. It compiles a content-addressed
//! observation schema and optical constitution into fixed body, field, and
//! coupling packet slots. It contains no physical law and no page-specific
//! game or instrument vocabulary.

#![forbid(unsafe_code)]

mod compiler;
mod plan;

pub use compiler::{OpticalCompileError, OpticalPacketCompiler};
pub use plan::{
    canonical_optical_observation_program, BodyOpticalSpec,
    CouplingOpticalSpec, OpticalBounds2, OpticalConstitutionPlan,
    OpticalPlanError, OpticalSurfaceSpec, SharedFieldOpticalSpec,
    OPTICAL_CONSTITUTION_VERSION,
};
