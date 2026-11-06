//! The surface: every knob is a physical quantity of the Body, with its unit,
//! range, default, what it is, and what it does to the sound. A [`BodySpec`]
//! is a name and a map `knob id -> value`; anything not set is the default.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// How a knob's value is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnobKind {
    /// A continuous physical quantity.
    Real,
    /// A count (rounded to the nearest integer).
    Integer,
    /// 0 or 1.
    Switch,
}

/// One knob of the surface.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Knob {
    pub id: &'static str,
    pub group: &'static str,
    pub unit: &'static str,
    pub kind: KnobKind,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    /// The physical thing this is.
    pub physics: &'static str,
    /// What moving it does to the sound.
    pub sound: &'static str,
}

const fn real(id: &'static str, group: &'static str, unit: &'static str, min: f64, max: f64, default: f64, physics: &'static str, sound: &'static str) -> Knob {
    Knob { id, group, unit, kind: KnobKind::Real, min, max, default, physics, sound }
}

const fn int(id: &'static str, group: &'static str, min: f64, max: f64, default: f64, physics: &'static str, sound: &'static str) -> Knob {
    Knob { id, group, unit: "count", kind: KnobKind::Integer, min, max, default, physics, sound }
}

const fn switch(id: &'static str, group: &'static str, default: f64, physics: &'static str, sound: &'static str) -> Knob {
    Knob { id, group, unit: "0/1", kind: KnobKind::Switch, min: 0.0, max: 1.0, default, physics, sound }
}

/// Number of bump slots on the bore, of side-branch slots, and of resonator slots.
pub const BUMPS: usize = 3;
pub const BRANCHES: usize = 3;
pub const RESONATORS: usize = 3;

macro_rules! resonator {
    ($k:literal, $def_kind:expr, $def_f1:expr) => {
        [
            switch(concat!("res", $k, ".on"), "resonators", 0.0,
                "Whether this resonator exists: a passive 24-mode body (a declared partial series) touching one air volume through a reciprocal, lossless direct coupling (a Givens exchange between the air's pressure there and the resonator's motion along its touch pattern), radiating mode by mode.",
                "A sympathetic body that rings at her harmonics where its partials meet them and pushes back on the air: strings behind a voice, a drumhead across a pipe, a plate wall, a bell."),
            int(concat!("res", $k, ".kind"), "resonators", 0.0, 4.0, $def_kind,
                "Partial series: 0 string (n f1 sqrt(1 + B n^2)), 1 free-free bar (beta_n^2: 1, 2.76, 5.40, ...), 2 circular membrane (Bessel zeros: 1, 1.59, 2.14, 2.30, ...), 3 rectangular plate 1:1.3 (m^2 + (n/1.3)^2), 4 bell (hum 0.5, prime 1, tierce 1.2, quint 1.5, nominal 2, ...; a design profile, not a measurement).",
                "Harmonic series lock to her pitch; stretched series (bar, membrane, plate) add inharmonic shimmer; the bell adds a minor third and a hum below the prime."),
            real(concat!("res", $k, ".f1_hz"), "resonators", "Hz", 20.0, 8000.0, $def_f1,
                "The first partial (the prime, for a bell).",
                "Where the resonator's series starts; partials above 0.47 x 48 kHz are inert."),
            real(concat!("res", $k, ".inharmonicity"), "resonators", "B", 0.0, 0.05, 0.0,
                "Stiffness stretch B of a string's partials (f_n = n f1 sqrt(1 + B n^2)); kinds other than string ignore it.",
                "Piano-like stretch of the upper partials."),
            real(concat!("res", $k, ".t60_s"), "resonators", "s", 0.01, 30.0, 2.0,
                "Decay time (60 dB) of the first partial from internal loss.",
                "How long it rings after her; short: a thud, long: a halo that outlasts the word."),
            real(concat!("res", $k, ".t60_slope"), "resonators", "exponent", 0.0, 3.0, 1.0,
                "Partial n's T60 is t60_s / ratio_n^slope.",
                "0: all partials ring equally; 1 or more: the upper partials die first (warmer tail)."),
            real(concat!("res", $k, ".radiation_per_s"), "resonators", "1/s", 0.0, 2000.0, 20.0,
                "Radiation amplitude rate of the first partial (how fast it gives its energy to the air outside).",
                "Louder and shorter when high; 0: it does not radiate (you hear it only through the air it pushes back on)."),
            real(concat!("res", $k, ".radiation_slope"), "resonators", "exponent", 0.0, 4.0, 2.0,
                "Partial n radiates at radiation_per_s x ratio_n^slope (2: a small source's efficiency rising with frequency).",
                "Brightness of the resonator's own voice."),
            real(concat!("res", $k, ".touch"), "resonators", "0..1 of its extent", 0.01, 0.99, 0.3,
                "Where the air touches the resonator: partial n is weighed sin(n pi touch) (a pinned string's shape; a design hypothesis for the other kinds).",
                "At 0.5 the even partials are untouched; near an end everything is touched weakly and evenly."),
            real(concat!("res", $k, ".coupling_hz"), "resonators", "Hz", 0.0, 12000.0, 200.0,
                "Rate of the reciprocal exchange between the air volume's pressure and the resonator's touch coordinate (the direct coupling's rotation, 2 pi rate / fs per sample). 0 disconnects it exactly.",
                "Weak: a faint sympathetic halo; strong: the resonator loads the air, splits the air's modes near its partials, and takes her energy into itself."),
            int(concat!("res", $k, ".couple_tube"), "resonators", 0.0, 3.0, 0.0,
                "Which tube the touched volume is in: 0 the bore, 1-3 that side branch (the bore if the branch is off).",
                "A resonator on a side branch is excited only by what reaches the branch."),
            real(concat!("res", $k, ".couple_position"), "resonators", "0..1 along the tube", 0.0, 1.0, 0.5,
                "Where along that tube the touched volume is.",
                "The resonator feels the air's pressure pattern there: at a mode's node it is deaf to that mode."),
            real(concat!("res", $k, ".level"), "resonators", "x", 0.0, 100.0, 1.0,
                "Listener-side scale of the resonator's radiated power (converted to pascals as a spherical wave at its distance).",
                "How loud the resonator's own radiation is at the ear; no physical state."),
            real(concat!("res", $k, ".x_m"), "resonators", "m", -2.0, 2.0, 0.0,
                "Sideways position of the resonator relative to the mouth, for the listener.",
                "Pans and delays it against the mouth."),
        ]
    };
}

