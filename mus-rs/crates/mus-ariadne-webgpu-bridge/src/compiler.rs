use std::collections::BTreeMap;

use mus_ariadne::WorldPlan;
use mus_ariadne_webgpu::ObserverImpulse2D;
use mus_observation::{ObservationFrame, ObservationSchema, ObservationValue};

use crate::plan::{
    BodyOpticalSpec, CouplingOpticalSpec, OpticalConstitutionPlan,
    OpticalPlanError, SharedFieldOpticalSpec,
};

#[derive(Debug, Clone)]
struct BoundBody {
    position: [f32; 2],
    radius: f32,
    color: [f32; 3],
    material_energy: usize,
    reservoir_energy: usize,
    emission: usize,
    source_work: usize,
}

#[derive(Debug, Clone)]
struct BoundField {
    position: [f32; 2],
    radius: f32,
    color: [f32; 3],
    stored_energy: usize,
    emitted_left: usize,
    emitted_right: usize,
    exchange: usize,
}

#[derive(Debug, Clone)]
struct BoundCoupling {
    position: [f32; 2],
    radius: f32,
    color: [f32; 3],
    exchange: usize,
}

#[derive(Debug, Clone)]
pub struct OpticalPacketCompiler {
    physical_world_digest: String,
    observation_schema_digest: String,
    optical_constitution_digest: String,
    bodies: Vec<BoundBody>,
    field: BoundField,
    couplings: Vec<BoundCoupling>,
}

