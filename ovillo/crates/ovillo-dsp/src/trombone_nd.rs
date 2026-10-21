//! Trombone v1: one N-D Ariadne body (see `mus_analysis/trombone_nd_anatomy.py`).
//!
//! The World holds one `modal_space` body: a `tube_network` map of the
//! declared bore (mouthpiece, leadpipe, slide, bell; the bell mouth radiates),
//! whose 24 modes the runtime computes at every slide extension, and the
//! runtime's flow valve as the lips, paid from the lungs. A player touches it
//! three ways: mouth pressure (the valve's supply; the lungs' refill is the
//! source work), lip tension (the valve's tuning, billed as control work by
//! `World::set_body_valve_tuning`), and the slide (the body's one coordinate,
//! moved by `World::set_body_configuration_point` under the mass-momentum
//! connection, billed as control work). What `tick` returns is the World's
//! own emission: the bell volume's pressure (Pa), times [`OUTPUT_SCALE`].
//! The slide is the network's `tube_length` area term.

use mus_ariadne::{
    BodyControls, ConstitutiveBodyControls, ConstitutiveWorldPlan, FlowValveControls, SpaceBody, SpaceBodyControls,
    World, WorldControls, WorldError,
};

/// The compiled plan.
pub const PLAN_JSON: &str = include_str!("../../../../profiles/ariadne-trombone-v1.plan.json");
/// Pascal at the bell volume to full scale: a fixed property of the listening
/// program, set once (so that the solo peaks near -3 dBFS), never per render.
pub const OUTPUT_SCALE: f64 = 1.0 / 300.0;

/// What the player does this sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Breath {
    /// Mouth pressure as a fraction of the lips' declared maximum (0..1).
    pub mouth: f64,
    /// Lip tension: the valve's tuning (Hz).
    pub lip_hz: f64,
    /// Embouchure firmness: the valve's adduction (0 open .. 1 pressed).
    pub embouchure: f64,
    /// Each slide leg's extension (m): 0 is first position.
    pub slide_m: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize)]
pub struct Moves {
    pub slide_moves: u64,
    pub slide_work: f64,
    pub slide_disposal: f64,
    pub lip_retunes: u64,
    pub lip_work: f64,
}

pub struct TromboneNd {
    world: World,
    controls: [ConstitutiveBodyControls; 1],
    lung_refill_w: f64,
    /// Samples between slide moves (each recomputes the modes).
    pub slide_interval: usize,
    phase: usize,
    slide: f64,
    lip: (f64, f64),
    moves: Moves,
}

impl TromboneNd {
    pub fn new() -> Result<Self, String> {
        Self::from_json(PLAN_JSON)
    }

    pub fn from_json(plan_json: &str) -> Result<Self, String> {
        let plan: ConstitutiveWorldPlan = serde_json::from_str(plan_json).map_err(|e| e.to_string())?;
        let lung_refill_w = 3.0;
        let world = World::from_constitutive_plan(plan).map_err(|e| e.to_string())?;
        let slide = world.body_configuration_point(0).map_or(0.0, |p| p[0]);
        Ok(Self {
            controls: [ConstitutiveBodyControls::ModalSpace(SpaceBodyControls {
                body: BodyControls { work_input: lung_refill_w, ..Default::default() },
                valve: FlowValveControls { supply: 0.0 },
                radiation_scale: 1.0,
                // no knock on the wall: the lips are the only source
                node_source: None,
            })],
            world,
            lung_refill_w,
            slide_interval: 16,
            phase: 0,
            slide,
            lip: (f64::NAN, f64::NAN),
            moves: Moves::default(),
        })
    }

    /// The World as the instrument built it (the replay test builds another).
    pub fn world_from_json(plan_json: &str) -> Result<World, String> {
        let plan: ConstitutiveWorldPlan = serde_json::from_str(plan_json).map_err(|e| e.to_string())?;
        World::from_constitutive_plan(plan).map_err(|e| e.to_string())
    }

