//! What the control socket can ask of the instrument, answered on the UI
//! thread through the same methods the keys, the softkeys and the knob use.
//! Each op takes a JSON object of arguments and returns JSON (or an error
//! sentence). The MCP server's tools map onto these one to one.

use crate::menu::{self, Action, Active, Menu, AMP_DIVS, MAP_Y, TIME_DIVS_MS};
use crate::{db, name_chord, patch, Ovillo, SquareMode, OPEN_MIDI};
use ovillo_dsp::ariadne::{AriadneControls, PARAMS};
use ovillo_dsp::compose::SHAPES;
use ovillo_dsp::guitar::{GuitarSettings, Pickup, STRINGS, STRING_NAMES};
use ovillo_dsp::player::{Event, Pattern};
use ovillo_dsp::rig::{Command, MASTER_MAX_DB, MASTER_MIN_DB};
use ovillo_dsp::score;
use serde_json::{json, Map, Value};

/// The serde field names of the twenty parameters, in `PARAMS` order (what `live.json` uses).
const FIELDS: [&str; 20] = [
    "material_organization",
    "source_coupling",
    "work_input",
    "active_coupling",
    "active_drive",
    "headroom",
    "sustain",
    "sag",
    "bloom",
    "feedback",
    "active_space",
    "interlace",
    "memory",
    "space_coupling",
    "space_decay",
    "space_radiation",
    "body_radiation",
    "listener_scale",
    "mix",
    "output_gain",
];

const OPS: &[&str] = &[
    "ping",
    "get_state",
    "press_keys",
    "set_params",
    "get_patch",
    "load_patch",
    "save_patch",
    "default_patch",
    "set_guitar",
    "set_display",
    "transport",
    "pluck",
    "strum",
    "mute_strings",
    "reset_world",
    "set_output",
    "record",
    "play_score",
];

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase()
}

/// A parameter by id (`active_drive`), patch field (`output_gain`), name
/// (`Hunger`) or label (`HUNGER`), or by a word of its name that no other
/// parameter's has (`Persistence`).
pub fn param_index(name: &str) -> Option<usize> {
    let n = norm(name);
    let exact = PARAMS
        .iter()
        .enumerate()
        .position(|(i, p)| norm(p.id) == n || norm(FIELDS[i]) == n || norm(p.name) == n || norm(p.label) == n);
    if exact.is_some() || n.len() < 4 {
        return exact;
    }
    let mut hits = PARAMS.iter().enumerate().filter(|(_, p)| norm(p.name).contains(&n)).map(|(i, _)| i);
    match (hits.next(), hits.next()) {
        (Some(i), None) => Some(i),
        _ => None,
    }
}

fn args_obj(op: &str, args: &Value, allowed: &[&str]) -> Result<Map<String, Value>, String> {
    let o = match args {
        Value::Null => Map::new(),
        Value::Object(o) => o.clone(),
        other => return Err(format!("{op}: arguments must be an object, not {other}")),
    };
    for k in o.keys() {
        if !allowed.contains(&k.as_str()) {
            let list = if allowed.is_empty() { "none".to_string() } else { allowed.join(", ") };
            return Err(format!("{op}: unknown argument \"{k}\" (takes: {list})"));
        }
    }
    Ok(o)
}

fn f64_arg(o: &Map<String, Value>, k: &str) -> Result<Option<f64>, String> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().filter(|x| x.is_finite()).map(Some).ok_or_else(|| format!("\"{k}\" must be a number, not {v}")),
    }
}

fn bool_arg(o: &Map<String, Value>, k: &str) -> Result<Option<bool>, String> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(v) => Err(format!("\"{k}\" must be true or false, not {v}")),
    }
}

fn str_arg<'a>(o: &'a Map<String, Value>, k: &str) -> Result<Option<&'a str>, String> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.as_str())),
        Some(v) => Err(format!("\"{k}\" must be a string, not {v}")),
    }
}

