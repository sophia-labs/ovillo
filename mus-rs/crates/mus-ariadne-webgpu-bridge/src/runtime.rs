use mus_ariadne::WorldPlan;
use mus_ariadne_webgpu::{
    ObserverImpulse2D, OpticalClock, OpticalClockError, SurfaceStepReceipt,
    WaveSheetParams, WaveSurface2D, WaveSurfaceConfig,
};
use mus_observation::{ObservationFrame, ObservationSchema, ObservationValue};

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
    pub fn state_view_at(&self, index: usize) -> Option<&wgpu::TextureView> {
        self.surface.state_view_at(index)
    }

    #[must_use]
    pub const fn state_read_index(&self) -> usize {
        self.surface.state_read_index()
    }

    #[must_use]
    pub fn state_texture(&self) -> &wgpu::Texture {
        self.surface.state_texture()
    }

    #[must_use]
    pub const fn completed_steps(&self) -> u64 {
        self.clock.completed_steps()
    }

    #[must_use]
    pub const fn pending_steps(&self) -> u64 {
        self.clock.pending_steps()
    }

    #[must_use]
    pub const fn is_caught_up(&self) -> bool {
        self.clock.is_caught_up()
    }

    #[must_use]
    pub const fn reset_pending(&self) -> bool {
        self.reset_pending
    }

    /// Encode a pending observer reset even when no constitutive step is due.
    /// Returns whether a reset pass was written.
    pub fn encode_pending_reset(&mut self, encoder: &mut wgpu::CommandEncoder) -> bool {
        if !std::mem::take(&mut self.reset_pending) {
            return false;
        }
        self.surface.encode_reset(encoder);
        true
    }

    #[must_use]
    pub fn next_step_tick(&self) -> Option<u64> {
        self.clock.next_step_tick()
    }

    /// Schedule bounded fixed-rate observer work toward an absolute material
    /// tick. Repeating a target is how historical replay progresses in batches.
    pub fn schedule_to_tick(&mut self, material_tick: u64) -> Result<u32, OpticalRuntimeError> {
        Ok(self.clock.schedule_to_tick(material_tick)?)
    }

    /// Encode exactly one fixed optical step from one typed observation frame.
    pub fn encode_step(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &ObservationFrame,
    ) -> Result<OpticalAdvanceReceipt, OpticalRuntimeError> {
        self.encode_values_step(queue, encoder, frame.tick, &frame.values)
    }

    /// Encode exactly one fixed optical step from caller-owned observation
    /// storage. No allocation occurs on this path.
    pub fn encode_values_step(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        material_tick: u64,
        values: &[ObservationValue],
    ) -> Result<OpticalAdvanceReceipt, OpticalRuntimeError> {
        let expected = self
            .clock
            .next_step_tick()
            .ok_or(OpticalRuntimeError::NoScheduledStep)?;
        if material_tick != expected {
            return Err(OpticalRuntimeError::UnexpectedObservationTick {
                expected,
                actual: material_tick,
            });
        }
        if self
            .last_encoded_tick
            .is_some_and(|previous| material_tick <= previous)
        {
            return Err(OpticalRuntimeError::NonMonotoneObservation {
                previous: self.last_encoded_tick.unwrap_or_default(),
                next: material_tick,
            });
        }
        let packet_count = self.compiler.compile_values_into(values, &mut self.packets)?;
        let reset = self.encode_pending_reset(encoder);
        let receipt: SurfaceStepReceipt = self.surface.encode(
            queue,
            encoder,
            &self.packets[..packet_count],
            self.parameters,
        )?;
        let optical_step = self.clock.complete_next_step(material_tick)?;
        self.last_encoded_tick = Some(material_tick);
        Ok(OpticalAdvanceReceipt {
            material_tick,
            optical_step,
            impulse_count: receipt.impulse_count,
            upload_bytes: receipt.upload_bytes,
            dispatch_workgroups: receipt.dispatch_workgroups,
            reset,
        })
    }

    /// Reset observer history and scheduling. The physical World is untouched.
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
    #[error("no optical step has been scheduled")]
    NoScheduledStep,
    #[error("optical step requires observation tick {expected}, received {actual}")]
    UnexpectedObservationTick { expected: u64, actual: u64 },
    #[error("optical observation tick is not monotone: {previous} then {next}")]
    NonMonotoneObservation { previous: u64, next: u64 },
}
