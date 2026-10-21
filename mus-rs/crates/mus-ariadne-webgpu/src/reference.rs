use crate::types::{ObserverImpulse2D, WaveSheetParams};

/// CPU reference grid for conformance and refinement tests.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceField2D {
    width: usize,
    height: usize,
    cells: Vec<[f32; 4]>,
}

impl ReferenceField2D {
    #[must_use]
    pub fn zero(width: usize, height: usize) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Self {
            width,
            height,
            cells: vec![[0.0; 4]; width * height],
        }
    }

    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    #[must_use]
    pub fn cells(&self) -> &[[f32; 4]] {
        &self.cells
    }

    #[must_use]
    pub fn get(&self, x: usize, y: usize) -> [f32; 4] {
        self.cells[self.index(x, y)]
    }

    pub fn add(&mut self, x: usize, y: usize, value: [f32; 4]) {
        let index = self.index(x, y);
        for (target, source) in self.cells[index].iter_mut().zip(value) {
            *target += source;
        }
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y.min(self.height - 1) * self.width + x.min(self.width - 1)
    }
}

/// Local-support CPU analogue of `ObserverImpulseRaster2D`.
#[must_use]
pub fn raster_observer_impulses_reference(
    width: usize,
    height: usize,
    impulses: &[ObserverImpulse2D],
) -> ReferenceField2D {
    let width = width.max(1);
    let height = height.max(1);
    let mut output = ReferenceField2D::zero(width, height);
    for impulse in impulses.iter().copied().map(ObserverImpulse2D::sanitized) {
        let center_x = impulse.position_radius.x * width as f32;
        let center_y = impulse.position_radius.y * height as f32;
        let radius_x = impulse.position_radius.z * width as f32;
        let radius_y = impulse.position_radius.z * height as f32;
        let minimum_x = (center_x - radius_x).floor().max(0.0) as usize;
        let maximum_x = (center_x + radius_x)
            .ceil()
            .min((width.saturating_sub(1)) as f32) as usize;
        let minimum_y = (center_y - radius_y).floor().max(0.0) as usize;
        let maximum_y = (center_y + radius_y)
            .ceil()
            .min((height.saturating_sub(1)) as f32) as usize;
        for y in minimum_y..=maximum_y {
            for x in minimum_x..=maximum_x {
                let uv_x = (x as f32 + 0.5) / width as f32;
                let uv_y = (y as f32 + 0.5) / height as f32;
                let local_x = (uv_x - impulse.position_radius.x) / impulse.position_radius.z;
                let local_y = (uv_y - impulse.position_radius.y) / impulse.position_radius.z;
                let radius_squared = local_x * local_x + local_y * local_y;
                if radius_squared <= 1.0 {
                    let kernel = (-impulse.position_radius.w * radius_squared).exp();
                    let channels = impulse.channels * kernel;
                    output.add(x, y, channels.to_array());
                }
            }
        }
    }
    output
}

/// CPU analogue of one `WaveSheet2D` update using clamp-to-edge boundaries.
#[must_use]
pub fn step_wave_reference(
    previous: &ReferenceField2D,
    impulse: &ReferenceField2D,
    parameters: WaveSheetParams,
) -> ReferenceField2D {
    assert_eq!(previous.width, impulse.width);
    assert_eq!(previous.height, impulse.height);
    let parameters = parameters.sanitized();
    let mut next = ReferenceField2D::zero(previous.width, previous.height);
    if parameters.dimensions.z != 0 {
        return next;
    }
    let dt = parameters.integration.x;
    for y in 0..previous.height {
        for x in 0..previous.width {
            let center = previous.get(x, y);
            let left = previous.get(x.saturating_sub(1), y)[0];
            let right = previous.get((x + 1).min(previous.width - 1), y)[0];
            let up = previous.get(x, y.saturating_sub(1))[0];
            let down = previous.get(x, (y + 1).min(previous.height - 1))[0];
            let laplacian = left + right + up + down - 4.0 * center[0];
            let source = impulse.get(x, y);
            let velocity = (center[1]
                + (parameters.integration.z * laplacian - parameters.integration.w * center[0])
                    * dt
                + source[0] * parameters.constitutive.w)
                * parameters.integration.y;
            let height = center[0] + velocity * dt;
            let structure = center[2] * (-parameters.constitutive.x * dt).exp()
                + parameters.constitutive.y * velocity.abs()
                + source[1];
            let emission = center[3] * (-parameters.constitutive.z * dt).exp() + source[2];
            let output_index = next.index(x, y);
            next.cells[output_index] = [
                height.clamp(-parameters.limits.x, parameters.limits.x),
                velocity.clamp(-parameters.limits.y, parameters.limits.y),
                structure.clamp(0.0, parameters.limits.z),
                emission.clamp(0.0, parameters.limits.w),
            ];
        }
    }
    next
}

/// Simple nonnegative storage proxy for visual regression tests. It is not an
/// Ariadne physical energy ledger.
#[must_use]
pub fn storage_proxy(field: &ReferenceField2D) -> f64 {
    field
        .cells
        .iter()
        .map(|cell| {
            let height = cell[0] as f64;
            let velocity = cell[1] as f64;
            0.5 * (height * height + velocity * velocity)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn zero_impulses_produce_an_exactly_zero_target() {
        let field = raster_observer_impulses_reference(32, 32, &[]);
        assert!(field.cells().iter().all(|cell| *cell == [0.0; 4]));
    }

    #[test]
    fn compact_impulse_has_local_support() {
        let impulse = ObserverImpulse2D::new([0.5, 0.5], 0.1, 1.0, 0.5);
        let field = raster_observer_impulses_reference(64, 64, &[impulse]);
        assert!(field.get(32, 32)[0] > 0.0);
        assert_eq!(field.get(0, 0), [0.0; 4]);
        assert_eq!(field.get(63, 63), [0.0; 4]);
    }

    #[test]
    fn zero_state_and_zero_impulse_remain_zero() {
        let field = ReferenceField2D::zero(16, 16);
        let impulse = ReferenceField2D::zero(16, 16);
        let next = step_wave_reference(
            &field,
            &impulse,
            WaveSheetParams::portable(16, 16, 1.0 / 120.0),
        );
        assert_eq!(storage_proxy(&next), 0.0);
    }

    #[test]
    fn reset_is_exact() {
        let impulse = raster_observer_impulses_reference(
            16,
            16,
            &[ObserverImpulse2D::new([0.5, 0.5], 0.2, 1.0, 0.0)],
        );
        let next = step_wave_reference(
            &impulse,
            &impulse,
            WaveSheetParams::portable(16, 16, 1.0 / 60.0).with_reset(true),
        );
        assert_abs_diff_eq!(storage_proxy(&next), 0.0, epsilon = f64::EPSILON);
    }
}
