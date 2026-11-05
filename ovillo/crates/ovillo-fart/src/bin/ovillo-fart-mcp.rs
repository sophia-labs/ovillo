//! `ovillo-fart-mcp`: an MCP server on stdio with a gut. One
//! `TractNdAnatomy::fart_v1` World (`ovillo_tract_world::fart::Gut`) lives
//! for the whole session: `eat` puts food in the stomach, `digest` pays its
//! gas into the World's reservoir (silently, faster than real time),
//! `fart` spends the reservoir through the sphincter, writes the sound to a
//! WAV and plays it (`afplay`, macOS), and `gut` reads the books.
//!
//! ```text
//! ovillo-fart-mcp [--out <dir>] [--silent]
//! ```
//!
//! JSON-RPC 2.0, one message per line, written by hand as `ovillo-mcp`'s:
//! `initialize`, notifications (ignored), `ping`, `tools/list`,
//! `tools/call`. WAVs go to `--out`, else `ovillo-fart` in the per-user
//! temporary directory. `--silent` never plays (the WAV is still written).

use ovillo_tract_world::fart::{self, Gut, TuningCurve};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

const VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
const SR: f64 = 48_000.0;
/// Digesting stops here unless asked for longer (simulated seconds).
const DIGEST_DEFAULT_S: f64 = 30.0;

const INSTRUCTIONS: &str = "\
A gut on Vera's Mac: one physical World (the Ariadne vocal-tract World of site/tract, built on another anatomy, \
TractNdAnatomy::fart_v1) that lives for the whole session. A reservoir of gas (1.5 J when full; it starts full) \
feeds a self-oscillating valve (the sphincter: a mass on a tension spring with contact, a jet with viscous and \
Bernoulli loss) into a 20 cm canal whose 24 modes follow the cheeks; turbulence and exact seals as in the tract. \
eat puts food in the stomach; digest pays its gas into the reservoir (the World books it as source work); fart \
spends it in a style and tune plays a melody (NOTE:beats text), each playing the sound (someone may be listening: ask before playing more than one or two) and returns what the \
World measured. An empty reservoir makes no sound; below 40% the push sags, which is how a fart ends. \
ensemble renders score directories of parts (one gut per part, in parallel), for arrangements. \
Report numbers from the results only. Invented: every anatomy number and every food's gas and time. \
Not modeled: cheeks flapping in the flow, a sphincter as a ring, three-dimensional air, smell.";

