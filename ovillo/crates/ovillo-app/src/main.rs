//! Ovillo — a bench instrument for playing a guitar through the Ariadne World
//! and watching what happens inside it.
//!
//! One window drawn as a 90s instrument (after Tesitura's interface doctrine):
//! black screens behind fixed 10 × 8 graticules, stroke traces only, colour as
//! meaning, annotation on the edges, eight softkeys on the right bezel.
//!
//! * The upper screen is a triggered scope (dry DI and Ariadne's output) or a
//!   spectrum analyzer with Ariadne's 24 body modes drawn as stems at their
//!   current frequencies and the guitar's six strings marked on the top edge.
//! * The lower left screen is a strip chart of the World's own energies:
//!   material, reservoir, shared space.
//! * The lower right square is the configuration map (drag: horizontal is
//!   Material Organization, string to bore; vertical is a parameter of your
//!   choice), or an XY display of the output's left and right channels.
//!
//! The guitar is simulated for now (`ovillo_dsp::guitar`); keys Z X C V B N
//! pluck the open strings over whatever the pattern plays. The live patch is
//! `patches/live.json`, written as you play and re-read when edited on disk.
//! A control socket ([`control`]) lets a Claude session drive all of it
//! through the `ovillo-mcp` server.
//!
//! ```text
//! ovillo [--keys "<key> <key> …"] [--socket <path>|off] [--mute] [--no-activate]
//!        [--input <device>] [--output <device>] [--input-gain <dB>] [--rate <Hz>]
//! ```
//! `--keys` presses keys through the same path as the keyboard, at startup
//! (e.g. `--keys "5 5 6 6 escape"` opens the TAB and the CHORD BOX).
//! `--socket` puts the control socket somewhere other than the per-user
//! default (`$OVILLO_SOCKET` does the same; `off` disables it). `--mute`
//! (or `OVILLO_MUTE=1`) starts with the master muted, silent from the first
//! sample. `--no-activate` opens the window without taking focus. `--input`
//! plays a guitar from an audio interface (a substring of its name) instead
//! of the simulated one; `--output` picks the output device; `--rate` (or
//! `OVILLO_RATE`) runs the devices and the rig at that sample rate.

mod analysis;
mod audio;
mod control;
mod menu;
mod patch;
mod remote;

use analysis::{trigger, History, Spectrum, WorldHistory, WORLD_SPAN_S};
use gpui::{
    canvas, div, point, prelude::*, px, quad, rems, rgb, size, App, BorderStyle, Bounds, ClickEvent, ContentMask,
    Context, FocusHandle, Hsla, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder,
    Pixels, Point, Rems, ScrollDelta, ScrollWheelEvent, SharedString, TitlebarOptions, Window, WindowBounds,
    WindowOptions,
};
use gpui_platform::application;
use menu::{Action, Active, Menu, Softkey, View, AMP_DIVS, MAP_Y, TIME_DIVS_MS};
use ovillo_dsp::ariadne::{AriadneControls, PARAMS};
use ovillo_dsp::guitar::{GuitarSettings, Pickup, STRINGS, STRING_NAMES};
use ovillo_dsp::player::{Event, Pattern};
use ovillo_dsp::rig::{master_gain, Command, ScopeFrame, Source, Telemetry, MASTER_MAX_DB, MASTER_MIN_DB};
use ovillo_dsp::score::Score;
use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::time::{Instant, SystemTime};

embed_plist::embed_info_plist!("../Info.plist");

const BUNDLE_ID: &str = "dev.sophia.ovillo";
const DESIGN_W: f32 = 1240.0;
const DESIGN_H: f32 = 750.0;
const MIN_SCALE: f32 = 0.5;
const UPPER_H: f32 = 300.0;
const LOWER_H: f32 = 212.0;
const LABEL_W: f32 = 52.0;
const RIGHT_W: f32 = 40.0;
const BEZEL_W: f32 = 118.0;
const FONT: &str = "Menlo";
const TEXT_PX: f32 = 12.0;
const SMALL_PX: f32 = 10.5;

// ---- the palette: Okabe–Ito hues, colour is meaning ----
const BLACK: u32 = 0x000000;
const GRAT: u32 = 0x2a2a2a;
const GRAT_AXIS: u32 = 0x4a4a4a;
const TEXT: u32 = 0xc8c8c8;
const BRIGHT: u32 = 0xffffff;
const DIM: u32 = 0x555555;
const DRY: u32 = 0x56b4e9; // sky blue: the guitar as it leaves the jack
const WET: u32 = 0xe69f00; // orange: what Ariadne radiates
const BODY: u32 = 0x009e73; // bluish green: the body's material and its modes
const RESERVOIR: u32 = 0xcc79a7; // reddish purple: the active material's reservoir
const SPACE: u32 = 0x5f9fd8; // blue: the shared space
const MARKER: u32 = 0xf0e442; // yellow: the map cursor
const WARN: u32 = 0xd55e00; // vermillion: warnings

// Spectrum: 20 Hz – 20 kHz, 0 to −80 dBFS.
const F_LO: f64 = 20.0;
const F_HI: f64 = 20_000.0;
const DB_TOP: f32 = 0.0;
const DB_SPAN: f32 = 80.0;
// World strip chart: −10 to −130 dB (10·log10 of World energy).
const W_TOP: f32 = -10.0;
const W_SPAN: f32 = 120.0;

fn col(c: u32, a: f32) -> Hsla {
    let mut h: Hsla = rgb(c).into();
    h.a = a;
    h
}

/// A layout length in design pixels; the window's rem size scales with the CRT.
fn u(v: f32) -> Rems {
    rems(v / 16.0)
}

struct Fit {
    scale: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

fn fit(w: f32, h: f32) -> Fit {
    let scale = (w / DESIGN_W).min(h / DESIGN_H).max(MIN_SCALE);
    let (fw, fh) = (DESIGN_W * scale, DESIGN_H * scale);
    Fit { scale, x: (w - fw) / 2.0, y: (h - fh) / 2.0, w: fw, h: fh }
}

fn x_of_hz(f: f64) -> f32 {
    ((f.max(1.0) / F_LO).log10() / (F_HI / F_LO).log10()) as f32
}

fn db(v: f32) -> f32 {
    20.0 * v.max(1e-9).log10()
}

// ---------------------------------------------------------------------------
// The instrument
// ---------------------------------------------------------------------------

struct Meters {
    dry_pk: f32,
    dry_rms: f32,
    wet_pk: f32,
    wet_rms: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SquareMode {
    Map,
    Xy,
    Chord,
}

impl SquareMode {
    fn label(self) -> &'static str {
        match self {
            SquareMode::Map => "MAP",
            SquareMode::Xy => "XY L/R",
            SquareMode::Chord => "CHORD BOX",
        }
    }
    fn next(self) -> Self {
        match self {
            SquareMode::Map => SquareMode::Xy,
            SquareMode::Xy => SquareMode::Chord,
            SquareMode::Chord => SquareMode::Map,
        }
    }
}

/// A note on the scrolling tab: when, where, and whose (the simulated
/// guitarist's truth, or what the string analyzer heard).
#[derive(Debug, Clone, Copy)]
struct TabNote {
    t: f64,
    string: u8,
    fret: u8,
    truth: bool,
}

/// What the string analyzer currently hears: a fret per string and a chord.
#[derive(Debug, Clone, Default)]
struct Heard {
    strings: [Option<u8>; STRINGS],
    chord: Option<(String, f32)>,
}

/// MIDI note of each open string, low E to high e.
const OPEN_MIDI: [u8; STRINGS] = [40, 45, 50, 55, 59, 64];
const PITCH_NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

/// Name the chord a set of sounding MIDI notes spells, by template; the bass
/// note breaks ties. `None` under three notes.
fn name_chord(notes: &[u8]) -> Option<String> {
    if notes.len() < 3 {
        return None;
    }
    let mut pcs = [false; 12];
    for n in notes {
        pcs[(n % 12) as usize] = true;
    }
    let bass = notes.iter().min().map(|n| n % 12)?;
    const TEMPLATES: [(&str, &[u8]); 10] = [
        ("", &[0, 4, 7]),
        ("m", &[0, 3, 7]),
        ("7", &[0, 4, 7, 10]),
        ("maj7", &[0, 4, 7, 11]),
        ("m7", &[0, 3, 7, 10]),
        ("sus2", &[0, 2, 7]),
        ("sus4", &[0, 5, 7]),
        ("5", &[0, 7]),
        ("dim", &[0, 3, 6]),
        ("aug", &[0, 4, 8]),
    ];
    let mut best: Option<(i32, String)> = None;
    for root in 0..12u8 {
        for (q, iv) in TEMPLATES {
            let tpl: Vec<usize> = iv.iter().map(|i| ((root + i) % 12) as usize).collect();
            let hit = tpl.iter().filter(|&&p| pcs[p]).count() as i32;
            let miss = tpl.len() as i32 - hit;
            let extra = (0..12).filter(|p| pcs[*p] && !tpl.contains(p)).count() as i32;
            let score = 4 * hit - 5 * miss - 3 * extra + if root == bass { 2 } else { 0 } - iv.len() as i32 / 4;
            if best.as_ref().is_none_or(|(b, _)| score > *b) {
                best = Some((score, format!("{}{}", PITCH_NAMES[root as usize], q)));
            }
        }
    }
    best.map(|(_, n)| n)
}

/// The command line.
#[derive(Debug, Clone, Default)]
struct Cli {
    keys: Option<String>,
    socket: Option<String>,
    mute: bool,
    no_activate: bool,
    /// Substrings of the audio devices' names (`--input Scarlett`).
    input: Option<String>,
    output: Option<String>,
    input_gain: Option<f64>,
    /// Sample rate to run at (`--rate 96000`, or `OVILLO_RATE`).
    rate: Option<u32>,
}

impl Cli {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
        let env_on = |k: &str| std::env::var(k).is_ok_and(|v| !v.is_empty() && v != "0");
        Self {
            keys: value("--keys"),
            socket: value("--socket"),
            mute: args.iter().any(|a| a == "--mute") || env_on("OVILLO_MUTE"),
            no_activate: args.iter().any(|a| a == "--no-activate"),
            input: value("--input"),
            output: value("--output"),
            input_gain: value("--input-gain").and_then(|g| g.parse().ok()),
            rate: value("--rate").or_else(|| std::env::var("OVILLO_RATE").ok().filter(|r| !r.is_empty())).and_then(|r| r.parse().ok()),
        }
    }
}

