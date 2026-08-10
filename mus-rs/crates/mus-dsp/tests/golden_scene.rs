//! Golden-vector harness for the scene renderer (PW7, `mus_dsp::scene`):
//! loads `mus-rs/fixtures/scene/manifest.json`, renders each named scene
//! through [`mus_dsp::scene::render_stereo`]/[`mus_dsp::scene::
//! render_foa`], and checks the output against the oracle-dumped planar
//! f32 binaries at the manifest's own metric/tolerance — a local loader
//! adapted from `mus_dsp::fixtures`'s pattern (different manifest shape:
//! scene name + JSON scene path + PCM binary + safetyGainDb, not a DSP
//! function-call case), since the scene manifest isn't the `fixtures/dsp`
//! shape that module reads. See `mus-rs/tools/gen_scene_fixtures.py` for
//! how the fixtures were produced.
//!
//! Also covers the invariants the PW7 spec calls out by name:
//! determinism, silence-in/silence-out, azimuth mirror symmetry on a
//! room-off scene, and a typed refusal for an unsupported control.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use mus_dsp::scene::{
    self, Listener, Position, PsychoacousticControls, Room, SceneError, SpatialObject, SpatialScene,
};

// ------------------------------------------------------------- manifest

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    name: String,
    scene: String,
    stereo: String,
    channels: u32,
    frames: usize,
    layout: String,
    metric: String,
    tol: f64,
    #[serde(rename = "safetyGainDb")]
    safety_gain_db: f64,
    renderer: String,
    #[serde(default)]
    foa: Option<String>,
    #[serde(default, rename = "foaChannels")]
    foa_channels: Option<u32>,
    #[serde(default, rename = "foaRenderer")]
    foa_renderer: Option<String>,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/scene")
}

fn manifest() -> Vec<ManifestEntry> {
    let path = fixtures_dir().join("manifest.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e} — run gen_scene_fixtures.py", path.display()));
    serde_json::from_str(&text).expect("scene manifest.json parses")
}

fn read_planar_f32(path: &Path, channels: usize, frames: usize) -> Vec<f32> {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert_eq!(
        bytes.len(),
        channels * frames * 4,
        "{}: byte length != channels*frames*4",
        path.display()
    );
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// This crate's audio convention is interleaved (`[l0, r0, l1, r1, ..]`);
/// the fixtures are planar/channel-major (`mix.T` in the oracle: all of
/// channel 0, then all of channel 1, ...). Reshape to compare like with
/// like.
fn interleaved_to_planar(interleaved: &[f32], channels: usize) -> Vec<f32> {
    let frames = interleaved.len() / channels;
    let mut planar = vec![0.0f32; interleaved.len()];
    for f in 0..frames {
        for c in 0..channels {
            planar[c * frames + f] = interleaved[f * channels + c];
        }
    }
    planar
}

/// Same `max_abs` metric `mus_dsp::fixtures::Case::check` uses, adapted
/// for a plain `&[f32]` pair rather than a `Case` (the scene manifest
/// carries its tolerance per-entry, not per-array like the DSP fixtures
/// do, so there's no shared `Case` type to reuse here).
fn check_max_abs(actual: &[f32], expected: &[f32], tol: f64, label: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{label}: length {} != expected {}",
        actual.len(),
        expected.len()
    );
    let mut worst = 0.0f64;
    let mut at = 0usize;
    for (i, (&a, &e)) in actual.iter().zip(expected.iter()).enumerate() {
        let d = (a as f64 - e as f64).abs();
        if d > worst {
            worst = d;
            at = i;
        }
    }
    assert!(
        worst <= tol,
        "{label}: max_abs {worst:.3e} > tol {tol:.1e} at [{at}] (got {:.9}, expected {:.9})",
        actual[at],
        expected[at]
    );
}