fn tools() -> Value {
    let styles: Vec<&str> = fart::STYLES.iter().map(|s| s.name).collect();
    let styles_about: Vec<String> = fart::STYLES.iter().map(|s| format!("{} ({:.1} s): {}", s.name, s.seconds, s.about)).collect();
    let foods: Vec<&str> = fart::FOODS.iter().map(|f| f.name).collect();
    let foods_about: Vec<String> = fart::FOODS.iter().map(|f| format!("{} {:.2} J over {} s", f.name, f.gas_j, f.seconds)).collect();
    json!([
        {
            "name": "eat",
            "title": "Eat",
            "description": format!("Put food in the stomach (it does nothing until digest). Foods, with the gas each will pay into the reservoir and how long it takes (invented numbers): {}. Returns the stomach.", foods_about.join("; ")),
            "inputSchema": {"type": "object", "properties": {"foods": {"type": "array", "items": {"type": "string", "enum": foods}, "minItems": 1, "maxItems": 20}}, "required": ["foods"], "additionalProperties": false}
        },
        {
            "name": "digest",
            "title": "Digest",
            "description": "Digest the stomach into the reservoir: each food pays its gas at gas/time watts (the sum capped at the World's 1 W), simulated silently and faster than real time, until the stomach is empty, the reservoir is full (the rest waits in the stomach), or max_seconds of simulated time pass. Returns the joules the World actually booked, the reservoir's fill, and what is left.",
            "inputSchema": {"type": "object", "properties": {"max_seconds": {"type": "number", "minimum": 0.1, "maximum": 120, "description": "Simulated seconds at most (default 30)."}}, "additionalProperties": false}
        },
        {
            "name": "fart",
            "title": "Fart",
            "description": format!("Release the reservoir in a style, write the sound to a WAV and play it (unless play is false). Styles: {}. Returns the World's measurements: median F0 of the valve's swing, seconds it swung, gas before and after, joules radiated as sound against joules spent, peak level, sealed samples, turbulence, and the ledger's closure residuals.", styles_about.join("; ")),
            "inputSchema": {"type": "object", "properties": {
                "style": {"type": "string", "enum": styles},
                "play": {"type": "boolean", "description": "Play it through the speakers (default true)."},
                "path": {"type": "string", "description": "Write the WAV here instead of the default directory."}
            }, "required": ["style"], "additionalProperties": false},
            "annotations": {"destructiveHint": false, "openWorldHint": false}
        },
        {
            "name": "tune",
            "title": "Play a tune",
            "description": "Play a melody on the sphincter and return how close the World came to each note. notes is text: NOTE:beats tokens separated by spaces, e.g. \"G2:1 G2:1 Eb2:0.75 Bb2:0.25 r:1\" (note names with # or b and an octave, A4 = 440 Hz; a bare number is Hz; r is a rest; a bare note is one beat). Before its first tune the server measures the tuning curve of its tune voice on a separate practice World (its own gas, not the gut's): the valve runs sharp of the commanded tension by up to a few semitones, so each note starts at the tension the curve says lands on it and is corrected by ear (the World's measured F0) while it sounds; the curve's measured range is in the result, and notes outside it are attempted and reported. articulation tongued (default) clenches the cheeks 50 ms before each next note so it starts on a release; legato glides. digest (default true) keeps the stomach paying the reservoir while it plays; with no gas the notes do not sound, and the result says so. Returns per note: target, the World's median measured F0, cents off, how much of it swung.",
            "inputSchema": {"type": "object", "properties": {
                "notes": {"type": "string", "description": "NOTE:beats tokens, e.g. \"Bb2:1 Bb2:0.75 Bb2:0.25 Db3:1\""},
                "bpm": {"type": "number", "minimum": 20, "maximum": 300, "description": "Beats per minute (default 100)."},
                "articulation": {"type": "string", "enum": ["tongued", "legato"]},
                "digest": {"type": "boolean", "description": "Keep digesting while playing (default true)."},
                "play": {"type": "boolean", "description": "Play it through the speakers (default true)."},
                "path": {"type": "string", "description": "Write the WAV here instead of the default directory."}
            }, "required": ["notes"], "additionalProperties": false}
        },
        {
            "name": "ensemble",
            "title": "Render an ensemble",
            "description": "Work on a score directory of fart parts (the ovillo-fart format: score.json with voices and sections; one <section>/<voice>.notes file per part, the player's hands on its own gut: notes G2@84:1 (target G2, the valve's tension commanded at 84 Hz, one beat), r:1 rests, X:1 cannon shots, push= cheeks= tone= wobble= vibrato= tongue= settings, eat= digest= pre= feeding; nothing is fed or tuned for the player). action check parses every part against its section's beats and bar lines and renders nothing. action render renders the selected parts in parallel (unchanged ones from a cache) and reports per part what the World measured: notes within 50 cents of their targets, silent notes, notes started dry, gas at the end, loudness, sharpness, attack pops; then mixes every complete section into out/mix.wav and plays it (or the section) unless play is false. action mix only remixes. Separate from the session's gut: an ensemble never spends it.",
            "inputSchema": {"type": "object", "properties": {
                "dir": {"type": "string", "description": "The score directory (holds score.json)."},
                "action": {"type": "string", "enum": ["check", "render", "mix"]},
                "section": {"type": "string", "description": "Only this section."},
                "voice": {"type": "string", "description": "Only this voice."},
                "force": {"type": "boolean", "description": "Re-render even unchanged parts."},
                "play": {"type": "boolean", "description": "After render or mix, play the result (default true): the section if one is selected, else the whole mix."}
            }, "required": ["dir", "action"], "additionalProperties": false}
        },
        {
            "name": "gut",
            "title": "Read the gut",
            "description": "The gut's state without changing it: reservoir fill and joules, the stomach, and the World's books since the session began (source work = everything food paid, radiation = everything heard, losses, closure residuals).",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false},
            "annotations": {"readOnlyHint": true}
        }
    ])
}

struct Server {
    gut: Gut,
    /// Measured on first use (`TuningCurve::measure`, a practice World).
    curve: Option<TuningCurve>,
    out_dir: PathBuf,
    silent: bool,
    count: u32,
}

fn stomach(gut: &Gut) -> Value {
    json!(gut.stomach().iter().map(|(f, left)| json!({"food": f.name, "gasLeftJ": round(*left, 4)})).collect::<Vec<_>>())
}

fn round(v: f64, digits: i32) -> f64 {
    let k = 10f64.powi(digits);
    (v * k).round() / k
}

