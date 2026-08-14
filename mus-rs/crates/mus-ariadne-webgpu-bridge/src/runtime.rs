use mus_ariadne::WorldPlan;
use mus_ariadne_webgpu::{
    ObserverImpulse2D, OpticalClock, OpticalClockError, SurfaceStepReceipt,
    WaveSheetParams, WaveSurface2D, WaveSurfaceConfig,
};
use mus_observation::{ObservationFrame, ObservationSchema};

use crate::compiler::{OpticalCompileError, OpticalPacketCompiler};
use crate::plan::{OpticalConstitutionPlan, OpticalPlanError};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OpticalAdvanceReceipt {
    pub material_tick: u64,
    pub optical_step: u64,
    pub impulse_count: u32,
    pub upload_bytes: u64,
    pub dispatch_workgroups: [u32; 3],
    pub reset: bool,
}

/// Persistent observer-owned GPU world bound to one physical World digest,
/// observation schema, and optical constitution.
pub struct WorldOpticalSurface {
    compiler: OpticalPacketCompiler,
    clock: OpticalClock,
    surface: WaveSurface2D,
    packets: Vec<ObserverImpulse2D>,
    parameters: WaveSheetParams,
    reset_pending: bool,
    last_encoded_tick: Option<u64>,
}

impl WorldOpticalSurface {
    pub fn new(
        device: &wgpu::Device,
        optical: &OpticalConstitutionPlan,
        world: &WorldPlan,
        schema: &ObservationSchema,
    ) -> Result<Self, OpticalRuntimeError> {
        optical.validate(world, schema)?;
        let compiler = OpticalPacketCompiler::compile(optical, world, schema)?;
        let clock = OpticalClock::new(
            schema.sample_rate_hz,
            optical.surface.fixed_rate_hz,
            optical.surface.maximum_catch_up_steps,
        )?;
        let surface = WaveSurface2D::new(
            device,
            WaveSurfaceConfig {
                width: optical.surface.width,
                height: optical.surface.height,
                max_impulses: optical.surface.maximum_impulses,
            },
        )?;
        let parameters = WaveSheetParams::portable(
            optical.surface.width,
            optical.surface.height,
            clock.fixed_dt_seconds(),
        );
        Ok(Self {
            packets: vec![ObserverImpulse2D::zeroed(); compiler.packet_count()],
            compiler,
            clock,
            surface,
            parameters,
            reset_pending: true,
            last_encoded_tick: None,
        })
    }

    #[must_use]
    pub fn physical_world_digest(&self) -> &str {
        self.compiler.physical_world_digest()
    }

    #[must_use]
    pub fn observation_schema_digest(&self) -> &str {
        self.compiler.observation_schema_digest()
    }

    #[must_use]
    pub fn optical_constitution_digest(&self) -> &str {
        self.compiler.optical_constitution_digest()
    }

    #[must_use]
    pub fn state_view(&self) -> &wgpu::TextureView {
        self.surface.state_view()
    }

    #[must_use]
    pub fn state_texture(&self) -> &wgpu::Texture {
        self.surface.state_texture()
    }

    /// Return how many fixed optical steps are due at an absolute material
    /// tick. The caller must then supply exactly that many schema-valid
    /// observations at the corresponding step ticks; stale observations are
    /// never duplicated implicitly.
    pub fn steps_due(&mut self, material_tick: u64) -> Result<u32, OpticalRuntimeError> {
        Ok(self.clock.advance_to_tick(material_tick)?)
    }

    #[must_use]
    pub fn material_tick_for_step(&self, step: u64) -> u64 {
        self.clock.material_tick_for_step(step)
    }

    /// Encode exactly one fixed optical step from one typed observation.
    pub fn encode_step(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &ObservationFrame,
    ) -> Result<OpticalAdvanceReceipt, OpticalRuntimeError> {
        if self
            .last_encoded_tick
            .is_some_and(|previous| frame.tick <= previous)
        {
            return Err(OpticalRuntimeError::NonMonotoneObservation {
                previous: self.last_encoded_tick.unwrap_or_default(),
                next: frame.tick,
            });
        }
        let packet_count = self.compiler.compile_into(frame, &mut self.packets)?;
        let reset = std::mem::take(&mut self.reset_pending);
        let receipt: SurfaceStepReceipt = self.surface.encode(
            queue,
            encoder,
            &self.packets[..packet_count],
            self.parameters.with_reset(reset),
        )?;
        self.last_encoded_tick = Some(frame.tick);
        Ok(OpticalAdvanceReceipt {
            material_tick: frame.tick,
            optical_step: self.clock.completed_steps(),
            impulse_count: receipt.impulse_count,
            upload_bytes: receipt.upload_bytes,
            dispatch_workgroups: receipt.dispatch_workgroups,
            reset,
        })
    }

    /// Reset observer and sensor history. The physical World is untouched.
    pub fn reset(&mut self) {
        self.clock.reset();
        self.reset_pending = true;
        self.last_encoded_tick = None;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OpticalRuntimeError {
    #[error(transparent)]
    Plan(#[from] OpticalPlanError),
    #[error(transparent)]
    Compile(#[from] OpticalCompileError),
    #[error(transparent)]
    Clock(#[from] OpticalClockError),
    #[error(transparent)]
    Primitive(#[from] mus_ariadne_webgpu::PrimitiveError),
    #[error("optical observation tick is not monotone: {previous} then {next}")]
    NonMonotoneObservation { previous: u64, next: u64 },
}