#[test]
fn scene_fixtures_match_oracle_stereo_and_foa() {
    let entries = manifest();
    assert_eq!(entries.len(), 3, "expected 3 scene fixtures");

    for entry in entries {
        assert_eq!(entry.metric, "max_abs", "{}: unexpected metric", entry.name);
        assert_eq!(
            entry.layout, "planar-f32le",
            "{}: unexpected layout",
            entry.name
        );

        let scene_path = fixtures_dir().join(&entry.scene);
        let scene =
            SpatialScene::read(&scene_path).unwrap_or_else(|e| panic!("{}: {e}", entry.name));

        let (interleaved, receipt) = scene::render_stereo(&scene, &fixtures_dir())
            .unwrap_or_else(|e| panic!("{}: render_stereo: {e}", entry.name));
        assert_eq!(receipt.renderer, entry.renderer, "{}: renderer", entry.name);
        assert_eq!(receipt.channels, entry.channels, "{}: channels", entry.name);
        let frames = interleaved.len() / entry.channels as usize;
        assert_eq!(frames, entry.frames, "{}: frame count", entry.name);

        let planar = interleaved_to_planar(&interleaved, entry.channels as usize);
        let expected = read_planar_f32(
            &fixtures_dir().join(&entry.stereo),
            entry.channels as usize,
            entry.frames,
        );
        check_max_abs(
            &planar,
            &expected,
            entry.tol,
            &format!("{} stereo", entry.name),
        );
        assert!(
            (receipt.safety_gain_db - entry.safety_gain_db).abs() < 1e-6,
            "{}: safetyGainDb {} != manifest {}",
            entry.name,
            receipt.safety_gain_db,
            entry.safety_gain_db
        );

        if let (Some(foa_file), Some(foa_channels)) = (&entry.foa, entry.foa_channels) {
            let (foa_interleaved, foa_receipt) = scene::render_foa(&scene, &fixtures_dir())
                .unwrap_or_else(|e| panic!("{}: render_foa: {e}", entry.name));
            if let Some(foa_renderer) = &entry.foa_renderer {
                assert_eq!(
                    &foa_receipt.renderer, foa_renderer,
                    "{}: foaRenderer",
                    entry.name
                );
            }
            assert_eq!(
                foa_receipt.channels, foa_channels,
                "{}: foa channels",
                entry.name
            );
            let foa_frames = foa_interleaved.len() / foa_channels as usize;
            assert_eq!(foa_frames, entry.frames, "{}: foa frame count", entry.name);

            let foa_planar = interleaved_to_planar(&foa_interleaved, foa_channels as usize);
            let foa_expected = read_planar_f32(
                &fixtures_dir().join(foa_file),
                foa_channels as usize,
                entry.frames,
            );
            check_max_abs(
                &foa_planar,
                &foa_expected,
                entry.tol,
                &format!("{} foa", entry.name),
            );
        }
    }
}

// ------------------------------------------------------------ invariants

/// A short synthetic mono PCM_16 stem, written under `dir/name` — the
/// on-disk shape every fixture stem is also in (see `gen_scene_fixtures.
/// py`'s `stem()`, which soundfile likewise wrote as PCM_16 despite a
/// `float32` numpy array in, because `sf.write` without an explicit
/// `subtype=` defaults to the *format*'s own default subtype for WAV,
/// not the array dtype).
fn write_pcm16_stem(dir: &Path, name: &str, samples: &[i16]) -> String {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(dir.join(name), spec).expect("create stem wav");
    for &s in samples {
        writer.write_sample(s).expect("write stem sample");
    }
    writer.finalize().expect("finalize stem wav");
    name.to_string()
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "mus-pw7-scene-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn point_object(object_id: &str, audio: &str, position: Position) -> SpatialObject {
    SpatialObject {
        object_id: object_id.to_string(),
        label: object_id.to_string(),
        audio: audio.to_string(),
        start_seconds: 0.0,
        duration_seconds: None,
        kind: "point".to_string(),
        position,
        trajectory: Vec::new(),
        gain_db: 0.0,
        spread: 0.0,
        reverb_send: 0.0,
        r#loop: false,
        muted: false,
        controls: PsychoacousticControls::default(),
        analysis: serde_json::json!({}),
        source_region: None,
        placement_provenance: "composition-authored".to_string(),
        metadata: serde_json::json!({}),
    }
}

fn scene_with(object: SpatialObject, room: Room) -> SpatialScene {
    SpatialScene {
        scene_id: "urn:test:pw7".to_string(),
        title: "PW7 invariant scene".to_string(),
        sample_rate: 48_000,
        duration_seconds: 0.2,
        objects: vec![object],
        listener: Listener::default(),
        room,
        coordinate_system: "+X right, +Y up, -Z front".to_string(),
        metadata: serde_json::json!({}),
        schema: "https://sophia-labs.ai/schemas/mus-spatial-scene/1".to_string(),
    }
}

/// Determinism: the same scene, rendered twice, must byte-for-byte match
/// — the render is a pure function of its inputs, and this exercises the
/// full pipeline (multiple objects, seeded room reverb via `np-rand` +
/// FFT convolution, air absorption) via a real fixture scene rather than
/// a minimal synthetic one.
#[test]
fn same_scene_renders_identically_every_time() {
    let scene_path = fixtures_dir().join("static_room.scene.json");
    let scene = SpatialScene::read(&scene_path).expect("read static_room fixture scene");

    let (first, _) = scene::render_stereo(&scene, &fixtures_dir()).expect("render_stereo (first)");
    let (second, _) =
        scene::render_stereo(&scene, &fixtures_dir()).expect("render_stereo (second)");
    assert_eq!(
        first, second,
        "same scene must render byte-for-byte identical PCM"
    );
}

