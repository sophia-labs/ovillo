//! WebGPU constitutive visual-compute primitives for persistent Ariadne worlds.
//!
//! The crate intentionally owns neither a window nor a presentation surface.
//! Consumers provide a `wgpu::Device`, `Queue`, and command encoder, then
//! compose persistent primitives into an Ariadne law graph.

pub mod capability;
pub mod law;
pub mod primitive;
pub mod reference;
pub mod resource;
pub mod surface;
pub mod telemetry;
pub mod types;

pub use capability::CapabilityProfile;
pub use law::{Cadence, Domain, LawClass, PrimitiveDescriptor, SpatialSupport, ZeroLaw};
pub use primitive::{
    GpuPrimitive, ImpulseRaster2D, ImpulseRasterConfig, PrimitiveError, WaveSheet2D,
    WaveSheetConfig,
};
pub use surface::{SurfaceStepReceipt, WaveSurface2D, WaveSurfaceConfig};
pub use telemetry::{PerformanceReceipt, QualityGovernor, QualityLevel};
pub use types::{FramePacket, Impulse2D, WaveSheetParams};

#[cfg(feature = "profiling")]
pub use wgpu_profiler;
