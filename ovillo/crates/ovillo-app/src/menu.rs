//! The softkey menus, as data. Eight keys on the right bezel; a key opens a
//! menu, toggles a setting, does an action, or makes a quantity the *active
//! function*, which the knob (scroll wheel, ↑/↓) then turns.

use ovillo_dsp::ariadne::PARAMS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Root,
    Source,
    /// Ariadne's parameters, three pages.
    Ariadne(usize),
    Display,
    Patch,
    /// The monitor: master level and mute, takes, the World's reset.
    Output,
}

impl Menu {
    /// Every menu, with the name the control socket uses for it.
    pub const NAMED: [(&'static str, Menu); 8] = [
        ("main", Menu::Root),
        ("source", Menu::Source),
        ("material", Menu::Ariadne(0)),
        ("metabolism", Menu::Ariadne(1)),
        ("space", Menu::Ariadne(2)),
        ("display", Menu::Display),
        ("patch", Menu::Patch),
        ("output", Menu::Output),
    ];

    pub fn id(self) -> &'static str {
        Menu::NAMED.iter().find(|(_, m)| *m == self).map_or("main", |(n, _)| n)
    }
}

/// What the knob turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Active {
    Param(usize),
    Tempo,
    Tone,
    Volume,
    TimeDiv,
    AmpDiv,
    /// The master (monitor) level, in dB.
    Master,
    InputGain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open(Menu),
    Return,
    Activate(Active),
    CyclePattern,
    CyclePickup,
    ToggleHum,
    MuteStrings,
    PlayStop,
    ResetWorld,
    ToggleUpper,
    ToggleModes,
    ToggleHold,
    ToggleLower,
    CycleSquare,
    CycleMapY,
    Save(u8),
    Load(u8),
    DefaultPatch,
    ToggleMute,
    Record,
    ToggleSource,
    CycleInputChannel,
    Nothing,
}

#[derive(Debug, Clone)]
pub struct Softkey {
    pub lines: [String; 2],
    pub action: Action,
    pub lit: bool,
}

impl Softkey {
    pub fn blank() -> Self {
        Self { lines: [String::new(), String::new()], action: Action::Nothing, lit: false }
    }
    pub fn is_blank(&self) -> bool {
        self.action == Action::Nothing
    }
}

/// Parameter indices on each Ariadne page.
pub const PAGES: [(&str, &[usize]); 3] = [
    ("MATERIAL", &[0, 1, 2, 3, 4, 5, 6]),
    ("METABOLISM", &[7, 8, 9, 10, 11, 12]),
    ("SPACE + OUT", &[13, 14, 15, 16, 17, 18, 19]),
];

/// Parameters the configuration map's vertical axis can take (its horizontal
/// axis is always Material Organization).
pub const MAP_Y: [usize; 6] = [4, 2, 3, 13, 11, 7];

pub const TIME_DIVS_MS: [f64; 8] = [0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0];
pub const AMP_DIVS: [f64; 7] = [0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5];

/// Everything the menus show, read from the instrument.
pub struct View<'a> {
    pub params: [f64; 20],
    pub pattern: &'a str,
    pub playing: bool,
    pub bpm: f64,
    pub pickup: &'a str,
    pub tone: f64,
    pub volume: f64,
    pub hum: bool,
    pub upper_spectrum: bool,
    pub modes: bool,
    pub lower_tab: bool,
    pub square: &'a str,
    pub map_y: usize,
    pub time_div_ms: f64,
    pub amp_div: f64,
    pub active: Option<Active>,
    pub master_db: f64,
    pub muted: bool,
    pub recording: bool,
    /// With a live input: (live now, input gain dB, channel label).
    pub input: Option<(bool, f64, String)>,
}

pub fn fmt_master(db: f64, muted: bool) -> String {
    if muted {
        "MUTED".into()
    } else {
        format!("{db:+.1} dB")
    }
}

fn key(a: &str, b: impl Into<String>, action: Action) -> Softkey {
    Softkey { lines: [a.to_string(), b.into()], action, lit: false }
}

pub fn fmt_param(i: usize, v: f64) -> String {
    match PARAMS[i].id {
        "output" => format!("{:+.1} dB", 20.0 * v.max(1e-6).log10()),
        "mix" => format!("{:.0} %", v * 100.0),
        "listener_scale" | "source_coupling" => format!("{v:.2}"),
        _ => format!("{v:.3}"),
    }
}

pub fn title(m: Menu) -> String {
    match m {
        Menu::Root => "MAIN".into(),
        Menu::Source => "SOURCE".into(),
        Menu::Ariadne(p) => format!("ARIADNE {}", PAGES[p].0),
        Menu::Display => "DISPLAY".into(),
        Menu::Patch => "PATCH".into(),
        Menu::Output => "OUTPUT".into(),
    }
}

