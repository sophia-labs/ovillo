//! The C ABI of `ovillo-tract-wasm`, over the World tract: the same export
//! names, arguments, return values and state layout, so `site/tract/`'s
//! `tract.js` and `worklet.js` (and anything built from them) swap engines by
//! swapping the `.wasm`. One World per module instance; everything that
//! allocates happens in `tract_init*`.
//!
//! What fills the state vector is the World's own: its ledger (`receipt()`),
//! the tract body's cumulative totals, and non-loading observers. Fields the
//! World has no counterpart for are documented at each line.

use std::sync::Mutex;

use mus_ariadne::{ConnectionPolicy, DisposalPolicy};

use crate::gestures::Gesture;
use crate::world::{self as tw, Constriction, TractControls, TractWorldNd, Vibrato, CYCLE_POINTS, MAX_SLOTS};
use crate::TractNdAnatomy;

/// Largest render request per call.
pub const MAX_FRAMES: usize = 4096;
/// Layout capacities, as `ovillo-tract`'s.
pub const MAX_ORAL_SECTIONS: usize = 64;
pub const MAX_NASAL_SECTIONS: usize = 48;

/// The full schedule: a move every 32 samples, skipped below 0.001.
pub const FULL_SCHEDULE: (usize, f64) = (32, 1.0e-3);
/// The light schedule for slow devices (`tract_init_decimated(_, 2)`): the
/// same World at the full rate, moved four times less often and skipping
/// moves below 0.004. Its spectral cost is measured in `examples/zipper.rs`
/// and stated in `TRACT-WORLD.md`.
pub const LIGHT_SCHEDULE: (usize, f64) = (128, 4.0e-3);

/// Layout of the state vector (f32): identical indices to
/// `ovillo-tract-wasm::layout`.
pub mod layout {
    pub const ORAL_N: usize = 0;
    pub const NASAL_N: usize = 1;
    pub const VELUM_SECTION: usize = 2;
    pub const FOLD_GAP_MM: usize = 3;
    pub const FOLD_GAP_MIN_MM: usize = 4;
    pub const FOLD_GAP_MAX_MM: usize = 5;
    pub const GLOTTAL_FLOW_CM3_S: usize = 6;
    pub const LUNG_PRESSURE_PA: usize = 7;
    pub const LUNG_FRACTION: usize = 8;
    pub const F0_HZ: usize = 9;
    pub const TRACT_ENERGY_J: usize = 10;
    pub const FOLD_ENERGY_J: usize = 11;
    pub const LUNG_ENERGY_J: usize = 12;
    pub const MUSCLE_WORK_J: usize = 13;
    pub const IMPACT_WORK_J: usize = 14;
    pub const TRACT_CONTROL_WORK_J: usize = 15;
    pub const FOLD_CONTROL_WORK_J: usize = 16;
    pub const WALL_LOSS_J: usize = 17;
    pub const JET_LOSS_J: usize = 18;
    pub const FOLD_LOSS_J: usize = 19;
    pub const RADIATION_LIPS_J: usize = 20;
    pub const RADIATION_NOSE_J: usize = 21;
    pub const LEDGER_FRAMES: usize = 22;
    pub const MAX_FRAME_RESIDUAL_J: usize = 23;
    pub const GLOBAL_RESIDUAL_J: usize = 24;
    pub const CUMULATIVE_RESIDUAL_J: usize = 25;
    pub const CUR_TONGUE_INDEX: usize = 26;
    pub const CUR_TONGUE_DIAMETER: usize = 27;
    pub const CUR_LIPS: usize = 28;
    pub const CUR_VELUM: usize = 29;
    pub const CUR_PITCH_HZ: usize = 30;
    pub const CUR_TENSENESS: usize = 31;
    pub const CUR_BREATH: usize = 32;
    pub const GESTURE_ACTIVE: usize = 33;
    pub const SAFETY_LIMITED: usize = 34;
    pub const NON_FINITE: usize = 35;
    pub const IMPACT_COUNT: usize = 36;
    pub const TURBULENCE_WORK_J: usize = 37;
    pub const CONSTRICTION_LOSS_J: usize = 38;
    pub const MAX_REYNOLDS: usize = 39;
    pub const FOLD_CYCLE_MM: usize = 40;
    pub const ORAL_AREAS_CM2: usize = 72;
    pub const ORAL_PRESSURE_RMS_PA: usize = 136;
    pub const NASAL_AREAS_CM2: usize = 200;
    pub const CONSTRICTIONS: usize = 248;
    pub const MIN_APERTURE_CM2: usize = 258;
    pub const MIN_APERTURE_POS: usize = 259;
    /// 1 full schedule, 2 the light schedule (see `LIGHT_SCHEDULE`).
    pub const DECIMATE: usize = 260;
    pub const LEN: usize = 264;
}