macro_rules! bump {
    ($k:literal) => {
        [
            real(concat!("bump", $k, ".position"), "bore", "0..1 along the bore", 0.0, 1.0, 0.5,
                "Centre of a Gaussian change of the bore's cross-section, 0 = entry wall, 1 = mouth.",
                "Where along the pipe a chamber or a constriction sits; which modes it reshapes depends on the pressure pattern there (a bump at a mode's pressure node does nothing to it)."),
            real(concat!("bump", $k, ".width"), "bore", "fraction of the bore", 0.01, 0.5, 0.08,
                "Standard deviation of that Gaussian (as a fraction of the bore's length).",
                "Narrow: a lip-like constriction or a small pocket, touching only a few modes; wide: a gentle swell of the whole pipe."),
            real(concat!("bump", $k, ".depth"), "bore", "ln(area ratio)", -6.0, 6.0, 0.0,
                "Peak of the Gaussian added to ln(area): depth -2.3 narrows the bore to a tenth, +2.3 opens it tenfold; 0 removes the bump exactly.",
                "Negative: a constriction (formants shift, and with air.on it becomes the jet: loss and hiss when she is loud); positive: a chamber (a Helmholtz-like dip and a lowered resonance)."),
        ]
    };
}

macro_rules! branch {
    ($k:literal, $def_on:expr, $def_pos:expr, $def_len:expr, $def_area:expr, $def_neck:expr) => {
        [
            switch(concat!("branch", $k, ".on"), "branches", $def_on,
                "Whether this side tube exists. Off, it is absent from the network exactly.",
                "A closed side tube puts antiresonances (notches) at its odd quarter-wave frequencies c/4L, 3c/4L, ...; an open one adds a second radiator and its own half-wave series."),
            real(concat!("branch", $k, ".position"), "branches", "0..1 along the bore", 0.0, 1.0, $def_pos,
                "Where on the bore the branch joins (its first volume joins that bore volume through its neck).",
                "The notch is deepest where the bore's mode has pressure; a branch near the mouth notches cleanly, one near the entry wall also detunes the low modes."),
            real(concat!("branch", $k, ".length_m"), "branches", "m", 0.01, 3.0, $def_len,
                "Length of the side tube.",
                "Closed: first notch at 343/(4 L) Hz (17 cm: 500 Hz; 5.7 cm: 1500 Hz); every odd multiple above it too."),
            real(concat!("branch", $k, ".area_cm2"), "branches", "cm2", 0.05, 2000.0, $def_area,
                "Cross-section of the side tube behind its neck.",
                "Wider: deeper, broader notches (more of the bore's flow goes in); a wide short branch with a narrow neck is a Helmholtz bottle: one deep low notch."),
            real(concat!("branch", $k, ".neck_cm2"), "branches", "cm2", 0.02, 2000.0, $def_neck,
                "Area of the branch's first volume, the neck through which it joins the bore.",
                "Narrow neck: the branch becomes a Helmholtz resonator (notch at c/(2 pi) sqrt(neck/(V l_neck))) and couples weakly above it; equal to the area: a plain stub."),
            switch(concat!("branch", $k, ".open"), "branches", 0.0,
                "Whether the far end radiates to the outside (1) or is a closed wall (0).",
                "Open: a second mouth (a nostril, a vent) heard at its own place by the listener, with half-wave resonances of the branch; closed: a pure notch filter made of air."),
            real(concat!("branch", $k, ".wall_loss_np_m"), "branches", "Np/m", 0.0, 60.0, 1.0,
                "Plane-wave attenuation inside the branch (frequency-flat): its lining.",
                "High: the branch absorbs instead of reflecting; the notch becomes a broad dip that eats energy (fur, cloth)."),
            int(concat!("branch", $k, ".sections"), "branches", 2.0, 32.0, 12.0,
                "Volumes the branch is cut into (the grid).",
                "More sections: less grid dispersion in the branch's upper modes; all tubes together must stay within 128 volumes."),
            real(concat!("branch", $k, ".vent_x_m"), "branches", "m", -1.0, 1.0, 0.0,
                "Sideways position of the branch's open end relative to the mouth (left negative), for the listener's geometry.",
                "Pans the branch's radiation and delays it against the mouth's; only matters when the branch is open."),
        ]
    };
}

