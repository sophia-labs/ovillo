//! World-level attachment, controls and observation for bows (`law::bow`).
//!
//! A bow is a hand, not anatomy: it is attached to a built World at a point
//! on one modal string body, the way the engine folds a plucking hand's
//! position into a string. A World with no bow attached runs the exact
//! code path it ran before bows existed; a bow lifted (`force = 0`) steps
//! and books nothing.

use crate::ir::MODE_COUNT;
use crate::law::bow::{bow_shape, BowControls, BowFrame, BowSpec, BowTotals, FrictionBow};
use crate::law::hammer::{apply_force, wire_state};

use super::{World, WorldError};

/// One bow on one modal string: the body and the bow's mass-normalized
/// shapes at the bow point.
#[derive(Debug, Clone)]
pub(crate) struct BowBinding {
    pub body: usize,
    pub shape: [f64; MODE_COUNT],
    pub bow: FrictionBow,
}

impl World {
    /// Rest a bow on modal body `body` at `bow_fraction` of its speaking
    /// length (from the bridge, like a pluck point), lifted. Returns the bow's
    /// index for [`World::set_bow`]. Only a modal string body can be bowed.
    pub fn attach_bow(
        &mut self,
        body: usize,
        bow_fraction: f64,
        spec: BowSpec,
    ) -> Result<usize, WorldError> {
        let id = self
            .body_ids
            .get(body)
            .cloned()
            .ok_or_else(|| WorldError::UnknownBody(format!("index/{body}")))?;
        let error = |message: &str| WorldError::Body {
            id: id.clone(),
            message: message.to_owned(),
        };
        if self.bodies[body].modal_body().is_none() {
            return Err(error("a bow rests only on a modal string body"));
        }
        if !bow_fraction.is_finite() || bow_fraction <= 0.0 || bow_fraction >= 0.5 {
            return Err(error(
                "the bow point must lie inside the near half of the string",
            ));
        }
        spec.validate().map_err(error)?;
        self.bows.push(BowBinding {
            body,
            shape: bow_shape(bow_fraction),
            bow: FrictionBow::new(spec, self.sample_rate_hz),
        });
        Ok(self.bows.len() - 1)
    }

    /// The arm's velocity and normal force on bow `bow` from the next sample on.
    pub fn set_bow(&mut self, bow: usize, controls: BowControls) -> Result<(), WorldError> {
        let binding = self
            .bows
            .get_mut(bow)
            .ok_or_else(|| WorldError::UnknownBody(format!("bow/{bow}")))?;
        binding.bow.set_controls(controls);
        Ok(())
    }

    /// Every resting bow's contact step; returns `(source work, friction loss)`
    /// and credits each to its string's own books.
    pub(super) fn process_bows(&mut self) -> (f64, f64) {
        let sqrt_dt = self.sample_rate_hz.recip().sqrt();
        let mut source_work = 0.0;
        let mut loss = 0.0;
        for binding in &mut self.bows {
            if binding.bow.controls().force <= 0.0 {
                continue;
            }
            let body = self.bodies[binding.body]
                .modal_body_mut()
                .expect("bound to a modal body");
            let wire = wire_state(body.field(), &binding.shape, sqrt_dt);
            let (force, work, heat) = binding.bow.step(wire);
            apply_force(body.field_mut(), &binding.shape, force, sqrt_dt);
            self.body_source_work[binding.body] += work;
            self.body_internal_loss[binding.body] += heat;
            source_work += work;
            loss += heat;
        }
        (source_work, loss)
    }

    #[must_use]
    pub fn bow_count(&self) -> usize {
        self.bows.len()
    }

    /// The body bow `bow` rests on.
    #[must_use]
    pub fn bow_body(&self, bow: usize) -> Option<usize> {
        self.bows.get(bow).map(|b| b.body)
    }

    /// Bow `bow`'s last contact step (non-loading).
    #[must_use]
    pub fn bow_frame(&self, bow: usize) -> Option<BowFrame> {
        self.bows.get(bow).map(|b| b.bow.last_frame())
    }

    /// Bow `bow`'s cumulative energies and counts.
    #[must_use]
    pub fn bow_totals(&self, bow: usize) -> Option<BowTotals> {
        self.bows.get(bow).map(|b| b.bow.totals())
    }

    /// The bow's shape at its point: the string's mass-normalized partials there.
    #[must_use]
    pub fn bow_shape(&self, bow: usize) -> Option<&[f64; MODE_COUNT]> {
        self.bows.get(bow).map(|b| &b.shape)
    }
}
