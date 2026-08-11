//! `mus service` — the warm render engine as a local process.
//!
//! The Playable Graph's S1 seam (`SPECS/playable/README.md`): a host (the
//! Atril vite dev server, a test harness, any tool) spawns this once and
//! speaks line-delimited JSON-RPC over stdin/stdout. The engine stays hot —
//! parsed graphs, decoded packs, and rendered PCM are cached in-process —
//! so audition requests land in milliseconds instead of process-spawn
//! seconds. Stdout is protocol-pure (one JSON object per line); every
//! diagnostic goes to stderr.
//!
//! Protocol v0 (this file, the keystone): `ping`, `load`, `renderScore`.
//! PW1 extends it with `renderWindow`/`renderEvent`/`renderDiff`/`peaks`
//! and the cache/GC discipline — see `SPECS/playable/PW1-service.md` for
//! the window-exactness law (`window(a,b) == full[a..b]`, bit-exact) that
//! governs every extension.
//!
//! Determinism is the contract that makes this service honest: the same
//! `docKey` always yields byte-identical PCM (the engine is sample-exact
//! and content-keyed), so caching is not an optimization heuristic — it is
//! memoization of a pure function.
//!
//! PCM transfer: responses carry an absolute path to a raw file the
//! service owns — interleaved stereo f32le at the engine rate — plus its
//! frame count. Callers stream the bytes; the service garbage-collects its
//! own tmpdir (v0: everything lives until exit; PW1 adds the LRU budget).
//! Base64-in-JSON was rejected up front: a 33% tax on every audition, paid
//! at the latency-critical edge, for no gain over a loopback file read.

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use mus_dsp::backbone::{ProtectedBackbone, BACKBONE_MODES};
use mus_dsp::pluck::{ControlFrame, ControlTarget, GestureFrame, StringNetworkVoice};
use mus_engine::{render, sha256_bytes, ENGINE_VERSION};
use mus_watsn::{
    state_energy, Complex64, ComplexGate, ComplexOperatorProgram, ComplexPhaseGate,
    ComplexTwoModeGate,
};
use serde::Serialize;
use serde_json::{json, Value};

/// Parse a compiled complex word from the committed-artifact schedule
/// format. Validation (finiteness, unitarity, index range) happens in
/// `ComplexOperatorProgram::new` — the U1 gates, not re-derived here.
fn parse_complex_word(value: &Value) -> Result<ComplexOperatorProgram, ErrorBody> {
    let entries = value.as_array().ok_or_else(|| ErrorBody {
        code: "bad-request",
        message: "braid word must be an array of schedule gates".into(),
    })?;
    let mut gates = Vec::with_capacity(entries.len());
    for gate in entries {
        let parsed = match gate["kind"].as_str() {
            Some("phase_shift") => ComplexGate::Phase(ComplexPhaseGate {
                mode: gate["i"].as_u64().unwrap_or(u64::MAX) as usize,
                phase: gate["phase"].as_f64().unwrap_or(f64::NAN),
            }),
            Some("two_mode_unitary") => {
                let block = &gate["block"];
                let entry = |row: usize, column: usize| {
                    Complex64::new(
                        block[row][column][0].as_f64().unwrap_or(f64::NAN),
                        block[row][column][1].as_f64().unwrap_or(f64::NAN),
                    )
                };
                ComplexGate::TwoMode(ComplexTwoModeGate {
                    left: gate["i"].as_u64().unwrap_or(u64::MAX) as usize,
                    right: gate["j"].as_u64().unwrap_or(u64::MAX) as usize,
                    matrix: [entry(0, 0), entry(0, 1), entry(1, 0), entry(1, 1)],
                })
            }
            other => {
                return Err(ErrorBody {
                    code: "bad-request",
                    message: format!("braid word gate kind {other:?} is not recognised"),
                })
            }
        };
        gates.push(parsed);
    }
    ComplexOperatorProgram::new(
        BACKBONE_MODES,
        gates,
        None,
        ComplexOperatorProgram::DEFAULT_UNITARITY_TOLERANCE,
    )
    .map_err(|error| ErrorBody {
        code: "bad-request",
        message: format!("braid word failed U1 validation: {error:?}"),
    })
}

pub const SERVICE_VERSION: &str = "mus-service/0.1.0";