struct Engine {
    world: TractWorldNd,
    out: Vec<f32>,
    state: Vec<f32>,
    pressure_ms: Vec<f64>,
    decimate: usize,
    sample_rate: f64,
    non_finite: u64,
}

static ENGINE: Mutex<Option<Engine>> = Mutex::new(None);

fn with<R>(default: R, f: impl FnOnce(&mut Engine) -> R) -> R {
    match ENGINE.lock() {
        Ok(mut guard) => match guard.as_mut() {
            Some(engine) => f(engine),
            None => default,
        },
        Err(_) => default,
    }
}

fn build(sample_rate: f64, decimate: usize) -> Option<TractWorldNd> {
    let mut world = TractWorldNd::with(
        sample_rate,
        TractNdAnatomy::public_v1(),
        ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate },
        tw::ART_REST,
    )
    .ok()?;
    let (interval, min_move) = if decimate == 2 { LIGHT_SCHEDULE } else { FULL_SCHEDULE };
    world.update_interval = interval;
    world.min_move = min_move;
    world.set_controls(TractControls { articulation: tw::ART_REST, ..TractControls::default() });
    Some(world)
}

/// Build the World at `sample_rate`. Returns the number of oral sections.
#[no_mangle]
pub extern "C" fn tract_init(sample_rate: f32) -> u32 {
    tract_init_decimated(sample_rate, 1)
}

/// `decimate` 2 selects the light schedule (the World stays at the full
/// rate: its 24 modes need it); 1 the full schedule.
#[no_mangle]
pub extern "C" fn tract_init_decimated(sample_rate: f32, decimate: u32) -> u32 {
    let decimate = decimate.clamp(1, 2) as usize;
    let sample_rate = f64::from(sample_rate);
    let Some(world) = build(sample_rate, decimate) else { return 0 };
    let n = world.anatomy().oral_sections;
    let engine = Engine {
        pressure_ms: vec![0.0; n],
        world,
        out: vec![0.0; MAX_FRAMES],
        state: vec![0.0; layout::LEN],
        decimate,
        sample_rate,
        non_finite: 0,
    };
    if let Ok(mut guard) = ENGINE.lock() {
        *guard = Some(engine);
    }
    n as u32
}

#[no_mangle]
pub extern "C" fn tract_out_ptr() -> *const f32 {
    with(std::ptr::null(), |e| e.out.as_ptr())
}

#[no_mangle]
pub extern "C" fn tract_state_ptr() -> *const f32 {
    with(std::ptr::null(), |e| e.state.as_ptr())
}

#[no_mangle]
pub extern "C" fn tract_state_len() -> u32 {
    layout::LEN as u32
}

/// Render `frames` samples (at most `MAX_FRAMES`); returns how many. A World
/// error is a fault, not a sound: the block is silence, the World is rebuilt
/// and the fault counted (`NON_FINITE`).
#[no_mangle]
pub extern "C" fn tract_process(frames: u32) -> u32 {
    with(0, |e| {
        let n = (frames as usize).min(MAX_FRAMES);
        if e.world.process(&mut e.out[..n]).is_err() {
            e.out[..n].fill(0.0);
            e.non_finite += 1;
            if let Some(world) = build(e.sample_rate, e.decimate) {
                e.world = world;
            }
        }
        n as u32
    })
}

