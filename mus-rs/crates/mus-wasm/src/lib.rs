//! MUS in the browser: check a score and render it on its Ariadne Worlds,
//! behind a plain C ABI so a Worker can `WebAssembly.instantiate` the module
//! from bytes with no bindgen glue and no imports.
//!
//! **Ariadne only.** The browser plays Ariadne Worlds and nothing else: an
//! instrument with `anatomy=steel-string-v0` (the built-in dreadnought)
//! played by its dot-voice hands. Every other track (`synth=`, `voice=`,
//! `sample=`, a sample pack, the weave voice, an instrument without an
//! anatomy) is refused with a diagnostic and never rendered.
//!
//! The render is `mus render`'s, from the same code: `mus_engine::StagedRender`
//! (which `mus_engine::render` itself runs), stepped a block of samples at a
//! time so the page can show progress, and finished into the same PCM_24 WAV
//! bytes, the same `mus.audio.render-receipt.v1` (renderDigest, ariadneWorlds[]
//! and their ledgers) and the same `mus.ariadne.energy-trace.v1`.
//!
//! The C ABI (all lengths in bytes; text is UTF-8 JSON in one shared buffer):
//!
//! | export | does |
//! |---|---|
//! | `mus_input_ptr(len) -> *mut u8` | room for `len` bytes of input; write the score there |
//! | `mus_text_ptr() -> *const u8`, `mus_text_len() -> u32` | the last JSON text an export produced |
//! | `mus_info() -> len` | `{engineVersion, sampleRate, anatomies}` |
//! | `mus_check(len) -> len` | check the input score: `ovillo.mus-wasm.check.v1` |
//! | `mus_render_begin(len) -> i32` | 0 started; 1 refused (text: the check); 2 failed (text: `{error}`) |
//! | `mus_render_total() -> f64`, `mus_render_done() -> f64` | World samples in all, and simulated so far |
//! | `mus_render_step(n) -> i32` | simulate up to `n` samples: 0 more to do, 1 all done, 2 failed (text) |
//! | `mus_render_finish(wall_seconds) -> i32` | 0 done (text: the receipt), 2 failed (text) |
//! | `mus_render_frames()`, `mus_render_samples_ptr()` | the finished audio: interleaved stereo f32 at 48 kHz |
//! | `mus_render_wav_ptr()`, `mus_render_wav_len()` | the PCM_24 WAV `renderDigest` digests |
//! | `mus_render_energy() -> len` | text: the energy trace |
//! | `mus_render_reset()` | drop the render and its audio |
//!
//! A call that returns a length or pointer may grow memory: take views after it.
//!
//! **Native and browser renders.** Compiled natively, this path is `mus render` to the byte (`mus-cli`'s
//! `wasm_parity` test). Compiled to wasm32, the transcendental functions (`sin`, `powf`, `log2`, `exp`) are Rust's
//! own libm instead of the platform's, which rounds a few results differently in the last bit. Measured on the
//! steel-string tour and its first four bars (node, 2026-10-20): every ledger figure agrees with the native receipt to
//! within 2e-12 relative (the closure residual, itself ~1e-16, to 3e-3 of itself); `planDigest` and `listenerDigest`
//! differ, because the anatomy's retuning (`apply_hands`) runs through those functions before the plan is hashed;
//! the PCM_24 WAV was bit-identical for the full tour and not for the four bars. A browser receipt is a true
//! receipt of the browser's render, not a copy of a native one.
//!
//! Build: `crates/mus-wasm/build.sh [out.wasm]` (simd128 on); measure: `node crates/mus-wasm/rtf.mjs <wasm> <score> [native receipt]`.

use std::cell::RefCell;
use std::path::Path;

use mus_engine::receipt::{energy_trace_json, RenderReceipt};
use mus_engine::tab_check::{self, TabAnchor, TabReport};
use mus_engine::{
    sha256_bytes, wav_bytes, RenderOutcome, StagedRender, ENGINE_VERSION, SAMPLE_RATE,
};
use serde::Serialize;
use serde_json::{json, Value};