/// The score the guitarist was last given, as the UI remembers it.
struct ScoreInfo {
    summary: serde_json::Value,
}

struct Ovillo {
    focus: FocusHandle,
    focused: bool,
    audio: Option<audio::Audio>,
    error: Option<String>,
    controls: AriadneControls,
    guitar: GuitarSettings,
    pattern: Pattern,
    last_pattern: Pattern,
    bpm: f64,
    history: History,
    frames: Vec<ScopeFrame>,
    spectrum: Spectrum,
    world: WorldHistory,
    tele: Option<Telemetry>,
    meters: Meters,
    clip_at: Option<Instant>,
    last_limited: u64,
    menu: Menu,
    active: Option<Active>,
    upper_spectrum: bool,
    modes: bool,
    hold: bool,
    lower_tab: bool,
    square: SquareMode,
    tab: VecDeque<TabNote>,
    heard: Heard,
    truth_chord: Option<String>,
    /// A chord name seen but not yet held long enough to count (strums roll).
    truth_pending: Option<(String, f64)>,
    chord_marks: VecDeque<(f64, String, bool)>,
    map_y: usize,
    time_div: usize,
    amp_div: usize,
    trail: VecDeque<(f32, f32, Instant)>,
    dragging: bool,
    square_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    scroll_acc: f32,
    message: Option<(String, Instant, bool)>,
    patch_dirty: Option<Instant>,
    patch_seen: Option<SystemTime>,
    take: Option<patch::Take>,
    last_take: Option<std::path::PathBuf>,
    /// The master (monitor) stage: level in dB, and mute.
    master_db: f64,
    muted: bool,
    control: control::Control,
    score: Option<ScoreInfo>,
    score_ends_seen: u32,
    /// The dry signal is the audio interface (only if one was opened).
    live: bool,
    input_gain_db: f64,
    /// Whether macOS is presenting the window (not fully covered, minimized
    /// or on another Space), and when a frame was last drawn.
    window_visible: bool,
    last_drawn: Option<Instant>,
}

