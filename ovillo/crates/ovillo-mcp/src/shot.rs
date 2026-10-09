//! A picture of one Ovillo window, and nothing else on the screen.
//!
//! The instrument tells us its pid; CoreGraphics' window list gives that
//! process's own window number; `screencapture -l<window>` captures just that
//! window (even if other windows cover it); `sips` scales it down. Taking the
//! picture needs the Screen Recording permission of whatever launched this
//! server (the terminal, for Claude Code).

use std::ffi::{c_void, CString};
use std::path::{Path, PathBuf};
use std::process::Command;

type CFTypeRef = *const c_void;

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CFTypeRef;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(array: CFTypeRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CFTypeRef, index: isize) -> CFTypeRef;
    fn CFDictionaryGetValue(dict: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
    fn CFNumberGetValue(number: CFTypeRef, the_type: isize, value: *mut c_void) -> bool;
    fn CFStringCreateWithCString(alloc: CFTypeRef, s: *const std::ffi::c_char, encoding: u32) -> CFTypeRef;
    fn CFRelease(cf: CFTypeRef);
}

const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
const K_CF_NUMBER_SINT64: isize = 4;
const K_CF_NUMBER_DOUBLE: isize = 13;
const ON_SCREEN_ONLY: u32 = 1;
const EXCLUDE_DESKTOP: u32 = 16;

/// A window of `pid`: its number and size in points.
#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub id: u32,
    pub width: f64,
    pub height: f64,
}

#[cfg(target_os = "macos")]
struct Key(CFTypeRef);

#[cfg(target_os = "macos")]
impl Key {
    fn new(s: &str) -> Self {
        let c = CString::new(s).unwrap_or_default();
        // SAFETY: a NUL-terminated UTF-8 string; the result is released in Drop.
        Self(unsafe { CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), K_CF_STRING_ENCODING_UTF8) })
    }
}

#[cfg(target_os = "macos")]
impl Drop for Key {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: created by CFStringCreateWithCString above.
            unsafe { CFRelease(self.0) };
        }
    }
}

#[cfg(target_os = "macos")]
unsafe fn number(dict: CFTypeRef, key: &Key, ty: isize) -> Option<f64> {
    let v = unsafe { CFDictionaryGetValue(dict, key.0) };
    if v.is_null() {
        return None;
    }
    if ty == K_CF_NUMBER_SINT64 {
        let mut out: i64 = 0;
        unsafe { CFNumberGetValue(v, ty, (&raw mut out).cast()) }.then_some(out as f64)
    } else {
        let mut out: f64 = 0.0;
        unsafe { CFNumberGetValue(v, ty, (&raw mut out).cast()) }.then_some(out)
    }
}

/// The largest normal-layer window owned by `pid`.
#[cfg(target_os = "macos")]
pub fn find_window(pid: u32) -> Option<Window> {
    let (k_pid, k_num, k_layer, k_bounds) =
        (Key::new("kCGWindowOwnerPID"), Key::new("kCGWindowNumber"), Key::new("kCGWindowLayer"), Key::new("kCGWindowBounds"));
    let (k_w, k_h) = (Key::new("Width"), Key::new("Height"));
    for option in [ON_SCREEN_ONLY | EXCLUDE_DESKTOP, 0] {
        // SAFETY: CoreGraphics returns a CFArray of CFDictionaries (or null), released below.
        let list = unsafe { CGWindowListCopyWindowInfo(option, 0) };
        if list.is_null() {
            continue;
        }
        let mut best: Option<Window> = None;
        // SAFETY: indices stay within the array's count; values are borrowed from the array.
        unsafe {
            for i in 0..CFArrayGetCount(list) {
                let d = CFArrayGetValueAtIndex(list, i);
                if number(d, &k_pid, K_CF_NUMBER_SINT64) != Some(pid as f64) || number(d, &k_layer, K_CF_NUMBER_SINT64) != Some(0.0) {
                    continue;
                }
                let Some(id) = number(d, &k_num, K_CF_NUMBER_SINT64) else { continue };
                let b = CFDictionaryGetValue(d, k_bounds.0);
                let (w, h) = if b.is_null() {
                    (0.0, 0.0)
                } else {
                    (number(b, &k_w, K_CF_NUMBER_DOUBLE).unwrap_or(0.0), number(b, &k_h, K_CF_NUMBER_DOUBLE).unwrap_or(0.0))
                };
                if best.is_none_or(|bw| w * h > bw.width * bw.height) {
                    best = Some(Window { id: id as u32, width: w, height: h });
                }
            }
            CFRelease(list);
        }
        if best.is_some() {
            return best;
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
pub fn find_window(_pid: u32) -> Option<Window> {
    None
}

/// Width and height of a PNG, from its header.
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((w, h))
}

pub struct Shot {
    pub png: Vec<u8>,
    pub window: Window,
    pub width: u32,
    pub height: u32,
    pub saved: Option<PathBuf>,
}

/// Capture `pid`'s window as a PNG no wider than `max_width` pixels.
pub fn capture(pid: u32, max_width: u32, save_to: Option<&Path>) -> Result<Shot, String> {
    let window = find_window(pid).ok_or_else(|| format!("no window of pid {pid} in the window list (is it minimized or on another Space?)"))?;
    let tmp = std::env::temp_dir().join(format!("ovillo-shot-{}-{}.png", std::process::id(), window.id));
    let out = Command::new("/usr/sbin/screencapture")
        .args(["-x", "-o", "-t", "png", &format!("-l{}", window.id)])
        .arg(&tmp)
        .output()
        .map_err(|e| format!("screencapture: {e}"))?;
    if !out.status.success() || !tmp.exists() {
        return Err(format!(
            "screencapture failed ({}): {} (the app that launched this server needs the Screen Recording permission)",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let mut png = std::fs::read(&tmp).map_err(|e| e.to_string())?;
    let (mut width, mut height) = png_size(&png).ok_or("screencapture did not write a PNG")?;
    if width > max_width {
        let small = tmp.with_extension("small.png");
        let st = Command::new("/usr/bin/sips")
            .args(["--resampleWidth", &max_width.to_string()])
            .arg(&tmp)
            .arg("--out")
            .arg(&small)
            .output()
            .map_err(|e| format!("sips: {e}"))?;
        if st.status.success() {
            if let Ok(b) = std::fs::read(&small) {
                if let Some((w, h)) = png_size(&b) {
                    png = b;
                    width = w;
                    height = h;
                }
            }
        }
        let _ = std::fs::remove_file(&small);
    }
    let _ = std::fs::remove_file(&tmp);
    let saved = match save_to {
        Some(p) => {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            std::fs::write(p, &png).map_err(|e| format!("could not save to {}: {e}", p.display()))?;
            Some(p.to_path_buf())
        }
        None => None,
    };
    Ok(Shot { png, window, width, height, saved })
}

/// Standard base64 with padding.
pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_examples() {
        for (i, o) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")] {
            assert_eq!(base64(i.as_bytes()), o);
        }
    }

    #[test]
    fn this_process_has_no_window_to_find() {
        // A test binary owns no windows: the search must come back empty, not with someone else's.
        assert!(find_window(std::process::id()).is_none());
    }
}
