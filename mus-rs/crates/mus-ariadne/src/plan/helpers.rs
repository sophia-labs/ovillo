use super::PlanError;

const EPS: f64 = 1.0e-12;
const INDEPENDENCE_EPS: f64 = 1.0e-8;

pub(super) fn validate_id(kind: &str, id: &str) -> Result<(), PlanError> {
    if id.trim().is_empty() || id.len() > 128 || id.chars().any(char::is_whitespace) {
        return Err(PlanError::new(
            "invalid-world-plan-id",
            format!("{kind} id must contain 1..=128 non-whitespace characters"),
        ));
    }
    Ok(())
}

pub(super) fn validate_direction<const N: usize>(
    name: &str,
    direction: &[f64; N],
) -> Result<(), PlanError> {
    if direction.iter().any(|value| !value.is_finite()) || norm(direction) <= EPS {
        return Err(PlanError::new(
            "invalid-direction",
            format!("{name} must be finite and nondegenerate"),
        ));
    }
    Ok(())
}

pub(super) fn validate_independent<const R: usize, const N: usize>(
    name: &str,
    directions: &[[f64; N]; R],
) -> Result<(), PlanError> {
    let mut basis = [[0.0; N]; R];
    for rank in 0..R {
        let mut candidate = directions[rank];
        validate_direction(name, &candidate)?;
        for previous in basis.iter().take(rank) {
            let projection = dot(&candidate, previous);
            for index in 0..N {
                candidate[index] -= projection * previous[index];
            }
        }
        let candidate_norm = norm(&candidate);
        if candidate_norm <= INDEPENDENCE_EPS {
            return Err(PlanError::new(
                "dependent-directions",
                format!("{name} are linearly dependent at rank {rank}"),
            ));
        }
        for value in &mut candidate {
            *value /= candidate_norm;
        }
        basis[rank] = candidate;
    }
    Ok(())
}

pub(super) fn normalized_dot<const N: usize>(
    left: &[f64; N],
    right: &[f64; N],
) -> Result<f64, PlanError> {
    validate_direction("left direction", left)?;
    validate_direction("right direction", right)?;
    Ok(dot(left, right) / (norm(left) * norm(right)))
}

fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn norm<const N: usize>(values: &[f64; N]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}