fn books(gut: &Gut) -> Value {
    let w = gut.world();
    let r = w.world().receipt();
    let t = w.totals();
    json!({
        "sourceWorkJ": r.source_work, "radiationJ": r.radiation, "internalLossJ": r.internal_loss,
        "jetLossJ": t.valve_jet_loss, "constrictionLossJ": t.aero.jet_loss, "turbulenceInjectedJ": t.aero.turbulence_injected,
        "sphincterLossJ": t.fold_loss, "wallLossJ": t.wall_loss, "controlWorkJ": r.control_work,
        "initialEnergyJ": r.initial_energy, "currentEnergyJ": r.current_energy,
        "maximumFrameResidualJ": r.maximum_frame_residual, "cumulativeClosureResidualJ": r.cumulative_closure_residual,
        "sampleFrames": r.sample_frames,
    })
}

fn gas(gut: &Gut) -> Value {
    let fraction = gut.gas_fraction();
    json!({"fraction": round(fraction, 4), "joules": round(fraction * gut.world().anatomy().lung_capacity_j, 4)})
}

fn write_wav(path: &Path, samples: &[f32]) -> Result<(), String> {
    ovillo_fart::write_wav(path, 1, samples)
}

impl Server {
    fn call(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        match name {
            "eat" => {
                let names = args.get("foods").and_then(Value::as_array).ok_or("foods: a list of food names")?;
                let mut eaten = Vec::new();
                for n in names {
                    let n = n.as_str().ok_or("each food is a name")?;
                    let food = fart::food(n).ok_or_else(|| format!("no food \"{n}\" (the table has {})", fart::FOODS.map(|f| f.name).join(", ")))?;
                    self.gut.eat(food);
                    eaten.push(food.name);
                }
                let coming: f64 = self.gut.stomach().iter().map(|(_, left)| left).sum();
                Ok(json!({"ate": eaten, "stomach": stomach(&self.gut), "gasToComeJ": round(coming, 4), "reservoir": gas(&self.gut),
                    "note": "nothing is paid into the reservoir until digest"}))
            }
            "digest" => {
                if self.gut.stomach().is_empty() {
                    return Ok(json!({"note": "the stomach is empty: eat first", "reservoir": gas(&self.gut)}));
                }
                let max = args.get("max_seconds").and_then(Value::as_f64).unwrap_or(DIGEST_DEFAULT_S).clamp(0.1, 120.0);
                let d = self.gut.digest(max).map_err(|e| format!("the World refused: {e:?}"))?;
                Ok(json!({
                    "simulatedSeconds": round(d.seconds, 2), "paidJ": d.paid_j, "stoppedBecauseFull": d.stopped_full,
                    "reservoir": gas(&self.gut), "stomach": stomach(&self.gut),
                }))
            }
            "fart" => {
                let s = args.get("style").and_then(Value::as_str).ok_or("style is required")?;
                let style = fart::style(s).ok_or_else(|| format!("no style \"{s}\" (styles: {})", fart::STYLES.map(|s| s.name).join(", ")))?;
                let before = self.gut.gas_fraction();
                let w0 = self.gut.world().world().receipt();
                let (spent0, rad0, closed0, turb0) = (w0.internal_loss + w0.radiation, w0.radiation, self.gut.world().totals().aero.closed_samples, self.gut.world().totals().aero.turbulence_injected);
                let clipped0 = self.gut.world().observers().safety_limited;
                let mut f0s = Vec::new();
                let audio = self
                    .gut
                    .release_with(&style, |_, w| {
                        let f0 = w.observers().measured_f0_hz;
                        if f0 > 0.0 {
                            f0s.push(f0);
                        }
                    })
                    .map_err(|e| format!("the World refused: {e:?}"))?;
                f0s.sort_by(f64::total_cmp);
                let path = self.wav_path(args, style.name)?;
                write_wav(&path, &audio)?;
                let play = args.get("play").and_then(Value::as_bool).unwrap_or(true) && !self.silent;
                let played = if play { play_wav(&path) } else { Err("not asked to play".into()) };
                let w = self.gut.world();
                let r = w.world().receipt();
                let t = w.totals();
                let radiated = r.radiation - rad0;
                let spent = r.internal_loss + r.radiation - spent0;
                let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
                let mut out = json!({
                    "style": style.name, "seconds": style.seconds, "wav": path.display().to_string(),
                    "played": played.is_ok(),
                    "medianF0Hz": f0s.get(f0s.len() / 2).map(|f| round(*f, 1)),
                    "swingSeconds": round(f0s.len() as f64 * fart::HOP_S, 2),
                    "gasBefore": round(before, 4), "gasAfter": round(self.gut.gas_fraction(), 4),
                    "radiatedJ": radiated, "spentJ": spent, "soundFractionOfSpent": if spent > 0.0 { Some(radiated / spent) } else { None },
                    "peak": peak, "clippedSamples": w.observers().safety_limited - clipped0,
                    "sealedSamples": t.aero.closed_samples - closed0, "turbulenceInjectedJ": t.aero.turbulence_injected - turb0,
                    "maximumFrameResidualJ": r.maximum_frame_residual, "cumulativeClosureResidualJ": r.cumulative_closure_residual,
                });
                if let Err(why) = played {
                    out["notPlayed"] = json!(why);
                }
                if before < 0.02 {
                    out["note"] = json!("the reservoir was empty: nothing to push. eat and digest first");
                }
                Ok(out)
            }
            "tune" => self.tune(args),
            "ensemble" => self.ensemble(args),
            "gut" => Ok(json!({"reservoir": gas(&self.gut), "stomach": stomach(&self.gut), "books": books(&self.gut)})),
            other => Err(format!("unknown tool \"{other}\"")),
        }
    }
}