/// 0 tongue index, 1 tongue diameter, 2 lips, 3 velum, 4 fold tuning (Hz),
/// 5 tenseness, 6 breath, 7 aspiration (accepted and stored; the World has
/// no glottal turbulence source). Manual articulation stops a gesture.
#[no_mangle]
pub extern "C" fn tract_set(param: u32, value: f32) {
    with((), |e| {
        let v = f64::from(value);
        if param == 7 {
            e.world.set_aspiration(v);
            return;
        }
        let mut c = e.world.controls();
        match param {
            0 => c.articulation.tongue_index = v.clamp(0.0, 1.0),
            1 => c.articulation.tongue_diameter = v.clamp(0.0, 1.0),
            2 => c.articulation.lips = v.clamp(0.0, 1.0),
            3 => c.articulation.velum = v.clamp(0.0, 1.0),
            4 => c.pitch_hz = v,
            5 => c.tenseness = v.clamp(0.0, 1.0),
            6 => c.breath = v.clamp(0.0, 1.0),
            _ => return,
        }
        if e.world.gesture_active() && param <= 3 {
            e.world.set_gesture(None);
        }
        e.world.set_controls(c);
    });
}

#[no_mangle]
pub extern "C" fn tract_get(param: u32) -> f32 {
    with(0.0, |e| {
        let c = e.world.controls();
        (match param {
            0 => c.articulation.tongue_index,
            1 => c.articulation.tongue_diameter,
            2 => c.articulation.lips,
            3 => c.articulation.velum,
            4 => c.pitch_hz,
            5 => c.tenseness,
            6 => c.breath,
            7 => e.world.aspiration(),
            _ => 0.0,
        }) as f32
    })
}

/// 0 /a/, 1 /i/, 2 /u/, 3 rest.
#[no_mangle]
pub extern "C" fn tract_preset(id: u32) {
    with((), |e| {
        let art = match id {
            0 => tw::PUBLIC_ART_A,
            1 => tw::PUBLIC_ART_I,
            2 => tw::PUBLIC_ART_U,
            3 => tw::ART_REST,
            _ => return,
        };
        e.world.set_gesture(None);
        let mut c = e.world.controls();
        c.articulation = art;
        e.world.set_controls(c);
    });
}

/// Place (`degree` >= 0) or release (`degree` < 0) constriction `slot`
/// (1..4 touches, 0 the gestures').
#[no_mangle]
pub extern "C" fn tract_constrict(slot: u32, position: f32, degree: f32) {
    with((), |e| {
        let c = if degree < 0.0 { None } else { Some(Constriction { position: f64::from(position), degree: f64::from(degree) }) };
        e.world.set_constriction(slot as usize, c);
    });
}

#[no_mangle]
pub extern "C" fn tract_vibrato(rate_hz: f32, depth_semitones: f32, wobble: f32) {
    with((), |e| e.world.set_vibrato(Vibrato { rate_hz: f64::from(rate_hz), depth_semitones: f64::from(depth_semitones), wobble: f64::from(wobble) }));
}

/// 0 mama loop, 1 a-i-u-a, 2 sss, 3 shh, 4 fff, 5 zzz, 6 vvv, 7 pa, 8 ta,
/// 9 ka, 10 papapa loop; any other id stops.
#[no_mangle]
pub extern "C" fn tract_gesture(id: i32) {
    with((), |e| {
        let g = if id >= 0 { Gesture::from_index(id as u32) } else { None };
        e.world.set_gesture(g);
    });
}

#[no_mangle]
pub extern "C" fn tract_impact(position: f32, energy_j: f32) {
    with((), |e| e.world.impact(f64::from(position), f64::from(energy_j)));
}

