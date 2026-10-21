//! A trombone that is one Ariadne World, and nothing else.
//!
//! The plan is compiled from declared geometry by
//! `mus_analysis/trombone_anatomy.py` into
//! `profiles/ariadne-trombone-v0.plan.json` (its derivation receipt sits
//! beside it). It has two bodies and one edge:
//!
//! - `lips`: the compliant aperture is the player's lips, a valve driven by
//!   mouth pressure (the source wave). Its opening modulates the flow it lets
//!   into the cup mode, and its motion sweeps volume into it. Its
//!   configuration path is lip tension: the aperture's natural frequency from
//!   slack (amount 0) to taut (amount 1).
//! - `bore`: 24 computed input-impedance resonances of a trombone-shaped bore.
//!   Its configuration path is the slide, first position (amount 0) to
//!   seventh (amount 1), position `k` at `(k-1)/6`. Wall loss is each mode's
//!   T60; the bell's radiation is the body's radiation direction and fraction.
//! - `throat`: a reciprocal Givens exchange between the cup's momentum and the
//!   bore's momentum projected on its input.
//!
//! A player touches it in three ways only: mouth pressure (source work),
//! lip tension and slide (configuration transport, each with a work
//! receipt). What [`TromboneWorld::tick`] returns is the World's stereo
//! listener observation of the bodies' radiation, unscaled and unprocessed.
//! The listener's own `output_scale` is the only gain, and it is part of the
//! listener spec's digest.

use mus_ariadne::law::active::ActiveInclusionControls;
use mus_ariadne::law::transport::ResonantTransportControls;
use mus_ariadne::{
    BodyControls, StereoFrame, StereoListener, StereoListenerSpec, World, WorldControls, WorldError, WorldPlan,
    MODE_COUNT,
};
use serde::Deserialize;

/// The compiled plan (data, content-addressed by the World).
pub const PLAN_JSON: &str = include_str!("../../../../profiles/ariadne-trombone-v0.plan.json");
/// Its derivation receipt (radiation fractions, slide extensions, evidence labels).
pub const RECEIPT_JSON: &str = include_str!("../../../../profiles/ariadne-trombone-v0.receipt.json");
/// The listener's output scale: a fixed property of the listening program
/// (chosen once, so that the solo peaks near -3 dBFS), never adjusted per render.
pub const LISTENER_SCALE: f64 = 0.5;

/// What the player does this sample.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Breath {
    /// Mouth pressure: the lips' source wave (World units).
    pub pressure: f64,
    /// Lip tension as the lips' natural frequency in Hz.
    pub lip_hz: f64,
    /// Slide amount: 0 first position .. 1 seventh, position k at (k-1)/6.
    pub slide: f64,
}

/// The World's books for a render, as the receipt reports them.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize)]
pub struct Ledger {
    pub sample_frames: u64,
    pub initial_energy: f64,
    pub final_energy: f64,
    /// Work the mouth put in through the lips (net of what the lips returned).
    pub source_work: f64,
    /// All configuration work (lip tension + slide).
    pub control_work: f64,
    pub lip_control_work: f64,
    pub slide_control_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub cumulative_closure_residual: f64,
    pub global_closure_residual: f64,
    pub maximum_frame_residual: f64,
    pub maximum_local_residual: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct RadiationFraction {
    first: f64,
    seventh: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct Receipt {
    radiation_fraction: RadiationFraction,
}

/// The instrument: one World, its listener, and the bookkeeping of where
/// configuration work went.
pub struct TromboneWorld {
    world: World,
    listener: StereoListener,
    lips: usize,
    bore: usize,
    controls: [BodyControls; 2],
    radiation_first: f64,
    radiation_seventh: f64,
    lip_slack_hz: f64,
    lip_taut_hz: f64,
    bore_first_hz: [f64; MODE_COUNT],
    bore_seventh_hz: [f64; MODE_COUNT],
    lip_control_work: f64,
    slide_control_work: f64,
    lip_amount: f64,
    slide_amount: f64,
}

/// The two bodies' controls and configuration amounts applied in one tick:
/// enough to replay the World exactly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Applied {
    pub lip_amount: f64,
    pub slide_amount: f64,
    pub controls: [BodyControls; 2],
}

fn quiet(radiation_fraction: f64, source_wave: f64) -> BodyControls {
    BodyControls {
        source_wave,
        work_input: 0.0,
        active_coupling: 0.0,
        active: ActiveInclusionControls { drive: 0.0, ..Default::default() },
        transport: ResonantTransportControls { interlace: 0.0, memory: 0.0 },
        radiation_fraction,
    }
}

impl TromboneWorld {
    /// The embedded plan.
    pub fn new() -> Result<Self, String> {
        Self::from_json(PLAN_JSON, RECEIPT_JSON)
    }