/// The anatomies compiled into the engine; nothing is read from a file system.
pub const ANATOMIES: &[&str] = &["steel-string-v0"];

/// Instrument parameters that make a track something other than an Ariadne World.
const NOT_ARIADNE_PARAMS: &[&str] = &[
    "synth", "voice", "sample", "osc2", "mix2", "detune", "sub", "cutoff", "famt", "fdec", "satk",
    "sdec", "ssus", "srel",
];
/// Headers that bring in audio that is not an Ariadne World's.
const NOT_ARIADNE_HEADERS: &[(&str, &str)] = &[
    ("pack", "a sample pack"),
    ("tape", "a tape"),
    ("gestures", "quoted gestures"),
];
const ONLY_ARIADNE: &str = "Ovillo plays Ariadne Worlds only: give the instrument anatomy=steel-string-v0 and write its notes on its hands";

/// Where a diagnostic sits in the score text, in UTF-16 code units (what a
/// JavaScript string and a ProseMirror text position count). `line` is 1-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Span {
    pub from: usize,
    pub to: usize,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diagnostic {
    pub severity: &'static str,
    pub code: String,
    pub message: String,
    pub anchor: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

/// The browser's verdict on a score: `clean` means it renders.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckReport {
    pub schema: &'static str,
    pub producer: &'static str,
    pub engine_version: &'static str,
    pub clean: bool,
    pub source_digest: String,
    pub score: Value,
    pub worlds: Vec<tab_check::TabInstrumentSummary>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckReport {
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(|d| d.severity == "error")
    }
}