/// The surface, in order.
pub static KNOBS: &[Knob] = &{
    const BORE: [Knob; 7] = [
        real("bore.length_m", "bore", "m", 0.05, 4.0, 0.175,
            "Length of the main tube from the entry wall (where her voice comes in) to the mouth (where it leaves).",
            "Sets the quarter-wave series c/4L, 3c/4L, ... that colours everything: 17.5 cm is a throat (490 Hz, 1470, ...); 1 m is a speaking tube; longer than about 1 m the 24 retained modes run out below 4 kHz and the body goes dark above them."),
        int("bore.sections", "bore", 12.0, 96.0, 40.0,
            "Volumes the bore is cut into (the grid the modes are computed on).",
            "Finer: less dispersion of the upper modes (the grid flattens them as omega = (2c/dx) sin(k dx/2)); coarser: faster moves. Sound changes little above 40."),
        real("bore.entry_cm2", "bore", "cm2", 0.1, 500.0, 8.0,
            "Cross-section at the entry wall.",
            "With exit_cm2 it makes the taper. A narrow entry raises the pressure her flow makes (louder, and more jet loss with air.on)."),
        real("bore.exit_cm2", "bore", "cm2", 0.1, 500.0, 8.0,
            "Cross-section at the mouth (before mouth.open).",
            "A wide mouth radiates well and damps the resonances (closer to transparent); a narrow one reflects, rings, and radiates less."),
        real("bore.flare", "bore", "exponent", 0.2, 5.0, 1.0,
            "Shape of the taper: ln(area) goes from entry to exit as s^flare along the bore (1 = exponential horn; <1 flares early, like a bell; >1 stays narrow then opens).",
            "A horn that opens early lowers the cutoff below which it does not radiate: brighter, more forward; a late flare keeps a long narrow throat (darker, more resonant)."),
        real("bore.wall_loss_np_m", "bore", "Np/m", 0.0, 60.0, 0.5,
            "Plane-wave attenuation of the bore's walls, frequency-flat (the runtime's law): hard pipe about 0.3-0.5, flesh about 0.5-2, felt or fur 5-30.",
            "Widens every resonance (bandwidth about 109 x alpha Hz for a uniform tube) and tilts nothing; the way to make a body more transparent or more muffled without moving its formants."),
        real("bore.end_correction", "bore", "radii", 0.0, 1.2, 0.61,
            "Unflanged end correction at the mouth as a multiple of its radius (0.61 unflanged, 0.85 flanged in a wall).",
            "Lengthens the bore acoustically (lower resonances) and sets the radiation conductance with it."),
    ];
    const MOUTH: [Knob; 1] = [real("mouth.open", "bore", "0..1", 0.02, 1.0, 1.0,
        "The Body's one live coordinate: the last volume's opening, 1 fully open, 0.02 nearly shut (a constriction pressed into the mouth). A live host may move it; a move is a receipted World move.",
        "Closing it darkens and mutes (less radiation), lowers the first resonance, and with air.on makes the mouth the jet (hiss)."),
    ];
    const DRIVE: [Knob; 4] = [
        switch("drive.integrate", "drive", 1.0,
            "A microphone hears pressure, and a mouth's far-field pressure is the time derivative of its volume velocity (a monopole: p ~ rho dU/dt / 4 pi r). The Body wants the flow, so the recording is integrated (a leaky integrator with corner drive.corner_hz) before it becomes U. Off: the sample itself is the flow (one extra +6 dB/octave through the Body).",
            "On: a flat Body passes her spectrum flat; off: everything tilts up 6 dB per octave."),
        real("drive.corner_hz", "drive", "Hz", 2.0, 200.0, 20.0,
            "Corner of the leaky integrator (below it the recording is passed as flow without integration, so DC and rumble do not accumulate).",
            "Lower: the lowest voice components keep their full flow; higher: thins the bass."),
        real("drive.position", "drive", "0..1 along the bore", 0.0, 1.0, 0.0,
            "Which volume of the bore her voice enters as a volume velocity (0 = entry wall, 1 = the mouth volume).",
            "At the wall every mode is driven (all-pole colour); nearer the mouth the pipe behind becomes a closed stub and its quarter-wave notches appear; a mode is not driven where its pressure pattern has a node."),
        real("drive.gain_m3s", "drive", "m3/s per full scale", 1.0e-6, 1.0e-2, 3.0e-4,
            "Calibration: the volume velocity one full-scale sample of her recording pushes into the drive volume (0.3 L/s is a loud vowel's peak flow at the lips).",
            "Level, and with air.on or flap.on the amount of nonlinearity: jet loss grows as U^3 and the flap's slam with pressure. Double it and the body is driven twice as hard, not just louder."),
    ];
    const AIR: [Knob; 6] = [
        switch("air.on", "air", 0.0,
            "The air beyond linear acoustics (POC-3): contact at the bore's narrowest column, the jet's separation loss there, a turbulent dipole downstream, DC-correct wall loss.",
            "Off: linear acoustics, bit for bit. On: louder passages lose energy at the constriction (compression) and gain hiss when the jet's Reynolds number passes the onset."),
        real("air.jet_loss", "air", "x law", 0.0, 4.0, 1.0,
            "Multiplier on the free-jet separation loss rho |u|^3 / (2 A^2) at the narrowest column (1 = the law).",
            "How hard a constriction eats loud flow: level-dependent damping, strongest where the area is smallest."),
        real("air.turbulence", "air", "efficiency at Mach 0.1", 0.0, 0.05, 0.004,
            "Share of the jet's loss that returns as a fluctuating dipole force three columns downstream, at Mach 0.1 (scaled as M^3, capped at 0.05).",
            "Fricative hiss that only exists when she pushes flow through a narrow place; its colour is the bore's response downstream of the jet."),
        real("air.reynolds_onset", "air", "Re", 100.0, 20_000.0, 3000.0,
            "Reynolds number of the jet at which the turbulent dipole starts (smoothstep to reynolds_full).",
            "Lower: hiss on quieter flow; higher: only on loud flow."),
        real("air.reynolds_full", "air", "Re", 200.0, 40_000.0, 6000.0,
            "Reynolds number at which the dipole is fully on (must exceed the onset).",
            "The width of the loudness band over which hiss fades in."),
        int("air.obstacle_offset", "air", 1.0, 8.0, 3.0,
            "How many columns downstream of the jet the dipole acts.",
            "Where the hiss is born along the pipe, hence which resonances colour it."),
    ];
    const FLAP: [Knob; 9] = [
        switch("flap.on", "flap", 0.0,
            "A flap valve in the entry wall: a mass on a spring with unilateral contact, opening a slit to the outside (the runtime's flow valve, with no supply pressure: it never blows, it only leaks and slams). Off: the slit is sealed (a 1 kg, 2 kHz plate at a 1e-15 m gap; its flow is exactly zero).",
            "On: positive pressure pushes it open and air leaks out through a Bernoulli slit (level-dependent loss); negative pressure sucks it shut and it hits the stop: a rattle, a buzz, on the loud half-cycles only."),
        real("flap.mass_g", "flap", "g", 0.01, 50.0, 0.5,
            "The flap's moving mass.",
            "Light: follows her pressure and rattles easily; heavy: barely moves (a tighter, duller buzz)."),
        real("flap.tuning_hz", "flap", "Hz", 20.0, 2000.0, 120.0,
            "The flap's spring, as its natural frequency.",
            "Where the rattle rings; near her pitch it locks to the voice, far above it clicks."),
        real("flap.damping", "flap", "ratio", 0.0, 2.0, 0.1,
            "The flap's damping ratio.",
            "Less: rings after each slam; more: a dead thud."),
        real("flap.gap_mm", "flap", "mm", 0.001, 5.0, 0.5,
            "Resting half-gap of the slit.",
            "Wider: more leak (loss) and a later slam; narrow: slams on small pressure (constant buzz)."),
        real("flap.length_mm", "flap", "mm", 1.0, 100.0, 14.0,
            "Length of the slit along the flap's edge.",
            "Leak area is 2 x length x gap: longer, leakier; and more force on the flap for the same pressure."),
        real("flap.thickness_mm", "flap", "mm", 0.5, 20.0, 3.0,
            "Thickness of the flap's edge (the slit's depth).",
            "Viscous resistance of the slit grows with it; and the pressure force on the flap."),
        real("flap.contact_stiffness", "flap", "x spring", 0.0, 10.0, 3.0,
            "Stiffness of the stop the flap hits, relative to its spring.",
            "Harder stop: sharper click in the rattle."),
        real("flap.contact_damping", "flap", "ratio", 0.0, 5.0, 1.0,
            "Damping while in contact.",
            "How much of each slam is lost as heat versus bounced."),
    ];
    const ROOM: [Knob; 5] = [
        real("room.port", "room", "weight", 0.0, 1.0, 0.0,
            "Weight of the mouth's port into the World's shared delayed field (eight delay courses, 7 to 90 ms, reciprocal: the room pushes back on the mouth). 0 disconnects the room exactly.",
            "A late diffuse halo behind the voice with real back-action (the mouth volume's pressure is loaded by the returning field). Not geometry: a modal room, the same one the guitar World uses."),
        real("room.coupling", "room", "0..1", 0.0, 1.0, 0.18,
            "Exchange rate between the port and the courses (the shared space's `coupling`).",
            "More: the room takes and returns more of the mouth's pressure per sample."),
        real("room.decay", "room", "0..1", 0.0, 1.0, 0.42,
            "The room's decay (0.08 + 18 x decay^2 seconds time constant).",
            "Short to cavernous."),
        real("room.radiation", "room", "0..1", 0.0, 1.0, 0.16,
            "Share of the room's field radiated to the two listening coordinates per sample.",
            "How much of the room you hear (and how fast it empties by radiating)."),
        real("room.level", "room", "x", 0.0, 20.0, 1.0,
            "Listener-side scale of the room's two emitted coordinates, mapped to pascals by the mouth's modal norm.",
            "Wet/dry of the room at the listener; changes no physical state."),
    ];
    const LISTENER: [Knob; 5] = [
        real("listener.distance_m", "listener", "m", 0.01, 20.0, 1.0,
            "Distance from the mouth to the midpoint between the listener's ears. Each radiating end is a monopole: p(r) = p_end A_end / (4 pi r l_end), delayed by r/c.",
            "Closer: louder and the vents' panning widens; farther: quieter, mono-ish."),
        real("listener.ear_spacing_m", "listener", "m", 0.0, 0.5, 0.17,
            "Distance between the two ears (0 = one ear, mono output).",
            "Stereo width of the mouth-versus-vent geometry."),
        real("listener.azimuth_deg", "listener", "degrees", -90.0, 90.0, 0.0,
            "Angle of the listener off the mouth's axis (positive: the listener stands to the mouth's right).",
            "Which vent is nearer which ear."),
        real("listener.pa_full_scale", "listener", "Pa", 0.01, 200.0, 1.0,
            "Pascals at the ear that map to full scale in the output file (1 Pa is 94 dB SPL).",
            "Output level only; no physical state."),
        real("listener.dry", "listener", "0..1", 0.0, 1.0, 0.0,
            "Her dry recording added at the listener, delayed by the mouth-to-ear path, full scale to full scale. An observer path: it alters no physical state.",
            "Blends her own air back in (the band above the Body's 24th mode, her sibilants) without touching the Body."),
    ];
    const POWERED: [Knob; 18] = [
        switch("powered.on", "powered", 0.0,
            "The canonical powered material (`shared-wave-material-v1`, the guitar World's body): 24 modes on a string-to-brass path with a compliant boundary, ordered internal transport, and a reservoir-paid active inclusion that adds work below a threshold and removes it above (a saturable limit cycle, never unbounded gain). It touches one air volume through a direct coupling, as a resonator does.",
            "Sustain, compression, bloom and sag that are paid from a finite supply: the body keeps ringing and thickens with level the way a powered instrument does. The RADIO's commander lives here."),
        real("powered.organization", "powered", "0..1", 0.0, 1.0, 0.06,
            "The material's configuration coordinate: 0 a six-course string-like metric, 1 a compliant brass-bore-like metric (the plug-in's `material_organization`).",
            "String-like: harmonic, bright partials; bore-like: fewer, rounder resonances."),
        real("powered.supply_w", "powered", "W", 0.0, 1.0, 0.004,
            "Refill of the material's reservoir (its finite stored work capacity is 2 mJ). The only source work besides her voice.",
            "How much sustaining the body can afford per second; 0 removes all active work exactly (it goes passive)."),
        real("powered.coupling", "powered", "0..1", 0.0, 1.0, 0.46,
            "Participation of the active inclusion (the plug-in's `active_coupling`); 0 turns it off exactly.",
            "Amount of powered behaviour."),
        real("powered.drive", "powered", "0..1.5", 0.0, 1.5, 0.42, "Active drive (threshold sensitivity).", "How easily the material lights up."),
        real("powered.headroom", "powered", "0.02..1", 0.02, 1.0, 0.72, "Saturation amplitude A_r of the active law.", "Lower: compresses sooner."),
        real("powered.sustain", "powered", "0..1", 0.0, 1.0, 0.24, "Sustain rate of the inclusion.", "Longer self-support of a held sound."),
        real("powered.sag", "powered", "0..1", 0.0, 1.0, 0.35, "Reservoir sag: loud events reduce later headroom.", "Soft-after-loud memory."),
        real("powered.bloom", "powered", "0..1", 0.0, 1.0, 0.36, "Slow hidden storage feeding the radiating modes late.", "Energy arriving late in the tail."),
        real("powered.feedback", "powered", "0..1", 0.0, 1.0, 0.04, "Returning shared-space activity's hold on the activation coordinate.", "With room.port on: feedback-like capture."),
        real("powered.interlace", "powered", "0..1", 0.0, 1.0, 0.42, "Ordered internal resonance transport between mode pairs.", "Intermodulation and chord-like spreading of her harmonics."),
        real("powered.memory", "powered", "0..1", 0.0, 1.0, 0.28, "Transport memory (path dependence).", "Hysteresis in how the spread settles."),
        real("powered.radiation", "powered", "fraction/sample", 0.0, 0.05, 0.008,
            "Fraction of the material's momentum along its radiation direction removed per sample (the plug-in's `body_radiation`).",
            "How much of it you hear directly, and how fast that drains it."),
        int("powered.couple_tube", "powered", 0.0, 3.0, 0.0, "Which air tube it touches (0 bore, 1-3 branch).", "As for a resonator."),
        real("powered.couple_position", "powered", "0..1 along the tube", 0.0, 1.0, 0.9, "Where along that tube.", "As for a resonator."),
        real("powered.coupling_hz", "powered", "Hz", 0.0, 12000.0, 300.0,
            "Rate of the reciprocal exchange between the air's pressure there and the material's source direction (where its pick or lips would act). 0 disconnects it exactly.",
            "How much of her voice the material takes, and how hard it pushes back."),
        real("powered.level", "powered", "x", 0.0, 100.0, 1.0, "Listener-side scale of its radiated power.", "Its loudness at the ear; no physical state."),
        real("powered.x_m", "powered", "m", -2.0, 2.0, 0.0, "Sideways position for the listener.", "Pan and delay against the mouth."),
    ];
    let mut knobs = [BORE[0]; 7 + 1 + 3 * 3 + 4 + 3 * 9 + 6 + 9 + 3 * 14 + 18 + 5 + 5];
    let mut i = 0;
    macro_rules! push {
        ($arr:expr) => {{
            let a = $arr;
            let mut j = 0;
            while j < a.len() {
                knobs[i] = a[j];
                i += 1;
                j += 1;
            }
        }};
    }
    push!(BORE);
    push!(MOUTH);
    push!(bump!("1"));
    push!(bump!("2"));
    push!(bump!("3"));
    push!(DRIVE);
    push!(branch!("1", 0.0, 0.3, 0.17, 2.0, 2.0));
    push!(branch!("2", 0.0, 0.6, 0.057, 2.0, 2.0));
    push!(branch!("3", 0.0, 0.8, 0.10, 2.0, 2.0));
    push!(AIR);
    push!(FLAP);
    push!(resonator!("1", 0.0, 110.0));
    push!(resonator!("2", 2.0, 150.0));
    push!(resonator!("3", 3.0, 300.0));
    push!(POWERED);
    push!(ROOM);
    push!(LISTENER);
    knobs
};

