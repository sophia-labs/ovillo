use super::{GpuPrimitive, PrimitiveError};
use crate::law::{Cadence, Domain, LawClass, PrimitiveDescriptor, SpatialSupport, ZeroLaw};
use crate::resource::{PingPongTexture2D, Texture2DConfig};
use crate::types::WaveSheetParams;
use wgpu::util::DeviceExt;

pub const DESCRIPTOR: PrimitiveDescriptor = PrimitiveDescriptor {
    name: "WaveSheet2D",
    domain: Domain::Observer,
    law: LawClass::ConstitutiveEvolution,
    cadence: Cadence::EveryFrame,
    support: SpatialSupport::Dense,
    zero_law: ZeroLaw {
        name: "zero-state",
        statement: "zero state plus zero impulse remains exactly zero",
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaveSheetConfig {
    pub width: u32,
    pub height: u32,
}

impl WaveSheetConfig {
    pub fn validate(self) -> Result<Self, PrimitiveError> {
        if self.width == 0 {
            return Err(PrimitiveError::ZeroDimension { name: "width" });
        }
        if self.height == 0 {
            return Err(PrimitiveError::ZeroDimension { name: "height" });
        }
        Ok(self)
    }
}

/// Persistent tiled wave/foam/optical field. The shader loads one 18 x 18
/// height tile into workgroup memory for each 16 x 16 output tile.
pub struct WaveSheet2D {
    config: WaveSheetConfig,
    state: PingPongTexture2D,
    parameters: wgpu::Buffer,
    bind_groups: [wgpu::BindGroup; 2],
    pipeline: wgpu::ComputePipeline,
}

impl WaveSheet2D {
    pub fn new(
        device: &wgpu::Device,
        impulse_view: &wgpu::TextureView,
        config: WaveSheetConfig,
    ) -> Result<Self, PrimitiveError> {
        let config = config.validate()?;
        let state = PingPongTexture2D::new(
            device,
            &Texture2DConfig {
                label: "Ariadne WaveSheet2D state",
                width: config.width,
                height: config.height,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC,
            },
        );
        let initial = WaveSheetParams::portable(config.width, config.height, 1.0 / 60.0);
        let parameter_bytes = bytemuck::bytes_of(&initial);
        let parameters = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ariadne WaveSheet2D parameters"),
            contents: parameter_bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ariadne WaveSheet2D WESL"),
            source: wgpu::ShaderSource::Wgsl(wesl::include_wesl!("ariadne_wave_sheet")),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Ariadne WaveSheet2D pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let layout = pipeline.get_bind_group_layout(0);
        let create_bind_group = |label: &'static str,
                                 read: &wgpu::TextureView,
                                 write: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(read),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(write),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(impulse_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: parameters.as_entire_binding(),
                    },
                ],
            })
        };
        let bind_groups = [
            create_bind_group("Ariadne WaveSheet2D A to B", state.read_view(), state.write_view()),
            create_bind_group("Ariadne WaveSheet2D B to A", state.write_view(), state.read_view()),
        ];
        Ok(Self {
            config,
            state,
            parameters,
            bind_groups,
            pipeline,
        })
    }

    pub fn encode(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        parameters: WaveSheetParams,
    ) -> Result<(), PrimitiveError> {
        if !parameters.integration.is_finite()
            || !parameters.constitutive.is_finite()
            || !parameters.limits.is_finite()
        {
            return Err(PrimitiveError::NonFiniteParameter {
                name: "wave-sheet parameters",
            });
        }
        let mut parameters = parameters.sanitized();
        parameters.dimensions.x = self.config.width;
        parameters.dimensions.y = self.config.height;
        queue.write_buffer(
            &self.parameters,
            0,
            bytemuck::bytes_of(&parameters),
        );

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Ariadne WaveSheet2D pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_groups[self.state.read_index()], &[]);
            pass.dispatch_workgroups(
                self.config.width.div_ceil(16),
                self.config.height.div_ceil(16),
                1,
            );
        }
        self.state.swap();
        Ok(())
    }

    #[must_use]
    pub fn state_view(&self) -> &wgpu::TextureView {
        self.state.read_view()
    }

    #[must_use]
    pub fn state_texture(&self) -> &wgpu::Texture {
        self.state.read_texture()
    }

    #[must_use]
    pub const fn config(&self) -> WaveSheetConfig {
        self.config
    }
}

impl GpuPrimitive for WaveSheet2D {
    fn descriptor(&self) -> &'static PrimitiveDescriptor {
        &DESCRIPTOR
    }
}