impl Ovillo {
    fn new(cx: &mut Context<Self>, cli: &Cli) -> Self {
        let live = patch::live_path();
        let (controls, msg) = match patch::read(&live) {
            Ok(c) => (c, format!("LIVE PATCH {}", live.display())),
            Err(_) => {
                let c = patch::default_patch();
                let _ = patch::write(&live, &c);
                (c, "DEFAULT PATCH".to_string())
            }
        };
        let seed = SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32);
        let opts = audio::Options { output: cli.output.clone(), input: cli.input.clone(), muted: cli.mute, seed, rate: cli.rate };
        let (audio, error) = match audio::start(&opts) {
            Ok(a) => (Some(a), None),
            Err(e) => (None, Some(format!("NO AUDIO: {e}"))),
        };
        // Everything the screens size in samples follows the rate the device runs at.
        let rate = audio.as_ref().map_or(48_000.0, |a| a.sample_rate as f64);
        let mut s = Self {
            focus: cx.focus_handle(),
            focused: false,
            audio,
            error,
            controls,
            guitar: GuitarSettings::default(),
            pattern: Pattern::Wander,
            last_pattern: Pattern::Wander,
            bpm: 92.0,
            history: History::new(analysis::history_len(rate)),
            frames: Vec::with_capacity(analysis::history_len(rate)),
            spectrum: Spectrum::new(analysis::fft_len(rate)),
            world: WorldHistory::new(),
            tele: None,
            meters: Meters { dry_pk: 0.0, dry_rms: 0.0, wet_pk: 0.0, wet_rms: 0.0 },
            clip_at: None,
            last_limited: 0,
            menu: Menu::Root,
            active: None,
            upper_spectrum: true,
            modes: true,
            hold: false,
            lower_tab: false,
            square: SquareMode::Map,
            tab: VecDeque::new(),
            heard: Heard::default(),
            truth_chord: None,
            truth_pending: None,
            chord_marks: VecDeque::new(),
            map_y: 0,
            time_div: 3,
            amp_div: 4,
            trail: VecDeque::new(),
            dragging: false,
            square_bounds: Rc::new(Cell::new(None)),
            scroll_acc: 0.0,
            message: Some((msg, Instant::now(), false)),
            patch_dirty: None,
            patch_seen: patch::mtime(&live),
            take: None,
            last_take: None,
            master_db: 0.0,
            muted: cli.mute,
            control: control::Control::start(control::socket_path(cli.socket.as_deref())),
            score: None,
            score_ends_seen: 0,
            live: false,
            input_gain_db: cli.input_gain.unwrap_or(0.0).clamp(-24.0, 36.0),
            window_visible: true,
            last_drawn: None,
        };
        if let Some(p) = s.control.path.clone().filter(|_| s.control.status == control::Status::Listening) {
            let _ = OWNED_SOCKET.set(p);
        }
        if let Some(i) = s.audio.as_ref().and_then(|a| a.input.as_ref()) {
            s.live = true;
            s.message = Some((format!("LIVE INPUT: {} ({} CH)", i.device.to_uppercase(), i.channels), Instant::now(), false));
        }
        s.send(Command::InputGain(10f64.powf(s.input_gain_db / 20.0) as f32));
        s.send(Command::Ariadne(s.controls));
        s.send(Command::Guitar(s.guitar));
        s.send(Command::Pattern(s.pattern));
        s.send(Command::Tempo(s.bpm));
        s.mark_trail();
        if cli.mute {
            s.say("STARTED MUTED: OUTPUT ▸ MUTE OR \\ TO HEAR IT", false);
        }
        for k in cli.keys.as_deref().unwrap_or("").split_whitespace() {
            s.key(k, false);
        }
        s
    }

    /// The master stage: level in dB (−60…0) and mute. Not part of any patch.
    fn set_master(&mut self, db: f64, mute: bool) {
        self.master_db = db.clamp(MASTER_MIN_DB, MASTER_MAX_DB);
        self.muted = mute;
        self.send(Command::Master { gain: master_gain(self.master_db), mute });
    }

    /// The dry signal: the audio interface (`true`) or the simulated guitar.
    fn set_source(&mut self, live: bool) -> Result<(), String> {
        if live && !self.audio.as_ref().is_some_and(|a| a.input.is_some()) {
            return Err("NO LIVE INPUT: START WITH --input <DEVICE>".into());
        }
        self.live = live;
        self.send(Command::Source(if live { Source::Live } else { Source::Sim }));
        self.say(if live { "SOURCE: LIVE INPUT" } else { "SOURCE: SIMULATED GUITAR" }, false);
        Ok(())
    }

    /// The live input's gain in dB (−24…36).
    fn set_input_gain(&mut self, db: f64) {
        self.input_gain_db = db.clamp(-24.0, 36.0);
        self.send(Command::InputGain(10f64.powf(self.input_gain_db / 20.0) as f32));
    }

    /// Pin the live input to a channel (0-based), or follow the loudest (`None`).
    fn pin_input_channel(&mut self, ch: Option<usize>) -> Result<(), String> {
        let i = self.audio.as_ref().and_then(|a| a.input.as_ref()).ok_or("NO LIVE INPUT: START WITH --input <DEVICE>")?;
        match ch {
            Some(c) if c >= i.channels as usize => Err(format!("{} HAS {} INPUT CHANNELS", i.device.to_uppercase(), i.channels)),
            _ => {
                i.pinned.store(ch.unwrap_or(usize::MAX), Ordering::Relaxed);
                Ok(())
            }
        }
    }

    /// Hand a checked score to the guitarist.
    fn play_score(&mut self, sc: Score) {
        self.bpm = sc.tempo;
        let name = sc.name.clone();
        let secs = sc.seconds();
        self.score = Some(ScoreInfo { summary: sc.summary() });
        if self.pattern != Pattern::Score && self.pattern != Pattern::Off {
            self.last_pattern = self.pattern;
        }
        self.pattern = Pattern::Score;
        self.send(Command::Score(Box::new(sc)));
        let named = if name == "score" { String::new() } else { format!(" {}", name.to_uppercase()) };
        self.say(format!("PLAYING SCORE{named} ({secs:.1} S A PASS)"), false);
    }

    fn active_text(&self) -> String {
        match self.active {
            Some(Active::Param(i)) => format!("{}  {}", PARAMS[i].name.to_uppercase(), menu::fmt_param(i, self.controls.get(i))),
            Some(Active::Tempo) => format!("TEMPO  {:.1} BPM", self.bpm),
            Some(Active::Tone) => format!("TONE  {:.3}", self.guitar.tone),
            Some(Active::Volume) => format!("VOLUME  {:.3}", self.guitar.volume),
            Some(Active::TimeDiv) => format!("TIME/DIV  {} ms", TIME_DIVS_MS[self.time_div]),
            Some(Active::AmpDiv) => format!("AMPL/DIV  {} FS", AMP_DIVS[self.amp_div]),
            Some(Active::Master) => format!("MASTER  {:+.1} dB{}", self.master_db, if self.muted { "  MUTED" } else { "" }),
            Some(Active::InputGain) => format!("INPUT GAIN  {:+.1} dB", self.input_gain_db),
            None => String::new(),
        }
    }

    /// The annunciator row: label, lit, colour when lit.
    fn annunciators(&self) -> Vec<(&'static str, bool, u32)> {
        let tele = self.tele;
        let clip = self.clip_at.is_some_and(|c| c.elapsed().as_secs_f32() < 2.0);
        let werr = tele.is_some_and(|t| t.world_errors > 0);
        let drop = tele.is_some_and(|t| t.dropped_scope > 0);
        let aerr = self.audio.as_ref().is_some_and(|a| a.stream_error.load(Ordering::Relaxed));
        let ctl = match self.control.status {
            control::Status::Listening => ("CTL", self.control.active(), BRIGHT),
            _ => ("CTL OFF", true, WARN),
        };
        vec![
            ("PLAY", self.pattern != Pattern::Off, BRIGHT),
            ("REC", self.take.is_some(), WARN),
            ("MUTE", self.muted, WARN),
            ("CLIP", clip, WARN),
            ("WORLD ERR", werr, WARN),
            ("DROP", drop, WARN),
            ("AUDIO ERR", aerr || self.audio.is_none(), WARN),
            ("HOLD", self.hold, WET),
            ("MODES", self.modes, BODY),
            ctl,
        ]
    }

    fn send(&mut self, c: Command) {
        if let Some(a) = &mut self.audio {
            let _ = a.ends.commands.push(c);
        }
    }

    fn say(&mut self, m: impl Into<String>, warn: bool) {
        self.message = Some((m.into(), Instant::now(), warn));
    }

    fn sr(&self) -> f64 {
        self.audio.as_ref().map_or(48_000.0, |a| a.sample_rate as f64)
    }

    fn mark_trail(&mut self) {
        let y = MAP_Y[self.map_y];
        let info = PARAMS[y];
        let x = self.controls.material_organization as f32;
        let yv = ((self.controls.get(y) - info.min) / (info.max - info.min)) as f32;
        if self.trail.back().is_none_or(|(a, b, _)| (a - x).abs() + (b - yv).abs() > 1e-4) {
            self.trail.push_back((x, yv, Instant::now()));
        }
        while self.trail.len() > 600 {
            self.trail.pop_front();
        }
    }

    /// New Ariadne controls from the panel: send, remember to save, trail.
    fn set_controls(&mut self, c: AriadneControls) {
        self.controls = c.sanitized();
        self.send(Command::Ariadne(self.controls));
        self.patch_dirty = Some(Instant::now());
        self.mark_trail();
    }

    fn set_guitar(&mut self, g: GuitarSettings) {
        self.guitar = g;
        self.send(Command::Guitar(g));
    }

    fn set_pattern(&mut self, p: Pattern) {
        if p == Pattern::Score && self.score.is_none() {
            self.say("NO SCORE LOADED: SEND ONE OVER THE CONTROL SOCKET", true);
            return;
        }
        if p != Pattern::Off {
            self.last_pattern = p;
        }
        self.pattern = p;
        self.send(Command::Pattern(p));
    }

    /// Pull everything the audio thread has published.
    fn drain(&mut self) {
        let Some(a) = &mut self.audio else { return };
        // Scores the rig has finished with are freed here, off the audio thread.
        while let Ok(old) = a.ends.garbage.pop() {
            drop(old);
        }
        while let Ok(f) = a.ends.scope.pop() {
            self.history.push(f);
            if let Some(t) = &mut self.take {
                t.push(f.dry, f.left, f.right);
            }
        }
        let sr = a.sample_rate as f64;
        while let Ok((at, e)) = a.ends.events.pop() {
            if let Event::Pluck { string, fret, .. } = e {
                self.tab.push_back(TabNote { t: at as f64 / sr, string, fret, truth: true });
            }
        }
        let mut latest = None;
        while let Ok(t) = a.ends.telemetry.pop() {
            self.world.push(&t);
            let m = &mut self.meters;
            m.dry_pk = (m.dry_pk * 0.97).max(t.dry_peak);
            m.wet_pk = (m.wet_pk * 0.97).max(t.wet_peak);
            m.dry_rms = (0.95 * m.dry_rms.powi(2) + 0.05 * t.dry_rms.powi(2)).sqrt();
            m.wet_rms = (0.95 * m.wet_rms.powi(2) + 0.05 * t.wet_rms.powi(2)).sqrt();
            latest = Some(t);
        }
        if let Some(t) = latest {
            if t.safety_limited_frames > self.last_limited {
                self.clip_at = Some(Instant::now());
                self.last_limited = t.safety_limited_frames;
            }
            self.tele = Some(t);
            if t.score_ends != self.score_ends_seen {
                // A one-shot score finished: follow the guitarist to what it plays next.
                self.score_ends_seen = t.score_ends;
                if self.pattern == Pattern::Score {
                    self.pattern = t.pattern;
                    self.say(format!("SCORE ENDED: {}", t.pattern.label()), false);
                }
            }
            let now = t.sample as f64 / t.sample_rate;
            let sounding: Vec<u8> = t
                .strings
                .iter()
                .enumerate()
                .filter(|(_, s)| s.sounding)
                .map(|(i, s)| OPEN_MIDI[i] + s.semitones.round().max(0.0) as u8)
                .collect();
            if let Some(name) = name_chord(&sounding) {
                if self.truth_chord.as_deref() == Some(name.as_str()) {
                    self.truth_pending = None;
                } else {
                    match &self.truth_pending {
                        Some((p, since)) if *p == name => {
                            if now - since >= 0.12 {
                                self.chord_marks.push_back((*since, name.clone(), true));
                                self.truth_chord = Some(name);
                                self.truth_pending = None;
                            }
                        }
                        _ => self.truth_pending = Some((name, now)),
                    }
                }
            }
            while self.tab.front().is_some_and(|n| n.t < now - WORLD_SPAN_S - 1.0) {
                self.tab.pop_front();
            }
            while self.chord_marks.front().is_some_and(|c| c.0 < now - WORLD_SPAN_S - 1.0) {
                self.chord_marks.pop_front();
            }
        }
    }

    /// Save the live patch after the panel settles; follow it when edited on disk.
    fn tick_files(&mut self) {
        let live = patch::live_path();
        if let Some(t) = self.patch_dirty {
            if t.elapsed().as_millis() > 250 {
                self.patch_dirty = None;
                match patch::write(&live, &self.controls) {
                    Ok(()) => self.patch_seen = patch::mtime(&live),
                    Err(e) => self.say(format!("COULD NOT SAVE PATCH: {e}"), true),
                }
            }
            return;
        }
        let m = patch::mtime(&live);
        if m.is_some() && m != self.patch_seen {
            self.patch_seen = m;
            match patch::read(&live) {
                Ok(c) if c != self.controls => {
                    self.controls = c;
                    self.send(Command::Ariadne(c));
                    self.mark_trail();
                    self.say("LIVE PATCH CHANGED ON DISK: FOLLOWING IT", false);
                }
                Ok(_) => {}
                Err(e) => self.say(format!("LIVE PATCH UNREADABLE: {e}"), true),
            }
        }
    }

    fn view(&self) -> View<'_> {
        View {
            params: self.controls.as_array(),
            pattern: self.pattern.label(),
            playing: self.pattern != Pattern::Off,
            bpm: self.bpm,
            pickup: self.guitar.pickup.label(),
            tone: self.guitar.tone,
            volume: self.guitar.volume,
            hum: self.guitar.hum,
            upper_spectrum: self.upper_spectrum,
            modes: self.modes,
            lower_tab: self.lower_tab,
            square: self.square.label(),
            map_y: MAP_Y[self.map_y],
            time_div_ms: TIME_DIVS_MS[self.time_div],
            amp_div: AMP_DIVS[self.amp_div],
            active: self.active,
            master_db: self.master_db,
            muted: self.muted,
            recording: self.take.is_some(),
            input: self.audio.as_ref().and_then(|a| a.input.as_ref()).map(|i| (self.live, self.input_gain_db, self.input_channel_label(i))),
        }
    }

    fn input_channel_label(&self, i: &audio::Input) -> String {
        let pinned = i.pinned.load(std::sync::atomic::Ordering::Relaxed);
        let ch = i.channel.load(std::sync::atomic::Ordering::Relaxed);
        if pinned < i.channels as usize { format!("{}", pinned + 1) } else { format!("AUTO {}", ch + 1) }
    }

    fn press(&mut self, a: Action) {
        match a {
            Action::Open(m) => {
                self.menu = m;
                self.active = None;
            }
            Action::Return => {
                self.menu = Menu::Root;
                self.active = None;
            }
            Action::Activate(x) => self.active = Some(x),
            Action::CyclePattern => {
                let next = Pattern::CYCLE.iter().position(|p| *p == self.pattern).map_or(0, |i| (i + 1) % Pattern::CYCLE.len());
                self.set_pattern(Pattern::CYCLE[next]);
            }
            Action::CyclePickup => {
                let i = Pickup::ALL.iter().position(|p| *p == self.guitar.pickup).unwrap_or(0);
                let g = GuitarSettings { pickup: Pickup::ALL[(i + 1) % Pickup::ALL.len()], ..self.guitar };
                self.set_guitar(g);
            }
            Action::ToggleHum => self.set_guitar(GuitarSettings { hum: !self.guitar.hum, ..self.guitar }),
            Action::MuteStrings => self.send(Command::MuteStrings),
            Action::PlayStop => {
                if self.pattern == Pattern::Off {
                    self.set_pattern(self.last_pattern);
                } else {
                    self.set_pattern(Pattern::Off);
                }
            }
            Action::ResetWorld => {
                self.send(Command::ResetWorld);
                self.say("WORLD RESET: BODY, RESERVOIR AND SPACE AT REST", false);
            }
            Action::ToggleUpper => self.upper_spectrum = !self.upper_spectrum,
            Action::ToggleModes => self.modes = !self.modes,
            Action::ToggleHold => {
                self.hold = !self.hold;
                self.spectrum.clear_hold();
            }
            Action::ToggleLower => self.lower_tab = !self.lower_tab,
            Action::CycleSquare => self.square = self.square.next(),
            Action::CycleMapY => {
                self.map_y = (self.map_y + 1) % MAP_Y.len();
                self.trail.clear();
                self.mark_trail();
            }
            Action::Save(slot) => {
                let p = patch::slot_path(slot);
                match patch::write(&p, &self.controls) {
                    Ok(()) => self.say(format!("SAVED {}", p.display()), false),
                    Err(e) => self.say(format!("SAVE FAILED: {e}"), true),
                }
            }
            Action::Load(slot) => match patch::read(&patch::slot_path(slot)) {
                Ok(c) => {
                    self.set_controls(c);
                    self.say(format!("LOADED SLOT {}", (b'A' + slot) as char), false);
                }
                Err(_) => self.say(format!("SLOT {} IS EMPTY", (b'A' + slot) as char), true),
            },
            Action::DefaultPatch => {
                self.set_controls(patch::default_patch());
                self.say("DEFAULT PATCH", false);
            }
            Action::ToggleMute => {
                self.set_master(self.master_db, !self.muted);
                self.say(if self.muted { "MASTER MUTED" } else { "MASTER ON" }, false);
            }
            Action::Record => self.toggle_record(),
            Action::ToggleSource => {
                let live = !self.live;
                if let Err(e) = self.set_source(live) {
                    self.say(e, true);
                }
            }
            Action::CycleInputChannel => {
                if let Some((n, p)) = self.audio.as_ref().and_then(|a| a.input.as_ref()).map(|i| (i.channels as usize, i.pinned.load(Ordering::Relaxed))) {
                    let next = if p >= n { Some(0) } else if p + 1 < n { Some(p + 1) } else { None };
                    let _ = self.pin_input_channel(next);
                }
            }
            Action::Nothing => {}
        }
    }

    /// Turn the knob `n` notches (`fine`: a tenth of a notch).
    fn knob(&mut self, n: i32, fine: bool) {
        let Some(a) = self.active else {
            self.say("NO ACTIVE FUNCTION: PRESS A PARAMETER'S SOFTKEY", true);
            return;
        };
        let n = n as f64;
        match a {
            Active::Param(i) => {
                let info = PARAMS[i];
                let mut c = self.controls;
                let v = if info.id == "output" {
                    // Output turns in dB.
                    let d = 20.0 * c.get(i).max(1e-3).log10() + n * if fine { 0.1 } else { 0.5 };
                    10f64.powf(d / 20.0)
                } else {
                    c.get(i) + n * (info.max - info.min) / if fine { 1000.0 } else { 100.0 }
                };
                c.set(i, v.clamp(info.min, info.max));
                self.set_controls(c);
            }
            Active::Tempo => {
                self.bpm = (self.bpm + n * if fine { 0.5 } else { 2.0 }).clamp(40.0, 220.0);
                self.send(Command::Tempo(self.bpm));
            }
            Active::Tone => {
                let t = (self.guitar.tone + n * if fine { 0.002 } else { 0.02 }).clamp(0.0, 1.0);
                self.set_guitar(GuitarSettings { tone: t, ..self.guitar });
            }
            Active::Volume => {
                let v = (self.guitar.volume + n * if fine { 0.002 } else { 0.02 }).clamp(0.0, 1.0);
                self.set_guitar(GuitarSettings { volume: v, ..self.guitar });
            }
            Active::TimeDiv => {
                self.time_div = (self.time_div as i64 + n.signum() as i64).clamp(0, TIME_DIVS_MS.len() as i64 - 1) as usize
            }
            Active::AmpDiv => {
                self.amp_div = (self.amp_div as i64 + n.signum() as i64).clamp(0, AMP_DIVS.len() as i64 - 1) as usize
            }
            Active::Master => self.set_master(self.master_db + n * if fine { 0.1 } else { 1.0 }, self.muted),
            Active::InputGain => self.set_input_gain(self.input_gain_db + n * if fine { 0.1 } else { 0.5 }),
        }
    }

    /// A point on the configuration map (0..1 each way, y up) sets two parameters.
    fn map_to(&mut self, x: f32, y: f32) {
        let yi = MAP_Y[self.map_y];
        let info = PARAMS[yi];
        let mut c = self.controls;
        c.material_organization = x.clamp(0.0, 1.0) as f64;
        c.set(yi, info.min + y.clamp(0.0, 1.0) as f64 * (info.max - info.min));
        self.set_controls(c);
    }

    fn map_event(&mut self, pos: Point<Pixels>) -> bool {
        if self.square != SquareMode::Map {
            return false;
        }
        let Some(b) = self.square_bounds.get() else { return false };
        let w = f32::from(b.size.width).max(1.0);
        let h = f32::from(b.size.height).max(1.0);
        let x = f32::from(pos.x - b.origin.x) / w;
        let y = 1.0 - f32::from(pos.y - b.origin.y) / h;
        if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
            return false;
        }
        self.map_to(x, y);
        true
    }

    fn toggle_record(&mut self) {
        if let Some(t) = self.take.take() {
            let secs = t.frames as f64 / self.sr();
            match t.finish() {
                Ok(dir) => {
                    self.say(format!("TAKE SAVED ({secs:.1} s) {}", dir.display()), false);
                    self.last_take = Some(dir);
                }
                Err(e) => self.say(format!("TAKE FAILED: {e}"), true),
            }
        } else {
            match patch::Take::start(self.sr() as u32, &self.controls) {
                Ok(t) => {
                    let d = t.dir.display().to_string();
                    self.take = Some(t);
                    self.say(format!("RECORDING DRY + WET TO {d}"), false);
                }
                Err(e) => self.say(format!("CANNOT RECORD: {e}"), true),
            }
        }
    }

    /// A key press. True if the instrument knows the key.
    fn key(&mut self, key: &str, shift: bool) -> bool {
        let softkey = match key {
            "1" | "f1" => Some(0),
            "2" | "f2" => Some(1),
            "3" | "f3" => Some(2),
            "4" | "f4" => Some(3),
            "5" | "f5" => Some(4),
            "6" | "f6" => Some(5),
            "7" | "f7" => Some(6),
            "8" | "f8" => Some(7),
            _ => None,
        };
        if let Some(i) = softkey {
            let keys = menu::softkeys(self.menu, &self.view());
            if let Some(k) = keys.get(i) {
                self.press(k.action);
            }
            return true;
        }
        if let Some(s) = ["z", "x", "c", "v", "b", "n"].iter().position(|k| *k == key) {
            self.send(Command::Pluck { string: s as u8, fret: 0, velocity: if shift { 1.0 } else { 0.75 } });
            return true;
        }
        match key {
            "escape" | "backspace" => self.press(Action::Return),
            "space" => self.press(Action::PlayStop),
            "up" | "right" => self.knob(1, shift),
            "down" | "left" => self.knob(-1, shift),
            "tab" => self.press(Action::ToggleUpper),
            "h" => self.press(Action::ToggleHold),
            "m" => self.press(Action::MuteStrings),
            "r" => self.press(Action::Record),
            "i" => self.press(Action::ToggleSource),
            "0" => self.press(Action::ResetWorld),
            // The master: − and = step it a dB (Shift: a tenth); \ or ` mutes.
            "-" | "_" => self.set_master(self.master_db - if shift { 0.1 } else { 1.0 }, self.muted),
            "=" | "+" => self.set_master(self.master_db + if shift { 0.1 } else { 1.0 }, self.muted),
            "\\" | "`" => self.press(Action::ToggleMute),
            _ => return false,
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Painting (strokes on canvases)
// ---------------------------------------------------------------------------

/// A trace: points in unit coordinates (x right, y down), drawn as a polyline.
struct Trace {
    pts: Vec<(f32, f32)>,
    color: Hsla,
    width: f32,
    dash: Option<[f32; 2]>,
}

/// A mark in unit coordinates.
enum Mark {
    /// A vertical stem from the bottom edge up to `y`.
    Stem { x: f32, y: f32, color: Hsla },
    /// A small downward triangle on the top edge.
    TopTick { x: f32, color: Hsla },
    /// A cross.
    Cross { x: f32, y: f32, color: Hsla },
    /// A hollow circle of radius `r` design px.
    Ring { x: f32, y: f32, r: f32, color: Hsla },
    /// An ×, for a string not played.
    XMark { x: f32, y: f32, color: Hsla },
}

struct ScreenData {
    z: f32,
    xdivs: u32,
    ydivs: u32,
    traces: Vec<Trace>,
    marks: Vec<Mark>,
}

fn rect(window: &mut Window, b: Bounds<Pixels>, c: Hsla) {
    window.paint_quad(quad(b, px(0.0), c, px(0.0), gpui::transparent_black(), BorderStyle::default()));
}

fn polyline(window: &mut Window, pts: &[Point<Pixels>], width: f32, c: Hsla, dash: Option<&[Pixels]>) {
    if pts.len() < 2 {
        return;
    }
    let mut b = PathBuilder::stroke(px(width));
    if let Some(d) = dash {
        b = b.dash_array(d);
    }
    b.move_to(pts[0]);
    for p in &pts[1..] {
        b.line_to(*p);
    }
    if let Ok(path) = b.build() {
        window.paint_path(path, c);
    }
}

fn paint_screen(bounds: Bounds<Pixels>, window: &mut Window, d: &ScreenData) {
    let z = d.z;
    let o = bounds.origin;
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    let r = |x: f32, y: f32, ww: f32, hh: f32| Bounds { origin: point(o.x + px(x), o.y + px(y)), size: size(px(ww), px(hh)) };
    let grid = d.xdivs > 0 && d.ydivs > 0;
    for i in (0..=d.xdivs).filter(|_| grid) {
        let x = (w - 1.0) * i as f32 / d.xdivs as f32;
        let c = if i == 0 || i == d.xdivs || i * 2 == d.xdivs { GRAT_AXIS } else { GRAT };
        rect(window, r(x, 0.0, z, h), rgb(c).into());
    }
    for j in (0..=d.ydivs).filter(|_| grid) {
        let y = (h - 1.0) * j as f32 / d.ydivs as f32;
        let c = if j == 0 || j == d.ydivs || j * 2 == d.ydivs { GRAT_AXIS } else { GRAT };
        rect(window, r(0.0, y, w, z), rgb(c).into());
    }
    let (cx0, cy0) = ((w - 1.0) / 2.0, (h - 1.0) / 2.0);
    for i in (0..(d.xdivs * 5)).filter(|_| grid) {
        let x = (w - 1.0) * i as f32 / (d.xdivs * 5) as f32;
        rect(window, r(x, cy0 - 3.0 * z, z, 7.0 * z), rgb(GRAT_AXIS).into());
    }
    for j in (0..(d.ydivs * 5)).filter(|_| grid) {
        let y = (h - 1.0) * j as f32 / (d.ydivs * 5) as f32;
        rect(window, r(cx0 - 3.0 * z, y, 7.0 * z, z), rgb(GRAT_AXIS).into());
    }
    let at = |x: f32, y: f32| point(o.x + px(x.clamp(0.0, 1.0) * (w - 1.0)), o.y + px(y.clamp(0.0, 1.0) * (h - 1.0)));
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        for m in &d.marks {
            match *m {
                Mark::Stem { x, y, color } => polyline(window, &[at(x, 1.0), at(x, y)], z, color, None),
                Mark::TopTick { x, color } => {
                    let p = at(x, 0.0);
                    let mut b = PathBuilder::stroke(px(z));
                    b.move_to(point(p.x - px(4.0 * z), p.y + px(z)));
                    b.line_to(point(p.x + px(4.0 * z), p.y + px(z)));
                    b.line_to(point(p.x, p.y + px(7.0 * z)));
                    b.close();
                    if let Ok(path) = b.build() {
                        window.paint_path(path, color);
                    }
                }
                Mark::Cross { x, y, color } => {
                    let p = at(x, y);
                    let k = px(7.0 * z);
                    polyline(window, &[point(p.x - k, p.y), point(p.x + k, p.y)], 1.5 * z, color, None);
                    polyline(window, &[point(p.x, p.y - k), point(p.x, p.y + k)], 1.5 * z, color, None);
                }
                Mark::Ring { x, y, r, color } => {
                    let p = at(x, y);
                    let pts: Vec<Point<Pixels>> = (0..=24)
                        .map(|i| {
                            let a = std::f32::consts::TAU * i as f32 / 24.0;
                            point(p.x + px(r * z * a.cos()), p.y + px(r * z * a.sin()))
                        })
                        .collect();
                    polyline(window, &pts, 1.2 * z, color, None);
                }
                Mark::XMark { x, y, color } => {
                    let p = at(x, y);
                    let k = px(4.0 * z);
                    polyline(window, &[point(p.x - k, p.y - k), point(p.x + k, p.y + k)], 1.2 * z, color, None);
                    polyline(window, &[point(p.x - k, p.y + k), point(p.x + k, p.y - k)], 1.2 * z, color, None);
                }
            }
        }
        for t in &d.traces {
            let pts: Vec<Point<Pixels>> = t.pts.iter().map(|(x, y)| at(*x, *y)).collect();
            let dash = t.dash.map(|[a, b]| [px(a * z), px(b * z)]);
            polyline(window, &pts, t.width * z, t.color, dash.as_ref().map(|d| &d[..]));
        }
    });
}

