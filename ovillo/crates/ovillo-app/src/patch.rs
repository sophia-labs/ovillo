//! Patches and takes on disk.
//!
//! A patch is the plug-in's twenty parameters as JSON (`AriadneControls`), so
//! it loads into the Ariadne World plug-in unchanged. The live patch,
//! `patches/live.json`, is written whenever a control moves and re-read
//! whenever it changes on disk: edit it in any editor (or let Claude edit it)
//! and the instrument follows. Takes go to `takes/<time>/`: the dry DI, the
//! wet stereo, and the patch they were played with.

use ovillo_dsp::ariadne::AriadneControls;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn root() -> PathBuf {
    // crates/ovillo-app → the ovillo workspace.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn patches_dir() -> PathBuf {
    root().join("patches")
}

pub fn live_path() -> PathBuf {
    patches_dir().join("live.json")
}

pub fn slot_path(slot: u8) -> PathBuf {
    patches_dir().join(format!("slot-{}.json", (b'a' + slot) as char))
}

pub fn default_patch() -> AriadneControls {
    read(&patches_dir().join("ovillo-default.json")).unwrap_or_default()
}

pub fn read(path: &Path) -> Result<AriadneControls, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let c: AriadneControls = serde_json::from_str(&s).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(c.sanitized())
}

pub fn write(path: &Path, c: &AriadneControls) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let s = serde_json::to_string_pretty(c).map_err(|e| e.to_string())?;
    // Write then rename, so a watcher never reads half a file.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, s + "\n").map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// A take in progress: three WAV writers fed from the scope tap.
pub struct Take {
    pub dir: PathBuf,
    dry: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    wet: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    pub frames: u64,
}

impl Take {
    pub fn start(sr: u32, patch: &AriadneControls) -> Result<Self, String> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dir = root().join("takes").join(format!("take-{stamp}"));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        write(&dir.join("patch.json"), patch)?;
        let spec = |channels| hound::WavSpec {
            channels,
            sample_rate: sr,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let dry = hound::WavWriter::create(dir.join("dry.wav"), spec(1)).map_err(|e| e.to_string())?;
        let wet = hound::WavWriter::create(dir.join("wet.wav"), spec(2)).map_err(|e| e.to_string())?;
        Ok(Self { dir, dry, wet, frames: 0 })
    }

    pub fn push(&mut self, dry: f32, left: f32, right: f32) {
        let _ = self.dry.write_sample(dry);
        let _ = self.wet.write_sample(left);
        let _ = self.wet.write_sample(right);
        self.frames += 1;
    }

    pub fn finish(self) -> Result<PathBuf, String> {
        self.dry.finalize().map_err(|e| e.to_string())?;
        self.wet.finalize().map_err(|e| e.to_string())?;
        Ok(self.dir)
    }
}