/// Silence in, silence out: every stage in the object pipeline (peak-
/// safety clamp, gain, air-absorption filter, equal-power pan, distance
/// gain, decorrelation-and-RMS-match) maps an all-zero signal to an
/// all-zero signal exactly — no stage introduces energy from nothing.
/// Room is left at its (enabled) default: a strictly-zero object leaves
/// `room_bus` strictly zero too, so `_synthetic_room` never even fires
/// (the `np.any(room_bus)` guard) — this test's silence covers that path
/// as well, not just the disabled-room case.
#[test]
fn silent_object_renders_exact_silence() {
    let dir = temp_dir("silence");
    let stem = write_pcm16_stem(&dir, "silence.wav", &[0i16; 4800]);

    let object = point_object("s", &stem, Position::spherical(15.0, 5.0, 3.0).unwrap());
    let scene = scene_with(object, Room::default());

    let (interleaved, receipt) = scene::render_stereo(&scene, &dir).expect("render_stereo");
    assert!(
        interleaved.iter().all(|&v| v == 0.0),
        "silent input must render exact silence, got a nonzero sample"
    );
    assert_eq!(receipt.peak_before_safety, 0.0);
    assert_eq!(receipt.output_peak, 0.0);
    assert_eq!(receipt.safety_gain_db, 0.0);

    fs::remove_dir_all(&dir).ok();
}

/// Azimuth mirror symmetry: negating an object's azimuth (same distance,
/// same elevation, room off) swaps the stereo channels — the equal-power
/// pan law is symmetric about center, and every other stage (air
/// absorption, distance gain) here depends only on distance, which is
/// unchanged by the mirror.
#[test]
fn negated_azimuth_swaps_stereo_channels() {
    let dir = temp_dir("mirror");
    let samples: Vec<i16> = (0..4800)
        .map(|i| {
            let t = i as f64 / 48_000.0;
            (8000.0 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()) as i16
        })
        .collect();
    let stem = write_pcm16_stem(&dir, "tone.wav", &samples);

    let right_object = point_object("o", &stem, Position::spherical(30.0, 0.0, 2.0).unwrap());
    let left_object = point_object("o", &stem, Position::spherical(-30.0, 0.0, 2.0).unwrap());
    let right_scene = scene_with(
        right_object,
        Room {
            enabled: false,
            ..Room::default()
        },
    );
    let left_scene = scene_with(
        left_object,
        Room {
            enabled: false,
            ..Room::default()
        },
    );

    let (right_pan, _) = scene::render_stereo(&right_scene, &dir).expect("render +30deg");
    let (left_pan, _) = scene::render_stereo(&left_scene, &dir).expect("render -30deg");
    assert_eq!(right_pan.len(), left_pan.len());

    let mut worst = 0.0f64;
    for frame in 0..(right_pan.len() / 2) {
        let (rl, rr) = (right_pan[frame * 2] as f64, right_pan[frame * 2 + 1] as f64);
        let (ll, lr) = (left_pan[frame * 2] as f64, left_pan[frame * 2 + 1] as f64);
        worst = worst.max((rl - lr).abs()).max((rr - ll).abs());
    }
    assert!(
        worst < 1e-6,
        "mirrored azimuth should swap L/R within tolerance, worst diff {worst:.3e}"
    );
    // Sanity: the pan actually did something (not a degenerate all-silent
    // or perfectly-centered case that would make the symmetry trivial).
    let right_has_signal = right_pan.iter().any(|&v| v.abs() > 1e-4);
    assert!(right_has_signal, "test tone must actually reach the mix");

    fs::remove_dir_all(&dir).ok();
}

/// A single non-default psychoacoustic control refuses the whole render
/// with a typed, named [`SceneError::UnsupportedControl`] — never a
/// silent skip of the (unimplemented) brightness shelf.
#[test]
fn non_default_control_is_a_typed_refusal_not_a_silent_skip() {
    let dir = temp_dir("refusal");
    let stem = write_pcm16_stem(&dir, "tone.wav", &[1000i16; 4800]);

    let mut object = point_object("b", &stem, Position::spherical(0.0, 0.0, 2.0).unwrap());
    object.controls = PsychoacousticControls {
        brightness_db: 3.0,
        ..PsychoacousticControls::default()
    };
    let scene = scene_with(
        object,
        Room {
            enabled: false,
            ..Room::default()
        },
    );

    let err = scene::render_stereo(&scene, &dir).expect_err("non-default control must refuse");
    assert_eq!(err.code(), "unsupported-control");
    match err {
        SceneError::UnsupportedControl {
            control, object_id, ..
        } => {
            assert_eq!(control, "brightnessDb");
            assert_eq!(object_id, "b");
        }
        other => panic!("expected UnsupportedControl, got {other:?}"),
    }

    fs::remove_dir_all(&dir).ok();
}

/// A relative audio path that resolves outside `root` refuses rather
/// than reading arbitrary filesystem paths.
#[test]
fn audio_path_escaping_root_is_refused() {
    let dir = temp_dir("escape");
    let outside = temp_dir("escape-outside");
    write_pcm16_stem(&outside, "secret.wav", &[42i16; 100]);

    let escape_relative = format!(
        "../{}/secret.wav",
        outside.file_name().unwrap().to_string_lossy()
    );
    let object = point_object(
        "e",
        &escape_relative,
        Position::spherical(0.0, 0.0, 2.0).unwrap(),
    );
    let scene = scene_with(
        object,
        Room {
            enabled: false,
            ..Room::default()
        },
    );

    let err = scene::render_stereo(&scene, &dir).expect_err("path escape must be refused");
    assert_eq!(err.code(), "path-escape");

    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&outside).ok();
}