fn slot_arg(v: &Value) -> Result<u8, String> {
    match v {
        Value::String(s) if s.len() == 1 && "abcABC".contains(s.as_str()) => Ok(s.to_ascii_lowercase().as_bytes()[0] - b'a'),
        Value::Number(_) if v.as_u64().is_some_and(|n| n < 3) => Ok(v.as_u64().unwrap_or(0) as u8),
        other => Err(format!("slot must be \"a\", \"b\" or \"c\", not {other}")),
    }
}

fn pickup_arg(s: &str) -> Result<Pickup, String> {
    match norm(s).as_str() {
        "neck" | "necksc" => Ok(Pickup::Neck),
        "middle" | "mid" | "midsc" => Ok(Pickup::Middle),
        "bridge" | "bridgesc" => Ok(Pickup::Bridge),
        "humbucker" | "hb" | "bucker" => Ok(Pickup::Humbucker),
        _ => Err(format!("unknown pickup \"{s}\" (neck, middle, bridge, humbucker)")),
    }
}

fn pickup_id(p: Pickup) -> &'static str {
    match p {
        Pickup::Neck => "neck",
        Pickup::Middle => "middle",
        Pickup::Bridge => "bridge",
        Pickup::Humbucker => "humbucker",
    }
}

fn active_id(a: Active) -> String {
    match a {
        Active::Param(i) => PARAMS[i].id.to_string(),
        Active::Tempo => "tempo".into(),
        Active::Tone => "tone".into(),
        Active::Volume => "volume".into(),
        Active::TimeDiv => "time_div".into(),
        Active::AmpDiv => "amp_div".into(),
        Active::Master => "master".into(),
    }
}

fn active_arg(s: &str) -> Result<Active, String> {
    Ok(match norm(s).as_str() {
        "tempo" | "bpm" => Active::Tempo,
        "tone" => Active::Tone,
        "volume" => Active::Volume,
        "timediv" | "time" => Active::TimeDiv,
        "ampdiv" | "ampldiv" | "amp" => Active::AmpDiv,
        "master" => Active::Master,
        _ => Active::Param(param_index(s).ok_or_else(|| {
            format!("unknown function \"{s}\" (tempo, tone, volume, time_div, amp_div, master, or a parameter id)")
        })?),
    })
}

/// Key names as the instrument knows them, from what a person might type.
fn key_name(tok: &str) -> (bool, String) {
    let (shift, k) = match tok.split_once('+') {
        Some((m, k)) if m.eq_ignore_ascii_case("shift") && !k.is_empty() => (true, k),
        _ => (false, tok),
    };
    // An upper-case letter is that letter with Shift.
    if k.len() == 1 && k.chars().all(|c| c.is_ascii_uppercase()) {
        return (true, k.to_ascii_lowercase());
    }
    let k = match k.to_ascii_lowercase().as_str() {
        "esc" => "escape".to_string(),
        "spc" | " " => "space".into(),
        "bksp" => "backspace".into(),
        "minus" => "-".into(),
        "equal" | "equals" | "plus" => "=".into(),
        "backslash" => "\\".into(),
        "backtick" | "grave" => "`".into(),
        other => other.to_string(),
    };
    (shift, k)
}

impl Ovillo {
    /// Answer whatever the control socket has brought. True if anything was.
    pub(crate) fn serve_control(&mut self) -> bool {
        let pending = self.control.pending();
        let any = !pending.is_empty();
        for r in pending {
            let result = self.handle(&r.op, &r.args);
            r.answer(result);
        }
        any
    }