/// Fill the state vector (no allocation).
#[no_mangle]
pub extern "C" fn tract_snapshot() {
    with((), |e| {
        use layout::*;
        let w = &mut e.world;
        w.take_pressure_ms(&mut e.pressure_ms);
        let obs = w.observers();
        w.reset_windows();
        let world = w.world();
        let receipt = world.receipt();
        let body = world.space_body(0).expect("the tract body");
        let o = body.observation();
        let t = body.totals().unwrap_or_default();
        let s = &mut e.state;
        let a = w.anatomy();
        let (n, m) = (a.oral_sections, a.nasal_sections);
        let cur = w.current();
        s[ORAL_N] = n as f32;
        s[NASAL_N] = m as f32;
        s[VELUM_SECTION] = a.velum_section() as f32;
        s[FOLD_GAP_MM] = (o.valve_half_gap_m * 1e3) as f32;
        let (lo, hi) = if obs.gap_min_m <= obs.gap_max_m { (obs.gap_min_m, obs.gap_max_m) } else { (o.valve_half_gap_m, o.valve_half_gap_m) };
        s[FOLD_GAP_MIN_MM] = (lo * 1e3) as f32;
        s[FOLD_GAP_MAX_MM] = (hi * 1e3) as f32;
        s[GLOTTAL_FLOW_CM3_S] = (o.valve.flow_m3_s * 1e6) as f32;
        s[LUNG_PRESSURE_PA] = o.valve.supply_pressure_pa as f32;
        s[LUNG_FRACTION] = (body.reservoir_energy() / a.lung_capacity_j) as f32;
        s[F0_HZ] = obs.measured_f0_hz as f32;
        s[TRACT_ENERGY_J] = body.field_energy() as f32;
        s[FOLD_ENERGY_J] = o.valve_energy as f32;
        s[LUNG_ENERGY_J] = body.reservoir_energy() as f32;
        s[MUSCLE_WORK_J] = (receipt.source_work - t.knock_work) as f32;
        s[IMPACT_WORK_J] = t.knock_work as f32;
        let stats = w.stats();
        s[TRACT_CONTROL_WORK_J] = stats.control_work as f32;
        s[FOLD_CONTROL_WORK_J] = stats.valve_work as f32;
        s[WALL_LOSS_J] = t.wall_loss as f32;
        s[JET_LOSS_J] = t.valve_jet_loss as f32;
        s[FOLD_LOSS_J] = t.fold_loss as f32;
        s[RADIATION_LIPS_J] = t.radiation_by_end[0] as f32;
        s[RADIATION_NOSE_J] = t.radiation_by_end[1] as f32;
        s[LEDGER_FRAMES] = receipt.sample_frames as f32;
        s[MAX_FRAME_RESIDUAL_J] = receipt.maximum_frame_residual as f32;
        s[GLOBAL_RESIDUAL_J] = receipt.global_closure_residual as f32;
        s[CUMULATIVE_RESIDUAL_J] = receipt.cumulative_closure_residual as f32;
        s[CUR_TONGUE_INDEX] = cur.articulation.tongue_index as f32;
        s[CUR_TONGUE_DIAMETER] = cur.articulation.tongue_diameter as f32;
        s[CUR_LIPS] = cur.articulation.lips as f32;
        s[CUR_VELUM] = cur.articulation.velum as f32;
        s[CUR_PITCH_HZ] = cur.pitch_hz as f32;
        s[CUR_TENSENESS] = cur.tenseness as f32;
        s[CUR_BREATH] = cur.breath as f32;
        s[GESTURE_ACTIVE] = if w.gesture_active() { 1.0 } else { 0.0 };
        s[SAFETY_LIMITED] = obs.safety_limited as f32;
        s[NON_FINITE] = e.non_finite as f32;
        s[IMPACT_COUNT] = obs.knocks as f32;
        // "Turbulence paid": the fluctuation energy the source injected (its
        // net work on the field, booked in the ledger, adds a zero-mean cross
        // term with the flow in its column).
        s[TURBULENCE_WORK_J] = t.aero.turbulence_injected as f32;
        s[CONSTRICTION_LOSS_J] = t.aero.jet_loss as f32;
        s[MAX_REYNOLDS] = obs.max_reynolds as f32;
        for k in 0..CYCLE_POINTS {
            s[FOLD_CYCLE_MM + k] = (obs.fold_cycle_m[k] * 1e3) as f32;
        }
        let areas = body.network_areas_m2().unwrap_or(&[]);
        for i in 0..MAX_ORAL_SECTIONS {
            s[ORAL_AREAS_CM2 + i] = if i < n { (areas[i] * 1e4) as f32 } else { 0.0 };
            s[ORAL_PRESSURE_RMS_PA + i] = if i < n { e.pressure_ms[i].sqrt() as f32 } else { 0.0 };
        }
        for k in 0..MAX_NASAL_SECTIONS {
            s[NASAL_AREAS_CM2 + k] = if k < m { (areas[n + k] * 1e4) as f32 } else { 0.0 };
        }
        for k in 0..MAX_SLOTS {
            let c = w.constriction(k).unwrap_or_default();
            s[CONSTRICTIONS + 2 * k] = c.position as f32;
            s[CONSTRICTIONS + 2 * k + 1] = c.degree as f32;
        }
        s[MIN_APERTURE_CM2] = (o.aero.jet_aperture_m2 * 1e4) as f32;
        s[MIN_APERTURE_POS] = ((o.aero.jet_junction as f64 + 1.0) / n as f64) as f32;
        s[DECIMATE] = e.decimate as f32;
    });
}

