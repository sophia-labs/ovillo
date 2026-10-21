//! C ABI over the tract World for an `AudioWorkletProcessor`.
//!
//! No wasm-bindgen: the worklet instantiates the module from bytes and calls
//! these functions directly, reading audio and state out of linear memory at
//! the pointers they return. One World per module instance. Everything that
//! allocates happens in `tract_init`; `tract_process` and `tract_snapshot` do
//! not allocate.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p ovillo-tract-wasm`

use ovillo_tract::anatomy::{MAX_NASAL_SECTIONS, MAX_ORAL_SECTIONS};
use ovillo_tract::world::CYCLE_POINTS;
use ovillo_tract::{Gesture, Preset, TractControls, TractReadings, TractWorld};
use std::sync::Mutex;

/// Largest render request per call.
pub const MAX_FRAMES: usize = 4096;

/// Layout of the state vector (f32), for the page.
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
    pub const FOLD_CYCLE_MM: usize = 40;
    pub const ORAL_AREAS_CM2: usize = 72;
    pub const ORAL_PRESSURE_RMS_PA: usize = 136;
    pub const NASAL_AREAS_CM2: usize = 200;
    pub const LEN: usize = 248;
}

struct Engine {
    world: TractWorld,
    out: Vec<f32>,
    state: Vec<f32>,
    readings: TractReadings,
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

/// Build the World at `sample_rate`. Returns the number of oral sections.
#[no_mangle]
pub extern "C" fn tract_init(sample_rate: f32) -> u32 {
    let mut world = TractWorld::new(f64::from(sample_rate));
    let mut c = TractControls::default();
    c.articulation = Preset::Rest.articulation();
    world.set_controls(c);
    world.snap_configuration();
    let readings = world.readings();
    let n = world.oral_sections() as u32;
    let engine = Engine { world, out: vec![0.0; MAX_FRAMES], state: vec![0.0; layout::LEN], readings };
    if let Ok(mut guard) = ENGINE.lock() {
        *guard = Some(engine);
    }
    n
}

/// Where `tract_process` writes its mono output.
#[no_mangle]
pub extern "C" fn tract_out_ptr() -> *const f32 {
    with(std::ptr::null(), |e| e.out.as_ptr())
}

/// Where `tract_snapshot` writes the state vector (see [`layout`]).
#[no_mangle]
pub extern "C" fn tract_state_ptr() -> *const f32 {
    with(std::ptr::null(), |e| e.state.as_ptr())
}

#[no_mangle]
pub extern "C" fn tract_state_len() -> u32 {
    layout::LEN as u32
}

/// Render `frames` samples (at most `MAX_FRAMES`); returns how many.
#[no_mangle]
pub extern "C" fn tract_process(frames: u32) -> u32 {
    with(0, |e| {
        let n = (frames as usize).min(MAX_FRAMES);
        e.world.process(&mut e.out[..n]);
        n as u32
    })
}

/// Set one control target. 0 tongue index, 1 tongue diameter, 2 lips,
/// 3 velum, 4 fold tuning (Hz), 5 tenseness, 6 breath, 7 aspiration.
/// Manual control stops a running gesture.
#[no_mangle]
pub extern "C" fn tract_set(param: u32, value: f32) {
    with((), |e| {
        let mut c = e.world.controls();
        let v = f64::from(value);
        match param {
            0 => c.articulation.tongue_index = v,
            1 => c.articulation.tongue_diameter = v,
            2 => c.articulation.lips = v,
            3 => c.articulation.velum = v,
            4 => c.pitch_hz = v,
            5 => c.tenseness = v,
            6 => c.breath = v,
            7 => c.aspiration = v,
            _ => return,
        }
        if e.world.gesture_active() && param <= 3 {
            e.world.set_gesture(None);
        }
        e.world.set_controls(c);
    });
}

/// The current target of one control (same numbering as `tract_set`).
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
            7 => c.aspiration,
            _ => 0.0,
        }) as f32
    })
}

/// Aim the articulators at a preset: 0 /a/, 1 /i/, 2 /u/, 3 rest.
#[no_mangle]
pub extern "C" fn tract_preset(id: u32) {
    with((), |e| {
        if let Some(p) = Preset::from_index(id) {
            e.world.set_gesture(None);
            let mut c = e.world.controls();
            c.articulation = p.articulation();
            e.world.set_controls(c);
        }
    });
}

/// Run a gesture (0 "mama" loop, 1 /a i u a/), or stop with any other id.
#[no_mangle]
pub extern "C" fn tract_gesture(id: i32) {
    with((), |e| {
        let g = if id >= 0 { Gesture::from_index(id as u32) } else { None };
        e.world.set_gesture(g);
    });
}