    fn handle(&mut self, op: &str, args: &Value) -> Result<Value, String> {
        match op {
            "ping" => Ok(json!({"pong": true, "pid": std::process::id()})),
            "get_state" => {
                args_obj(op, args, &[])?;
                Ok(self.state_json())
            }
            "press_keys" => self.op_press_keys(args),
            "set_params" => self.op_set_params(args),
            "get_patch" => {
                args_obj(op, args, &[])?;
                Ok(self.patch_json())
            }
            "load_patch" => self.op_load_patch(args),
            "save_patch" => {
                let o = args_obj(op, args, &["slot"])?;
                let slot = slot_arg(o.get("slot").ok_or("save_patch needs \"slot\": \"a\", \"b\" or \"c\"")?)?;
                self.press(Action::Save(slot));
                Ok(json!({"saved": patch::slot_path(slot).display().to_string(), "patch": self.controls}))
            }
            "default_patch" => {
                args_obj(op, args, &[])?;
                self.press(Action::DefaultPatch);
                Ok(self.patch_json())
            }
            "set_guitar" => self.op_set_guitar(args),
            "set_display" => self.op_set_display(args),
            "transport" => self.op_transport(args),
            "pluck" => self.op_pluck(args),
            "strum" => self.op_strum(args),
            "mute_strings" => {
                let o = args_obj(op, args, &["strings"])?;
                match o.get("strings") {
                    None | Some(Value::Null) => self.press(Action::MuteStrings),
                    Some(Value::String(s)) if s == "all" => self.press(Action::MuteStrings),
                    Some(Value::Array(a)) => {
                        for s in a {
                            let s = score::string_ref(s)?;
                            self.send(Command::Event { event: Event::Mute { string: s as u8 }, delay_s: 0.0 });
                        }
                    }
                    Some(other) => {
                        let s = score::string_ref(other)?;
                        self.send(Command::Event { event: Event::Mute { string: s as u8 }, delay_s: 0.0 });
                    }
                }
                Ok(json!({"muted": o.get("strings").cloned().unwrap_or(json!("all"))}))
            }
            "reset_world" => {
                args_obj(op, args, &[])?;
                self.press(Action::ResetWorld);
                Ok(json!({"reset": true}))
            }
            "set_output" => {
                let o = args_obj(op, args, &["master_db", "mute", "toggle_mute"])?;
                let mut db = self.master_db;
                let mut mute = self.muted;
                if let Some(d) = f64_arg(&o, "master_db")? {
                    if !(MASTER_MIN_DB..=MASTER_MAX_DB).contains(&d) {
                        return Err(format!("master_db {d} is out of range {MASTER_MIN_DB}..={MASTER_MAX_DB} (use mute for silence)"));
                    }
                    db = d;
                }
                if let Some(m) = bool_arg(&o, "mute")? {
                    mute = m;
                }
                if bool_arg(&o, "toggle_mute")? == Some(true) {
                    mute = !mute;
                }
                self.set_master(db, mute);
                Ok(json!({"master_db": self.master_db, "muted": self.muted}))
            }
            "record" => {
                let o = args_obj(op, args, &["action"])?;
                let action = str_arg(&o, "action")?.unwrap_or("toggle");
                let want = match action {
                    "start" => true,
                    "stop" => false,
                    "toggle" => self.take.is_none(),
                    other => return Err(format!("record action \"{other}\" must be start, stop or toggle")),
                };
                if want != self.take.is_some() {
                    self.press(Action::Record);
                }
                Ok(self.record_json())
            }
            "play_score" => self.op_play_score(args),
            other => Err(format!("unknown op \"{other}\" (ops: {})", OPS.join(", "))),
        }
    }

    fn op_press_keys(&mut self, args: &Value) -> Result<Value, String> {
        let o = args_obj("press_keys", args, &["keys"])?;
        let tokens: Vec<String> = match o.get("keys") {
            Some(Value::String(s)) => s.split_whitespace().map(str::to_string).collect(),
            Some(Value::Array(a)) => a.iter().map(|v| v.as_str().map(str::to_string).ok_or("keys must be strings")).collect::<Result<_, _>>()?,
            _ => return Err("press_keys needs \"keys\": \"5 5 6 escape\" (or a list)".into()),
        };
        let mut pressed = Vec::new();
        let mut ignored = Vec::new();
        for tok in tokens {
            let (shift, k) = key_name(&tok);
            if self.key(&k, shift) {
                pressed.push(tok);
            } else {
                ignored.push(tok);
            }
        }
        Ok(json!({"pressed": pressed, "ignored": ignored, "menu": self.menu_json(), "message": self.message_text()}))
    }