#[must_use]
pub fn knob(id: &str) -> Option<&'static Knob> {
    KNOBS.iter().find(|k| k.id == id)
}

/// A Body: a name and its knob values (missing knobs are defaults).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodySpec {
    pub name: String,
    #[serde(default)]
    pub about: String,
    #[serde(default)]
    pub knobs: BTreeMap<String, f64>,
}

impl BodySpec {
    #[must_use]
    pub fn new(name: &str, about: &str) -> Self {
        Self { name: name.into(), about: about.into(), knobs: BTreeMap::new() }
    }

    /// Set a knob; unknown ids are an error, out-of-range values are clamped
    /// (the clamp is reported).
    pub fn set(&mut self, id: &str, value: f64) -> Result<Option<String>, String> {
        let k = knob(id).ok_or_else(|| format!("unknown knob {id:?}"))?;
        if !value.is_finite() {
            return Err(format!("knob {id}: {value} is not finite"));
        }
        let v = match k.kind {
            KnobKind::Real => value,
            KnobKind::Integer => value.round(),
            KnobKind::Switch => if value >= 0.5 { 1.0 } else { 0.0 },
        };
        let clamped = v.clamp(k.min, k.max);
        self.knobs.insert(id.to_owned(), clamped);
        Ok((clamped != v).then(|| format!("knob {id}: {value} clamped to {clamped} (range {}..{})", k.min, k.max)))
    }