/// The spectrum as one point per column on the log axis.
fn spectrum_trace(dbs: &[f32], sr: f64, cols: usize) -> Vec<(f32, f32)> {
    // The FFT's length, from its bins.
    let bin_hz = sr / (2 * (dbs.len() - 1)) as f64;
    let ratio = F_HI / F_LO;
    let n = dbs.len();
    (0..cols)
        .filter_map(|c| {
            let f0 = F_LO * ratio.powf(c as f64 / cols as f64);
            let f1 = F_LO * ratio.powf((c + 1) as f64 / cols as f64);
            if f1 > sr / 2.0 {
                return None;
            }
            let (k0, k1) = (f0 / bin_hz, f1 / bin_hz);
            let v = if k1 - k0 < 1.0 {
                let k = 0.5 * (k0 + k1);
                let i = (k.floor() as usize).min(n - 2);
                let t = (k - i as f64) as f32;
                dbs[i] * (1.0 - t) + dbs[i + 1] * t
            } else {
                dbs[k0.ceil() as usize..=(k1.floor() as usize).min(n - 1)]
                    .iter()
                    .fold(f32::MIN, |m, v| m.max(*v))
            };
            let x = (c as f32 + 0.5) / cols as f32;
            Some((x, (DB_TOP - v) / DB_SPAN))
        })
        .collect()
}