    fn op_set_params(&mut self, args: &Value) -> Result<Value, String> {
        let o = args_obj("set_params", args, &["params"])?;
        let params = o.get("params").and_then(Value::as_object).ok_or("set_params needs \"params\": {\"active_drive\": 0.6, ...}")?;
        if params.is_empty() {
            return Err("set_params: \"params\" is empty".into());
        }
        let mut c = self.controls;
        let mut clamped = Vec::new();
        for (k, v) in params {
            let i = param_index(k).ok_or_else(|| {
                format!("unknown parameter \"{k}\" (ids: {})", PARAMS.iter().map(|p| p.id).collect::<Vec<_>>().join(", "))
            })?;
            let x = v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| format!("{k}: {v} is not a number"))?;
            let info = PARAMS[i];
            let y = x.clamp(info.min, info.max);
            if y != x {
                clamped.push(json!({"id": info.id, "asked": x, "set": y, "min": info.min, "max": info.max}));
            }
            c.set(i, y);
        }
        self.set_controls(c);
        let set: Map<String, Value> =
            params.keys().filter_map(|k| param_index(k)).map(|i| (PARAMS[i].id.to_string(), json!(self.controls.get(i)))).collect();
        Ok(json!({"set": set, "clamped": clamped, "live_patch": patch::live_path().display().to_string()}))
    }

    fn op_load_patch(&mut self, args: &Value) -> Result<Value, String> {
        let o = args_obj("load_patch", args, &["slot", "patch", "default"])?;
        if let Some(s) = o.get("slot") {
            let slot = slot_arg(s)?;
            let path = patch::slot_path(slot);
            patch::read(&path).map_err(|_| format!("slot {} is empty ({})", (b'A' + slot) as char, path.display()))?;
            self.press(Action::Load(slot));
        } else if let Some(p) = o.get("patch") {
            let v = match p {
                Value::String(s) => serde_json::from_str::<Value>(s).map_err(|e| format!("patch is not JSON: {e}"))?,
                other => other.clone(),
            };
            let c: AriadneControls = serde_json::from_value(v).map_err(|e| format!("not an Ariadne patch: {e}"))?;
            self.set_controls(c);
            self.say("PATCH LOADED OVER THE CONTROL SOCKET", false);
        } else if bool_arg(&o, "default")? == Some(true) {
            self.press(Action::DefaultPatch);
        } else {
            return Err("load_patch needs \"slot\" (a, b, c), \"patch\" (the 20 fields, as in live.json) or \"default\": true".into());
        }
        Ok(self.patch_json())
    }

    fn op_set_guitar(&mut self, args: &Value) -> Result<Value, String> {
        let o = args_obj("set_guitar", args, &["pickup", "tone", "volume", "hum"])?;
        let mut g = self.guitar;
        if let Some(p) = str_arg(&o, "pickup")? {
            g.pickup = pickup_arg(p)?;
        }
        for (k, slot) in [("tone", &mut g.tone), ("volume", &mut g.volume)] {
            if let Some(x) = f64_arg(&o, k)? {
                if !(0.0..=1.0).contains(&x) {
                    return Err(format!("{k} {x} is out of range 0..=1"));
                }
                *slot = x;
            }
        }
        if let Some(h) = bool_arg(&o, "hum")? {
            g.hum = h;
        }
        self.set_guitar(g);
        Ok(self.guitar_json())
    }

    fn op_set_display(&mut self, args: &Value) -> Result<Value, String> {
        let o = args_obj(
            "set_display",
            args,
            &["upper", "lower", "square", "map_y", "time_div_ms", "amp_div", "modes", "hold", "clear_hold", "menu", "active"],
        )?;
        if let Some(u) = str_arg(&o, "upper")? {
            self.upper_spectrum = match u {
                "spectrum" => true,
                "scope" => false,
                _ => return Err(format!("upper \"{u}\" must be spectrum or scope")),
            };
        }
        if let Some(l) = str_arg(&o, "lower")? {
            self.lower_tab = match l {
                "tab" => true,
                "world" => false,
                _ => return Err(format!("lower \"{l}\" must be world or tab")),
            };
        }
        if let Some(sq) = str_arg(&o, "square")? {
            self.square = match sq {
                "map" => SquareMode::Map,
                "xy" => SquareMode::Xy,
                "chord" => SquareMode::Chord,
                _ => return Err(format!("square \"{sq}\" must be map, xy or chord")),
            };
        }
        if let Some(y) = str_arg(&o, "map_y")? {
            let i = param_index(y).ok_or_else(|| format!("unknown parameter \"{y}\""))?;
            let k = MAP_Y.iter().position(|m| *m == i).ok_or_else(|| {
                format!("map_y can be {}", MAP_Y.iter().map(|m| PARAMS[*m].id).collect::<Vec<_>>().join(", "))
            })?;
            self.map_y = k;
            self.trail.clear();
            self.mark_trail();
        }
        if let Some(t) = f64_arg(&o, "time_div_ms")? {
            self.time_div = nearest(&TIME_DIVS_MS, t);
        }
        if let Some(a) = f64_arg(&o, "amp_div")? {
            self.amp_div = nearest(&AMP_DIVS, a);
        }
        if let Some(m) = bool_arg(&o, "modes")? {
            self.modes = m;
        }
        if let Some(h) = bool_arg(&o, "hold")? {
            if h != self.hold {
                self.press(Action::ToggleHold);
            }
        }
        if bool_arg(&o, "clear_hold")? == Some(true) {
            self.spectrum.clear_hold();
        }
        if let Some(m) = str_arg(&o, "menu")? {
            let menu = Menu::NAMED
                .iter()
                .find(|(n, _)| *n == m)
                .map(|(_, m)| *m)
                .ok_or_else(|| format!("menu \"{m}\" is one of {}", Menu::NAMED.map(|(n, _)| n).join(", ")))?;
            self.press(Action::Open(menu));
        }
        match o.get("active") {
            None => {}
            Some(Value::Null) => self.active = None,
            Some(Value::String(a)) => self.active = Some(active_arg(a)?),
            Some(other) => return Err(format!("active must be a function name or null, not {other}")),
        }
        Ok(self.display_json())
    }

    fn op_transport(&mut self, args: &Value) -> Result<Value, String> {
        let o = args_obj("transport", args, &["play", "tempo", "composition"])?;
        if let Some(c) = str_arg(&o, "composition")? {
            let p = Pattern::from_id(c).ok_or_else(|| {
                format!("unknown composition \"{c}\" ({})", Pattern::ALL.map(|p| p.id()).join(", "))
            })?;
            if p == Pattern::Score && self.score.is_none() {
                return Err("no score loaded: send one with play_score".into());
            }
            self.set_pattern(p);
        }
        if let Some(t) = f64_arg(&o, "tempo")? {
            if !(40.0..=220.0).contains(&t) {
                return Err(format!("tempo {t} is out of range 40..=220 BPM"));
            }
            self.bpm = t;
            self.send(Command::Tempo(t));
        }
        if let Some(play) = bool_arg(&o, "play")? {
            if play != (self.pattern != Pattern::Off) {
                self.press(Action::PlayStop);
            }
        }
        Ok(self.transport_json())
    }

    fn op_pluck(&mut self, args: &Value) -> Result<Value, String> {
        let o = args_obj("pluck", args, &["string", "fret", "velocity", "palm", "bend", "bend_after_ms", "vibrato", "vibrato_after_ms"])?;
        let s = score::string_ref(o.get("string").ok_or("pluck needs \"string\" (0 = low E … 5 = high e, or E A D G B e)")?)?;
        let fret = f64_arg(&o, "fret")?.unwrap_or(0.0);
        if fret.fract() != 0.0 || !(0.0..=22.0).contains(&fret) {
            return Err(format!("fret {fret} must be a whole number 0..=22"));
        }
        let v = f64_arg(&o, "velocity")?.unwrap_or(0.75);
        if !(0.0..=1.0).contains(&v) {
            return Err(format!("velocity {v} is out of range 0..=1"));
        }
        let palm = bool_arg(&o, "palm")?.unwrap_or(false);
        let string = s as u8;
        self.send(Command::Event { event: Event::Pluck { string, fret: fret as u8, velocity: v as f32, palm }, delay_s: 0.0 });
        if let Some(b) = f64_arg(&o, "bend")? {
            if !(0.0..=3.0).contains(&b) {
                return Err(format!("bend {b} is out of range 0..=3 semitones"));
            }
            let after = f64_arg(&o, "bend_after_ms")?.unwrap_or(80.0).clamp(0.0, 10_000.0);
            self.send(Command::Event { event: Event::Bend { string, semitones: b as f32 }, delay_s: (after / 1000.0) as f32 });
        }
        if let Some(d) = f64_arg(&o, "vibrato")? {
            if !(0.0..=1.0).contains(&d) {
                return Err(format!("vibrato {d} is out of range 0..=1 semitone"));
            }
            let after = f64_arg(&o, "vibrato_after_ms")?.unwrap_or(250.0).clamp(0.0, 10_000.0);
            self.send(Command::Event { event: Event::Vibrato { string, depth: d as f32 }, delay_s: (after / 1000.0) as f32 });
        }
        Ok(json!({"plucked": {"string": s, "name": STRING_NAMES[s], "fret": fret as u8, "midi": OPEN_MIDI[s] as u32 + fret as u32, "velocity": v, "palm": palm}}))
    }

    fn op_strum(&mut self, args: &Value) -> Result<Value, String> {
        let o = args_obj("strum", args, &["chord", "dir", "velocity", "spread_ms"])?;
        let frets = score::chord_ref(o.get("chord").ok_or("strum needs \"chord\": a name (\"Em9\"), six frets low E to high e, or tab (\"x32010\")")?)?;
        let down = match str_arg(&o, "dir")? {
            None | Some("down") => true,
            Some("up") => false,
            Some(d) => return Err(format!("dir \"{d}\" must be down or up")),
        };
        let v = f64_arg(&o, "velocity")?.unwrap_or(0.8);
        if !(0.0..=1.0).contains(&v) {
            return Err(format!("velocity {v} is out of range 0..=1"));
        }
        let spread = f64_arg(&o, "spread_ms")?.unwrap_or(30.0);
        if !(0.0..=1000.0).contains(&spread) {
            return Err(format!("spread_ms {spread} is out of range 0..=1000"));
        }
        self.send(Command::Strum { frets, down, velocity: v as f32, spread_s: (spread / 1000.0) as f32 });
        let notes: Vec<u8> = (0..STRINGS).filter(|s| frets[*s] >= 0).map(|s| OPEN_MIDI[s] + frets[s] as u8).collect();
        Ok(json!({"strummed": frets, "dir": if down { "down" } else { "up" }, "spells": name_chord(&notes)}))
    }

    fn op_play_score(&mut self, args: &Value) -> Result<Value, String> {
        let o = args_obj("play_score", args, &["score"])?;
        let v = match o.get("score") {
            Some(Value::String(s)) => serde_json::from_str::<Value>(s).map_err(|e| format!("score is not JSON: {e}"))?,
            Some(v @ Value::Object(_)) => v.clone(),
            _ => return Err("play_score needs \"score\": {\"tempo\": 96, \"events\": [...]}".into()),
        };
        let sc = score::parse(&v, self.bpm)?;
        let summary = sc.summary();
        self.play_score(sc);
        Ok(summary)
    }

    // ---- state as data ----

    pub(crate) fn message_text(&self) -> Option<String> {
        self.message.as_ref().filter(|(_, at, _)| at.elapsed().as_secs_f32() < 6.0).map(|(m, _, _)| m.clone())
    }

    fn menu_json(&self) -> Value {
        let view = self.view();
        let keys = menu::softkeys(self.menu, &view);
        let softkeys: Vec<Value> = keys
            .iter()
            .enumerate()
            .map(|(i, k)| json!({"key": i + 1, "label": [k.lines[0], k.lines[1]], "lit": k.lit, "blank": k.is_blank()}))
            .collect();
        json!({
            "id": self.menu.id(),
            "title": menu::title(self.menu),
            "softkeys": softkeys,
            "active": self.active.map(|a| json!({"id": active_id(a), "text": self.active_text()})),
        })
    }

    fn patch_json(&self) -> Value {
        json!({
            "patch": self.controls,
            "live_patch": patch::live_path().display().to_string(),
            "slots": (0..3u8).map(|s| json!({"slot": ((b'a' + s) as char).to_string(), "path": patch::slot_path(s).display().to_string(), "saved": patch::slot_path(s).exists()})).collect::<Vec<_>>(),
        })
    }

    fn guitar_json(&self) -> Value {
        let g: GuitarSettings = self.guitar;
        json!({"pickup": pickup_id(g.pickup), "pickup_label": g.pickup.label(), "tone": g.tone, "volume": g.volume, "hum": g.hum})
    }

    fn display_json(&self) -> Value {
        json!({
            "upper": if self.upper_spectrum { "spectrum" } else { "scope" },
            "lower": if self.lower_tab { "tab" } else { "world" },
            "square": match self.square { SquareMode::Map => "map", SquareMode::Xy => "xy", SquareMode::Chord => "chord" },
            "map_y": PARAMS[MAP_Y[self.map_y]].id,
            "map_y_choices": MAP_Y.map(|m| PARAMS[m].id),
            "time_div_ms": TIME_DIVS_MS[self.time_div],
            "time_div_choices": TIME_DIVS_MS,
            "amp_div": AMP_DIVS[self.amp_div],
            "amp_div_choices": AMP_DIVS,
            "modes": self.modes,
            "hold": self.hold,
            "menu": self.menu_json(),
        })
    }

    fn transport_json(&self) -> Value {
        let t = self.tele;
        json!({
            "composition": self.pattern.id(),
            "label": self.pattern.label(),
            "key": t.map_or("", |t| t.key),
            "playing": self.pattern != Pattern::Off,
            "tempo": self.bpm,
            "score": self.score.as_ref().map(|s| json!({
                "summary": s.summary,
                "beat": t.map(|t| t.score_beat),
                "length_beats": t.map(|t| t.score_length),
                "loops": t.map(|t| t.score_loops),
            })),
            "compositions": Pattern::ALL.iter().map(|p| json!({"id": p.id(), "label": p.label(), "about": p.about()})).collect::<Vec<_>>(),
        })
    }

    fn record_json(&self) -> Value {
        match &self.take {
            Some(t) => json!({
                "recording": true,
                "take_dir": t.dir.display().to_string(),
                "files": {"dry": t.dir.join("dry.wav").display().to_string(), "wet": t.dir.join("wet.wav").display().to_string(), "patch": t.dir.join("patch.json").display().to_string()},
                "seconds": t.frames as f64 / self.sr(),
            }),
            None => json!({
                "recording": false,
                "last_take": self.last_take.as_ref().map(|d| json!({
                    "take_dir": d.display().to_string(),
                    "files": {"dry": d.join("dry.wav").display().to_string(), "wet": d.join("wet.wav").display().to_string(), "patch": d.join("patch.json").display().to_string()},
                })),
            }),
        }
    }

    /// Everything on the screen, as data.
    pub(crate) fn state_json(&self) -> Value {
        let t = self.tele;
        let m = &self.meters;
        let w = t.map(|t| t.world).unwrap_or_default();
        let edb = |e: f64| (10.0 * e.max(1e-30).log10() * 10.0).round() / 10.0;
        let r1 = |x: f32| (db(x) * 10.0).round() / 10.0;
        let params: Vec<Value> = PARAMS
            .iter()
            .enumerate()
            .map(|(i, p)| {
                json!({"id": p.id, "field": FIELDS[i], "name": p.name, "label": p.label, "value": self.controls.get(i), "min": p.min, "max": p.max, "display": menu::fmt_param(i, self.controls.get(i))})
            })
            .collect();
        let strings: Vec<Value> = (0..STRINGS)
            .map(|i| {
                let s = t.map(|t| t.strings[i]).unwrap_or_default();
                json!({"string": i, "name": STRING_NAMES[i], "fret": s.fret, "semitones": (s.semitones * 100.0).round() / 100.0, "hz": (s.hz * 10.0).round() / 10.0, "level": s.level, "sounding": s.sounding})
            })
            .collect();
        let annunciators: Map<String, Value> = self.annunciators().into_iter().map(|(n, lit, _)| (n.to_string(), json!(lit))).collect();
        let audio = self.audio.as_ref();
        json!({
            "pid": std::process::id(),
            "control": self.control.describe(),
            "audio": {
                "device": audio.map(|a| a.device.clone()),
                "channels": audio.map(|a| a.channels),
                "sample_rate": self.sr(),
                "plan_digest": audio.map(|a| a.digest.clone()),
                "error": self.error.clone(),
            },
            "time_s": t.map_or(0.0, |t| t.sample as f64 / t.sample_rate),
            "menu": self.menu_json(),
            "ariadne": params,
            "guitar": self.guitar_json(),
            "transport": self.transport_json(),
            "display": self.display_json(),
            "output": {"master_db": self.master_db, "muted": self.muted, "master_gain_now": t.map(|t| t.master)},
            "record": self.record_json(),
            "meters": {
                "in_peak_dbfs": r1(m.dry_pk), "in_rms_dbfs": r1(m.dry_rms),
                "out_peak_dbfs": r1(m.wet_pk), "out_rms_dbfs": r1(m.wet_rms),
                "note": "OUT is Ariadne's output before the master stage",
            },
            "world": {
                "material_db": edb(w.material_energy), "reservoir_db": edb(w.reservoir_energy), "space_db": edb(w.space_energy),
                "configuration": w.configuration, "ledger_residual": w.global_closure_residual,
                "errors": t.map_or(0, |t| t.world_errors), "safety_limited_frames": t.map_or(0, |t| t.safety_limited_frames),
            },
            "dsp_load": t.map_or(0.0, |t| t.load),
            "truth": {"chord": self.truth_chord, "strings": strings},
            "heard": {"chord": self.heard.chord.as_ref().map(|(c, conf)| json!({"name": c, "confidence": conf})), "frets": self.heard.strings},
            "annunciators": annunciators,
            "message": self.message_text(),
            "chord_shapes": SHAPES.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
            "live_patch": patch::live_path().display().to_string(),
            "socket": self.control.path.as_ref().map(|p| p.display().to_string()),
        })
    }
}