    #[must_use]
    pub fn world(&self) -> &World {
        &self.world
    }

    #[must_use]
    pub fn moves(&self) -> Moves {
        self.moves
    }

    #[must_use]
    pub fn controls(&self) -> ConstitutiveBodyControls {
        self.controls[0].clone()
    }

    #[must_use]
    pub fn slide(&self) -> f64 {
        self.slide
    }

    /// Current computed bore frequencies (Hz), from the World's observation.
    #[must_use]
    pub fn frequencies_hz(&self) -> Vec<f64> {
        self.world.body_space_observation(0).map(|o| o.frequency_hz.to_vec()).unwrap_or_default()
    }

    /// Move the slide now (outside the sample schedule).
    pub fn set_slide_now(&mut self, slide_m: f64) -> Result<(), WorldError> {
        let f = self.world.set_body_configuration_point(0, &[slide_m])?;
        self.moves.slide_moves += u64::from(f.modes_recomputed);
        self.moves.slide_work += f.control_work;
        self.moves.slide_disposal += f.disposal;
        self.slide = self.world.body_configuration_point(0).map_or(slide_m, |p| p[0]);
        Ok(())
    }

    /// Advance one sample; returns the World's emission scaled to full scale.
    pub fn tick(&mut self, b: Breath) -> Result<f64, WorldError> {
        if self.phase == 0 && (b.slide_m - self.slide).abs() > 1.0e-5 {
            self.set_slide_now(b.slide_m)?;
        }
        self.phase = (self.phase + 1) % self.slide_interval.max(1);
        if b.lip_hz.to_bits() != self.lip.0.to_bits() || b.embouchure.to_bits() != self.lip.1.to_bits() {
            let w = self.world.set_body_valve_tuning(0, b.lip_hz, b.embouchure)?;
            self.moves.lip_retunes += 1;
            self.moves.lip_work += w;
            self.lip = (b.lip_hz, b.embouchure);
        }
        if let ConstitutiveBodyControls::ModalSpace(c) = &mut self.controls[0] {
            c.valve.supply = b.mouth;
            c.body.work_input = self.lung_refill_w;
        }
        self.world.process_constitutive_sample(&self.controls, WorldControls::default())?;
        Ok(self.emission() * OUTPUT_SCALE)
    }

    /// The bell volume's pressure (Pa): the World's non-loading emission.
    #[must_use]
    pub fn emission(&self) -> f64 {
        self.world.body_radiation(0).map_or(0.0, |e| e.iter().sum())
    }

    /// Where the World's energy is now, read off the body (reading it changes nothing).
    #[must_use]
    pub fn stored_energy(&self) -> StoredEnergy {
        let body = self.world.space_body(0);
        let air = body.map_or(0.0, SpaceBody::field_energy);
        let material = body.map_or(0.0, SpaceBody::material_stored_energy);
        StoredEnergy { air, lips: material - air, lungs: body.map_or(0.0, SpaceBody::reservoir_energy) }
    }
}

/// The one body's stored energy in its three parts, in the World's own units
/// (the receipt's joules). `air + lips` is the body's material energy;
/// `air + lips + lungs` is everything the World stores (it has no room).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StoredEnergy {
    /// The bore's air: the body's field (its 24 network modes).
    pub air: f64,
    /// The lips: the flow valve's mass and spring (material energy less the field).
    pub lips: f64,
    /// The lungs: the body's reservoir, which the source work refills and the valve draws on.
    pub lungs: f64,
}

/// Hop of the energy trace, as mus-engine's steel-string trace.
pub const ENERGY_HOP_SECONDS: f64 = 0.01;

