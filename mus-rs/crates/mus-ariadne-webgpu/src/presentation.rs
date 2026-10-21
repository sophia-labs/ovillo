use bytemuck::{Pod, Zeroable};
use glam::Vec4;
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

use crate::law::{Cadence, Domain, LawClass, PrimitiveDescriptor, SpatialSupport, ZeroLaw};

pub const DESCRIPTOR: PrimitiveDescriptor = PrimitiveDescriptor {
    name: "WorldPresenter",
    domain: Domain::Presentation,
    law: LawClass::NonLoadingObservation,
    cadence: Cadence::EveryFrame,
    support: SpatialSupport::FullScreen,
    zero_law: ZeroLaw {
        name: "zero-field",
        statement: "a zero optical field presents the declared black background",
    },
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct PresentationParams {
    /// `(aspect, absolute_seconds, exposure, gamma)`.
    pub viewport: Vec4,
    /// `(structure_gain, motion_gain, emission_gain, vignette)`.
    pub palette: Vec4,
}

impl Default for PresentationParams {
    fn default() -> Self {
        Self {
            viewport: Vec4::new(1.0, 0.0, 1.45, 2.2),
            palette: Vec4::new(0.90, 0.70, 1.15, 0.34),
        }
    }
}

impl PresentationParams {
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        self.viewport.x = finite(self.viewport.x, 1.0).clamp(0.05, 20.0);
        self.viewport.y = finite(self.viewport.y, 0.0).max(0.0);
        self.viewport.z = finite(self.viewport.z, 1.45).clamp(0.01, 64.0);
        self.viewport.w = finite(self.viewport.w, 2.2).clamp(0.2, 8.0);
        self.palette.x = finite(self.palette.x, 0.90).clamp(0.0, 32.0);
        self.palette.y = finite(self.palette.y, 0.70).clamp(0.0, 32.0);
        self.palette.z = finite(self.palette.z, 1.15).clamp(0.0, 32.0);
        self.palette.w = finite(self.palette.w, 0.34).clamp(0.0, 1.0);
        self
    }
}

/// Presentation-only full-screen observer over both persistent ping-pong views.
/// Bind groups are created once; choosing the currently readable field requires
/// only an index and performs no per-frame resource allocation.
pub struct WorldPresenter {
    parameters: wgpu::Buffer,
    bind_groups: [wgpu::BindGroup; 2],
    pipeline: wgpu::RenderPipeline,
}

impl WorldPresenter {
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        state_views: [&wgpu::TextureView; 2],
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let initial = PresentationParams::default();
        let parameters = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ariadne World presentation parameters"),
            contents: bytemuck::bytes_of(&initial),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ariadne World presenter WESL"),
            source: wgpu::ShaderSource::Wgsl(wesl::include_wesl!("ariadne_world_present").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ariadne World presenter pipeline"),
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
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let layout = pipeline.get_bind_group_layout(0);
        let bind_groups = state_views.map(|state_view| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Ariadne World presenter bind group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(state_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: parameters.as_entire_binding(),
                    },
                ],
            })
        });
        Self {
            parameters,
            bind_groups,
            pipeline,
        }
    }

    pub fn encode(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        state_read_index: usize,
        parameters: PresentationParams,
    ) {
        queue.write_buffer(
            &self.parameters,
            0,
            bytemuck::bytes_of(&parameters.sanitized()),
        );
        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view: target_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.002,
                    g: 0.003,
                    b: 0.008,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Ariadne World presentation pass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_groups[state_read_index.min(1)], &[]);
        pass.draw(0..3, 0..1);
    }

    #[must_use]
    pub const fn descriptor(&self) -> &'static PrimitiveDescriptor {
        &DESCRIPTOR
    }
}

#[inline]
fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_parameters_reject_nonfinite_values() {
        let parameters = PresentationParams {
            viewport: Vec4::splat(f32::NAN),
            palette: Vec4::splat(f32::INFINITY),
        }
        .sanitized();
        assert!(parameters.viewport.is_finite());
        assert!(parameters.palette.is_finite());
        assert!(parameters.viewport.z > 0.0);
        assert!(parameters.viewport.w > 0.0);
    }
}