fn nearest(choices: &[f64], x: f64) -> usize {
    choices
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
        .map_or(0, |(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_are_found_by_id_field_name_label_or_a_unique_word() {
        let id = |s: &str| param_index(s).map(|i| PARAMS[i].id);
        assert_eq!(id("active_drive"), Some("active_drive"));
        assert_eq!(id("output_gain"), Some("output"));
        assert_eq!(id("material_organization"), Some("organization"));
        assert_eq!(id("Hunger"), Some("sag"));
        assert_eq!(id("METABOL"), Some("bloom"));
        assert_eq!(id("Reciprocity"), Some("feedback"));
        assert_eq!(id("Persistence"), Some("space_decay"));
        assert_eq!(id("Space"), None, "ambiguous");
        assert_eq!(id("drve"), None);
    }

    #[test]
    fn key_names_read_as_a_person_types_them() {
        assert_eq!(key_name("esc"), (false, "escape".to_string()));
        assert_eq!(key_name("shift+up"), (true, "up".to_string()));
        assert_eq!(key_name("Z"), (true, "z".to_string()));
        assert_eq!(key_name("backslash"), (false, "\\".to_string()));
        assert_eq!(key_name("-"), (false, "-".to_string()));
        assert_eq!(key_name("F3"), (false, "f3".to_string()));
    }
}
