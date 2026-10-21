use crate::primitive::{
    ImpulseRasterConfig, ObserverImpulseRaster2D, PrimitiveError, WaveSheet2D, WaveSheetConfig,
};
use crate::types::{ObserverImpulse2D, WaveSheetParams};
use serde::{Deserialize, Serialize};
use std::mem::size_of_val;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaveSurfaceConfig {
    pub width: u32,
    pub height: u32,
    pub max_impulses: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceStepReceipt {
    pub impulse_count: u32,
    pub upload_bytes: u64,
    pub dispatch_workgroups: [u32; 3],
}

/// Observer-owned optical graph: local excitation rasterization followed by one
/// tiled persistent wave-sheet step.
pub struct WaveSurface2D {
    config: WaveSurfaceConfig,
    impulses: ObserverImpulseRaster2D,
    wave: WaveSheet2D,
}

impl WaveSurface2D {
    pub fn new(device: &wgpu::Device, config: WaveSurfaceConfig) -> Result<Self, PrimitiveError> {
        let impulses = ObserverImpulseRaster2D::new(
            device,
            ImpulseRasterConfig {
                width: config.width,
                height: config.height,
                max_impulses: config.max_impulses,
            },
        )?;
        let wave = WaveSheet2D::new(
            device,
            impulses.view(),
            WaveSheetConfig {
                width: config.width,
                height: config.height,
            },
        )?;
        Ok(Self {
            config,
            impulses,
            wave,
        })
    }

    pub fn encode(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        impulses: &[ObserverImpulse2D],
        parameters: WaveSheetParams,
    ) -> Result<SurfaceStepReceipt, PrimitiveError> {
        self.impulses.upload(queue, impulses)?;
        self.impulses.encode(encoder);
        self.wave.encode(queue, encoder, parameters)?;
        Ok(SurfaceStepReceipt {
            impulse_count: self.impulses.impulse_count(),
            upload_bytes: size_of_val(impulses) as u64,
            dispatch_workgroups: [
                self.config.width.div_ceil(16),
                self.config.height.div_ceil(16),
                1,
            ],
        })
    }

    /// Clear the persistent observer field without consuming an observation.
    pub fn encode_reset(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.wave.encode_reset(encoder);
    }

    #[must_use]
    pub fn state_view(&self) -> &wgpu::TextureView {
        self.wave.state_view()
    }

    #[must_use]
    pub fn state_view_at(&self, index: usize) -> Option<&wgpu::TextureView> {
        self.wave.state_view_at(index)
    }

    #[must_use]
    pub const fn state_read_index(&self) -> usize {
        self.wave.state_read_index()
    }

    #[must_use]
    pub fn state_texture(&self) -> &wgpu::Texture {
        self.wave.state_texture()
    }

    #[must_use]
    pub fn impulse_view(&self) -> &wgpu::TextureView {
        self.impulses.view()
    }

    #[must_use]
    pub const fn config(&self) -> WaveSurfaceConfig {
        self.config
    }
}