/// One field of the World's own receipt (`World::receipt`), in f64 (an extra
/// export outside the shared ABI, like `ndt_*` in `wasm.rs`; reading it loads
/// nothing): 0 source work, 1 control work, 2 internal loss, 3 radiation,
/// 4 initial energy, 5 current energy, 6 cumulative closure residual,
/// 7 maximum frame residual, 8 global closure residual, 9 sample frames.
/// Any other field, or no World, is NaN.
#[no_mangle]
pub extern "C" fn ndt_receipt(field: u32) -> f64 {
    with(f64::NAN, |e| {
        let r = e.world.world().receipt();
        match field {
            0 => r.source_work,
            1 => r.control_work,
            2 => r.internal_loss,
            3 => r.radiation,
            4 => r.initial_energy,
            5 => r.current_energy,
            6 => r.cumulative_closure_residual,
            7 => r.maximum_frame_residual,
            8 => r.global_closure_residual,
            9 => r.sample_frames as f64,
            _ => f64::NAN,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tube tract's own ABI test, against the World engine.
    #[test]
    fn the_abi_renders_and_snapshots() {
        let n = tract_init(48_000.0);
        assert_eq!(n, 44);
        tract_set(6, 0.6);
        tract_preset(0);
        for _ in 0..400 {
            assert_eq!(tract_process(128), 128);
        }
        tract_impact(0.5, 1.0e-4);
        tract_process(128);
        tract_snapshot();
        tract_constrict(1, 0.86, 0.93);
        for _ in 0..100 {
            tract_process(128);
        }
        tract_snapshot();
        let state = with(Vec::new(), |e| e.state.clone());
        assert_eq!(state[layout::ORAL_N], 44.0);
        assert!((state[layout::CONSTRICTIONS + 3] - 0.93).abs() < 1e-3, "slot 1 degree {}", state[layout::CONSTRICTIONS + 3]);
        println!("after touch: turbulence {:e} J, constriction loss {:e} J, narrowest {} cm2 at {}, max Re {}, glottal flow {} cm3/s", state[layout::TURBULENCE_WORK_J], state[layout::CONSTRICTION_LOSS_J], state[layout::MIN_APERTURE_CM2], state[layout::MIN_APERTURE_POS], state[layout::MAX_REYNOLDS], state[layout::GLOTTAL_FLOW_CM3_S]);
        assert!(state[layout::TURBULENCE_WORK_J] > 0.0, "a touch constriction hisses");
        assert!(state[layout::F0_HZ] > 80.0, "F0 {}", state[layout::F0_HZ]);
        assert!(state[layout::MAX_FRAME_RESIDUAL_J].abs() < 1e-12);
        assert!(state[layout::IMPACT_WORK_J] != 0.0);
        assert_eq!(state[layout::NON_FINITE], 0.0);
        let out = with(Vec::new(), |e| e.out[..128].to_vec());
        assert!(out.iter().all(|v| v.is_finite()));
        // The light schedule.
        assert_eq!(tract_init_decimated(48_000.0, 2), 44);
        tract_set(6, 0.6);
        for _ in 0..200 {
            assert_eq!(tract_process(128), 128);
        }
        tract_snapshot();
        let state = with(Vec::new(), |e| e.state.clone());
        assert!(state[layout::F0_HZ] > 60.0, "light schedule voices: {}", state[layout::F0_HZ]);
        assert!(state[layout::MAX_FRAME_RESIDUAL_J].abs() < 1e-12);
        assert_eq!(state[layout::DECIMATE], 2.0);
    }
}