pub fn softkeys(m: Menu, v: &View) -> Vec<Softkey> {
    let mut keys: Vec<Softkey> = match m {
        Menu::Root => vec![
            key(
                "SOURCE",
                if v.input.as_ref().is_some_and(|i| i.0) { "LIVE ▸".to_string() } else { format!("{} ▸", v.pattern) },
                Action::Open(Menu::Source),
            ),
            key("MATERIAL", "▸", Action::Open(Menu::Ariadne(0))),
            key("METABOL", "▸", Action::Open(Menu::Ariadne(1))),
            key("SPACE+OUT", "▸", Action::Open(Menu::Ariadne(2))),
            key("DISPLAY", "▸", Action::Open(Menu::Display)),
            key("PATCH", "▸", Action::Open(Menu::Patch)),
            Softkey { lit: v.playing, ..key("PLAY", if v.playing { "STOP" } else { "START" }, Action::PlayStop) },
            key("OUTPUT", format!("{} ▸", fmt_master(v.master_db, v.muted)), Action::Open(Menu::Output)),
        ],
        Menu::Source if v.input.is_some() => {
            let (live, gain, ch) = v.input.clone().unwrap_or_default();
            vec![
                Softkey { lit: live, ..key("SOURCE", if live { "LIVE IN" } else { "SIM GUITAR" }, Action::ToggleSource) },
                key("IN GAIN", format!("{gain:+.1} dB"), Action::Activate(Active::InputGain)),
                key("IN CHAN", ch, Action::CycleInputChannel),
                key("PATTERN", v.pattern, Action::CyclePattern),
                key("TEMPO", format!("{:.0} BPM", v.bpm), Action::Activate(Active::Tempo)),
                key("PICKUP", v.pickup, Action::CyclePickup),
                key("MUTE", "STRINGS", Action::MuteStrings),
                key("RETURN", "", Action::Return),
            ]
        }
        Menu::Source => vec![
            key("PATTERN", v.pattern, Action::CyclePattern),
            key("TEMPO", format!("{:.0} BPM", v.bpm), Action::Activate(Active::Tempo)),
            key("PICKUP", v.pickup, Action::CyclePickup),
            key("TONE", format!("{:.2}", v.tone), Action::Activate(Active::Tone)),
            key("VOLUME", format!("{:.2}", v.volume), Action::Activate(Active::Volume)),
            Softkey { lit: v.hum, ..key("HUM", if v.hum { "ON" } else { "OFF" }, Action::ToggleHum) },
            key("MUTE", "STRINGS", Action::MuteStrings),
            key("RETURN", "", Action::Return),
        ],
        Menu::Ariadne(p) => {
            let mut k: Vec<Softkey> = PAGES[p]
                .1
                .iter()
                .map(|&i| key(PARAMS[i].label, fmt_param(i, v.params[i]), Action::Activate(Active::Param(i))))
                .collect();
            while k.len() < 7 {
                k.push(Softkey::blank());
            }
            k.push(key("RETURN", "", Action::Return));
            k
        }
        Menu::Display => vec![
            key("UPPER", if v.upper_spectrum { "SPECTRUM" } else { "SCOPE" }, Action::ToggleUpper),
            key("TIME/DIV", format!("{} ms", v.time_div_ms), Action::Activate(Active::TimeDiv)),
            key("AMPL/DIV", format!("{} FS", v.amp_div), Action::Activate(Active::AmpDiv)),
            Softkey { lit: v.modes, ..key("MODES", if v.modes { "ON" } else { "OFF" }, Action::ToggleModes) },
            key("LOWER", if v.lower_tab { "TAB" } else { "WORLD" }, Action::ToggleLower),
            key("SQUARE", v.square, Action::CycleSquare),
            key("MAP Y", PARAMS[v.map_y].label, Action::CycleMapY),
            key("RETURN", "", Action::Return),
        ],
        Menu::Patch => vec![
            key("SAVE", "A", Action::Save(0)),
            key("SAVE", "B", Action::Save(1)),
            key("SAVE", "C", Action::Save(2)),
            key("LOAD", "A", Action::Load(0)),
            key("LOAD", "B", Action::Load(1)),
            key("LOAD", "C", Action::Load(2)),
            key("DEFAULT", "PATCH", Action::DefaultPatch),
            key("RETURN", "", Action::Return),
        ],
        Menu::Output => vec![
            key("MASTER", format!("{:+.1} dB", v.master_db), Action::Activate(Active::Master)),
            Softkey { lit: v.muted, ..key("MUTE", if v.muted { "ON" } else { "OFF" }, Action::ToggleMute) },
            Softkey { lit: v.recording, ..key("REC", if v.recording { "STOP" } else { "TAKE" }, Action::Record) },
            key("RESET", "WORLD", Action::ResetWorld),
            Softkey::blank(),
            Softkey::blank(),
            Softkey::blank(),
            key("RETURN", "", Action::Return),
        ],
    };
    for k in &mut keys {
        if let Action::Activate(a) = k.action {
            k.lit = v.active == Some(a);
        }
    }
    keys
}