/// An orb hits the tube at `position` (0 glottis .. 1 lips) with `energy_j`.
#[no_mangle]
pub extern "C" fn tract_impact(position: f32, energy_j: f32) {
    with((), |e| e.world.impact(f64::from(position), f64::from(energy_j)));
}

/// Fill the state vector from the World's readings (no allocation).
#[no_mangle]
pub extern "C" fn tract_snapshot() {
    with((), |e| {
        use layout::*;
        e.world.readings_into(&mut e.readings);
        let r = &e.readings;
        let s = &mut e.state;
        let w = &e.world;
        let cur = w.current_controls();
        s[ORAL_N] = w.oral_sections() as f32;
        s[NASAL_N] = w.nasal_sections() as f32;
        s[VELUM_SECTION] = w.velum_section() as f32;
        s[FOLD_GAP_MM] = (r.fold_half_gap_m * 1e3) as f32;
        s[FOLD_GAP_MIN_MM] = (r.fold_gap_min_m * 1e3) as f32;
        s[FOLD_GAP_MAX_MM] = (r.fold_gap_max_m * 1e3) as f32;
        s[GLOTTAL_FLOW_CM3_S] = (r.glottal_flow_m3_s * 1e6) as f32;
        s[LUNG_PRESSURE_PA] = r.lung_pressure_pa as f32;
        s[LUNG_FRACTION] = r.lung_fraction as f32;
        s[F0_HZ] = r.measured_f0_hz as f32;
        s[TRACT_ENERGY_J] = r.tract_energy_j as f32;
        s[FOLD_ENERGY_J] = r.fold_energy_j as f32;
        s[LUNG_ENERGY_J] = r.lung_energy_j as f32;
        let f = r.flows;
        s[MUSCLE_WORK_J] = f.muscle_work as f32;
        s[IMPACT_WORK_J] = f.impact_work as f32;
        s[TRACT_CONTROL_WORK_J] = f.tract_control_work as f32;
        s[FOLD_CONTROL_WORK_J] = f.fold_control_work as f32;
        s[WALL_LOSS_J] = f.wall_loss as f32;
        s[JET_LOSS_J] = f.jet_loss as f32;
        s[FOLD_LOSS_J] = f.fold_loss as f32;
        s[RADIATION_LIPS_J] = f.radiation_lips as f32;
        s[RADIATION_NOSE_J] = f.radiation_nose as f32;
        s[LEDGER_FRAMES] = r.ledger_frames as f32;
        s[MAX_FRAME_RESIDUAL_J] = r.max_frame_residual_j as f32;
        s[GLOBAL_RESIDUAL_J] = r.global_residual_j as f32;
        s[CUMULATIVE_RESIDUAL_J] = r.cumulative_residual_j as f32;
        s[CUR_TONGUE_INDEX] = cur.articulation.tongue_index as f32;
        s[CUR_TONGUE_DIAMETER] = cur.articulation.tongue_diameter as f32;
        s[CUR_LIPS] = cur.articulation.lips as f32;
        s[CUR_VELUM] = cur.articulation.velum as f32;
        s[CUR_PITCH_HZ] = cur.pitch_hz as f32;
        s[CUR_TENSENESS] = cur.tenseness as f32;
        s[CUR_BREATH] = cur.breath as f32;
        s[GESTURE_ACTIVE] = if w.gesture_active() { 1.0 } else { 0.0 };
        s[SAFETY_LIMITED] = r.safety_limited_frames as f32;
        s[NON_FINITE] = r.non_finite_events as f32;
        s[IMPACT_COUNT] = w.impact_count() as f32;
        for k in 0..CYCLE_POINTS {
            s[FOLD_CYCLE_MM + k] = (r.fold_cycle[k] * 1e3) as f32;
        }
        for i in 0..MAX_ORAL_SECTIONS {
            s[ORAL_AREAS_CM2 + i] = r.oral_areas_cm2.get(i).copied().unwrap_or(0.0) as f32;
            s[ORAL_PRESSURE_RMS_PA + i] = r.oral_pressure_rms.get(i).copied().unwrap_or(0.0) as f32;
        }
        for k in 0..MAX_NASAL_SECTIONS {
            s[NASAL_AREAS_CM2 + k] = r.nasal_areas_cm2.get(k).copied().unwrap_or(0.0) as f32;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let state = with(Vec::new(), |e| e.state.clone());
        assert_eq!(state[layout::ORAL_N], 44.0);
        assert!(state[layout::F0_HZ] > 80.0);
        assert!(state[layout::MAX_FRAME_RESIDUAL_J].abs() < 1e-12);
        assert!(state[layout::IMPACT_WORK_J] > 0.0);
        let out = with(Vec::new(), |e| e.out[..128].to_vec());
        assert!(out.iter().all(|v| v.is_finite()));
    }
}