/// Check a score as the browser renders it: parse, the Ariadne-only rule,
/// then SPEC-TAB §6 (`mus_engine::tab_check`, the checks `mus check` runs).
pub fn check(source: &str) -> CheckReport {
    let (proposal, parse_diags) = mus_text::parse_score_lossy(source);
    let index = SourceIndex::new(source);
    let mut diagnostics = Vec::new();
    for d in parse_diags {
        // Tokens nobody can parse are reported below, anchored to their bar and hand.
        if d.code == "unknown-token" {
            continue;
        }
        let error = matches!(d.code.as_str(), "unparseable-token" | "bad-header-value");
        diagnostics.push(Diagnostic {
            severity: if error { "error" } else { "warning" },
            code: d.code,
            message: d.message,
            anchor: json!({"kind": "score"}),
            span: None,
        });
    }

    // ---- Ariadne only
    for (key, what) in NOT_ARIADNE_HEADERS {
        if proposal.headers.contains_key(*key) {
            diagnostics.push(Diagnostic {
                severity: "error",
                code: "not-ariadne".into(),
                message: format!(
                    "# {key}: brings in {what}, not an Ariadne World. {ONLY_ARIADNE}."
                ),
                anchor: json!({"kind": "header", "key": key}),
                span: index.header(key),
            });
        }
    }
    let bases: Vec<&mus_text::ProposalInstrument> = proposal
        .instruments
        .iter()
        .filter(|i| i.params.contains_key("anatomy"))
        .collect();
    for inst in &proposal.instruments {
        let base = bases.iter().find(|b| {
            inst.abbrev == b.abbrev
                || inst
                    .abbrev
                    .strip_prefix(b.abbrev.as_str())
                    .is_some_and(|r| r.starts_with('.'))
        });
        let refuse = |code: &str, message: String| Diagnostic {
            severity: "error",
            code: code.into(),
            message,
            anchor: json!({"kind": "instrument", "abbrev": inst.abbrev}),
            span: index.instrument(&inst.abbrev),
        };
        let foreign: Vec<String> = NOT_ARIADNE_PARAMS
            .iter()
            .filter_map(|k| inst.params.get(*k).map(|v| format!("{k}={v}")))
            .collect();
        match base {
            None => {
                let what = if let Some(v) = inst.params.get("synth") {
                    if matches!(v.trim(), "weave" | "pluck") {
                        format!("the {} voice (synth={v})", v.trim())
                    } else {
                        format!("a synth (synth={v})")
                    }
                } else if let Some(v) = inst.params.get("voice") {
                    format!("a sample-pack voice (voice={v})")
                } else if let Some(v) = inst.params.get("sample") {
                    format!("a sample (sample={v})")
                } else {
                    "a track with no anatomy=".to_owned()
                };
                diagnostics.push(refuse(
                    "not-ariadne",
                    format!(
                        "{} = {} is {what}, not an Ariadne World. {ONLY_ARIADNE}.",
                        inst.abbrev, inst.name
                    ),
                ));
            }
            Some(base) => {
                if !foreign.is_empty() {
                    diagnostics.push(refuse(
                        "not-ariadne",
                        format!(
                            "{} declares {}: an Ariadne World's hands are pos=, pick=, pm=, gain= only. {ONLY_ARIADNE}.",
                            inst.abbrev,
                            foreign.join(", ")
                        ),
                    ));
                }
                if inst.abbrev == base.abbrev {
                    let anatomy = base.params["anatomy"].trim();
                    if !ANATOMIES.contains(&anatomy) {
                        diagnostics.push(refuse(
                            "anatomy-unavailable",
                            format!(
                                "anatomy={anatomy}: the browser has only the built-in {} (an anatomy file is not read here)",
                                ANATOMIES.join(", ")
                            ),
                        ));
                    }
                }
            }
        }
    }

    // ---- SPEC-TAB §6, only on what the browser could read (a refused anatomy is not read)
    let readable = !diagnostics.iter().any(|d| d.code == "anatomy-unavailable");
    let mut score =
        json!({"title": proposal.title, "bars": Value::Null, "durationSeconds": Value::Null});
    let mut worlds = Vec::new();
    if readable {
        match tab_check::check(source, Path::new(".")) {
            Ok(TabReport {
                bars,
                duration_seconds,
                instruments,
                diagnostics: tab,
            }) => {
                score["bars"] = json!(bars);
                score["durationSeconds"] = json!(duration_seconds);
                worlds = instruments;
                for d in tab {
                    let span = match &d.anchor {
                        TabAnchor::Event {
                            bar,
                            track,
                            token_index,
                            token,
                        } => index.event(*bar, track, *token_index, token),
                        TabAnchor::Instrument { abbrev } => index.instrument(abbrev),
                    };
                    diagnostics.push(Diagnostic {
                        severity: d.severity,
                        code: d.code.into(),
                        message: d.message,
                        anchor: serde_json::to_value(&d.anchor).unwrap_or(Value::Null),
                        span,
                    });
                }
            }
            Err(e) => diagnostics.push(Diagnostic {
                severity: "error",
                code: "bad-score".into(),
                message: e.to_string(),
                anchor: json!({"kind": "score"}),
                span: None,
            }),
        }
    }
    if proposal.instruments.is_empty() {
        diagnostics.push(Diagnostic {
            severity: "error",
            code: "no-instruments".into(),
            message: format!("the score declares no instruments (# instruments:). {ONLY_ARIADNE}."),
            anchor: json!({"kind": "score"}),
            span: None,
        });
    }
    let clean = !diagnostics.iter().any(|d| d.severity == "error");
    CheckReport {
        schema: "ovillo.mus-wasm.check.v1",
        producer: "mus-wasm",
        engine_version: ENGINE_VERSION,
        clean,
        source_digest: format!("sha256:{}", sha256_bytes(source.as_bytes())),
        score,
        worlds,
        diagnostics,
    }
}

/// Why a render did not start.
#[derive(Debug)]
pub enum Refusal {
    /// The check found errors (a track that is not an Ariadne World, a tab position that disagrees, ...).
    Check(Box<CheckReport>),
    /// The engine refused the score.
    Engine(String),
}