struct LoadedDoc {
    source: String,
    base_dir: PathBuf,
}

struct RenderedEntry {
    pcm_path: PathBuf,
    frames: usize,
    sample_rate: u32,
    peak_dbfs: f32,
    rms_dbfs: f32,
    render_digest: String,
    render_ms: u128,
    event_count: u32,
    skipped_count: u32,
    bad_tokens: Vec<String>,
    warnings: Vec<String>,
    duration_seconds: f64,
}

/// A memoized `renderScene` result (PW7). Keyed by content
/// (`sceneJson`+`root`), same discipline as [`RenderedEntry`]/[`Service::
/// render_score`] — the render is a pure function of those two inputs.
struct SceneRenderedEntry {
    pcm_path: PathBuf,
    frames: usize,
    channels: u16,
    sample_rate: u32,
    render_digest: String,
    render_ms: u128,
    receipt: Value,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

struct Service {
    docs: HashMap<String, LoadedDoc>,
    renders: HashMap<String, RenderedEntry>,
    probes: HashMap<String, Value>,
    scene_renders: HashMap<String, SceneRenderedEntry>,
    tmp_dir: PathBuf,
}

impl Service {
    fn new() -> Result<Self, String> {
        let tmp_dir = std::env::temp_dir().join(format!("mus-service-{}", std::process::id()));
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| format!("create tmpdir {}: {e}", tmp_dir.display()))?;
        Ok(Service {
            docs: HashMap::new(),
            renders: HashMap::new(),
            probes: HashMap::new(),
            scene_renders: HashMap::new(),
            tmp_dir,
        })
    }

    fn handle(&mut self, method: &str, params: &Value) -> Result<Value, ErrorBody> {
        match method {
            "ping" => Ok(json!({
                "ok": true,
                "service": SERVICE_VERSION,
                "engine": ENGINE_VERSION,
                // Announced so hosts can validate that any pcmPath they
                // are asked to stream lives inside the service's own
                // tmpdir — never stream arbitrary filesystem paths.
                "tmpDir": self.tmp_dir.to_string_lossy(),
            })),
            "load" => self.load(params),
            "renderScore" => self.render_score(params),
            "vocab" => Ok(crate::vocab_dump::dump()),
            "renderWeaveProbe" => self.render_weave_probe(params),
            "renderBraidProbe" => self.render_braid_probe(params),
            "renderScene" => self.render_scene(params),
            other => Err(ErrorBody {
                code: "unknown-method",
                message: format!(
                    "no method {other:?} (protocol v0: ping, load, renderScore, vocab, \
                     renderWeaveProbe, renderBraidProbe, renderScene)"
                ),
            }),
        }
    }

