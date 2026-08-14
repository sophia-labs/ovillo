//! Canonical browser boundary for Ariadne World audiovisual run bundles.
//!
//! The physical World has already run. This crate validates its immutable
//! bundle, advances one fixed-rate non-loading optical constitution, and
//! presents the resulting observer field through WebGPU. It contains no audio
//! analysis, page-specific channel layout, physical law, or compatibility path.

#![forbid(unsafe_code)]

use mus_observation::{RunIdentity, RunIdentityError};

/// Decode and validate one typed run identity.
pub fn decode_run_identity(source: &str) -> Result<RunIdentity, RunIdentityDecodeError> {
    let identity: RunIdentity = serde_json::from_str(source)?;
    identity.validate()?;
    Ok(identity)
}

#[derive(Debug, thiserror::Error)]
pub enum RunIdentityDecodeError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Identity(#[from] RunIdentityError),
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use mus_ariadne::{StereoListenerSpec, WorldObservationProgram, WorldPlan};
    use mus_ariadne_webgpu::{CapabilityProfile, PresentationParams, WorldPresenter};
    use mus_ariadne_webgpu_bridge::{OpticalConstitutionPlan, WorldOpticalSurface};
    use mus_observation::{
        decode_trace, digest_json, ObservationSchema, ObservationTrace, ObservationValue,
    };
    use wasm_bindgen::prelude::*;
    use web_sys::HtmlCanvasElement;

    use crate::decode_run_identity;

    const STATUS_CAUGHT_UP: u32 = 1;
    const STATUS_PRESENTED: u32 = 2;
    const STATUS_RECONFIGURED: u32 = 4;

    #[wasm_bindgen(start)]
    pub fn start() {
        console_error_panic_hook::set_once();
    }

    #[wasm_bindgen]
    pub struct AriadneWorldViewer {
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface_config: wgpu::SurfaceConfiguration,
        optical: WorldOpticalSurface,
        presenter: WorldPresenter,
        trace: ObservationTrace,
        sample_values: Vec<ObservationValue>,
        requested_tick: u64,
        presentation_frame: u64,
        capability_json: String,
    }

    #[wasm_bindgen]
    impl AriadneWorldViewer {
        #[wasm_bindgen(js_name = create)]
        pub async fn create(
            canvas: HtmlCanvasElement,
            world_plan_json: String,
            listener_json: String,
            intervention_json: String,
            observation_program_json: String,
            observation_schema_json: String,
            optical_plan_json: String,
            run_identity_json: String,
            trace_bytes: Vec<u8>,
            expected_run_digest: String,
        ) -> Result<AriadneWorldViewer, JsValue> {
            console_error_panic_hook::set_once();
            let run_identity = decode_run_identity(&run_identity_json).map_err(js_error)?;
            if run_identity.digest().map_err(js_error)? != expected_run_digest {
                return Err(JsValue::from_str(
                    "run.identity.json does not match manifest runDigest",
                ));
            }

            let world: WorldPlan = serde_json::from_str(&world_plan_json).map_err(js_error)?;
            world.validate().map_err(js_error)?;
            let listener: StereoListenerSpec =
                serde_json::from_str(&listener_json).map_err(js_error)?;
            listener.validate(&world).map_err(js_error)?;
            let intervention: serde_json::Value =
                serde_json::from_str(&intervention_json).map_err(js_error)?;
            let observation_program: WorldObservationProgram =
                serde_json::from_str(&observation_program_json).map_err(js_error)?;
            observation_program.validate(&world).map_err(js_error)?;
            let schema: ObservationSchema =
                serde_json::from_str(&observation_schema_json).map_err(js_error)?;
            schema.validate().map_err(js_error)?;
            if observation_program.schema().map_err(js_error)? != schema {
                return Err(JsValue::from_str(
                    "observation.program.json does not produce observation.schema.json",
                ));
            }
            let optical_plan: OpticalConstitutionPlan =
                serde_json::from_str(&optical_plan_json).map_err(js_error)?;
            optical_plan.validate(&world, &schema).map_err(js_error)?;
            let trace = decode_trace(&trace_bytes).map_err(js_error)?;
            if trace.metadata.schema != schema {
                return Err(JsValue::from_str(
                    "trace schema does not equal observation.schema.json",
                ));
            }
            if trace.metadata.run_digest != expected_run_digest {
                return Err(JsValue::from_str(
                    "trace run digest does not equal manifest runDigest",
                ));
            }
            let world_digest = world.digest().map_err(js_error)?;
            if schema.physical_world_digest != world_digest {
                return Err(JsValue::from_str(
                    "observation schema does not bind the supplied physical World",
                ));
            }
            if run_identity.physical_world_digest != world_digest
                || run_identity.listener_digest != listener.digest().map_err(js_error)?
                || run_identity.intervention_digest
                    != digest_json(&intervention).map_err(js_error)?
                || run_identity.observation_program_digest
                    != observation_program.digest().map_err(js_error)?
                || run_identity.observation_schema_digest
                    != schema.digest().map_err(js_error)?
                || run_identity.optical_constitution_digest
                    != optical_plan.digest().map_err(js_error)?
                || run_identity.sample_rate_hz != schema.sample_rate_hz
                || run_identity.horizon_ticks != trace.metadata.horizon_ticks
            {
                return Err(JsValue::from_str(
                    "run identity does not bind every supplied World program and horizon",
                ));
            }

            let width = canvas.width().max(1);
            let height = canvas.height().max(1);
            let instance = wgpu::Instance::default();
            let surface = instance
                .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
                .map_err(js_error)?;
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                    apply_limit_buckets: true,
                })
                .await
                .map_err(js_error)?;
            let capability = CapabilityProfile::from_adapter(&adapter);
            if !capability.supports_tiled_wave_16() {
                return Err(JsValue::from_str(
                    "WebGPU adapter cannot run the portable 16x16 Ariadne wave kernel",
                ));
            }
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("Ariadne World WebGPU device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                })
                .await
                .map_err(js_error)?;
            let mut surface_config = surface
                .get_default_config(&adapter, width, height)
                .ok_or_else(|| JsValue::from_str("adapter cannot present to the Ariadne canvas"))?;
            surface_config.present_mode = wgpu::PresentMode::AutoVsync;
            surface.configure(&device, &surface_config);

            let optical = WorldOpticalSurface::new(&device, &optical_plan, &world, &schema)
                .map_err(js_error)?;
            let first_view = optical
                .state_view_at(0)
                .ok_or_else(|| JsValue::from_str("optical field has no first persistent view"))?;
            let second_view = optical
                .state_view_at(1)
                .ok_or_else(|| JsValue::from_str("optical field has no second persistent view"))?;
            let presenter = WorldPresenter::new(
                &device,
                [first_view, second_view],
                surface_config.format,
            );
            let sample_values = trace.zero_value_buffer();
            let capability_json = serde_json::to_string(&capability).map_err(js_error)?;

            Ok(Self {
                surface,
                device,
                queue,
                surface_config,
                optical,
                presenter,
                trace,
                sample_values,
                requested_tick: 0,
                presentation_frame: 0,
                capability_json,
            })
        }

        #[wasm_bindgen(getter)]
        pub fn sample_rate_hz(&self) -> u32 {
            self.trace.metadata.schema.sample_rate_hz
        }

        #[wasm_bindgen(getter)]
        pub fn horizon_tick_low(&self) -> u32 {
            self.trace.metadata.horizon_ticks as u32
        }

        #[wasm_bindgen(getter)]
        pub fn horizon_tick_high(&self) -> u32 {
            (self.trace.metadata.horizon_ticks >> 32) as u32
        }

        #[wasm_bindgen(getter)]
        pub fn duration_seconds(&self) -> f64 {
            self.trace.duration_seconds()
        }

        #[wasm_bindgen(getter)]
        pub fn pending_steps(&self) -> f64 {
            self.optical.pending_steps() as f64
        }

        #[wasm_bindgen(getter)]
        pub fn capability_json(&self) -> String {
            self.capability_json.clone()
        }

        #[wasm_bindgen(getter)]
        pub fn physical_world_digest(&self) -> String {
            self.optical.physical_world_digest().to_owned()
        }

        #[wasm_bindgen(getter)]
        pub fn observation_schema_digest(&self) -> String {
            self.optical.observation_schema_digest().to_owned()
        }

        #[wasm_bindgen(getter)]
        pub fn optical_constitution_digest(&self) -> String {
            self.optical.optical_constitution_digest().to_owned()
        }

        /// Reset only observer history. The immutable physical run remains intact.
        pub fn reset_observer(&mut self) {
            self.optical.reset();
            self.requested_tick = 0;
            self.presentation_frame = 0;
        }

        /// Advance the fixed optical constitution toward one absolute material
        /// tick and present its current state. `tick_low`/`tick_high` avoid
        /// JavaScript number precision loss for long runs.
        pub fn render_tick(
            &mut self,
            tick_low: u32,
            tick_high: u32,
            width: u32,
            height: u32,
            absolute_seconds: f64,
        ) -> Result<u32, JsValue> {
            let material_tick = join_tick(tick_low, tick_high)
                .min(self.trace.metadata.horizon_ticks);
            if material_tick < self.requested_tick {
                self.reset_observer();
            }
            self.requested_tick = material_tick;
            self.resize(width, height);

            let due = self
                .optical
                .schedule_to_tick(material_tick)
                .map_err(js_error)?;
            if self.optical.reset_pending() {
                let mut encoder =
                    self.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Ariadne optical observer reset"),
                        });
                if self.optical.encode_pending_reset(&mut encoder) {
                    self.queue.submit([encoder.finish()]);
                }
            }
            for _ in 0..due {
                let step_tick = self
                    .optical
                    .next_step_tick()
                    .ok_or_else(|| JsValue::from_str("optical scheduler lost its pending step"))?;
                self.trace
                    .sample_at_tick_into(step_tick, &mut self.sample_values)
                    .map_err(js_error)?;
                let mut encoder =
                    self.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Ariadne optical constitutive step"),
                        });
                self.optical
                    .encode_values_step(
                        &self.queue,
                        &mut encoder,
                        step_tick,
                        &self.sample_values,
                    )
                    .map_err(js_error)?;
                // One submit per fixed step is intentional in v1: every step
                // writes the same uniform/event buffers, and wgpu queue writes
                // are ordered at submission boundaries.
                self.queue.submit([encoder.finish()]);
            }

            let mut status = if self.optical.is_caught_up() {
                STATUS_CAUGHT_UP
            } else {
                0
            };
            match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => {
                    self.present(frame, absolute_seconds);
                    status |= STATUS_PRESENTED;
                }
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    self.present(frame, absolute_seconds);
                    self.surface.configure(&self.device, &self.surface_config);
                    status |= STATUS_PRESENTED | STATUS_RECONFIGURED;
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {}
                wgpu::CurrentSurfaceTexture::Outdated => {
                    self.surface.configure(&self.device, &self.surface_config);
                    status |= STATUS_RECONFIGURED;
                }
                wgpu::CurrentSurfaceTexture::Lost => {
                    return Err(JsValue::from_str(
                        "WebGPU surface was lost; recreate AriadneWorldViewer",
                    ));
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err(JsValue::from_str(
                        "WebGPU surface acquisition reported a validation error",
                    ));
                }
            }
            Ok(status)
        }
    }

    impl AriadneWorldViewer {
        fn resize(&mut self, width: u32, height: u32) {
            let width = width.max(1);
            let height = height.max(1);
            if self.surface_config.width != width || self.surface_config.height != height {
                self.surface_config.width = width;
                self.surface_config.height = height;
                self.surface.configure(&self.device, &self.surface_config);
            }
        }

        fn present(&mut self, frame: wgpu::SurfaceTexture, absolute_seconds: f64) {
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder =
                self.device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Ariadne World presentation"),
                    });
            let aspect =
                self.surface_config.width as f32 / self.surface_config.height.max(1) as f32;
            let mut parameters = PresentationParams::default();
            parameters.viewport.x = aspect;
            parameters.viewport.y = finite_seconds(absolute_seconds) as f32;
            self.presenter.encode(
                &self.queue,
                &mut encoder,
                &view,
                self.optical.state_read_index(),
                parameters,
            );
            self.queue.submit([encoder.finish()]);
            self.queue.present(frame);
            self.presentation_frame = self.presentation_frame.saturating_add(1);
        }
    }

    #[inline]
    fn join_tick(low: u32, high: u32) -> u64 {
        u64::from(low) | (u64::from(high) << 32)
    }

    #[inline]
    fn finite_seconds(value: f64) -> f64 {
        if value.is_finite() {
            value.max(0.0)
        } else {
            0.0
        }
    }

    fn js_error(error: impl std::fmt::Display) -> JsValue {
        JsValue::from_str(&error.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
pub use browser::AriadneWorldViewer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_run_identity_survives_pretty_json() {
        let digest = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
        let identity = RunIdentity::new(
            digest('a'),
            digest('b'),
            digest('c'),
            digest('d'),
            digest('e'),
            digest('f'),
            48_000,
            120,
            48_000,
            4_800,
        )
        .unwrap();
        let pretty = serde_json::to_string_pretty(&identity).unwrap();
        assert_eq!(decode_run_identity(&pretty).unwrap(), identity);
    }
}