impl Server {
    fn wav_path(&mut self, args: &Value, name: &str) -> Result<PathBuf, String> {
        self.count += 1;
        match args.get("path").and_then(Value::as_str) {
            Some(p) => Ok(PathBuf::from(p)),
            None => {
                std::fs::create_dir_all(&self.out_dir).map_err(|e| e.to_string())?;
                Ok(self.out_dir.join(format!("fart-{:03}-{name}.wav", self.count)))
            }
        }
    }

    fn ensemble(&mut self, args: &Value) -> Result<Value, String> {
        let dir = PathBuf::from(args.get("dir").and_then(Value::as_str).ok_or("dir is required")?);
        let score = ovillo_fart::load(&dir)?;
        let filter = ovillo_fart::Filter {
            section: args.get("section").and_then(Value::as_str).map(str::to_owned),
            voice: args.get("voice").and_then(Value::as_str).map(str::to_owned),
        };
        let action = args.get("action").and_then(Value::as_str).unwrap_or("check");
        let mut out = match action {
            "check" => return Ok(ovillo_fart::check(&dir, &score, &filter)),
            "render" => {
                let jobs = std::thread::available_parallelism().map_or(4, |n| n.get());
                let force = args.get("force").and_then(Value::as_bool).unwrap_or(false);
                json!({"render": ovillo_fart::render(&dir, &score, &filter, force, jobs)})
            }
            "mix" => json!({}),
            other => return Err(format!("action is check, render or mix, not \"{other}\"")),
        };
        let mixed = ovillo_fart::mix(&dir, &score, filter.section.as_deref())?;
        let listen = match &filter.section {
            Some(s) => dir.join("out").join("sections").join(format!("{s}.wav")),
            None => PathBuf::from(mixed["mix"].as_str().unwrap_or_default()),
        };
        out["mix"] = mixed;
        let play = args.get("play").and_then(Value::as_bool).unwrap_or(true) && !self.silent;
        out["played"] = json!(if play && listen.exists() { play_wav(&listen).map(|()| listen.display().to_string()).ok() } else { None });
        Ok(out)
    }

