/// Configuration for a persistent two-dimensional texture pair.
#[derive(Debug, Clone)]
pub struct Texture2DConfig<'a> {
    pub label: &'a str,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
}

impl<'a> Texture2DConfig<'a> {
    #[must_use]
    pub fn extent(&self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width.max(1),
            height: self.height.max(1),
            depth_or_array_layers: 1,
        }
    }
}

/// Persistent ping-pong texture state with stable views and no hot-path
/// allocation.
pub struct PingPongTexture2D {
    textures: [wgpu::Texture; 2],
    views: [wgpu::TextureView; 2],
    read_index: usize,
}

impl PingPongTexture2D {
    #[must_use]
    pub fn new(device: &wgpu::Device, config: &Texture2DConfig<'_>) -> Self {
        let create = |suffix: &str| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("{} {suffix}", config.label)),
                size: config.extent(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: config.format,
                usage: config.usage,
                view_formats: &[],
            })
        };
        let textures = [create("A"), create("B")];
        let views = [
            textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
            textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
        ];
        Self {
            textures,
            views,
            read_index: 0,
        }
    }

    #[must_use]
    pub fn read_view(&self) -> &wgpu::TextureView {
        &self.views[self.read_index]
    }

    #[must_use]
    pub fn write_view(&self) -> &wgpu::TextureView {
        &self.views[1 - self.read_index]
    }

    #[must_use]
    pub fn view(&self, index: usize) -> Option<&wgpu::TextureView> {
        self.views.get(index)
    }

    #[must_use]
    pub const fn read_index(&self) -> usize {
        self.read_index
    }

    #[must_use]
    pub fn read_texture(&self) -> &wgpu::Texture {
        &self.textures[self.read_index]
    }

    #[must_use]
    pub fn texture(&self, index: usize) -> Option<&wgpu::Texture> {
        self.textures.get(index)
    }

    pub fn swap(&mut self) {
        self.read_index = 1 - self.read_index;
    }

    pub fn reset_orientation(&mut self) {
        self.read_index = 0;
    }
}
