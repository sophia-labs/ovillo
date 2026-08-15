#[inline]
fn kinetic(values: &[f64; MODE_COUNT], dt: f64) -> f64 {
    0.5 * dt * values.iter().map(|value| value * value).sum::<f64>()
}

#[inline]
fn dot(left: &[f64; MODE_COUNT], right: &[f64; MODE_COUNT]) -> f64 {
    left
        .iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

#[inline]
fn add_projection(
    state: &mut [f64; MODE_COUNT],
    direction: &[f64; MODE_COUNT],
    amount: f64,
) {
    for (value, basis) in state.iter_mut().zip(direction) {
        *value += amount * basis;
    }
}

fn normalize(mut direction: [f64; MODE_COUNT]) -> Option<[f64; MODE_COUNT]> {
    let norm = direction
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 {
        return None;
    }
    for value in &mut direction {
        *value /= norm;
    }
    Some(direction)
}

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}
