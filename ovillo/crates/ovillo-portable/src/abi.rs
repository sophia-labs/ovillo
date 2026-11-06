//! The C ABI: many World instances in one module, each behind a handle.
//!
//! Plain `extern "C"` exports, no imports and no bindgen, so the same `.wasm`
//! instantiates synchronously inside an `AudioWorkletGlobalScope` (which has
//! no `fetch`, no `TextDecoder`), in Node, in a Worker and on a page. Text
//! (JSON in, JSON out) goes through one shared byte buffer each way; numbers
//! go as arguments. Nothing on the per-quantum path (`ov_render`, `ov_event`,
//! `ov_set`, the pointers) allocates once an instance exists, except a
//! growing schedule.
//!
//! | export | does |
//! |---|---|
//! | `ov_abi_version() -> u32` | 1 |
//! | `ov_input_ptr(len) -> *mut u8` | room for `len` bytes of input text |
//! | `ov_text_ptr()`, `ov_text_len()` | the last text an export produced |
//! | `ov_catalog() -> len` | text: the catalog (`ovillo.portable.catalog.v1`) |
//! | `ov_create(len, sample_rate) -> handle` | input: `{"kind", "anatomy"?, "options"?}`; 0 = refused (text: why) |
//! | `ov_dispose(h)`, `ov_reset(h)` | |
//! | `ov_render(h, frames) -> frames` | at most `ov_max_frames()` |
//! | `ov_left_ptr(h)`, `ov_right_ptr(h)` | the rendered frames (f32) |
//! | `ov_args_ptr() -> *mut f64` | `MAX_ARGS` event arguments, written before `ov_event` |
//! | `ov_event(h, index, at, nargs) -> i32` | schedule catalog event `index` at frame `at` |
//! | `ov_set(h, index, value, at, ramp) -> i32` | schedule control `index` (ramp in frames) |
//! | `ov_control(h, index) -> f64` | its current value |
//! | `ov_play_score(h, len, at) -> i32` | input: a score; text: what reading it found |
//! | `ov_telemetry_ptr(h)`, `ov_telemetry_len(h)` | f64 slots (`ov_info` names them) |
//! | `ov_frame(h) -> f64` | the World's clock |
//! | `ov_receipt(h) -> len`, `ov_info(h) -> len`, `ov_error(h) -> len` | text |
//!
//! Return codes: `0` done, `-1` no such instance, `-2` refused (text: why).
//! A call that returns a length or creates an instance may grow memory: take
//! views after it.

use std::cell::RefCell;

use serde_json::{json, Value};

use crate::{Instance, MAX_ARGS, MAX_FRAMES};

#[derive(Default)]
struct Registry {
    instances: Vec<Option<Instance>>,
    input: Vec<u8>,
    text: Vec<u8>,
    args: [f64; MAX_ARGS],
}

thread_local! {
    static REG: RefCell<Registry> = RefCell::new(Registry::default());
}

fn with<R>(f: impl FnOnce(&mut Registry) -> R) -> R {
    REG.with(|r| f(&mut r.borrow_mut()))
}

fn put_text(r: &mut Registry, s: &str) -> u32 {
    r.text.clear();
    r.text.extend_from_slice(s.as_bytes());
    r.text.len() as u32
}

fn input(r: &Registry, len: u32) -> Result<String, String> {
    let bytes = r.input.get(..len as usize).ok_or("input is shorter than the length given")?;
    String::from_utf8(bytes.to_vec()).map_err(|e| format!("input is not UTF-8: {e}"))
}

fn instance(r: &mut Registry, h: u32) -> Option<&mut Instance> {
    r.instances.get_mut((h as usize).checked_sub(1)?)?.as_mut()
}

#[no_mangle]
pub extern "C" fn ov_abi_version() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn ov_max_frames() -> u32 {
    MAX_FRAMES as u32
}

#[no_mangle]
pub extern "C" fn ov_max_args() -> u32 {
    MAX_ARGS as u32
}

#[no_mangle]
pub extern "C" fn ov_input_ptr(len: u32) -> *mut u8 {
    with(|r| {
        r.input.clear();
        r.input.resize(len as usize, 0);
        r.input.as_mut_ptr()
    })
}

#[no_mangle]
pub extern "C" fn ov_text_ptr() -> *const u8 {
    with(|r| r.text.as_ptr())
}

#[no_mangle]
pub extern "C" fn ov_text_len() -> u32 {
    with(|r| r.text.len() as u32)
}

#[no_mangle]
pub extern "C" fn ov_args_ptr() -> *mut f64 {
    with(|r| r.args.as_mut_ptr())
}

#[no_mangle]
pub extern "C" fn ov_catalog() -> u32 {
    let text = crate::catalog::catalog_json().to_string();
    with(|r| put_text(r, &text))
}