    /// `renderWeaveProbe {params, midis, slotSeconds?, glissRatio?,
    /// traceStride?, trajectory?, gesture?}` — one string-network event
    /// through the voice itself (no score, no mix/master), returning mono
    /// PCM plus the frame-major telemetry the Observatory draws:
    /// per-course energy, per-edge applied |angle|, orbit phase. Memoized
    /// by the canonical request — the engine is deterministic, so
    /// identical probes return identical entries.
    ///
    /// `gesture: [{atSeconds, courses: [a, b], theta}]` (Ariadne Phase 2
    /// §B) schedules a compiled control word: explicit plane rotations on
    /// the scattering ring's own wave variables. It is a probe argument
    /// like `trajectory`, not a score param — no vocab entry — and it
    /// folds into the memo key for the same reason. Non-weave synths get
    /// `unsupported-gesture`; a word naming a course the network does not
    /// have gets the engine's typed refusal verbatim.
    fn render_weave_probe(&mut self, params: &Value) -> Result<Value, ErrorBody> {
        let midis = params["midis"].as_array().ok_or_else(|| ErrorBody {
            code: "bad-request",
            message: "renderWeaveProbe: params.midis (array of numbers) is required".into(),
        })?;
        let freqs: Vec<f64> = midis
            .iter()
            .filter_map(Value::as_f64)
            .map(|midi| 440.0 * 2_f64.powf((midi - 69.0) / 12.0))
            .collect();
        if freqs.is_empty() {
            return Err(ErrorBody {
                code: "bad-request",
                message: "renderWeaveProbe: params.midis must contain at least one number".into(),
            });
        }
        let slot_s = params["slotSeconds"]
            .as_f64()
            .unwrap_or(1.5)
            .clamp(0.05, 8.0);
        let gliss_ratio = params["glissRatio"]
            .as_f64()
            .unwrap_or(1.0)
            .clamp(0.25, 4.0);
        let stride = params["traceStride"]
            .as_u64()
            .unwrap_or(256)
            .clamp(32, 4096) as usize;
        let mut patch: BTreeMap<String, String> = BTreeMap::new();
        if let Some(map) = params["params"].as_object() {
            for (key, value) in map {
                let text = match value {
                    Value::String(text) => text.clone(),
                    Value::Number(number) => number.to_string(),
                    Value::Bool(flag) => (if *flag { "1" } else { "0" }).to_string(),
                    other => other.to_string(),
                };
                patch.insert(key.clone(), text);
            }
        }
        let synth = patch.get("synth").map(String::as_str).unwrap_or("weave");
        if synth != "weave" && synth != "pluck" {
            return Err(ErrorBody {
                code: "unsupported-synth",
                message: format!(
                    "renderWeaveProbe: synth {synth:?} is not a string-network voice (weave, pluck)"
                ),
            });
        }

        let mut trajectory: Vec<ControlFrame> = Vec::new();
        if let Some(frames) = params["trajectory"].as_array() {
            for frame in frames {
                let at_s = frame["atSeconds"].as_f64().unwrap_or(0.0).clamp(0.0, 600.0);
                let mut sets = Vec::new();
                if let Some(map) = frame["set"].as_object() {
                    for (name, value) in map {
                        let target = ControlTarget::from_name(name).ok_or_else(|| ErrorBody {
                            code: "unsupported-automation",
                            message: format!(
                                "renderWeaveProbe: {name:?} is not block-automatable \
                                 (vocab automation classes name the legal set)"
                            ),
                        })?;
                        let Some(number) = value.as_f64() else {
                            return Err(ErrorBody {
                                code: "bad-request",
                                message: format!(
                                    "renderWeaveProbe: trajectory value for {name:?} must be a number"
                                ),
                            });
                        };
                        sets.push((target, number));
                    }
                }
                trajectory.push(ControlFrame {
                    at_sample: (at_s * 48_000.0).round() as usize,
                    sets,
                });
            }
        }

        // Ariadne Phase 2 §B: an optional compiled control word. Same
        // shape of argument as `trajectory` — content, not a score param
        // (there is no vocab entry for `gesture`; a word is authored as
        // frames, exactly as automation is).
        let mut gestures: Vec<GestureFrame> = Vec::new();
        // Absent is a word of no gates; anything else that is not a list
        // of gates is a refusal. A dropped word would render a probe the
        // caller did not ask for and hand it back as if it had.
        if !params["gesture"].is_null() && !params["gesture"].is_array() {
            return Err(ErrorBody {
                code: "bad-request",
                message: "renderWeaveProbe: params.gesture must be an array of \
                          {atSeconds, courses: [a, b], theta} gates"
                    .into(),
            });
        }
        if let Some(frames) = params["gesture"].as_array() {
            if !frames.is_empty() && synth != "weave" {
                return Err(ErrorBody {
                    code: "unsupported-gesture",
                    message: format!(
                        "renderWeaveProbe: gestures are the Weave scattering ring's seam; \
                         synth {synth:?} has no ring to rotate"
                    ),
                });
            }
            for frame in frames {
                let at_s = frame["atSeconds"].as_f64().unwrap_or(0.0).clamp(0.0, 600.0);
                let courses = frame["courses"].as_array().ok_or_else(|| ErrorBody {
                    code: "bad-request",
                    message: "renderWeaveProbe: gesture.courses (array of two course indices) \
                              is required"
                        .into(),
                })?;
                if courses.len() != 2 {
                    return Err(ErrorBody {
                        code: "bad-request",
                        message: format!(
                            "renderWeaveProbe: gesture.courses must name exactly two courses, \
                             got {}",
                            courses.len()
                        ),
                    });
                }
                let mut pair = [0_usize; 2];
                for (slot, value) in pair.iter_mut().zip(courses.iter()) {
                    // Negative and fractional indices are refused here
                    // rather than rounded: a course index is an identity,
                    // not a knob.
                    *slot = value.as_u64().ok_or_else(|| ErrorBody {
                        code: "bad-request",
                        message: format!(
                            "renderWeaveProbe: gesture course index {value} must be a \
                             non-negative integer"
                        ),
                    })? as usize;
                }
                let Some(theta) = frame["theta"].as_f64() else {
                    return Err(ErrorBody {
                        code: "bad-request",
                        message: "renderWeaveProbe: gesture.theta (number, radians) is required"
                            .into(),
                    });
                };
                gestures.push(GestureFrame {
                    at_sample: (at_s * 48_000.0).round() as usize,
                    course_a: pair[0],
                    course_b: pair[1],
                    theta,
                });
            }
        }

        let canonical = json!({
            "params": patch,
            "midis": midis,
            "slotSeconds": slot_s,
            "glissRatio": gliss_ratio,
            "traceStride": stride,
            "trajectory": params["trajectory"],
            // The word is part of the render's identity, exactly like the
            // trajectory: two probes that differ only by a gate are two
            // different renders and must not share a memo entry.
            "gesture": params["gesture"],
        });
        let probe_key = sha256_bytes(canonical.to_string().as_bytes());
        if !self.probes.contains_key(&probe_key) {
            let started = Instant::now();
            let voice = if synth == "pluck" {
                StringNetworkVoice::guitar(&patch, &freqs, slot_s, gliss_ratio)
            } else {
                StringNetworkVoice::weave(&patch, &freqs, slot_s, gliss_ratio)
            };
            // One entry point for both cases: an empty word is the
            // byte-identical zero limit of a word (mus-dsp Phase 2 §B
            // law 1), so the un-gestured probe is not a different render
            // path that could drift from this one.
            let (audio, trace, gesture_trace) = voice
                .render_traced_gestured(stride, &trajectory, &gestures)
                .map_err(|error| ErrorBody {
                    code: error.code(),
                    message: format!("renderWeaveProbe: {error}"),
                })?;
            let render_ms = started.elapsed().as_millis();

            let pcm_path = self.tmp_dir.join(format!("{probe_key}.probe.f32"));
            let mut bytes = Vec::with_capacity(audio.len() * 4);
            for sample in &audio {
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
            let render_digest = sha256_bytes(&bytes);
            std::fs::write(&pcm_path, &bytes).map_err(|e| ErrorBody {
                code: "render-failed",
                message: format!("write {}: {e}", pcm_path.display()),
            })?;

            let trace_path = self.tmp_dir.join(format!("{probe_key}.trace.f32"));
            let mut trace_bytes = Vec::with_capacity(trace.data.len() * 4);
            for value in &trace.data {
                trace_bytes.extend_from_slice(&value.to_le_bytes());
            }
            std::fs::write(&trace_path, &trace_bytes).map_err(|e| ErrorBody {
                code: "render-failed",
                message: format!("write {}: {e}", trace_path.display()),
            })?;

            let entry = json!({
                "probeKey": probe_key,
                "engine": ENGINE_VERSION,
                "renderMs": render_ms,
                "renderDigest": render_digest,
                "pcm": {
                    "path": pcm_path.to_string_lossy(),
                    "frames": audio.len(),
                    "channels": 1,
                    "sampleRate": 48_000,
                    "layout": "mono-f32le",
                },
                "trace": {
                    "path": trace_path.to_string_lossy(),
                    "frames": trace.frames,
                    "stride": trace.stride,
                    "courses": trace.courses,
                    "played": trace.played,
                    "courseFreqs": trace.course_freqs,
                    "values": ["energy", "edgeActivity", "orbitPhase"],
                    "layout": "frame-major-f32le",
                },
                // The word's receipt: how many gates actually executed
                // (one scheduled past the end of the note does not), and
                // the worst relative pair-energy drift over them — the
                // Phase 2 §B law-3 measurement, reported rather than
                // asserted away.
                "gesture": {
                    "gates": gestures.len(),
                    "applied": gesture_trace.applied.len(),
                    "worstPairEnergyDrift": gesture_trace.worst_pair_energy_drift(),
                },
            });
            self.probes.insert(probe_key.clone(), entry);
        }
        Ok(self.probes[&probe_key].clone())
    }

    /// `renderBraidProbe {params, events, couple?, eta?, prepare?}` —
    /// unified U3's audible face: ONE persistent sixteen-mode protected
    /// backbone across a sequence of weave events. Each event renders
    /// through the production voice with the backbone bridged at the
    /// gesture seam; between events the backbone's room port keeps
    /// radiating into the timeline (the braid rings between notes).
    ///
    /// `events: [{midis, atSeconds, slotSeconds, word?, wordSpanSeconds?}]`
    /// — `word` is a compiled complex gate list in the committed-artifact
    /// schedule format (`{kind: "phase_shift", i, phase}` /
    /// `{kind: "two_mode_unitary", i, j, block}`), taken as data exactly
    /// like the gesture seam takes its frames. Events must be
    /// non-overlapping and time-ordered; overlap is a refusal, not a mix
    /// (`UNIFIED-U3-persistent-backbone.md` §3).
    fn render_braid_probe(&mut self, params: &Value) -> Result<Value, ErrorBody> {
        let events = params["events"].as_array().ok_or_else(|| ErrorBody {
            code: "bad-request",
            message: "renderBraidProbe: params.events (array) is required".into(),
        })?;
        if events.is_empty() {
            return Err(ErrorBody {
                code: "bad-request",
                message: "renderBraidProbe: at least one event is required".into(),
            });
        }
        let couple = params["couple"].as_f64().unwrap_or(0.02).clamp(0.0, 0.3);
        let eta = params["eta"].as_f64().unwrap_or(0.002).clamp(0.0, 0.05);
        let mut patch: BTreeMap<String, String> = BTreeMap::new();
        if let Some(map) = params["params"].as_object() {
            for (key, value) in map {
                let text = match value {
                    Value::String(text) => text.clone(),
                    Value::Number(number) => number.to_string(),
                    Value::Bool(flag) => (if *flag { "1" } else { "0" }).to_string(),
                    other => other.to_string(),
                };
                patch.insert(key.clone(), text);
            }
        }
        let synth = patch.get("synth").map(String::as_str).unwrap_or("weave");
        if synth != "weave" {
            return Err(ErrorBody {
                code: "unsupported-synth",
                message: "renderBraidProbe: the backbone bridges the Weave ring only".into(),
            });
        }

        let mut backbone =
            ProtectedBackbone::new(4, couple, eta).map_err(|error| ErrorBody {
                code: "bad-request",
                message: format!("renderBraidProbe: {error:?}"),
            })?;
        if let Some(prepare) = params["prepare"].as_array() {
            if prepare.len() != BACKBONE_MODES {
                return Err(ErrorBody {
                    code: "bad-request",
                    message: format!(
                        "renderBraidProbe: prepare must list {BACKBONE_MODES} [re, im] pairs"
                    ),
                });
            }
            let mut state = [Complex64::ZERO; BACKBONE_MODES];
            for (slot, pair) in state.iter_mut().zip(prepare.iter()) {
                let (Some(re), Some(im)) = (pair[0].as_f64(), pair[1].as_f64()) else {
                    return Err(ErrorBody {
                        code: "bad-request",
                        message: "renderBraidProbe: prepare entries must be [re, im] numbers"
                            .into(),
                    });
                };
                *slot = Complex64::new(re, im);
            }
            backbone.prepare(&state);
        }
        backbone.open_frame();

        // Parse and validate every event before rendering anything.
        struct BraidEvent {
            start_sample: u64,
            slot_samples: usize,
            freqs: Vec<f64>,
            word: Option<(ComplexOperatorProgram, u64)>,
        }
        let mut parsed: Vec<BraidEvent> = Vec::with_capacity(events.len());
        for event in events {
            let midis = event["midis"].as_array().ok_or_else(|| ErrorBody {
                code: "bad-request",
                message: "renderBraidProbe: event.midis (array of numbers) is required".into(),
            })?;
            let freqs: Vec<f64> = midis
                .iter()
                .filter_map(Value::as_f64)
                .map(|midi| 440.0 * 2_f64.powf((midi - 69.0) / 12.0))
                .collect();
            if freqs.is_empty() {
                return Err(ErrorBody {
                    code: "bad-request",
                    message: "renderBraidProbe: event.midis must contain a number".into(),
                });
            }
            let at_s = event["atSeconds"].as_f64().unwrap_or(0.0).clamp(0.0, 600.0);
            let slot_s = event["slotSeconds"].as_f64().unwrap_or(1.5).clamp(0.05, 8.0);
            let word = if event["word"].is_null() {
                None
            } else {
                let gates = parse_complex_word(&event["word"])?;
                let span_s = event["wordSpanSeconds"]
                    .as_f64()
                    .unwrap_or(slot_s * 0.6)
                    .clamp(0.01, 8.0);
                Some((gates, (span_s * 48_000.0).round() as u64))
            };
            parsed.push(BraidEvent {
                start_sample: (at_s * 48_000.0).round() as u64,
                slot_samples: (slot_s * 48_000.0).round() as usize,
                freqs,
                word,
            });
        }
        parsed.sort_by_key(|event| event.start_sample);
        for window in parsed.windows(2) {
            let end = window[0].start_sample + window[0].slot_samples as u64;
            if window[1].start_sample < end {
                return Err(ErrorBody {
                    code: "block-schedule-ambiguous",
                    message: format!(
                        "renderBraidProbe: events overlap (one ends at sample {end}, the next \
                         starts at {}); eager rendering cannot interleave two sample loops \
                         over one backbone",
                        window[1].start_sample
                    ),
                });
            }
        }

        let total_samples = parsed
            .last()
            .map(|event| event.start_sample as usize + event.slot_samples)
            .unwrap_or(0)
            + 96_000; // two seconds of backbone ring-out
        let mut timeline = vec![0.0_f32; total_samples];
        let slot_s_key = json!(params).to_string();

        for event in &parsed {
            // silent gap up to this event: the backbone keeps radiating
            let gap = event.start_sample.saturating_sub(backbone.absolute_sample());
            if gap > 0 {
                let from = backbone.absolute_sample() as usize;
                let until = (from + gap as usize).min(timeline.len());
                backbone.advance_gap(gap, &mut timeline[from..until]);
            }
            if let Some((word, span)) = &event.word {
                backbone
                    .arm_word(word, event.start_sample, *span)
                    .map_err(|error| ErrorBody {
                        code: "block-schedule-ambiguous",
                        message: format!("renderBraidProbe: {error:?}"),
                    })?;
            }
            let slot_s = event.slot_samples as f64 / 48_000.0;
            let voice = StringNetworkVoice::weave(&patch, &event.freqs, slot_s, 1.0);
            let audio = voice.render_bridged(&mut backbone);
            let start = event.start_sample as usize;
            for (offset, sample) in audio.iter().enumerate() {
                if let Some(slot) = timeline.get_mut(start + offset) {
                    *slot += sample;
                }
            }
        }
        // final ring-out
        let from = backbone.absolute_sample() as usize;
        if from < timeline.len() {
            let remaining = (timeline.len() - from) as u64;
            backbone.advance_gap(remaining, &mut timeline[from..]);
        }
        let frame = backbone.close_frame();

        let probe_key = sha256_bytes(slot_s_key.as_bytes());
        let pcm_path = self.tmp_dir.join(format!("{probe_key}.braid.f32"));
        let mut bytes = Vec::with_capacity(timeline.len() * 4);
        for sample in &timeline {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let render_digest = sha256_bytes(&bytes);
        std::fs::write(&pcm_path, &bytes).map_err(|e| ErrorBody {
            code: "render-failed",
            message: format!("write {}: {e}", pcm_path.display()),
        })?;

        let final_state: Vec<[f64; 2]> = backbone
            .state()
            .iter()
            .map(|value| [value.re, value.im])
            .collect();
        Ok(json!({
            "probeKey": probe_key,
            "engine": ENGINE_VERSION,
            "renderDigest": render_digest,
            "pcm": {
                "path": pcm_path.to_string_lossy(),
                "frames": timeline.len(),
                "channels": 1,
                "sampleRate": 48_000,
                "layout": "mono-f32le",
            },
            "backbone": {
                "finalState": final_state,
                "finalEnergy": state_energy(backbone.state()),
                "ledger": {
                    "energyBefore": frame.energy_before,
                    "energyAfter": frame.energy_after,
                    "bridgeInjected": frame.bridge_injected,
                    "radiated": frame.radiated,
                    "retired": frame.retired,
                    "transportWork": frame.transport_work,
                    "closureError": frame.closure_error(),
                },
            },
        }))
    }

    /// `load {source, baseDir}` → parse + adopt once, cache under the
    /// content-addressed `docKey`. Loading is cheap and idempotent — the
    /// key IS the identity, so re-loading identical content is a no-op
    /// that returns the same key.
    fn load(&mut self, params: &Value) -> Result<Value, ErrorBody> {
        let source = params["source"].as_str().ok_or_else(|| ErrorBody {
            code: "bad-request",
            message: "load: params.source (string) is required".into(),
        })?;
        let base_dir = params["baseDir"].as_str().ok_or_else(|| ErrorBody {
            code: "bad-request",
            message: "load: params.baseDir (string) is required".into(),
        })?;

        let doc_key = sha256_bytes(format!("{source}\u{0}{base_dir}").as_bytes());
        // Parse eagerly so a refused score surfaces at `load`, not as a
        // surprise at first render. Refusal is a result: the diagnostics
        // travel in the response verbatim, structured.
        let (_proposal, diags) = mus_text::parse_score_lossy(source);
        let parse_diagnostics = serde_json::to_value(&diags).unwrap_or(Value::Null);

        self.docs.insert(
            doc_key.clone(),
            LoadedDoc {
                source: source.to_string(),
                base_dir: PathBuf::from(base_dir),
            },
        );
        Ok(json!({
            "docKey": doc_key,
            "engine": ENGINE_VERSION,
            "parseDiagnostics": parse_diagnostics,
        }))
    }

    /// `renderScore {docKey}` → the full piece through the whole pipeline,
    /// memoized. First call renders and writes `<docKey>.f32`; every later
    /// call returns the cached entry instantly.
    fn render_score(&mut self, params: &Value) -> Result<Value, ErrorBody> {
        let doc_key = params["docKey"].as_str().ok_or_else(|| ErrorBody {
            code: "bad-request",
            message: "renderScore: params.docKey (string) is required".into(),
        })?;
        if !self.renders.contains_key(doc_key) {
            let doc = self.docs.get(doc_key).ok_or_else(|| ErrorBody {
                code: "unknown-doc",
                message: format!("renderScore: no loaded doc {doc_key:?} — call load first"),
            })?;
            let started = Instant::now();
            let outcome = render(&doc.source, &doc.base_dir).map_err(|e| ErrorBody {
                code: "render-failed",
                message: e.to_string(),
            })?;
            let render_ms = started.elapsed().as_millis();

            let pcm_path = self.tmp_dir.join(format!("{doc_key}.f32"));
            let mut bytes = Vec::with_capacity(outcome.audio.samples.len() * 4);
            for s in &outcome.audio.samples {
                bytes.extend_from_slice(&s.to_le_bytes());
            }
            let render_digest = sha256_bytes(&bytes);
            std::fs::write(&pcm_path, &bytes).map_err(|e| ErrorBody {
                code: "render-failed",
                message: format!("write {}: {e}", pcm_path.display()),
            })?;

            let entry = RenderedEntry {
                frames: outcome.audio.frames(),
                sample_rate: outcome.audio.sample_rate,
                peak_dbfs: outcome.audio.peak_dbfs(),
                rms_dbfs: outcome.audio.rms_dbfs(),
                render_digest,
                render_ms,
                event_count: outcome.stats.notes,
                skipped_count: outcome.stats.skipped,
                bad_tokens: outcome.stats.bad.clone(),
                warnings: outcome.stats.warnings.clone(),
                duration_seconds: outcome.stats.total_seconds,
                pcm_path,
            };
            self.renders.insert(doc_key.to_string(), entry);
        }
        let e = &self.renders[doc_key];
        Ok(json!({
            "pcmPath": e.pcm_path.to_string_lossy(),
            "frames": e.frames,
            "channels": 2,
            "sampleRate": e.sample_rate,
            "layout": "interleaved-f32le",
            "peakDbfs": e.peak_dbfs,
            "rmsDbfs": e.rms_dbfs,
            "renderDigest": e.render_digest,
            "renderMs": e.render_ms,
            "eventCount": e.event_count,
            "skippedCount": e.skipped_count,
            "badTokens": e.bad_tokens,
            "warnings": e.warnings,
            "durationSeconds": e.duration_seconds,
        }))
    }

    /// `renderScene {sceneJson, root}` (PW7) → the spatial scene renderer,
    /// memoized by content exactly like [`Service::render_score`]:
    /// `sha256(sceneJson \0 root)` is the cache key, first call renders
    /// and writes `<key>.f32`, later calls are instant cache hits. Same
    /// PCM-envelope response shape as `renderScore` — `pcmPath`/`frames`/
    /// `channels`/`sampleRate`/`layout`/`renderDigest`/`renderMs` — with
    /// the spatial render receipt attached under `receipt` in place of
    /// `renderScore`'s score-specific stats fields (event/skip counts
    /// etc. have no scene-rendering analog). Stereo only, matching that
    /// fixed shape (`channels`/`layout` are always 2 / `interleaved-
    /// f32le`) — FOA is a CLI-only (`--foa`, to-file) output, not wired
    /// through this method.
    fn render_scene(&mut self, params: &Value) -> Result<Value, ErrorBody> {
        let scene_json = params["sceneJson"].as_str().ok_or_else(|| ErrorBody {
            code: "bad-request",
            message: "renderScene: params.sceneJson (string) is required".into(),
        })?;
        let root = params["root"].as_str().ok_or_else(|| ErrorBody {
            code: "bad-request",
            message: "renderScene: params.root (string) is required".into(),
        })?;

        let key = sha256_bytes(format!("{scene_json}\u{0}{root}").as_bytes());
        if !self.scene_renders.contains_key(&key) {
            let scene: mus_dsp::scene::SpatialScene =
                serde_json::from_str(scene_json).map_err(|e| ErrorBody {
                    code: "bad-scene-json",
                    message: e.to_string(),
                })?;
            let started = Instant::now();
            let (interleaved, receipt) = mus_dsp::scene::render_stereo(&scene, Path::new(root))
                .map_err(|e| ErrorBody {
                    code: e.code(),
                    message: e.to_string(),
                })?;
            let render_ms = started.elapsed().as_millis();

            let pcm_path = self.tmp_dir.join(format!("{key}.f32"));
            let mut bytes = Vec::with_capacity(interleaved.len() * 4);
            for s in &interleaved {
                bytes.extend_from_slice(&s.to_le_bytes());
            }
            let render_digest = sha256_bytes(&bytes);
            std::fs::write(&pcm_path, &bytes).map_err(|e| ErrorBody {
                code: "render-failed",
                message: format!("write {}: {e}", pcm_path.display()),
            })?;

            let entry = SceneRenderedEntry {
                frames: interleaved.len() / 2,
                channels: 2,
                sample_rate: receipt.sample_rate,
                render_digest,
                render_ms,
                receipt: serde_json::to_value(&receipt).unwrap_or(Value::Null),
                pcm_path,
            };
            self.scene_renders.insert(key.clone(), entry);
        }
        let e = &self.scene_renders[&key];
        Ok(json!({
            "pcmPath": e.pcm_path.to_string_lossy(),
            "frames": e.frames,
            "channels": e.channels,
            "sampleRate": e.sample_rate,
            "layout": "interleaved-f32le",
            "renderDigest": e.render_digest,
            "renderMs": e.render_ms,
            "receipt": e.receipt,
        }))
    }
}

/// The NDJSON loop. Malformed lines get an `id: null` error response
/// rather than killing the process — a confused client deserves an
/// answer, not a dead engine.
pub fn run_service() -> Result<(), String> {
    let mut service = Service::new()?;
    eprintln!(
        "[mus-service] up: {} / {} (tmp {})",
        SERVICE_VERSION,
        ENGINE_VERSION,
        service.tmp_dir.display()
    );
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("stdin: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => {
                let id = request["id"].clone();
                let method = request["method"].as_str().unwrap_or("");
                match service.handle(method, &request["params"]) {
                    Ok(result) => json!({ "id": id, "result": result }),
                    Err(err) => json!({ "id": id, "error": err }),
                }
            }
            Err(e) => json!({
                "id": Value::Null,
                "error": ErrorBody { code: "bad-request", message: format!("unparseable request line: {e}") },
            }),
        };
        serde_json::to_writer(&mut out, &response).map_err(|e| format!("stdout: {e}"))?;
        out.write_all(b"\n").map_err(|e| format!("stdout: {e}"))?;
        out.flush().map_err(|e| format!("stdout: {e}"))?;
    }
    Ok(())
}