/// The energy trace a render writes beside its audio, in mus-engine's
/// `mus.ariadne.energy-trace.v1` shape: stored energy at the first sample of
/// every hop (as mus-engine takes it), `air` and `lips` as the channels and the
/// `lungs` apart (they are the source's store, not the instrument's).
#[derive(Debug, Clone, Default)]
pub struct EnergyTrace {
    hop: usize,
    seen: usize,
    frames: Vec<[f32; 2]>,
    lungs: Vec<f32>,
}

impl EnergyTrace {
    #[must_use]
    pub fn new(sample_rate: f64) -> Self {
        Self { hop: (ENERGY_HOP_SECONDS * sample_rate).round().max(1.0) as usize, ..Self::default() }
    }

    /// Call once after every `tick`.
    pub fn observe(&mut self, tw: &TromboneNd) {
        if self.seen % self.hop == 0 {
            let e = tw.stored_energy();
            self.frames.push([e.air as f32, e.lips as f32]);
            self.lungs.push(e.lungs as f32);
        }
        self.seen += 1;
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    #[must_use]
    pub fn frames(&self) -> &[[f32; 2]] {
        &self.frames
    }

    #[must_use]
    pub fn lungs(&self) -> &[f32] {
        &self.lungs
    }

    /// The trace as JSON: `instrument` is the World's body id, `touches` the note list's starts.
    #[must_use]
    pub fn to_json(&self, instrument: &str, touches: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "schema": "mus.ariadne.energy-trace.v1",
            "worlds": [{
                "instrument": instrument,
                "hopSeconds": ENERGY_HOP_SECONDS,
                "startSeconds": 0.0,
                "channels": ["air", "lips"],
                "frames": self.frames,
                "reservoirs": ["lungs"],
                "reservoirFrames": self.lungs.iter().map(|v| [*v]).collect::<Vec<_>>(),
                "parts": {
                    "air": "the bore's air: the body's field energy (its network modes)",
                    "lips": "the lip valve's mass and spring: the body's material energy less its field",
                    "lungs": "the body's reservoir: the source work refills it, the valve draws on it",
                },
                "touches": touches,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    fn phrase(sr: f64) -> Vec<Breath> {
        // a tongued note, a lip move, a slide move under it, a release
        (0..(0.8 * sr) as usize)
            .map(|i| {
                let t = i as f64 / sr;
                let mouth = 0.4 * (1.0 - (-t / 0.006).exp()) * if t > 0.7 { 0.0 } else { 1.0 };
                let lip_hz = if t < 0.35 { 240.0 } else { 300.0 };
                let slide_m = if t < 0.5 { 0.0 } else { (0.18 * (t - 0.5) / 0.1).min(0.18) };
                Breath { mouth, lip_hz, embouchure: 0.1, slide_m }
            })
            .collect()
    }

    /// T1: the audio is exactly the World's emission times OUTPUT_SCALE. A second
    /// World from the same plan, given the same moves and controls, emits the
    /// same samples bit for bit.
    #[test]
    fn audio_is_exactly_the_world_emission() {
        let mut tw = TromboneNd::new().unwrap();
        let mut world = TromboneNd::world_from_json(PLAN_JSON).unwrap();
        let mut sounded = 0;
        for b in phrase(48_000.0) {
            let slide_before = tw.slide();
            let audio = tw.tick(b).unwrap();
            if tw.slide().to_bits() != slide_before.to_bits() {
                world.set_body_configuration_point(0, &[tw.slide()]).unwrap();
            }
            world.set_body_valve_tuning(0, b.lip_hz, b.embouchure).unwrap();
            world.process_constitutive_sample(&[tw.controls()], WorldControls::default()).unwrap();
            let heard: f64 = world.body_radiation(0).map_or(0.0, |e| e.iter().sum());
            assert_eq!(audio.to_bits(), (heard * OUTPUT_SCALE).to_bits());
            sounded += usize::from(audio.abs() > 1e-4);
        }
        assert!(sounded > 10_000, "the phrase should sound ({sounded})");
    }

    /// T1: nothing between the World and the samples synthesizes or filters.
    #[test]
    fn audio_path_names_no_oscillator_filter_or_noise() {
        let src = include_str!("trombone_nd.rs");
        let body = src.split("#[cfg(test)]").next().unwrap();
        for word in ["Biquad", "biquad", "highpass", "lowpass", "bandpass", "filter", "noise", "phase +=", ".sin()", ".cos()", "buzz", "additive"] {
            assert!(!body.contains(word), "audio path names {word:?}");
        }
    }

    /// T2: the books close at roundoff (<= 1e-12 per frame, 1e-15 per frame summed).
    #[test]
    fn the_ledger_closes() {
        let mut tw = TromboneNd::new().unwrap();
        for b in phrase(48_000.0) {
            tw.tick(b).unwrap();
        }
        let r = tw.world().receipt();
        assert!(r.source_work > 0.0 && r.radiation > 0.0);
        assert!(r.maximum_frame_residual <= 1e-12, "{r:?}");
        assert!(r.cumulative_closure_residual.abs() <= 1e-15 * r.sample_frames as f64, "{r:?}");
        assert!(tw.moves().slide_moves > 0 && tw.moves().lip_retunes > 0);
    }

    /// The energy trace is the World's: at every sample air + lips is the body's
    /// material energy and air + lips + lungs is all the World stores (nothing left
    /// out); one frame per 10 ms hop; and reading it does not change one bit of the sound.
    #[test]
    fn the_energy_trace_is_the_worlds_stored_energy() {
        let sr = 48_000.0;
        let notes = phrase(sr);
        let mut heard = TromboneNd::new().unwrap();
        let mut traced = TromboneNd::new().unwrap();
        let mut trace = EnergyTrace::new(sr);
        let (mut most_air, mut most_lips) = (0.0f64, 0.0f64);
        for b in &notes {
            let a = heard.tick(*b).unwrap();
            let t = traced.tick(*b).unwrap();
            trace.observe(&traced);
            assert_eq!(a.to_bits(), t.to_bits(), "reading the energy changed the sound");
            let e = traced.stored_energy();
            let material = traced.world().body_material_energy(0).unwrap();
            let total = traced.world().total_stored_energy();
            assert!((e.air + e.lips - material).abs() <= 1e-12 * (1.0 + material), "{e:?} vs material {material}");
            assert!((e.air + e.lips + e.lungs - total).abs() <= 1e-12 * (1.0 + total), "{e:?} vs total {total}");
            assert!(e.air >= 0.0 && e.lips >= -1e-15 && e.lungs >= 0.0, "{e:?}");
            most_air = most_air.max(e.air);
            most_lips = most_lips.max(e.lips);
        }
        assert_eq!(trace.len(), notes.len().div_ceil(480));
        assert!(most_air > 1e-6 && most_lips > 0.0, "the phrase stores energy (air {most_air}, lips {most_lips})");
        // the frames are the samples at k = 0, 480, 960, ...: replay and compare
        let mut again = TromboneNd::new().unwrap();
        for (k, b) in notes.iter().enumerate() {
            again.tick(*b).unwrap();
            if k % 480 == 0 {
                let e = again.stored_energy();
                assert_eq!(trace.frames()[k / 480], [e.air as f32, e.lips as f32]);
                assert_eq!(trace.lungs()[k / 480], e.lungs as f32);
            }
        }
        let j = trace.to_json("trombone", vec![serde_json::json!({ "seconds": 0.0, "kind": "tongue" })]);
        let w = &j["worlds"][0];
        assert_eq!(j["schema"], "mus.ariadne.energy-trace.v1");
        assert_eq!(w["frames"].as_array().unwrap().len(), trace.len());
        assert_eq!(w["reservoirFrames"].as_array().unwrap().len(), trace.len());
        assert_eq!(w["channels"], serde_json::json!(["air", "lips"]));
        assert_eq!(w["hopSeconds"], 0.01);
    }
}