/// Decimate a waveform to at most `cols` min/max pairs.
fn wave_trace(v: &[f32], cols: usize, amp_div: f32) -> Vec<(f32, f32)> {
    let y = |s: f32| 0.5 - s / (8.0 * amp_div);
    if v.len() <= cols * 2 {
        let last = v.len().saturating_sub(1).max(1) as f32;
        return v.iter().enumerate().map(|(i, s)| (i as f32 / last, y(*s))).collect();
    }
    let per = v.len() as f32 / cols as f32;
    let mut out = Vec::with_capacity(cols * 2);
    for c in 0..cols {
        let a = (c as f32 * per) as usize;
        let b = (((c + 1) as f32 * per) as usize).min(v.len());
        let chunk = &v[a..b.max(a + 1)];
        let (lo, hi) = chunk.iter().fold((f32::MAX, f32::MIN), |(l, h), s| (l.min(*s), h.max(*s)));
        let x = c as f32 / cols as f32;
        out.push((x, y(hi)));
        out.push((x, y(lo)));
    }
    out
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn t(s: impl Into<SharedString>, c: u32) -> gpui::Div {
    div().text_color(rgb(c)).child(s.into())
}

fn annunciator(label: &str, lit: bool, c: u32) -> gpui::Div {
    let c = if lit { c } else { DIM };
    div().px(u(5.0)).border_1().border_color(rgb(c)).text_color(rgb(c)).child(label.to_string())
}

fn softkey_box(i: usize, k: &Softkey, cx: &mut Context<Ovillo>) -> gpui::Stateful<gpui::Div> {
    let (txt, border) = if k.is_blank() {
        (DIM, GRAT)
    } else if k.lit {
        (BRIGHT, BRIGHT)
    } else {
        (TEXT, GRAT_AXIS)
    };
    let mut b = div()
        .id(("softkey", i))
        .h(u(52.0))
        .flex_1()
        .flex()
        .flex_col()
        .justify_center()
        .px(u(6.0))
        .border_1()
        .border_color(rgb(border))
        .text_color(rgb(txt))
        .child(div().child(k.lines[0].clone()))
        .child(div().child(k.lines[1].clone()));
    if k.lit {
        b = b.border_2();
    }
    let action = k.action;
    b.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
        this.press(action);
        cx.notify();
    }))
}