/// A render in progress.
pub struct Render {
    source: String,
    staged: StagedRender,
}

/// A finished render: the audio, the WAV bytes it digests to, the receipt and the energy trace.
pub struct Rendered {
    pub outcome: RenderOutcome,
    pub wav: Vec<u8>,
    pub receipt: RenderReceipt,
}

impl Rendered {
    pub fn receipt_json(&self) -> String {
        serde_json::to_string(&self.receipt)
            .unwrap_or_else(|e| json!({"error": e.to_string()}).to_string())
    }

    pub fn energy_json(&self) -> String {
        energy_trace_json(&self.outcome.stats).to_string()
    }
}

impl Render {
    /// Check, then set up every World. A score that does not check clean is refused.
    pub fn begin(source: &str) -> Result<Render, Refusal> {
        let report = check(source);
        if !report.clean {
            return Err(Refusal::Check(Box::new(report)));
        }
        let staged = StagedRender::begin(source, Path::new("."))
            .map_err(|e| Refusal::Engine(e.to_string()))?;
        Ok(Render {
            source: source.to_owned(),
            staged,
        })
    }

    pub fn total(&self) -> usize {
        self.staged.world_samples()
    }

    pub fn done(&self) -> usize {
        self.staged.world_samples_done()
    }

    /// Simulate up to `samples` more; true when every World has run to the end.
    pub fn step(&mut self, samples: usize) -> Result<bool, String> {
        self.staged.step(samples).map_err(|e| e.to_string())
    }

    /// Finish (running what is left), digest the WAV, and write the receipt.
    /// `wall_seconds` is measured by the caller (there is no clock in wasm32).
    pub fn finish(self, wall_seconds: f64) -> Result<Rendered, String> {
        let outcome = self.staged.finish().map_err(|e| e.to_string())?;
        let wav = wav_bytes(&outcome.audio).map_err(|e| e.to_string())?;
        let receipt = RenderReceipt::for_render(&self.source, &outcome, &wav, wall_seconds);
        Ok(Rendered {
            outcome,
            wav,
            receipt,
        })
    }
}

/// Byte offsets of the score's lines, and lookups from anchors to spans.
struct SourceIndex<'a> {
    source: &'a str,
    /// (byte start, text without the line break) per line.
    lines: Vec<(usize, &'a str)>,
}

impl<'a> SourceIndex<'a> {
    fn new(source: &'a str) -> Self {
        let mut lines = Vec::new();
        let mut at = 0;
        for line in source.split_inclusive('\n') {
            lines.push((at, line.trim_end_matches(['\n', '\r'])));
            at += line.len();
        }
        Self { source, lines }
    }

    fn utf16(&self, byte: usize) -> usize {
        self.source[..byte].encode_utf16().count()
    }

    fn span(&self, line: usize, from: usize, to: usize) -> Span {
        Span {
            from: self.utf16(from),
            to: self.utf16(to),
            line: line + 1,
        }
    }

    /// `# key: value` lines.
    fn header(&self, key: &str) -> Option<Span> {
        self.lines
            .iter()
            .enumerate()
            .find_map(|(n, (start, text))| {
                let body = text.trim_start().strip_prefix('#')?.trim_start();
                let rest = body.strip_prefix(key)?;
                if !rest.trim_start().starts_with(':') {
                    return None;
                }
                let from = start + (text.len() - text.trim_start().len());
                Some(self.span(n, from, start + text.len()))
            })
    }

    /// An `#   abbrev = name (...)` line in the instruments block: the abbrev.
    fn instrument(&self, abbrev: &str) -> Option<Span> {
        self.lines
            .iter()
            .enumerate()
            .find_map(|(n, (start, text))| {
                let after_hash = text.trim_start().strip_prefix('#')?;
                let body = after_hash.trim_start();
                let rest = body.strip_prefix(abbrev)?;
                if !rest.trim_start().starts_with('=') {
                    return None;
                }
                let from = start + (text.len() - body.len());
                Some(self.span(n, from, from + abbrev.len()))
            })
    }

