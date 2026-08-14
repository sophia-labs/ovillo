use serde::{Deserialize, Serialize};

/// Adapter facts used to select a portable quality profile without smuggling
/// optional WebGPU features into the baseline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityProfile {
    pub adapter_name: String,
    pub backend: String,
    pub device_type: String,
    pub timestamp_query: bool,
    pub shader_f16: bool,
    pub max_texture_dimension_2d: u32,
    pub max_storage_buffers_per_shader_stage: u32,
    pub max_storage_textures_per_shader_stage: u32,
    pub max_compute_workgroup_storage_size: u32,
    pub max_compute_invocations_per_workgroup: u32,
    pub max_compute_workgroups_per_dimension: u32,
}

impl CapabilityProfile {
    #[must_use]
    pub fn from_adapter(adapter: &wgpu::Adapter) -> Self {
        let info = adapter.get_info();
        let features = adapter.features();
        let limits = adapter.limits();
        Self {
            adapter_name: info.name,
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            timestamp_query: features.contains(wgpu::Features::TIMESTAMP_QUERY),
            shader_f16: features.contains(wgpu::Features::SHADER_F16),
            max_texture_dimension_2d: limits.max_texture_dimension_2d,
            max_storage_buffers_per_shader_stage: limits.max_storage_buffers_per_shader_stage,
            max_storage_textures_per_shader_stage: limits.max_storage_textures_per_shader_stage,
            max_compute_workgroup_storage_size: limits.max_compute_workgroup_storage_size,
            max_compute_invocations_per_workgroup: limits.max_compute_invocations_per_workgroup,
            max_compute_workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
        }
    }

    /// Whether the portable 16 x 16 tiled wave kernel fits the adapter.
    #[must_use]
    pub fn supports_tiled_wave_16(&self) -> bool {
        const TILE_FLOATS: u32 = 18 * 18;
        self.max_compute_invocations_per_workgroup >= 16 * 16
            && self.max_compute_workgroup_storage_size >= TILE_FLOATS * 4
    }
}