    fn tune(&mut self, args: &Value) -> Result<Value, String> {
        let text = args.get("notes").and_then(Value::as_str).ok_or("notes is required: \"G2:1 Eb2:0.75 Bb2:0.25\"")?;
        let bpm = args.get("bpm").and_then(Value::as_f64).unwrap_or(100.0);
        let notes = fart::parse_tune(text, bpm)?;
        let legato = match args.get("articulation").and_then(Value::as_str) {
            None | Some("tongued") => false,
            Some("legato") => true,
            Some(other) => return Err(format!("articulation is tongued or legato, not \"{other}\"")),
        };
        let digest = args.get("digest").and_then(Value::as_bool).unwrap_or(true);
        if self.curve.is_none() {
            self.curve = Some(TuningCurve::measure(SR).map_err(|e| format!("the practice World refused: {e:?}"))?);
        }
        let curve = self.curve.as_ref().expect("measured above");
        let (lo, hi) = curve.range();
        let before = self.gut.gas_fraction();
        let r0 = self.gut.world().world().receipt();
        let clipped0 = self.gut.world().observers().safety_limited;
        let (audio, reports) = self.gut.play_tune(&notes, curve, legato, digest).map_err(|e| format!("the World refused: {e:?}"))?;
        let path = self.wav_path(args, "tune")?;
        write_wav(&path, &audio)?;
        let play = args.get("play").and_then(Value::as_bool).unwrap_or(true) && !self.silent;
        let played = if play { play_wav(&path) } else { Err("not asked to play".into()) };
        let tokens = text.split(|c: char| c.is_whitespace() || c == ',').filter(|t| !t.is_empty());
        let per_note: Vec<Value> = tokens
            .zip(&reports)
            .map(|(tok, r)| {
                json!({
                    "note": tok, "targetHz": round(r.target_hz, 1),
                    "measuredHz": r.measured_hz.map(|m| round(m, 1)), "cents": r.cents.map(|c| round(c, 0)),
                    "swung": round(r.voiced_fraction, 2),
                    "outsideRange": r.target_hz > 0.0 && (r.target_hz < lo || r.target_hz > hi),
                })
            })
            .collect();
        let pitched: Vec<&fart::NoteReport> = reports.iter().filter(|r| r.target_hz > 0.0).collect();
        let cents: Vec<f64> = pitched.iter().filter_map(|r| r.cents).map(f64::abs).collect();
        let in_tune = cents.iter().filter(|c| **c <= 50.0).count();
        let silent_notes = pitched.iter().filter(|r| r.measured_hz.is_none()).count();
        let w = self.gut.world();
        let r = w.world().receipt();
        let mut out = json!({
            "notes": per_note,
            "inTune50Cents": format!("{in_tune}/{}", pitched.len()),
            "meanAbsCents": if cents.is_empty() { None } else { Some(round(cents.iter().sum::<f64>() / cents.len() as f64, 1)) },
            "tuningRangeHz": [round(lo, 1), round(hi, 1)],
            "seconds": round(audio.len() as f64 / SR, 2), "bpm": bpm, "articulation": if legato { "legato" } else { "tongued" },
            "wav": path.display().to_string(), "played": played.is_ok(),
            "gasBefore": round(before, 4), "gasAfter": round(self.gut.gas_fraction(), 4), "stomach": stomach(&self.gut),
            "paidDuringJ": r.source_work - r0.source_work, "radiatedJ": r.radiation - r0.radiation,
            "clippedSamples": w.observers().safety_limited - clipped0,
            "maximumFrameResidualJ": r.maximum_frame_residual, "cumulativeClosureResidualJ": r.cumulative_closure_residual,
        });
        if let Err(why) = played {
            out["notPlayed"] = json!(why);
        }
        if silent_notes > 0 {
            out["note"] = json!(format!("{silent_notes} note(s) did not sound: the valve did not swing (out of gas, or out of range)"));
        }
        Ok(out)
    }
}

fn play_wav(path: &Path) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("playing needs afplay (macOS); the WAV is written".into());
    }
    std::process::Command::new("afplay").arg(path).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map(|_| ()).map_err(|e| format!("afplay: {e}"))
}

fn text_result(v: &Value, is_error: bool) -> Value {
    let text = match v {
        Value::String(s) => s.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    };
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

/// Answer one JSON-RPC message; `None` for notifications.
fn handle(server: &mut Server, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned()?;
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or(VERSIONS[0]);
            let version = if VERSIONS.contains(&asked) { asked } else { VERSIONS[0] };
            Ok(json!({
                "protocolVersion": version,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "ovillo-fart", "title": "Ovillo Fart", "version": env!("CARGO_PKG_VERSION")},
                "instructions": INSTRUCTIONS,
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools()})),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            Ok(match server.call(name, &args) {
                Ok(v) => text_result(&v, false),
                Err(e) => text_result(&json!(e), true),
            })
        }
        "" => Err((-32600, "invalid request: no method".to_string())),
        other => Err((-32601, format!("method not found: {other}"))),
    };
    Some(match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err((code, message)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}),
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("ovillo-fart-mcp [--out <dir>] [--silent]  (MCP server on stdio: a gut that eats, digests and farts)");
        return;
    }
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).map(String::as_str);
    let out_dir = value("--out").map_or_else(|| std::env::temp_dir().join("ovillo-fart"), PathBuf::from);
    let gut = match Gut::new(SR) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("ovillo-fart-mcp: the World refused its plan: {e:?}");
            std::process::exit(1);
        }
    };
    let mut server = Server { gut, curve: None, out_dir, silent: args.iter().any(|a| a == "--silent"), count: 0 };
    eprintln!("ovillo-fart-mcp: WAVs to {}", server.out_dir.display());
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let replies: Vec<Value> = match serde_json::from_str::<Value>(&line) {
            Err(e) => vec![json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}})],
            Ok(Value::Array(batch)) => batch.iter().filter_map(|m| handle(&mut server, m)).collect(),
            Ok(msg) => handle(&mut server, &msg).into_iter().collect(),
        };
        for r in replies {
            if writeln!(out, "{r}").and_then(|()| out.flush()).is_err() {
                return;
            }
        }
    }
}