    /// The token a `{bar, track, tokenIndex, token}` anchor names, on the last
    /// `bar`/`bars` line that covers the bar (as the renderer reads it).
    fn event(&self, bar: u32, track: &str, token_index: usize, token: &str) -> Option<Span> {
        let (n, start, text) = self
            .lines
            .iter()
            .enumerate()
            .filter_map(|(n, (start, text))| {
                let (from, to) = bar_range(text)?;
                (from <= bar && bar <= to).then_some((n, *start, *text))
            })
            .next_back()?;
        let colon = text.find(':')?;
        let content = &text[colon + 1..];
        // The track's segment: `track=` at a token boundary, up to the next `x=` that names another track.
        let mut seg_start = None;
        for (i, _) in content.match_indices('=') {
            let mut s = i;
            while s > 0 && !content.as_bytes()[s - 1].is_ascii_whitespace() {
                s -= 1;
            }
            if &content[s..i] == track {
                seg_start = Some(i + 1);
                break;
            }
        }
        let seg_start = seg_start?;
        let seg = &content[seg_start..];
        let seg_end = next_assignment(seg).unwrap_or(seg.len());
        let seg = &seg[..seg_end];
        let base = start + colon + 1 + seg_start;
        let pieces = source_tokens(seg);
        let token = token.trim();
        if token.is_empty() {
            let (from, to) = pieces.first().copied().unwrap_or((0, seg.len()));
            return Some(self.span(n, base + from, base + to));
        }
        // The same text may come out of a pattern repetition several times; the
        // k-th occurrence in the walk is the k-th in the source, clamped to the last.
        let hits: Vec<(usize, usize)> = pieces
            .iter()
            .filter_map(|&(a, b)| {
                seg[a..b]
                    .find(token)
                    .map(|off| (a + off, a + off + token.len()))
            })
            .collect();
        let (from, to) = match hits.len() {
            0 => pieces
                .get(token_index)
                .copied()
                .or_else(|| pieces.last().copied())?,
            _ => hits[occurrence(self, bar, track, token_index, token).min(hits.len() - 1)],
        };
        Some(self.span(n, base + from, base + to))
    }
}

/// How many tokens before `token_index` in the track's walked token list have the same text.
fn occurrence(
    index: &SourceIndex<'_>,
    bar: u32,
    track: &str,
    token_index: usize,
    token: &str,
) -> usize {
    let content = index
        .lines
        .iter()
        .filter_map(|(_, text)| {
            let (from, to) = bar_range(text)?;
            (from <= bar && bar <= to).then_some(*text)
        })
        .next_back()
        .and_then(|text| text.split_once(':').map(|(_, c)| c))
        .unwrap_or("");
    let walked = segment_tokens(content, track);
    walked
        .iter()
        .take(token_index)
        .filter(|t| t.trim() == token)
        .count()
}

fn segment_tokens(content: &str, track: &str) -> Vec<String> {
    for (i, _) in content.match_indices('=') {
        let mut s = i;
        while s > 0 && !content.as_bytes()[s - 1].is_ascii_whitespace() {
            s -= 1;
        }
        if &content[s..i] == track {
            let seg = &content[i + 1..];
            let seg = &seg[..next_assignment(seg).unwrap_or(seg.len())];
            return mus_notation_walk(seg);
        }
    }
    Vec::new()
}

fn mus_notation_walk(seg: &str) -> Vec<String> {
    use mus_notation::{expand_pattern_repetitions, split_attached_dynamics, tokenize_bar_content};
    split_attached_dynamics(&expand_pattern_repetitions(&tokenize_bar_content(seg)))
}

