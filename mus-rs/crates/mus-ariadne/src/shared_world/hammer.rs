//! World-level controls, contact steps and observation for hammers and
//! dampers (`law::hammer`, `law::damper`).

use crate::ir::MODE_COUNT;
use crate::law::hammer::{apply_force, wire_state, HammerFrame, HammerTotals, WireState, MAX_HAMMER_WIRES};
use crate::plan::{BodyConstitutionSpec, ConstitutiveWorldPlan};

use super::{World, WorldError};

/// One hammer body and, per wire it strikes, the wire's body and its
/// mass-normalized shapes averaged over the felt at the strike point.
#[derive(Debug, Clone)]
pub(crate) struct HammerBinding {
    pub body: usize,
    pub wires: Vec<(usize, [f64; MODE_COUNT])>,
}

/// Resolve every hammer's wires to body indices and strike shapes. The plan
/// has already checked that each named wire is a stiff-wire body.
pub(super) fn bind_hammers(plan: &ConstitutiveWorldPlan, world: &World) -> Result<Vec<HammerBinding>, WorldError> {
    let mut bindings = Vec::new();
    for (index, body) in plan.bodies.iter().enumerate() {
        let BodyConstitutionSpec::Hammer { hammer } = &body.constitution else { continue };
        let mut wires = Vec::with_capacity(hammer.wires.len());
        for contact in &hammer.wires {
            let wire_index = world
                .body_index(&contact.body)
                .ok_or_else(|| WorldError::UnknownBody(contact.body.clone()))?;
            let (wire, retained) = world.bodies[wire_index]
                .space_body()
                .and_then(|space| space.wire())
                .ok_or_else(|| WorldError::Body {
                    id: contact.body.clone(),
                    message: "a hammer strikes only stiff-wire bodies".to_owned(),
                })?;
            wires.push((wire_index, wire.mode_shape_at(contact.strike_fraction, hammer.contact_width_m, retained)));
        }
        bindings.push(HammerBinding { body: index, wires });
    }
    Ok(bindings)
}

impl World {
    fn hammer_binding(&self, index: usize) -> Result<usize, WorldError> {
        self.hammers
            .iter()
            .position(|binding| binding.body == index)
            .ok_or_else(|| WorldError::Body {
                id: self.body_ids.get(index).cloned().unwrap_or_else(|| format!("index/{index}")),
                message: "body is not a hammer".to_owned(),
            })
    }

    /// Throw hammer body `index` at `velocity_m_s` (the action's escapement
    /// velocity) from its letoff below the nearest wire. Its kinetic energy
    /// enters the next ledger frame as source work; a hammer still flying back
    /// is caught first (its kinetic energy booked as loss). Returns `false`,
    /// changing nothing, while the felt still presses on a wire.
    pub fn strike_hammer(&mut self, index: usize, velocity_m_s: f64) -> Result<bool, WorldError> {
        let binding = self.hammer_binding(index)?;
        let sqrt_dt = self.sample_rate_hz.recip().sqrt();
        let mut displacement = [0.0; MAX_HAMMER_WIRES];
        for (slot, (wire, shape)) in self.hammers[binding].wires.iter().enumerate() {
            let field = self.bodies[*wire].space_body().expect("bound to a wire").field();
            displacement[slot] = sqrt_dt * field.displacement().iter().zip(shape).map(|(q, c)| q * c).sum::<f64>();
        }
        let energy_before = self.total_stored_energy();
        let hammer = self.bodies[index].hammer_mut().expect("bound to a hammer");
        let Some((work, caught)) = hammer.strike(velocity_m_s, &displacement) else {
            return Ok(false);
        };
        if self.pending_energy_start.is_none() {
            self.pending_energy_start = Some(energy_before);
        }
        self.pending_source_work += work;
        self.pending_contact_loss += caught;
        Ok(true)
    }

    /// Every thrown hammer's contact step; returns the felt and backcheck loss.
    pub(super) fn process_hammers(&mut self) -> f64 {
        let sqrt_dt = self.sample_rate_hz.recip().sqrt();
        let mut loss = 0.0;
        for binding in &self.hammers {
            if !self.bodies[binding.body].hammer().is_some_and(|h| h.is_active()) {
                continue;
            }
            let mut states = [WireState::default(); MAX_HAMMER_WIRES];
            for (slot, (wire, shape)) in binding.wires.iter().enumerate() {
                let field = self.bodies[*wire].space_body().expect("bound to a wire").field();
                states[slot] = wire_state(field, shape, sqrt_dt);
            }
            let mut forces = [0.0; MAX_HAMMER_WIRES];
            let hammer = self.bodies[binding.body].hammer_mut().expect("bound to a hammer");
            loss += hammer.step(&states[..binding.wires.len()], &mut forces);
            loss += hammer.catch_if_rebounded();
            for (slot, (wire, shape)) in binding.wires.iter().enumerate() {
                let field = self.bodies[*wire].space_body_mut().expect("bound to a wire").field_mut();
                apply_force(field, shape, forces[slot], sqrt_dt);
            }
        }
        loss
    }

    /// Rest the damper of wire body `index` on its wire (`1`), lift it (`0`),
    /// or anything between. A dashpot only dissipates: nothing is billed.
    pub fn set_body_damper(&mut self, index: usize, engagement: f64) -> Result<(), WorldError> {
        let id = self
            .body_ids
            .get(index)
            .cloned()
            .ok_or_else(|| WorldError::UnknownBody(format!("index/{index}")))?;
        let body = self.bodies[index].space_body_mut().ok_or_else(|| WorldError::Body {
            id: id.clone(),
            message: "body has no damper".to_owned(),
        })?;
        body.set_damper_engagement(engagement)
            .map_err(|error| WorldError::Body { id, message: error.to_string() })
    }

    /// How fully wire body `index`'s damper rests (`None` without one).
    #[must_use]
    pub fn body_damper(&self, index: usize) -> Option<f64> {
        self.bodies.get(index)?.space_body()?.damper_engagement()
    }

    /// The hammer body `index`'s last contact step (non-loading).
    #[must_use]
    pub fn hammer_frame(&self, index: usize) -> Option<HammerFrame> {
        self.bodies.get(index)?.hammer().map(|h| h.last_frame())
    }

    /// The hammer body `index`'s cumulative energies and counts.
    #[must_use]
    pub fn hammer_totals(&self, index: usize) -> Option<HammerTotals> {
        self.bodies.get(index)?.hammer().map(|h| h.totals())
    }

    /// True while hammer body `index`'s felt presses on a wire.
    #[must_use]
    pub fn hammer_in_contact(&self, index: usize) -> Option<bool> {
        self.bodies.get(index)?.hammer().map(|h| h.in_contact())
    }

    /// True while hammer body `index` is thrown (not resting on its backcheck).
    #[must_use]
    pub fn hammer_active(&self, index: usize) -> Option<bool> {
        self.bodies.get(index)?.hammer().map(|h| h.is_active())
    }
}
