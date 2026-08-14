use glam::Vec4;

use crate::types::ObserverImpulse2D;

impl ObserverImpulse2D {
    #[must_use]
    pub const fn zeroed() -> Self {
        Self {
            position_radius: Vec4::ZERO,
            channels: Vec4::ZERO,
        }
    }
}