    pub fn from_json(plan_json: &str, receipt_json: &str) -> Result<Self, String> {
        let plan: WorldPlan = serde_json::from_str(plan_json).map_err(|e| format!("plan: {e}"))?;
        let receipt: Receipt = serde_json::from_str(receipt_json).map_err(|e| format!("receipt: {e}"))?;
        let (world, listener) = Self::world_and_listener(&plan)?;
        let lips = world.body_index("lips").ok_or("plan has no lips body")?;
        let bore = world.body_index("bore").ok_or("plan has no bore body")?;
        let lip_path = &plan.bodies[lips].body.configuration_path;
        let bore_path = &plan.bodies[bore].body.configuration_path;
        let mut tw = Self {
            lips,
            bore,
            controls: [quiet(0.0, 0.0); 2],
            radiation_first: receipt.radiation_fraction.first,
            radiation_seventh: receipt.radiation_fraction.seventh,
            lip_slack_hz: lip_path.left.aperture.natural_frequency_hz,
            lip_taut_hz: lip_path.right.aperture.natural_frequency_hz,
            bore_first_hz: bore_path.left.metric.frequency_hz,
            bore_seventh_hz: bore_path.right.metric.frequency_hz,
            lip_control_work: 0.0,
            slide_control_work: 0.0,
            lip_amount: lip_path.initial_amount,
            slide_amount: bore_path.initial_amount,
            world,
            listener,
        };
        tw.controls[tw.bore] = quiet(tw.radiation_at(tw.slide_amount), 0.0);
        Ok(tw)
    }

    /// The World and its listener exactly as the instrument builds them (the
    /// replay test builds a second pair through this same door).
    pub fn world_and_listener(plan: &WorldPlan) -> Result<(World, StereoListener), String> {
        let spec = StereoListenerSpec::all_bodies(plan, LISTENER_SCALE).map_err(|e| e.to_string())?;
        let listener = StereoListener::bind(&spec, plan).map_err(|e| e.to_string())?;
        let world = World::from_plan(plan.clone()).map_err(|e| e.to_string())?;
        Ok((world, listener))
    }

    /// The bore's radiation fraction where the slide is: the receipt's
    /// first- and seventh-position values, linear in slide amount.
    #[must_use]
    pub fn radiation_at(&self, slide: f64) -> f64 {
        let s = slide.clamp(0.0, 1.0);
        self.radiation_first + s * (self.radiation_seventh - self.radiation_first)
    }

    /// The lip-tension amount for a natural frequency (the path is linear in Hz).
    #[must_use]
    pub fn lip_amount_for(&self, hz: f64) -> f64 {
        ((hz - self.lip_slack_hz) / (self.lip_taut_hz - self.lip_slack_hz)).clamp(0.0, 1.0)
    }

    /// Where the slide puts bore resonance `n` (1-based): the World's own
    /// log-frequency path between first and seventh position.
    #[must_use]
    pub fn bore_resonance_hz(&self, n: usize, slide: f64) -> f64 {
        let i = n.clamp(1, MODE_COUNT) - 1;
        let s = slide.clamp(0.0, 1.0);
        (self.bore_first_hz[i].ln() + s * (self.bore_seventh_hz[i].ln() - self.bore_first_hz[i].ln())).exp()
    }

    /// The slide amount that puts bore resonance `n` on `hz` (may leave 0..1).
    #[must_use]
    pub fn slide_for(&self, n: usize, hz: f64) -> f64 {
        let i = n.clamp(1, MODE_COUNT) - 1;
        (hz / self.bore_first_hz[i]).ln() / (self.bore_seventh_hz[i] / self.bore_first_hz[i]).ln()
    }

    /// Advance the World one sample and return its listener's observation.
    pub fn tick(&mut self, breath: Breath) -> Result<StereoFrame, WorldError> {
        let lip = self.lip_amount_for(breath.lip_hz);
        if lip.to_bits() != self.lip_amount.to_bits() {
            let f = self.world.set_body_configuration_index(self.lips, lip)?;
            self.lip_control_work += f.total_control_work;
            self.lip_amount = lip;
        }
        let slide = breath.slide.clamp(0.0, 1.0);
        if slide.to_bits() != self.slide_amount.to_bits() {
            let f = self.world.set_body_configuration_index(self.bore, slide)?;
            self.slide_control_work += f.total_control_work;
            self.slide_amount = slide;
        }
        self.controls[self.lips] = quiet(0.0, breath.pressure);
        self.controls[self.bore] = quiet(self.radiation_at(slide), 0.0);
        let frame = self.world.process_sample(&self.controls, WorldControls::default())?;
        self.listener.observe(&self.world, &frame).map_err(|e| WorldError::Plan(e.to_string()))
    }

