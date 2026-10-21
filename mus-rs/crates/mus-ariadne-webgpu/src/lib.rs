//! WebGPU constitutive visual-compute primitives for persistent Ariadne worlds.
//!
//! This crate owns persistent GPU resources and non-loading optical, sensor,
//! and presentation laws. It owns no Ariadne physical World, listener, window,
//! or page-specific interpretation. Consumers provide a `wgpu::Device`,
//! `Queue`, and command encoder, then bind typed observations through a separate
//! optical-constitution plan.

pub mod capability;
pub mod clock;
pub mod law;
pub mod presentation;
pub mod primitive;
pub mod reference;
pub mod resource;
pub mod surface;
pub mod telemetry;
pub mod types;
mod zero;

pub use capability::CapabilityProfile;
pub use clock::{OpticalClock, OpticalClockError};
pub use law::{Cadence, Domain, LawClass, PrimitiveDescriptor, SpatialSupport, ZeroLaw};
pub use presentation::{PresentationParams, WorldPresenter};
pub use primitive::{
    GpuPrimitive, ImpulseRasterConfig, ObserverImpulseRaster2D, PrimitiveError, WaveSheet2D,
    WaveSheetConfig,
};
pub use surface::{SurfaceStepReceipt, WaveSurface2D, WaveSurfaceConfig};
pub use telemetry::{PerformanceReceipt, QualityGovernor, QualityLevel};
pub use types::{ObserverImpulse2D, OpticalFrameHeader, WaveSheetParams};

#[cfg(feature = "profiling")]
pub use wgpu_profiler;
