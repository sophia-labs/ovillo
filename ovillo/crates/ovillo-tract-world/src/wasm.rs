//! C ABI for timing (and playing) the N-D tract World as wasm32. No
//! wasm-bindgen: instantiate from bytes, call these, read audio at the
//! returned pointer. `examples/wasm_bench.mjs` times them under node.

use crate::bench::Workload;
use std::sync::Mutex;

const MAX_FRAMES: usize = 4096;

struct State {
    workload: Workload,
    buffer: Vec<f32>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Build a World; returns 0 on success.
#[no_mangle]
pub extern "C" fn ndt_init(oral: u32, nasal: u32, interval: u32, moving: u32, min_move: f64) -> i32 {
    let workload = Workload::new(oral as usize, nasal as usize, interval as usize, moving != 0, 48_000.0, min_move);
    *STATE.lock().unwrap() = Some(State { workload, buffer: vec![0.0; MAX_FRAMES] });
    0
}

/// Render `frames` (<= 4096) samples; returns their sum of squares.
#[no_mangle]
pub extern "C" fn ndt_render(frames: u32) -> f64 {
    let mut guard = STATE.lock().unwrap();
    let Some(state) = guard.as_mut() else { return -1.0 };
    let frames = (frames as usize).min(MAX_FRAMES);
    let State { workload, buffer } = state;
    workload.render(&mut buffer[..frames])
}

#[no_mangle]
pub extern "C" fn ndt_buffer() -> *const f32 {
    STATE.lock().unwrap().as_ref().map_or(std::ptr::null(), |s| s.buffer.as_ptr())
}

#[no_mangle]
pub extern "C" fn ndt_max_frame_residual() -> f64 {
    STATE.lock().unwrap().as_ref().map_or(-1.0, |s| s.workload.world.world().receipt().maximum_frame_residual)
}

#[no_mangle]
pub extern "C" fn ndt_cumulative_residual() -> f64 {
    STATE.lock().unwrap().as_ref().map_or(-1.0, |s| s.workload.world.world().receipt().cumulative_closure_residual)
}

#[no_mangle]
pub extern "C" fn ndt_moves() -> f64 {
    STATE.lock().unwrap().as_ref().map_or(-1.0, |s| s.workload.world.stats().moves as f64)
}
