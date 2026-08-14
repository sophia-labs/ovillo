use bytemuck::{Pod, Zeroable};
use encase::{ShaderType, UniformBuffer};
use glam::{UVec4, Vec4};
use serde::{Deserialize, Serialize};

/// A compact event splatted into the impulse field.
///
/// `position_radius = (x, y, radius, compactness)` in normalized world space.
/// `channels = (momentum, foam, optical_work, auxiliary)`.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Pod, Zeroable, ShaderType, Serialize, Deserialize,
)]
pub struct Impulse2D {
    pub position_radius: Vec4,
    pub channels: Vec4,
}

impl Impulse2D {
    #[must_use]
    pub fn new(position: [f32; 2], radius: f32, momentum: f32, foam: f32) -> Self {
        Self {
            position_radius: Vec4::new(position[0], position[1], radius, 3.0),
            channels: Vec4::new(momentum, foam, 0.0, 0.0),
        }
    }

    #[must_use]
    pub fn with_optical_work(mut self, optical_work: f32) -> Self {
        self.channels.z = optical_work;
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
                finite(self.position_radius.z, 0.01).abs().clamp(1.0e-5, 4.0),
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

/// Fixed frame packet intended to become the shared hot-path upload contract.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Pod, Zeroable, ShaderType, Serialize, Deserialize,
)]
pub struct FramePacket {
    /// `(sample_lo, sample_hi, event_count, flags)`.
    pub clock: UVec4,
    /// `(dt_seconds, absolute_seconds, display_frame, quality_level)`.
    pub timing: Vec4,
    /// Consumer-defined observer pose or local frame.
    pub observer: Vec4,
    /// Compact scientific/performance receipt inputs.
    pub receipt: Vec4,
}

impl Default for FramePacket {
    fn default() -> Self {
        Self {
            clock: UVec4::ZERO,
            timing: Vec4::ZERO,
            observer: Vec4::ZERO,
            receipt: Vec4::ZERO,
        }
    }
}

/// Uniform block for one wave-sheet integration step.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Pod, Zeroable, ShaderType, Serialize, Deserialize,
)]
pub struct WaveSheetParams {
    /// `(width, height, reset_flag, reserved)`.
    pub dimensions: UVec4,
    /// `(dt, damping, wave_speed_squared, restoring_force)`.
    pub integration: Vec4,
    /// `(foam_decay, foam_velocity_gain, optical_decay, impulse_scale)`.
    pub constitutive: Vec4,
    /// `(max_height, max_velocity, max_foam, max_optical)`.
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

/// Encode the shared frame packet according to WGSL uniform layout.
#[must_use]
pub fn encode_frame_packet(value: &FramePacket) -> Vec<u8> {
    let mut buffer = UniformBuffer::new(Vec::new());
    buffer
        .write(value)
        .expect("fixed-footprint Ariadne frame packet must encode");
    buffer.into_inner()
}

/// Encode one wave-sheet parameter block according to WGSL uniform layout.
#[must_use]
pub fn encode_wave_sheet_params(value: &WaveSheetParams) -> Vec<u8> {
    let mut buffer = UniformBuffer::new(Vec::new());
    buffer
        .write(value)
        .expect("fixed-footprint Ariadne wave parameters must encode");
    buffer.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_packet_has_stable_uniform_footprint() {
        assert_eq!(encode_frame_packet(&FramePacket::default()).len(), 64);
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
        let impulse = Impulse2D {
            position_radius: Vec4::new(f32::NAN, f32::INFINITY, -0.0, f32::NEG_INFINITY),
            channels: Vec4::splat(f32::NAN),
        }
        .sanitized();
        assert!(impulse.position_radius.is_finite());
        assert!(impulse.channels.is_finite());
        assert!(impulse.position_radius.z > 0.0);
    }
}