/// Labels down the left edge of a screen, one per division line.
fn y_labels(h: f32, ydivs: u32, label: impl Fn(u32) -> String) -> gpui::Div {
    let mut d = div().relative().w(u(LABEL_W)).h(u(h)).flex_none().text_size(u(SMALL_PX));
    for j in 0..=ydivs {
        let y = (h - 1.0) * j as f32 / ydivs as f32 - 7.0;
        d = d.child(div().absolute().top(u(y)).right(u(6.0)).text_color(rgb(TEXT)).child(label(j)));
    }
    d
}

impl Render for Ovillo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.focused {
            window.focus(&self.focus, cx);
            self.focused = true;
        }
        let vs = window.viewport_size();
        let fitted = fit(f32::from(vs.width), f32::from(vs.height));
        window.set_rem_size(px(16.0 * fitted.scale));
        self.last_drawn = Some(Instant::now());
        self.serve_control();
        self.drain();
        window.request_animation_frame();
        let z = fitted.scale;
        let sr = self.sr();
        let amp_div = AMP_DIVS[self.amp_div] as f32;
        let tele = self.tele;

        // ---- upper screen ----
        let mut upper = ScreenData { z, xdivs: 10, ydivs: 8, traces: Vec::new(), marks: Vec::new() };
        let mut triggered = false;
        let x_labels;
        let upper_labels;
        if self.upper_spectrum {
            self.history.latest(self.spectrum.n, &mut self.frames);
            self.spectrum.update(&self.frames);
            let cols = 900;
            if self.modes {
                if let Some(t) = &tele {
                    let max = t.modes.iter().fold(0.0f32, |m, v| m.max(*v));
                    if max > 1e-30 {
                        for k in 0..t.modes.len() {
                            let rel = 10.0 * (t.modes[k] / max).max(1e-12).log10();
                            let y = 1.0 - 0.85 * ((rel + 60.0) / 60.0).clamp(0.0, 1.0);
                            upper.marks.push(Mark::Stem { x: x_of_hz(t.mode_hz[k] as f64), y, color: col(BODY, 0.9) });
                        }
                    }
                }
            }
            if let Some(t) = &tele {
                for f in t.string_hz {
                    upper.marks.push(Mark::TopTick { x: x_of_hz(f as f64), color: col(DRY, 1.0) });
                }
            }
            if self.hold {
                upper.traces.push(Trace {
                    pts: spectrum_trace(&self.spectrum.hold_db, sr, cols),
                    color: col(WET, 0.7),
                    width: 1.0,
                    dash: Some([2.0, 3.0]),
                });
            }
            upper.traces.push(Trace { pts: spectrum_trace(&self.spectrum.dry_db, sr, cols), color: col(DRY, 0.9), width: 1.0, dash: None });
            upper.traces.push(Trace { pts: spectrum_trace(&self.spectrum.wet_db, sr, cols), color: col(WET, 1.0), width: 1.5, dash: None });
            upper_labels = y_labels(UPPER_H, 8, |j| format!("{:.0}", DB_TOP - j as f32 * DB_SPAN / 8.0));
            let mut xl = div().relative().h(u(14.0)).ml(u(LABEL_W)).mr(u(RIGHT_W)).text_size(u(SMALL_PX));
            for (f, s) in [(20.0, "20"), (50.0, "50"), (100.0, "100"), (200.0, "200"), (500.0, "500"), (1e3, "1k"), (2e3, "2k"), (5e3, "5k"), (1e4, "10k"), (2e4, "20k Hz")] {
                xl = xl.child(div().absolute().left(gpui::relative(x_of_hz(f))).ml(u(-8.0)).text_color(rgb(TEXT)).child(s));
            }
            x_labels = xl;
        } else {
            let len = ((TIME_DIVS_MS[self.time_div] * 10.0 / 1000.0) * sr) as usize;
            self.history.latest((len * 3).max(sr as usize / 2).min(self.history.capacity()), &mut self.frames);
            let (start, ok) = trigger(&self.frames, len, sr);
            triggered = ok;
            let end = (start + len).min(self.frames.len());
            let win = &self.frames[start.min(end)..end];
            let dry: Vec<f32> = win.iter().map(|f| f.dry).collect();
            let wet: Vec<f32> = win.iter().map(|f| 0.5 * (f.left + f.right)).collect();
            upper.traces.push(Trace { pts: wave_trace(&dry, 900, amp_div), color: col(DRY, 1.0), width: 1.2, dash: None });
            upper.traces.push(Trace { pts: wave_trace(&wet, 900, amp_div), color: col(WET, 1.0), width: 1.2, dash: None });
            upper_labels = y_labels(UPPER_H, 8, |j| format!("{:+.3}", (4.0 - j as f32) * amp_div));
            let ms = TIME_DIVS_MS[self.time_div];
            x_labels = div()
                .flex()
                .justify_between()
                .h(u(14.0))
                .ml(u(LABEL_W))
                .mr(u(RIGHT_W))
                .text_size(u(SMALL_PX))
                .text_color(rgb(TEXT))
                .child("0 ms")
                .child(format!("{} ms", ms * 5.0))
                .child(format!("{} ms", ms * 10.0));
        }
        let upper_screen = div()
            .relative()
            .flex_1()
            .h(u(UPPER_H))
            .child(canvas(|_, _, _| (), move |b, _, window, _| paint_screen(b, window, &upper)).absolute().size_full());

        // ---- the World strip chart ----
        let now = tele.map_or(0.0, |t| t.sample as f64 / t.sample_rate);
        let mut world = ScreenData { z, xdivs: 10, ydivs: 8, traces: Vec::new(), marks: Vec::new() };
        let wx = |tt: f64| ((tt - (now - WORLD_SPAN_S)) / WORLD_SPAN_S) as f32;
        let wy = |v: f32| (W_TOP - v) / W_SPAN;
        let pts: Vec<&analysis::WorldPoint> = self.world.points.iter().filter(|p| p.t >= now - WORLD_SPAN_S).collect();
        world.traces.push(Trace { pts: pts.iter().map(|p| (wx(p.t), wy(p.space))).collect(), color: col(SPACE, 1.0), width: 1.0, dash: None });
        world.traces.push(Trace { pts: pts.iter().map(|p| (wx(p.t), wy(p.reservoir))).collect(), color: col(RESERVOIR, 1.0), width: 1.0, dash: Some([5.0, 3.0]) });
        world.traces.push(Trace { pts: pts.iter().map(|p| (wx(p.t), wy(p.material))).collect(), color: col(BODY, 1.0), width: 1.5, dash: None });
        let (world_screen, world_labels, lower_caption) = if self.lower_tab {
            // Scrolling tablature: high e on top, as written; the guitarist's
            // frets above each line (truth), the analyzer's below (heard).
            let mut tab = ScreenData { z, xdivs: 0, ydivs: 0, traces: Vec::new(), marks: Vec::new() };
            // Two header rows for chord names (truth, heard), then the six strings.
            let sy = |st: u8| 0.18 + 0.76 * (5 - st) as f32 / 5.0;
            for sec in 0..=(WORLD_SPAN_S as i64) {
                let x = wx(now.floor() - sec as f64);
                tab.traces.push(Trace { pts: vec![(x, 0.0), (x, 1.0)], color: col(GRAT, 1.0), width: 1.0, dash: None });
            }
            for st in 0..STRINGS as u8 {
                tab.traces.push(Trace { pts: vec![(0.0, sy(st)), (1.0, sy(st))], color: col(GRAT_AXIS, 1.0), width: 1.0, dash: None });
            }
            let mut glyphs: Vec<gpui::Div> = Vec::new();
            for n in self.tab.iter().filter(|n| n.t >= now - WORLD_SPAN_S) {
                let x = wx(n.t);
                let y = sy(n.string) * LOWER_H + if n.truth { -13.0 } else { 1.0 };
                glyphs.push(
                    div()
                        .absolute()
                        .left(gpui::relative(x))
                        .top(u(y))
                        .text_size(u(SMALL_PX))
                        .text_color(rgb(if n.truth { DRY } else { BRIGHT }))
                        .child(format!("{}", n.fret)),
                );
            }
            for (ct, name, truth) in self.chord_marks.iter().filter(|c| c.0 >= now - WORLD_SPAN_S) {
                glyphs.push(
                    div()
                        .absolute()
                        .left(gpui::relative(wx(*ct)))
                        .top(u(if *truth { -1.0 } else { 11.0 }))
                        .text_size(u(SMALL_PX))
                        .text_color(rgb(if *truth { DRY } else { BRIGHT }))
                        .child(name.clone()),
                );
            }
            let screen = div()
                .relative()
                .flex_1()
                .h(u(LOWER_H))
                .child(canvas(|_, _, _| (), move |b, _, window, _| paint_screen(b, window, &tab)).absolute().size_full())
                .children(glyphs);
            let mut labels = div().relative().w(u(LABEL_W)).h(u(LOWER_H)).flex_none().text_size(u(SMALL_PX));
            for st in 0..STRINGS as u8 {
                labels = labels.child(
                    div().absolute().top(u(sy(st) * LOWER_H - 7.0)).right(u(6.0)).text_color(rgb(TEXT)).child(STRING_NAMES[st as usize]),
                );
            }
            (screen, labels, "TAB  TRUTH ABOVE (BLUE)  HEARD BELOW (WHITE)")
        } else {
            let screen = div()
                .relative()
                .flex_1()
                .h(u(LOWER_H))
                .child(canvas(|_, _, _| (), move |b, _, window, _| paint_screen(b, window, &world)).absolute().size_full());
            (screen, y_labels(LOWER_H, 8, |j| format!("{:.0}", W_TOP - j as f32 * W_SPAN / 8.0)), "WORLD ENERGY dB (10 LOG10 E)")
        };

        // ---- the square: configuration map or XY ----
        let mut sq = ScreenData { z, xdivs: 8, ydivs: 8, traces: Vec::new(), marks: Vec::new() };
        let map_y = MAP_Y[self.map_y];
        let mut square_overlay: Vec<gpui::Div> = Vec::new();
        if self.square == SquareMode::Chord {
            // A chord diagram: strings vertical (low E left), frets down from the nut.
            sq.xdivs = 0;
            let sx = |s: usize| 0.14 + 0.72 * s as f32 / 5.0;
            let fy = |k: f32| 0.22 + 0.70 * k / 5.0;
            let truth = tele.map(|t| t.strings);
            let fretted = truth
                .iter()
                .flat_map(|ts| ts.iter().filter(|x| x.sounding && x.fret > 0).map(|x| x.fret))
                .chain(self.heard.strings.iter().flatten().copied().filter(|f| *f > 0));
            let (lo, hi) = fretted.fold((u8::MAX, 0u8), |(l, h), f| (l.min(f), h.max(f)));
            let base = if hi <= 5 { 1 } else { lo };
            let line = col(GRAT_AXIS, 1.0);
            for st in 0..STRINGS {
                sq.traces.push(Trace { pts: vec![(sx(st), fy(0.0)), (sx(st), fy(5.0))], color: line, width: 1.0, dash: None });
            }
            for k in 0..=5 {
                sq.traces.push(Trace { pts: vec![(sx(0), fy(k as f32)), (sx(5), fy(k as f32))], color: line, width: 1.0, dash: None });
            }
            if base == 1 {
                sq.traces.push(Trace { pts: vec![(sx(0), fy(0.0) - 0.012), (sx(5), fy(0.0) - 0.012)], color: col(TEXT, 1.0), width: 1.5, dash: None });
            } else {
                square_overlay.push(div().absolute().left(u(4.0)).top(u(fy(0.5) * LOWER_H - 7.0)).text_color(rgb(TEXT)).child(format!("{base}FR")));
            }
            let slot = |f: u8| fy((f - base) as f32 + 0.5);
            if let Some(ts) = truth {
                for (st, x) in ts.iter().enumerate() {
                    if !x.sounding {
                        sq.marks.push(Mark::XMark { x: sx(st), y: 0.11, color: col(DIM, 1.0) });
                    } else if x.fret == 0 {
                        sq.marks.push(Mark::Ring { x: sx(st), y: 0.11, r: 4.0, color: col(DRY, 1.0) });
                    } else if x.fret >= base && x.fret < base + 5 {
                        sq.marks.push(Mark::Ring { x: sx(st), y: slot(x.fret), r: 7.0, color: col(DRY, 1.0) });
                    }
                }
            }
            for (st, f) in self.heard.strings.iter().enumerate() {
                match *f {
                    Some(0) => sq.marks.push(Mark::Cross { x: sx(st), y: 0.11, color: col(BRIGHT, 1.0) }),
                    Some(f) if f >= base && f < base + 5 => sq.marks.push(Mark::Cross { x: sx(st), y: slot(f), color: col(BRIGHT, 1.0) }),
                    _ => {}
                }
            }
            square_overlay.push(
                div()
                    .absolute()
                    .left(u(6.0))
                    .top(u(4.0))
                    .text_color(rgb(DRY))
                    .child(self.truth_chord.clone().unwrap_or_else(|| "--".into())),
            );
            square_overlay.push(
                div()
                    .absolute()
                    .right(u(6.0))
                    .top(u(4.0))
                    .text_color(rgb(BRIGHT))
                    .child(self.heard.chord.as_ref().map_or("--".to_string(), |c| c.0.clone())),
            );
        } else if self.square == SquareMode::Xy {
            self.history.latest(2048, &mut self.frames);
            let range = 4.0 * amp_div;
            let pts = self
                .frames
                .iter()
                .map(|f| {
                    // M/S: mono is vertical, side is horizontal.
                    let m = (f.left + f.right) * std::f32::consts::FRAC_1_SQRT_2;
                    let s = (f.left - f.right) * std::f32::consts::FRAC_1_SQRT_2;
                    (0.5 + 0.5 * s / range, 0.5 - 0.5 * m / range)
                })
                .collect();
            sq.traces.push(Trace { pts, color: col(WET, 0.8), width: 1.0, dash: None });
        } else {
            let n = self.trail.len();
            for (i, w) in self.trail.iter().collect::<Vec<_>>().windows(2).enumerate() {
                let age = w[1].2.elapsed().as_secs_f32();
                let a = (1.0 - age / 8.0).clamp(0.0, 1.0) * (0.3 + 0.7 * i as f32 / n.max(1) as f32);
                if a > 0.02 {
                    sq.traces.push(Trace {
                        pts: vec![(w[0].0, 1.0 - w[0].1), (w[1].0, 1.0 - w[1].1)],
                        color: col(MARKER, a),
                        width: 1.0,
                        dash: None,
                    });
                }
            }
            let info = PARAMS[map_y];
            let yv = ((self.controls.get(map_y) - info.min) / (info.max - info.min)) as f32;
            sq.marks.push(Mark::Cross { x: self.controls.material_organization as f32, y: 1.0 - yv, color: col(MARKER, 1.0) });
        }
        let cell = self.square_bounds.clone();
        let square = div()
            .relative()
            .w(u(LOWER_H))
            .h(u(LOWER_H))
            .flex_none()
            .child(canvas(move |b, _, _| cell.set(Some(b)), move |b, _, window, _| paint_screen(b, window, &sq)).absolute().size_full())
            .children(square_overlay);
        let square_caption = match self.square {
            SquareMode::Xy => format!("XY  M/S  {:.2} FS/DIV", amp_div),
            SquareMode::Map => format!("MAP  X ORGANIZ  Y {}", PARAMS[map_y].label),
            SquareMode::Chord => "CHORD  O TRUTH  + HEARD".to_string(),
        };

        // ---- annotation ----
        let digest = self.audio.as_ref().map_or("--------".to_string(), |a| a.digest.chars().take(12).collect());
        let device = self
            .audio
            .as_ref()
            .map_or("NO OUTPUT".to_string(), |a| format!("{}  {} CH", a.device.to_uppercase(), a.channels));
        let piece = match (self.pattern, tele) {
            (Pattern::Score, Some(t)) if t.score_length > 0.0 => format!("SCORE {:.1}/{:.0}", t.score_beat, t.score_length),
            (p, Some(t)) if !t.key.is_empty() && t.pattern == p => format!("{} {}", p.label(), t.key),
            (p, _) => p.label().to_string(),
        };
        let top = div()
            .flex()
            .justify_between()
            .child(t(
                match self.audio.as_ref().and_then(|a| a.input.as_ref()).filter(|_| self.live) {
                    Some(i) => format!(
                        "LIVE IN  {}  CH {}  {:+.1} dB   ->   ARIADNE WORLD {}   {:.1} kHz",
                        i.device.to_uppercase(),
                        self.input_channel_label(i),
                        self.input_gain_db,
                        digest,
                        sr / 1000.0
                    ),
                    None => format!(
                        "SIM GUITAR  {}  {}  {:.0} BPM   ->   ARIADNE WORLD {}   {:.1} kHz",
                        self.guitar.pickup.label(),
                        piece,
                        self.bpm,
                        digest,
                        sr / 1000.0
                    ),
                },
                TEXT,
            ))
            .child(t(format!("T {:8.2} s", now), TEXT));
        let active_txt = self.active_text();
        let upper_title = if self.upper_spectrum {
            "SPECTRUM dBFS / Hz  (MODES REL 60 dB)"
        } else if triggered {
            "SCOPE FS / ms  TRIG'D ON DRY"
        } else {
            "SCOPE FS / ms  AUTO"
        };
        let chord_txt = format!(
            "CHORD  TRUTH {:6}  HEARD {}",
            self.truth_chord.clone().unwrap_or_else(|| "--".into()),
            self.heard.chord.as_ref().map_or("--".to_string(), |(c, conf)| format!("{c} {conf:.2}"))
        );
        let second = div()
            .flex()
            .justify_between()
            .h(u(16.0))
            .child(div().flex().gap(u(24.0)).child(t(active_txt, BRIGHT)).child(t(chord_txt, MARKER)))
            .child(
                div()
                    .flex()
                    .gap(u(10.0))
                    .child(t(upper_title, TEXT))
                    .child(t("DRY", DRY))
                    .child(t("WET", WET))
                    .child(t(if self.upper_spectrum && self.modes { "MODES" } else { "" }, BODY)),
            );
        let strings_row = div().flex().gap(u(10.0)).text_size(u(SMALL_PX)).child(t("STRINGS", DIM)).children(
            (0..STRINGS).map(|i| {
                let hz = tele.map_or(0.0, |t| t.string_hz[i]);
                t(format!("{} {:6.1}", STRING_NAMES[i], hz), DRY)
            }),
        );

        let w = tele.map(|t| t.world).unwrap_or_default();
        let edb = |e: f64| 10.0 * e.max(1e-30).log10();
        let world_row = div()
            .flex()
            .gap(u(14.0))
            .child(t(format!("MATERIAL {:6.1} dB", edb(w.material_energy)), BODY))
            .child(t(format!("RESERVOIR {:6.1} dB", edb(w.reservoir_energy)), RESERVOIR))
            .child(t(format!("SPACE {:6.1} dB", edb(w.space_energy)), SPACE))
            .child(t(format!("CONFIG {:.3}", w.configuration), TEXT))
            .child(t(format!("LEDGER {:+.1e}", w.global_closure_residual), TEXT));
        let m = &self.meters;
        let load = tele.map_or(0.0, |t| t.load);
        let levels = div()
            .flex()
            .gap(u(14.0))
            .child(t(format!("IN PK {:6.1} RMS {:6.1} dBFS", db(m.dry_pk), db(m.dry_rms)), DRY))
            .child(t(format!("OUT PK {:6.1} RMS {:6.1} dBFS", db(m.wet_pk), db(m.wet_rms)), WET))
            .child(t(format!("MASTER {}", menu::fmt_master(self.master_db, self.muted)), if self.muted { WARN } else { TEXT }))
            .child(t(format!("DSP LOAD {:4.0} %", load * 100.0), if load > 0.7 { WARN } else { TEXT }))
            .child(t(format!("{}", device), DIM));

        let annunciators =
            div().flex().gap(u(8.0)).children(self.annunciators().into_iter().map(|(label, lit, c)| annunciator(label, lit, c)));

        let audio_error = self.audio.as_ref().and_then(|a| a.last_error.lock().ok().and_then(|t| t.clone()));
        let (msg, msg_c) = match (&self.error, &self.message) {
            (Some(e), _) => (e.clone(), WARN),
            (None, None) if audio_error.is_some() => (format!("AUDIO: {}", audio_error.unwrap_or_default()).to_uppercase(), WARN),
            (None, Some((m, at, warn))) if at.elapsed().as_secs_f32() < 6.0 => (m.clone(), if *warn { WARN } else { TEXT }),
            _ => (String::new(), TEXT),
        };

        // ---- the bezel ----
        let keys = menu::softkeys(self.menu, &self.view());
        let mut bezel = div()
            .w(u(BEZEL_W))
            .flex_none()
            .flex()
            .flex_col()
            .gap(u(6.0))
            .child(t(menu::title(self.menu), DIM).h(u(16.0)).ml(u(12.0)));
        for (i, k) in keys.iter().enumerate() {
            bezel = bezel.child(
                div()
                    .flex()
                    .gap(u(4.0))
                    .items_center()
                    .child(t(format!("{}", i + 1), DIM).text_size(u(SMALL_PX)).w(u(8.0)))
                    .child(softkey_box(i, k, cx)),
            );
        }
        bezel = bezel.child(
            div()
                .mt(u(6.0))
                .ml(u(12.0))
                .text_size(u(SMALL_PX))
                .text_color(rgb(DIM))
                .flex()
                .flex_col()
                .child("1-8 SOFTKEYS")
                .child("WHEEL / \u{2191}\u{2193} KNOB")
                .child("SHIFT FINE")
                .child("Z X C V B N PLUCK")
                .child("SPACE PLAY/STOP")
                .child("TAB SCOPE/SPECT")
                .child("R REC  M MUTE")
                .child("H MAX HOLD")
                .child("0 RESET WORLD")
                .child("- = MASTER")
                .child("\\ MASTER MUTE")
                .child("ESC MAIN"),
        );

        let lower = div()
            .flex()
            .gap(u(10.0))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(div().flex().child(world_labels).child(world_screen))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .ml(u(LABEL_W))
                            .text_size(u(SMALL_PX))
                            .text_color(rgb(TEXT))
                            .child(format!("-{WORLD_SPAN_S:.0} s"))
                            .child(t(lower_caption, DIM))
                            .child("NOW"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(square)
                    .child(t(square_caption, DIM).text_size(u(SMALL_PX))),
            );

        div()
            .id("instrument")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                let ks = &e.keystroke;
                if ks.modifiers.platform || ks.modifiers.control {
                    return;
                }
                this.key(&ks.key, ks.modifiers.shift);
                cx.notify();
            }))
            .on_scroll_wheel(cx.listener(|this, e: &ScrollWheelEvent, _, cx| {
                let dy = match e.delta {
                    ScrollDelta::Lines(l) => l.y,
                    ScrollDelta::Pixels(p) => f32::from(p.y) / 24.0,
                };
                this.scroll_acc += dy;
                let n = this.scroll_acc.trunc() as i32;
                if n != 0 {
                    this.scroll_acc -= n as f32;
                    this.knob(n, e.modifiers.shift);
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    if this.map_event(e.position) {
                        this.dragging = true;
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                if this.dragging && e.pressed_button == Some(MouseButton::Left) {
                    this.map_event(e.position);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, _| {
                    this.dragging = false;
                }),
            )
            .size_full()
            .relative()
            .bg(rgb(BLACK))
            .child(
                div()
                    .absolute()
                    .left(px(fitted.x))
                    .top(px(fitted.y))
                    .w(px(fitted.w))
                    .h(px(fitted.h))
                    .overflow_hidden()
                    .font_family(FONT)
                    .text_size(u(TEXT_PX))
                    .text_color(rgb(TEXT))
                    .p(u(14.0))
                    .flex()
                    .gap(u(14.0))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(u(5.0))
                            .child(top)
                            .child(second)
                            .child(div().flex().child(upper_labels).child(upper_screen).child(div().w(u(RIGHT_W)).flex_none()))
                            .child(x_labels)
                            .child(strings_row)
                            .child(lower)
                            .child(world_row)
                            .child(levels)
                            .child(annunciators)
                            .child(t(msg, msg_c).h(u(16.0))),
                    )
                    .child(bezel),
            )
    }
}

/// The control socket this instance made, removed when the window closes.
static OWNED_SOCKET: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

fn main() {
    let cli = Cli::parse();
    let activate = !cli.no_activate;
    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(DESIGN_W), px(DESIGN_H)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions { title: Some("Ovillo".into()), ..Default::default() }),
                app_id: Some(BUNDLE_ID.into()),
                focus: activate,
                inactive_frame_interval: None,
                ..Default::default()
            },
            |window, cx| {
                let visible = window.is_visible();
                cx.new(|cx: &mut Context<Ovillo>| {
                    // macOS stops drawing a window that is fully covered,
                    // minimized or on another Space; remember, so a screenshot
                    // can say its picture is stale.
                    cx.observe_window_visibility(window, |o: &mut Ovillo, v, _, _| o.window_visible = v.is_visible()).detach();
                    // Drain the taps, keep takes gapless, and follow the live
                    // patch even when no frame is being drawn.
                    cx.spawn(async move |this, cx| loop {
                        cx.background_executor().timer(std::time::Duration::from_millis(50)).await;
                        if this
                            .update(cx, |o: &mut Ovillo, cx| {
                                o.serve_control();
                                o.drain();
                                o.tick_files();
                                cx.notify();
                            })
                            .is_err()
                        {
                            break;
                        }
                    })
                    .detach();
                    let mut o = Ovillo::new(cx, &cli);
                    o.window_visible = visible;
                    o
                })
            },
        )
        .expect("open window");
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                if let Some(p) = OWNED_SOCKET.get() {
                    control::remove_if_socket(p);
                }
                cx.quit();
            }
        })
        .detach();
        if activate {
            cx.activate(true);
        }
    });
}