    /// What the last tick applied to the World.
    #[must_use]
    pub fn applied(&self) -> Applied {
        Applied { lip_amount: self.lip_amount, slide_amount: self.slide_amount, controls: self.controls }
    }

    #[must_use]
    pub fn world(&self) -> &World {
        &self.world
    }

    #[must_use]
    pub fn listener_digest(&self) -> &str {
        self.listener.listener_digest()
    }

    #[must_use]
    pub fn lips_index(&self) -> usize {
        self.lips
    }

    #[must_use]
    pub fn bore_index(&self) -> usize {
        self.bore
    }

    #[must_use]
    pub fn ledger(&self) -> Ledger {
        let r = self.world.receipt();
        Ledger {
            sample_frames: r.sample_frames,
            initial_energy: r.initial_energy,
            final_energy: r.current_energy,
            source_work: r.source_work,
            control_work: r.control_work,
            lip_control_work: self.lip_control_work,
            slide_control_work: self.slide_control_work,
            internal_loss: r.internal_loss,
            radiation: r.radiation,
            cumulative_closure_residual: r.cumulative_closure_residual,
            global_closure_residual: r.global_closure_residual,
            maximum_frame_residual: r.maximum_frame_residual,
            maximum_local_residual: r.maximum_local_residual,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phrase(sr: f64) -> Vec<Breath> {
        // a tongued note, a lip slur, a slide smear, a release
        let n = (0.9 * sr) as usize;
        (0..n)
            .map(|i| {
                let t = i as f64 / sr;
                let pressure = 2500.0 * (t / 0.01).min(1.0) * if t > 0.8 { 0.0 } else { 1.0 };
                let lip_hz = if t < 0.4 { 200.0 } else { 280.0 };
                let slide = if t < 0.6 { 0.0 } else { ((t - 0.6) / 0.15).min(1.0) * 0.5 };
                Breath { pressure, lip_hz, slide }
            })
            .collect()
    }

    /// T1: the audio is exactly the World listener's output. A second World
    /// and listener, built from the same plan and driven with the same body
    /// controls and configuration amounts, observe the same samples bit for bit.
    #[test]
    fn audio_is_exactly_the_world_listener_output() {
        let mut tw = TromboneWorld::new().unwrap();
        let plan: WorldPlan = serde_json::from_str(PLAN_JSON).unwrap();
        let (mut world, listener) = TromboneWorld::world_and_listener(&plan).unwrap();
        assert_eq!(world.plan_digest(), tw.world().plan_digest());
        let sr = plan.internal_sample_rate_hz;
        let mut nonzero = 0usize;
        for b in phrase(sr) {
            let audio = tw.tick(b).unwrap();
            let a = tw.applied();
            world.set_body_configuration_index(tw.lips_index(), a.lip_amount).unwrap();
            world.set_body_configuration_index(tw.bore_index(), a.slide_amount).unwrap();
            let frame = world.process_sample(&a.controls, WorldControls::default()).unwrap();
            let heard = listener.observe(&world, &frame).unwrap();
            assert_eq!(audio.left.to_bits(), heard.left.to_bits());
            assert_eq!(audio.right.to_bits(), heard.right.to_bits());
            if audio.left != 0.0 {
                nonzero += 1;
            }
        }
        assert!(nonzero > 1000, "the phrase should sound");
    }

    /// T1: the audio path holds no synthesis or filtering. This module, the
    /// only code between the World and the samples, names none of them.
    #[test]
    fn audio_path_names_no_oscillator_filter_or_noise() {
        let src = include_str!("trombone_world.rs");
        let body = src.split("#[cfg(test)]").next().unwrap();
        for word in ["Biquad", "biquad", "highpass", "lowpass", "bandpass", "filter", "noise", "phase", ".sin()", ".cos()", "buzz", "additive", "mix"] {
            assert!(!body.contains(word), "audio path names {word:?}");
        }
    }

    /// T2 in miniature: the phrase's books close at roundoff (bound stated below).
    #[test]
    fn the_ledger_closes() {
        let mut tw = TromboneWorld::new().unwrap();
        for b in phrase(48_000.0) {
            tw.tick(b).unwrap();
        }
        let l = tw.ledger();
        assert!(l.source_work > 0.0 && l.radiation > 0.0);
        // roundoff: each frame's books are O(1)-O(10) energies in f64, so a frame
        // closes to ~1e-15; the bound is 1e-12 per frame and 1e-15 per frame summed
        assert!(l.maximum_frame_residual <= 1e-12, "{l:?}");
        assert!(l.cumulative_closure_residual.abs() <= 1e-15 * l.sample_frames as f64, "{l:?}");
        assert!((l.control_work - l.lip_control_work - l.slide_control_work).abs() <= 1e-12 * l.control_work.abs().max(1e-9));
    }
}