impl OpticalPacketCompiler {
    pub fn compile(
        plan: &OpticalConstitutionPlan,
        world: &WorldPlan,
        schema: &ObservationSchema,
    ) -> Result<Self, OpticalCompileError> {
        plan.validate(world, schema)?;
        let channel_indices = schema
            .channels
            .iter()
            .enumerate()
            .map(|(index, channel)| (channel.id.as_str(), index))
            .collect::<BTreeMap<_, _>>();
        let body_positions = world
            .bodies
            .iter()
            .map(|body| {
                (
                    body.id.as_str(),
                    plan.bounds
                        .normalize([body.pose.position[0], body.pose.position[1]]),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let bodies = plan
            .bodies
            .iter()
            .map(|body| bind_body(body, &body_positions, &channel_indices))
            .collect::<Result<Vec<_>, _>>()?;
        let field = bind_field(&plan.shared_field, &channel_indices)?;
        let couplings = plan
            .couplings
            .iter()
            .map(|coupling| {
                bind_coupling(coupling, world, &body_positions, &channel_indices)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            physical_world_digest: plan.physical_world_digest.clone(),
            observation_schema_digest: plan.observation_schema_digest.clone(),
            optical_constitution_digest: plan.digest()?,
            bodies,
            field,
            couplings,
        })
    }

    #[must_use]
    pub fn physical_world_digest(&self) -> &str {
        &self.physical_world_digest
    }

    #[must_use]
    pub fn observation_schema_digest(&self) -> &str {
        &self.observation_schema_digest
    }

    #[must_use]
    pub fn optical_constitution_digest(&self) -> &str {
        &self.optical_constitution_digest
    }

    #[must_use]
    pub fn packet_count(&self) -> usize {
        self.bodies.len() + self.couplings.len() + 1
    }

    /// Compile one typed observation frame into preallocated GPU packets.
    /// No allocation or name lookup occurs.
    pub fn compile_into(
        &self,
        frame: &ObservationFrame,
        output: &mut [ObserverImpulse2D],
    ) -> Result<usize, OpticalCompileError> {
        let required = self.packet_count();
        if output.len() < required {
            return Err(OpticalCompileError::OutputCapacity {
                required,
                actual: output.len(),
            });
        }
        for (index, body) in self.bodies.iter().enumerate() {
            let material = positive(frame, body.material_energy)?;
            let reservoir = positive(frame, body.reservoir_energy)?;
            let emission = signed(frame, body.emission)?;
            let source_work = signed(frame, body.source_work)?;
            output[index] = ObserverImpulse2D::new(
                body.position,
                body.radius * (1.0 + 0.20 * soft_positive(material, 2_000.0)),
                signed_soft(emission, 24.0),
                soft_positive(material, 4_000.0),
            )
            .with_emission(soft_positive(source_work.abs(), 100_000.0))
            .with_auxiliary(soft_positive(reservoir, 2_000.0))
            .with_compactness(3.0 + 1.5 * body.color[0]);
            tint_packet(&mut output[index], body.color);
        }
        let field_index = self.bodies.len();
        let field_energy = positive(frame, self.field.stored_energy)?;
        let left = signed(frame, self.field.emitted_left)?;
        let right = signed(frame, self.field.emitted_right)?;
        let exchange = signed(frame, self.field.exchange)?;
        output[field_index] = ObserverImpulse2D::new(
            self.field.position,
            self.field.radius * (1.0 + 0.25 * soft_positive(field_energy, 4_000.0)),
            signed_soft(left + right, 24.0),
            soft_positive(field_energy, 8_000.0),
        )
        .with_emission(signed_soft(exchange, 100_000.0).abs())
        .with_auxiliary(signed_soft(right - left, 24.0));
        tint_packet(&mut output[field_index], self.field.color);

        let coupling_offset = field_index + 1;
        for (offset, coupling) in self.couplings.iter().enumerate() {
            let exchange = signed(frame, coupling.exchange)?;
            output[coupling_offset + offset] = ObserverImpulse2D::new(
                coupling.position,
                coupling.radius,
                signed_soft(exchange, 100_000.0),
                signed_soft(exchange, 100_000.0).abs(),
            )
            .with_emission(signed_soft(exchange, 250_000.0).abs())
            .with_compactness(5.0);
            tint_packet(&mut output[coupling_offset + offset], coupling.color);
        }
        Ok(required)
    }
}

fn bind_body(
    body: &BodyOpticalSpec,
    body_positions: &BTreeMap<&str, [f32; 2]>,
    channels: &BTreeMap<&str, usize>,
) -> Result<BoundBody, OpticalCompileError> {
    Ok(BoundBody {
        position: *body_positions
            .get(body.body_id.as_str())
            .ok_or_else(|| OpticalCompileError::UnknownBody(body.body_id.clone()))?,
        radius: body.radius,
        color: body.color,
        material_energy: channel(channels, &body.material_energy_channel)?,
        reservoir_energy: channel(channels, &body.reservoir_energy_channel)?,
        emission: channel(channels, &body.emission_channel)?,
        source_work: channel(channels, &body.source_work_channel)?,
    })
}

fn bind_field(
    field: &SharedFieldOpticalSpec,
    channels: &BTreeMap<&str, usize>,
) -> Result<BoundField, OpticalCompileError> {
    Ok(BoundField {
        position: field.position,
        radius: field.radius,
        color: field.color,
        stored_energy: channel(channels, &field.stored_energy_channel)?,
        emitted_left: channel(channels, &field.emitted_left_channel)?,
        emitted_right: channel(channels, &field.emitted_right_channel)?,
        exchange: channel(channels, &field.exchange_channel)?,
    })
}

fn bind_coupling(
    coupling: &CouplingOpticalSpec,
    world: &WorldPlan,
    body_positions: &BTreeMap<&str, [f32; 2]>,
    channels: &BTreeMap<&str, usize>,
) -> Result<BoundCoupling, OpticalCompileError> {
    let edge = world
        .direct_couplings
        .iter()
        .find(|edge| edge.id == coupling.coupling_id)
        .ok_or_else(|| OpticalCompileError::UnknownCoupling(coupling.coupling_id.clone()))?;
    let left = body_positions
        .get(edge.left_body.as_str())
        .ok_or_else(|| OpticalCompileError::UnknownBody(edge.left_body.clone()))?;
    let right = body_positions
        .get(edge.right_body.as_str())
        .ok_or_else(|| OpticalCompileError::UnknownBody(edge.right_body.clone()))?;
    Ok(BoundCoupling {
        position: [0.5 * (left[0] + right[0]), 0.5 * (left[1] + right[1])],
        radius: coupling.radius,
        color: coupling.color,
        exchange: channel(channels, &coupling.exchange_channel)?,
    })
}

fn channel(
    channels: &BTreeMap<&str, usize>,
    id: &str,
) -> Result<usize, OpticalCompileError> {
    channels
        .get(id)
        .copied()
        .ok_or_else(|| OpticalCompileError::UnknownChannel(id.to_owned()))
}

fn value(frame: &ObservationFrame, index: usize) -> Result<f64, OpticalCompileError> {
    let value = frame
        .values
        .get(index)
        .copied()
        .ok_or(OpticalCompileError::FrameWidth)?
        .as_f64();
    if value.is_finite() {
        Ok(value)
    } else {
        Err(OpticalCompileError::NonFiniteFrame)
    }
}

fn positive(frame: &ObservationFrame, index: usize) -> Result<f64, OpticalCompileError> {
    Ok(value(frame, index)?.max(0.0))
}

fn signed(frame: &ObservationFrame, index: usize) -> Result<f64, OpticalCompileError> {
    value(frame, index)
}

fn soft_positive(value: f64, scale: f64) -> f32 {
    (value.max(0.0) * scale).ln_1p().clamp(0.0, 12.0) as f32
}

fn signed_soft(value: f64, scale: f64) -> f32 {
    (value.signum() * (value.abs() * scale).ln_1p()).clamp(-12.0, 12.0) as f32
}

fn tint_packet(packet: &mut ObserverImpulse2D, color: [f32; 3]) {
    // The packet remains interpretation-neutral at the GPU primitive layer. The
    // bridge uses channel magnitudes to carry a compact color-weighted optical
    // signature in its nonphysical auxiliary channels.
    packet.channels.x *= color[0].max(0.05);
    packet.channels.y *= color[1].max(0.05);
    packet.channels.z *= color[2].max(0.05);
}

#[derive(Debug, thiserror::Error)]
pub enum OpticalCompileError {
    #[error(transparent)]
    Plan(#[from] OpticalPlanError),
    #[error("optical compiler references unknown body {0:?}")]
    UnknownBody(String),
    #[error("optical compiler references unknown coupling {0:?}")]
    UnknownCoupling(String),
    #[error("optical compiler references unknown observation channel {0:?}")]
    UnknownChannel(String),
    #[error("observation frame does not match the bound schema width")]
    FrameWidth,
    #[error("observation frame contains a non-finite value")]
    NonFiniteFrame,
    #[error("optical packet output capacity is {actual}; at least {required} is required")]
    OutputCapacity { required: usize, actual: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{canonical_optical_observation_program, OpticalConstitutionPlan};
    use mus_ariadne::WorldPlan;
    use mus_observation::{ObservationFrame, ObservationValue};

    #[test]
    fn one_plan_compiles_without_instrument_or_page_specific_types() {
        let world = WorldPlan::string_brass_v1(48_000.0);
        let observation = canonical_optical_observation_program(&world, 400).unwrap();
        let schema = observation.schema().unwrap();
        let optical = OpticalConstitutionPlan::canonical(&world, &observation).unwrap();
        let compiler = OpticalPacketCompiler::compile(&optical, &world, &schema).unwrap();
        let frame = ObservationFrame::new(
            400,
            schema
                .channels
                .iter()
                .map(|channel| match channel.scalar_type {
                    mus_observation::ScalarType::F32 => ObservationValue::F32(0.0),
                    mus_observation::ScalarType::F64 => ObservationValue::F64(0.0),
                    mus_observation::ScalarType::U64 => ObservationValue::U64(400),
                })
                .collect(),
        );
        let mut packets = vec![ObserverImpulse2D::zeroed(); compiler.packet_count()];
        assert_eq!(
            compiler.compile_into(&frame, &mut packets).unwrap(),
            compiler.packet_count()
        );
        assert!(packets.iter().all(|packet| {
            packet.position_radius.is_finite() && packet.channels.is_finite()
        }));
    }
}
