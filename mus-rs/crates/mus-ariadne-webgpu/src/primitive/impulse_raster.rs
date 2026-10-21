use super::{GpuPrimitive, PrimitiveError};
use crate::law::{Cadence, Domain, LawClass, PrimitiveDescriptor, SpatialSupport, ZeroLaw};
use crate::types::ObserverImpulse2D;
use std::mem::size_of;

pub const DESCRIPTOR: PrimitiveDescriptor = PrimitiveDescriptor {
    name: "ObserverImpulseRaster2D",
    domain: Domain::Observer,
    law: LawClass::ObserverExcitation,
    cadence: Cadence::OnEvent,
    support: SpatialSupport::EventRaster,
    zero_law: ZeroLaw {
        name: "zero-event",
        statement: "clearing with zero observer impulses leaves every target channel exactly zero",
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpulseRasterConfig {
    pub width: u32,
    pub height: u32,
    pub max_impulses: usize,
}

impl ImpulseRasterConfig {
    pub fn validate(self) -> Result<Self, PrimitiveError> {
        if self.width == 0 {
            return Err(PrimitiveError::ZeroDimension { name: "width" });
        }
        if self.height == 0 {
            return Err(PrimitiveError::ZeroDimension { name: "height" });
        }
        if self.max_impulses == 0 {
            return Err(PrimitiveError::ZeroDimension {
                name: "max_impulses",
            });
        }
        Ok(self)
    }
}

/// Renders compact local observer events into one additive `rgba16float`
/// excitation texture. Cost follows covered event pixels instead of field cells
/// times event count.
pub struct ObserverImpulseRaster2D {
    config: ImpulseRasterConfig,
    target: wgpu::Texture,
    target_view: wgpu::TextureView,
    impulse_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    scratch: Vec<ObserverImpulse2D>,
    impulse_count: u32,
}

impl ObserverImpulseRaster2D {
    pub fn new(device: &wgpu::Device, config: ImpulseRasterConfig) -> Result<Self, PrimitiveError> {
        let config = config.validate()?;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Ariadne observer impulse target"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let impulse_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Ariadne observer impulses"),
            size: (config.max_impulses * size_of::<ObserverImpulse2D>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ariadne observer impulse WESL"),
            source: wgpu::ShaderSource::Wgsl(wesl::include_wesl!("ariadne_impulse_raster").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ariadne observer impulse pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ariadne observer impulse bind group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: impulse_buffer.as_entire_binding(),
            }],
        });
        Ok(Self {
            config,
            target,
            target_view,
            impulse_buffer,
            bind_group,
            pipeline,
            scratch: Vec::with_capacity(config.max_impulses),
            impulse_count: 0,
        })
    }

    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        impulses: &[ObserverImpulse2D],
    ) -> Result<(), PrimitiveError> {
        if impulses.len() > self.config.max_impulses {
            return Err(PrimitiveError::TooManyImpulses {
                observed: impulses.len(),
                capacity: self.config.max_impulses,
            });
        }
        self.scratch.clear();
        self.scratch
            .extend(impulses.iter().copied().map(ObserverImpulse2D::sanitized));
        self.impulse_count = self.scratch.len() as u32;
        if !self.scratch.is_empty() {
            queue.write_buffer(
                &self.impulse_buffer,
                0,
                bytemuck::cast_slice(self.scratch.as_slice()),
            );
        }
        Ok(())
    }

    /// Clear the excitation target and rasterize the currently uploaded events.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let color_attachment = Some(wgpu::RenderPassColorAttachment {
            view: &self.target_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        });
        let color_attachments = [color_attachment];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Ariadne observer impulse pass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if self.impulse_count > 0 {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..6, 0..self.impulse_count);
        }
    }

    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.target_view
    }

    #[must_use]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.target
    }

    #[must_use]
    pub const fn impulse_count(&self) -> u32 {
        self.impulse_count
    }

    #[must_use]
    pub const fn config(&self) -> ImpulseRasterConfig {
        self.config
    }
}

impl GpuPrimitive for ObserverImpulseRaster2D {
    fn descriptor(&self) -> &'static PrimitiveDescriptor {
        &DESCRIPTOR
    }
}
