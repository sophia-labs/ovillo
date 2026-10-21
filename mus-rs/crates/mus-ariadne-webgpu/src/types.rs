use bytemuck::{Pod, Zeroable};
use encase::{ShaderType, UniformBuffer};
use glam::{UVec4, Vec4};
use serde::{Deserialize, Serialize};

/// A compact, non-loading event splatted into an observer-owned field.
///
/// `position_radius = (x, y, radius, compactness)` in normalized optical-world
/// space. `channels = (motion, structure, emission, auxiliary)`; the semantic
/// bridge that creates the packet is content-addressed separately.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, ShaderType, Serialize, Deserialize)]
pub struct ObserverImpulse2D {
    pub position_radius: Vec4,
    pub channels: Vec4,
}

impl ObserverImpulse2D {
    #[must_use]
    pub fn new(position: [f32; 2], radius: f32, motion: f32, structure: f32) -> Self {
        Self {
            position_radius: Vec4::new(position[0], position[1], radius, 3.0),
            channels: Vec4::new(motion, structure, 0.0, 0.0),
        }
    }

    #[must_use]
    pub fn with_emission(mut self, emission: f32) -> Self {
        self.channels.z = emission;
        self
    }

    #[must_use]
    pub fn with_auxiliary(mut self, auxiliary: f32) -> Self {
        self.channels.w = auxiliary;
        self
    }

    #[must_use]
    pub fn with_compactness(mut self, compactness: f32) -> Self {
        self.position_radius.w = compactness;
        self
    }

    #[must_use]
    pub fn sanitized(self) -> Self {
        let finite = |value: f32, fallback: f32| {
            if value.is_finite() {
                value
            } else {
                fallback
            }
        };
        Self {
            position_radius: Vec4::new(
                finite(self.position_radius.x, 0.5).clamp(-4.0, 5.0),
                finite(self.position_radius.y, 0.5).clamp(-4.0, 5.0),
                finite(self.position_radius.z, 0.01)
                    .abs()
                    .clamp(1.0e-5, 4.0),
                finite(self.position_radius.w, 3.0).clamp(0.25, 32.0),
            ),
            channels: Vec4::new(
                finite(self.channels.x, 0.0).clamp(-1.0e4, 1.0e4),
                finite(self.channels.y, 0.0).clamp(-1.0e4, 1.0e4),
                finite(self.channels.z, 0.0).clamp(-1.0e4, 1.0e4),
                finite(self.channels.w, 0.0).clamp(-1.0e4, 1.0e4),
            ),
        }
    }
}

/// Fixed header for one optical-constitution step.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, ShaderType, Serialize, Deserialize)]
pub struct OpticalFrameHeader {
    /// `(material_tick_lo, material_tick_hi, event_count, flags)`.
    pub clock: UVec4,
    /// `(fixed_dt_seconds, absolute_seconds, optical_step, quality_level)`.
    pub timing: Vec4,
    /// Consumer-defined non-loading camera or local observer frame.
    pub observer: Vec4,
}

impl Default for OpticalFrameHeader {
    fn default() -> Self {
        Self {
            clock: UVec4::ZERO,
            timing: Vec4::ZERO,
            observer: Vec4::ZERO,
        }
    }
}

impl OpticalFrameHeader {
    #[must_use]
    pub fn with_material_tick(mut self, tick: u64) -> Self {
        self.clock.x = tick as u32;
        self.clock.y = (tick >> 32) as u32;
        self
    }

    #[must_use]
    pub fn material_tick(self) -> u64 {
        u64::from(self.clock.x) | (u64::from(self.clock.y) << 32)
    }
}

/// Uniform block for one observer wave-sheet integration step.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, ShaderType, Serialize, Deserialize)]
pub struct WaveSheetParams {
    /// `(width, height, reset_flag, reserved)`.
    pub dimensions: UVec4,
    /// `(dt, damping, wave_speed_squared, restoring_force)`.
    pub integration: Vec4,
    /// `(structure_decay, motion_gain, emission_decay, impulse_scale)`.
    pub constitutive: Vec4,
    /// `(max_height, max_velocity, max_structure, max_emission)`.
    pub limits: Vec4,
}

impl WaveSheetParams {
    #[must_use]
    pub fn portable(width: u32, height: u32, dt_seconds: f32) -> Self {
        Self {
            dimensions: UVec4::new(width, height, 0, 0),
            integration: Vec4::new(dt_seconds, 0.991, 42.0, 0.038),
            constitutive: Vec4::new(1.9, 0.014, 1.2, 1.0),
            limits: Vec4::new(2.0, 4.0, 3.0, 8.0),
        }
    }

    #[must_use]
    pub fn with_reset(mut self, reset: bool) -> Self {
        self.dimensions.z = u32::from(reset);
        self
    }

    #[must_use]
    pub fn sanitized(mut self) -> Self {
        self.dimensions.x = self.dimensions.x.max(1);
        self.dimensions.y = self.dimensions.y.max(1);
        self.integration.x = finite(self.integration.x, 1.0 / 60.0).clamp(1.0e-6, 0.25);
        self.integration.y = finite(self.integration.y, 0.991).clamp(0.0, 1.0);
        self.integration.z = finite(self.integration.z, 42.0).clamp(0.0, 1.0e5);
        self.integration.w = finite(self.integration.w, 0.038).clamp(0.0, 1.0e4);
        self.constitutive.x = finite(self.constitutive.x, 1.9).clamp(0.0, 1.0e4);
        self.constitutive.y = finite(self.constitutive.y, 0.014).clamp(0.0, 1.0e4);
        self.constitutive.z = finite(self.constitutive.z, 1.2).clamp(0.0, 1.0e4);
        self.constitutive.w = finite(self.constitutive.w, 1.0).clamp(-1.0e4, 1.0e4);
        self.limits = self.limits.abs().max(Vec4::splat(1.0e-6));
        self
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

#[must_use]
pub fn encode_optical_frame_header(value: &OpticalFrameHeader) -> Vec<u8> {
    let mut buffer = UniformBuffer::new(Vec::new());
    buffer
        .write(value)
        .expect("fixed-footprint optical frame header must encode");
    buffer.into_inner()
}

#[must_use]
pub fn encode_wave_sheet_params(value: &WaveSheetParams) -> Vec<u8> {
    let mut buffer = UniformBuffer::new(Vec::new());
    buffer
        .write(value)
        .expect("fixed-footprint observer-wave parameters must encode");
    buffer.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optical_header_has_stable_uniform_footprint_and_tick_roundtrip() {
        let header = OpticalFrameHeader::default().with_material_tick(u64::MAX - 7);
        assert_eq!(header.material_tick(), u64::MAX - 7);
        assert_eq!(encode_optical_frame_header(&header).len(), 48);
    }

    #[test]
    fn wave_uniform_has_stable_uniform_footprint() {
        assert_eq!(
            encode_wave_sheet_params(&WaveSheetParams::portable(384, 384, 1.0 / 60.0)).len(),
            64
        );
    }

    #[test]
    fn impulse_sanitization_refuses_nonfinite_values() {
        let impulse = ObserverImpulse2D {
            position_radius: Vec4::new(f32::NAN, f32::INFINITY, -0.0, f32::NEG_INFINITY),
            channels: Vec4::splat(f32::NAN),
        }
        .sanitized();
        assert!(impulse.position_radius.is_finite());
        assert!(impulse.channels.is_finite());
        assert!(impulse.position_radius.z > 0.0);
    }
}