    /// Builder form of [`set`](Self::set) for presets (panics on an unknown id).
    #[must_use]
    pub fn with(mut self, id: &str, value: impl Into<f64>) -> Self {
        self.set(id, value.into()).unwrap_or_else(|e| panic!("{e}"));
        self
    }

    #[must_use]
    pub fn get(&self, id: &str) -> f64 {
        match self.knobs.get(id) {
            Some(v) => *v,
            None => knob(id).map_or(0.0, |k| k.default),
        }
    }

    #[must_use]
    pub fn get_usize(&self, id: &str) -> usize {
        self.get(id).round().max(0.0) as usize
    }

    #[must_use]
    pub fn on(&self, id: &str) -> bool {
        self.get(id) >= 0.5
    }

    /// Every knob with its value (defaults filled in).
    #[must_use]
    pub fn resolved(&self) -> BTreeMap<String, f64> {
        KNOBS.iter().map(|k| (k.id.to_owned(), self.get(k.id))).collect()
    }

    /// Check every stored id and range (a spec read from a file).
    pub fn validate(&self) -> Result<Vec<String>, String> {
        let mut notes = Vec::new();
        let ids: Vec<String> = self.knobs.keys().cloned().collect();
        let mut fixed = self.clone();
        for id in ids {
            let v = self.knobs[&id];
            if let Some(note) = fixed.set(&id, v)? {
                notes.push(note);
            }
        }
        Ok(notes)
    }
}