/// Build a World from `{"kind", "anatomy"?, "options"?}` in the input buffer.
/// Returns its handle (>= 1), or 0 with the reason in the text buffer.
#[no_mangle]
pub extern "C" fn ov_create(len: u32, sample_rate: f64) -> u32 {
    let request = with(|r| input(r, len));
    let built = request.and_then(|text| {
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("create: {e}"))?;
        let kind = v.get("kind").and_then(Value::as_str).ok_or("create: give a kind")?;
        let anatomy = v.get("anatomy").and_then(Value::as_str);
        let options = v.get("options").cloned().unwrap_or_else(|| json!({}));
        let mut inst = Instance::create(kind, sample_rate, anatomy, &options)?;
        if let Some(Value::String(score)) = options.get("score") {
            inst.play_score(score, 0.0)?;
        }
        Ok(inst)
    });
    with(|r| match built {
        Ok(inst) => {
            let slot = r.instances.iter().position(Option::is_none).unwrap_or_else(|| {
                r.instances.push(None);
                r.instances.len() - 1
            });
            r.instances[slot] = Some(inst);
            put_text(r, "");
            (slot + 1) as u32
        }
        Err(e) => {
            put_text(r, &e);
            0
        }
    })
}

#[no_mangle]
pub extern "C" fn ov_dispose(h: u32) -> i32 {
    with(|r| match r.instances.get_mut((h as usize).wrapping_sub(1)) {
        Some(slot @ Some(_)) => {
            *slot = None;
            0
        }
        _ => -1,
    })
}

#[no_mangle]
pub extern "C" fn ov_instances() -> u32 {
    with(|r| r.instances.iter().filter(|i| i.is_some()).count() as u32)
}

#[no_mangle]
pub extern "C" fn ov_reset(h: u32) -> i32 {
    with(|r| {
        let Some(i) = instance(r, h) else { return -1 };
        match i.reset() {
            Ok(()) => 0,
            Err(e) => {
                put_text(r, &e);
                -2
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn ov_render(h: u32, frames: u32) -> u32 {
    with(|r| instance(r, h).map_or(0, |i| i.render(frames as usize) as u32))
}

#[no_mangle]
pub extern "C" fn ov_left_ptr(h: u32) -> *const f32 {
    with(|r| instance(r, h).map_or(std::ptr::null(), |i| i.left.as_ptr()))
}

#[no_mangle]
pub extern "C" fn ov_right_ptr(h: u32) -> *const f32 {
    with(|r| instance(r, h).map_or(std::ptr::null(), |i| i.right.as_ptr()))
}

/// Schedule event `index` at frame `at` with the first `nargs` of the
/// argument buffer (the rest take their catalog defaults).
#[no_mangle]
pub extern "C" fn ov_event(h: u32, index: u32, at: f64, nargs: u32) -> i32 {
    with(|r| {
        let mut args = [f64::NAN; MAX_ARGS];
        let n = (nargs as usize).min(MAX_ARGS);
        args[..n].copy_from_slice(&r.args[..n]);
        let Some(i) = instance(r, h) else { return -1 };
        match i.event(index as usize, at, &args) {
            Ok(()) => 0,
            Err(e) => {
                put_text(r, &e);
                -2
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn ov_set(h: u32, index: u32, value: f64, at: f64, ramp: f64) -> i32 {
    with(|r| {
        let Some(i) = instance(r, h) else { return -1 };
        match i.set(index as usize, value, at, ramp) {
            Ok(()) => 0,
            Err(e) => {
                put_text(r, &e);
                -2
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn ov_control(h: u32, index: u32) -> f64 {
    with(|r| instance(r, h).map_or(f64::NAN, |i| i.control(index as usize)))
}

/// Hand the score in the input buffer to the World from frame `at`.
#[no_mangle]
pub extern "C" fn ov_play_score(h: u32, len: u32, at: f64) -> i32 {
    with(|r| {
        let text = match input(r, len) {
            Ok(t) => t,
            Err(e) => {
                put_text(r, &e);
                return -2;
            }
        };
        let Some(i) = instance(r, h) else { return -1 };
        match i.play_score(&text, at) {
            Ok(read) => {
                let s = read.to_string();
                put_text(r, &s);
                0
            }
            Err(e) => {
                put_text(r, &e);
                -2
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn ov_telemetry_ptr(h: u32) -> *const f64 {
    with(|r| instance(r, h).map_or(std::ptr::null(), |i| i.telemetry.as_ptr()))
}

#[no_mangle]
pub extern "C" fn ov_telemetry_len(h: u32) -> u32 {
    with(|r| instance(r, h).map_or(0, |i| i.telemetry.len() as u32))
}

/// Re-read the ledger into telemetry now (it is otherwise read every 1024 frames).
#[no_mangle]
pub extern "C" fn ov_refresh(h: u32) -> i32 {
    with(|r| instance(r, h).map_or(-1, |i| {
        i.refresh();
        0
    }))
}

#[no_mangle]
pub extern "C" fn ov_frame(h: u32) -> f64 {
    with(|r| instance(r, h).map_or(f64::NAN, |i| i.frame() as f64))
}

#[no_mangle]
pub extern "C" fn ov_receipt(h: u32) -> u32 {
    with(|r| {
        let Some(text) = instance(r, h).map(|i| i.receipt().to_string()) else { return 0 };
        put_text(r, &text)
    })
}

#[no_mangle]
pub extern "C" fn ov_info(h: u32) -> u32 {
    with(|r| {
        let Some(text) = instance(r, h).map(|i| i.info().to_string()) else { return 0 };
        put_text(r, &text)
    })
}

#[no_mangle]
pub extern "C" fn ov_error(h: u32) -> u32 {
    with(|r| {
        let Some(text) = instance(r, h).map(|i| i.last_error().to_owned()) else { return 0 };
        put_text(r, &text)
    })
}