/// `bar N[...]:` / `bars N-M[...]:` → (N, M).
fn bar_range(line: &str) -> Option<(u32, u32)> {
    let body = line.trim();
    let after = body
        .strip_prefix("bar ")
        .or_else(|| body.strip_prefix("bars "))?;
    let head = &after[..after.find(':')?];
    let range = head.split('[').next()?.trim();
    match range.split_once('-') {
        Some((a, b)) => Some((a.trim().parse().ok()?, b.trim().parse().ok()?)),
        None => {
            let n = range.parse().ok()?;
            Some((n, n))
        }
    }
}

/// Byte offset of the next `abbrev=` assignment at a token boundary (another track's segment).
fn next_assignment(seg: &str) -> Option<usize> {
    let bytes = seg.as_bytes();
    let mut i = 0;
    let mut depth = 0i32;
    let mut token_start = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'[' | b'<' | b'{' | b'(' => depth += 1,
            b']' | b'>' | b'}' | b')' => depth -= 1,
            b' ' | b'\t' => token_start = i + 1,
            b'=' if depth <= 0 => {
                let name = &seg[token_start..i];
                if !name.is_empty()
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_')
                {
                    return Some(token_start);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Whitespace-separated tokens with byte ranges, keeping `<...>` chords, `{...}` groups and quoted lyrics whole.
fn source_tokens(seg: &str) -> Vec<(usize, usize)> {
    let chars: Vec<(usize, char)> = seg.char_indices().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].1.is_whitespace() {
            i += 1;
            continue;
        }
        let start = chars[i].0;
        let (mut angle, mut brace, mut quote) = (0usize, 0usize, false);
        while i < chars.len() {
            let c = chars[i].1;
            if quote {
                quote = c != '"';
                i += 1;
                continue;
            }
            match c {
                '"' => quote = true,
                '<' => angle += 1,
                '>' if angle > 0 => angle -= 1,
                '{' => brace += 1,
                '}' if brace > 0 => brace -= 1,
                _ => {}
            }
            if c.is_whitespace() && angle == 0 && brace == 0 {
                break;
            }
            i += 1;
        }
        let end = chars.get(i).map(|c| c.0).unwrap_or(seg.len());
        out.push((start, end));
    }
    out
}

// ---------------------------------------------------------------- the C ABI

#[derive(Default)]
struct State {
    input: Vec<u8>,
    text: Vec<u8>,
    render: Option<Render>,
    rendered: Option<Rendered>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with_state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn set_text(state: &mut State, text: String) -> u32 {
    state.text = text.into_bytes();
    state.text.len() as u32
}

fn input_text(state: &State, len: u32) -> Result<String, String> {
    let bytes = state
        .input
        .get(..len as usize)
        .ok_or("input is shorter than the length given")?;
    String::from_utf8(bytes.to_vec()).map_err(|e| format!("the score is not UTF-8: {e}"))
}

fn error_json(message: &str) -> String {
    json!({"error": message}).to_string()
}

/// Room for `len` bytes of input (a score, UTF-8).
#[no_mangle]
pub extern "C" fn mus_input_ptr(len: u32) -> *mut u8 {
    with_state(|s| {
        s.input.resize(len as usize, 0);
        s.input.as_mut_ptr()
    })
}

#[no_mangle]
pub extern "C" fn mus_text_ptr() -> *const u8 {
    with_state(|s| s.text.as_ptr())
}

#[no_mangle]
pub extern "C" fn mus_text_len() -> u32 {
    with_state(|s| s.text.len() as u32)
}

#[no_mangle]
pub extern "C" fn mus_info() -> u32 {
    with_state(|s| {
        let info = json!({
            "schema": "ovillo.mus-wasm.info.v1",
            "engineVersion": ENGINE_VERSION,
            "sampleRate": SAMPLE_RATE,
            "anatomies": ANATOMIES,
            "ariadneOnly": true,
        });
        set_text(s, info.to_string())
    })
}

#[no_mangle]
pub extern "C" fn mus_check(len: u32) -> u32 {
    with_state(|s| {
        let text = match input_text(s, len) {
            Ok(source) => serde_json::to_string(&check(&source))
                .unwrap_or_else(|e| error_json(&e.to_string())),
            Err(e) => error_json(&e),
        };
        set_text(s, text)
    })
}

/// 0 started; 1 refused (text: the check report); 2 failed (text: `{"error"}`).
#[no_mangle]
pub extern "C" fn mus_render_begin(len: u32) -> i32 {
    with_state(|s| {
        s.render = None;
        s.rendered = None;
        let source = match input_text(s, len) {
            Ok(source) => source,
            Err(e) => {
                set_text(s, error_json(&e));
                return 2;
            }
        };
        match Render::begin(&source) {
            Ok(render) => {
                s.render = Some(render);
                set_text(s, json!({"started": true}).to_string());
                0
            }
            Err(Refusal::Check(report)) => {
                set_text(
                    s,
                    serde_json::to_string(&report).unwrap_or_else(|e| error_json(&e.to_string())),
                );
                1
            }
            Err(Refusal::Engine(e)) => {
                set_text(s, error_json(&e));
                2
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn mus_render_total() -> f64 {
    with_state(|s| s.render.as_ref().map_or(0.0, |r| r.total() as f64))
}

#[no_mangle]
pub extern "C" fn mus_render_done() -> f64 {
    with_state(|s| s.render.as_ref().map_or(0.0, |r| r.done() as f64))
}

/// 0 more to do; 1 every World has run to the end; 2 failed (text).
#[no_mangle]
pub extern "C" fn mus_render_step(samples: u32) -> i32 {
    with_state(|s| {
        let Some(render) = s.render.as_mut() else {
            set_text(s, error_json("no render in progress"));
            return 2;
        };
        match render.step(samples as usize) {
            Ok(false) => 0,
            Ok(true) => 1,
            Err(e) => {
                s.render = None;
                set_text(s, error_json(&e));
                2
            }
        }
    })
}

/// 0 done (text: the receipt JSON); 2 failed (text).
#[no_mangle]
pub extern "C" fn mus_render_finish(wall_seconds: f64) -> i32 {
    with_state(|s| {
        let Some(render) = s.render.take() else {
            set_text(s, error_json("no render in progress"));
            return 2;
        };
        match render.finish(wall_seconds) {
            Ok(done) => {
                let receipt = done.receipt_json();
                s.rendered = Some(done);
                set_text(s, receipt);
                0
            }
            Err(e) => {
                set_text(s, error_json(&e));
                2
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn mus_render_frames() -> u32 {
    with_state(|s| {
        s.rendered
            .as_ref()
            .map_or(0, |r| r.outcome.audio.frames() as u32)
    })
}

#[no_mangle]
pub extern "C" fn mus_render_samples_ptr() -> *const f32 {
    with_state(|s| {
        s.rendered
            .as_ref()
            .map_or(std::ptr::null(), |r| r.outcome.audio.samples.as_ptr())
    })
}

#[no_mangle]
pub extern "C" fn mus_render_wav_ptr() -> *const u8 {
    with_state(|s| {
        s.rendered
            .as_ref()
            .map_or(std::ptr::null(), |r| r.wav.as_ptr())
    })
}

#[no_mangle]
pub extern "C" fn mus_render_wav_len() -> u32 {
    with_state(|s| s.rendered.as_ref().map_or(0, |r| r.wav.len() as u32))
}

#[no_mangle]
pub extern "C" fn mus_render_energy() -> u32 {
    with_state(|s| {
        let text = s
            .rendered
            .as_ref()
            .map_or_else(|| error_json("no finished render"), |r| r.energy_json());
        set_text(s, text)
    })
}

#[no_mangle]
pub extern "C" fn mus_render_reset() {
    with_state(|s| {
        s.render = None;
        s.rendered = None;
        s.input = Vec::new();
        s.text = Vec::new();
    })
}

#[cfg(test)]
mod tests;
