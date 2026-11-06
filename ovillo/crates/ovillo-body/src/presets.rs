//! The built-in bodies. Every number is a design choice made on this surface
//! and listened to on synthetic voices (`say -v Samantha|Daniel|Ralph`); none
//! is a measurement of a person. `SURFACE.md` says what each one is for.

use crate::surface::BodySpec;

pub fn names() -> Vec<&'static str> {
    vec!["neutral", "v", "gatekeeper", "radio", "guard_box", "coke", "bottle", "beehive", "cathedral", "sitar"]
}

pub fn get(name: &str) -> Option<BodySpec> {
    Some(match name {
        "neutral" => neutral(),
        "v" => v(),
        "gatekeeper" => gatekeeper(),
        "radio" => radio(),
        "guard_box" => guard_box(),
        "coke" => coke(),
        "bottle" => bottle(),
        "beehive" => beehive(),
        "cathedral" => cathedral(),
        "sitar" => sitar(),
        _ => return None,
    })
}

/// As transparent as the physics allows, measured on white noise: her voice
/// at the closed wall of a 19 cm pipe (the shortest whose 24 modes fit the
/// band) tapering 4 to 8 cm2, flesh-lined (3 Np/m), with the drive's
/// integrator corner at 120 Hz where the flat loss law turns the pipe
/// resistive. Transfer within +/-6.3 dB over 150-8000 Hz: +4 dB at 100 Hz,
/// flat 300 Hz-3 kHz within 3 dB, -9 dB at 8 kHz, dark above the 24th mode
/// (20 kHz). Everything else off.
pub fn neutral() -> BodySpec {
    BodySpec::new("neutral", "Her voice at the wall of a 19 cm flesh-lined pipe tapering 4 to 8 cm2: the flattest body this World can be (+/-6 dB over 150-8000 Hz, -9 dB at 8 kHz).")
        .with("bore.length_m", 0.19)
        .with("bore.entry_cm2", 4.0)
        .with("bore.exit_cm2", 8.0)
        .with("bore.flare", 1.0)
        .with("bore.wall_loss_np_m", 3.0)
        .with("bore.end_correction", 0.61)
        .with("drive.position", 0.0)
        .with("drive.corner_hz", 120.0)
        .with("listener.distance_m", 1.0)
        .with("listener.pa_full_scale", 0.08)
}

/// A rough feminization, built on Vera's first mic check (median F0 111 Hz;
/// running-speech formant medians 330/1343/2466/3408 Hz; apparent tract
/// about 18 cm): the neutral pipe shortened to 15 cm, three closed side
/// tubes whose antiresonances sit on her F1/F2/F3 medians (a bottle at about
/// 340 Hz, quarter-wave stubs of 6.4 and 3.6 cm at 1356 and 2416 Hz,
/// measured), each with its pole just above the notch (measured peaks 760,
/// 1610, 3040 Hz) where a 15 cm tract's formants would sit. Her pitch is
/// untouched. See SURFACE.md for what this can and cannot do.
pub fn v() -> BodySpec {
    BodySpec::new("v", "The neutral pipe at 15 cm with three closed side tubes notching her F1/F2/F3 medians (340/1356/2416 Hz, measured) and reinforcing just above them (760/1610/3040 Hz); her pitch stays hers.")
        .with("bore.length_m", 0.15)
        .with("bore.entry_cm2", 4.0)
        .with("bore.exit_cm2", 8.0)
        .with("bore.flare", 1.0)
        .with("bore.wall_loss_np_m", 3.0)
        .with("drive.position", 0.0)
        .with("drive.corner_hz", 120.0)
        // F2: a 6.4 cm closed stub (c/4L = 1340 Hz)
        .with("branch1.on", 1.0)
        .with("branch1.position", 0.6)
        .with("branch1.length_m", 0.064)
        .with("branch1.area_cm2", 4.0)
        .with("branch1.neck_cm2", 4.0)
        .with("branch1.sections", 8)
        // F3: a 3.6 cm closed stub (2380 Hz)
        .with("branch2.on", 1.0)
        .with("branch2.position", 0.85)
        .with("branch2.length_m", 0.036)
        .with("branch2.area_cm2", 3.0)
        .with("branch2.neck_cm2", 3.0)
        .with("branch2.sections", 6)
        // F1: a bottle (60 cm2 x 4 cm behind a 0.4 cm2 neck) near 340 Hz
        .with("branch3.on", 1.0)
        .with("branch3.position", 0.5)
        .with("branch3.length_m", 0.04)
        .with("branch3.area_cm2", 60.0)
        .with("branch3.neck_cm2", 0.4)
        .with("branch3.sections", 6)
        .with("listener.pa_full_scale", 0.074)
}

/// Fur coat, huge pointed nose, long thin beard, fleas in the collar.
pub fn gatekeeper() -> BodySpec {
    BodySpec::new("gatekeeper", "A long throat into a huge open nose (a second radiator, left of the mouth), a fur-lined coat cavity that eats the lows, a beard-damped mouth, a loose flap in the wall that rattles on loud syllables, and air that hisses through the throat.")
        .with("bore.length_m", 0.26)
        .with("bore.entry_cm2", 4.0)
        .with("bore.exit_cm2", 6.0)
        .with("bore.flare", 2.0)
        .with("bore.wall_loss_np_m", 4.0)
        .with("bore.end_correction", 0.61)
        .with("mouth.open", 0.5)
        // the nose: a long narrow open branch high on the bore, vented left
        .with("branch1.on", 1.0)
        .with("branch1.position", 0.6)
        .with("branch1.length_m", 0.22)
        .with("branch1.area_cm2", 2.5)
        .with("branch1.neck_cm2", 1.2)
        .with("branch1.open", 1.0)
        .with("branch1.wall_loss_np_m", 2.0)
        .with("branch1.sections", 18)
        .with("branch1.vent_x_m", -0.04)
        // the fur coat: a wide lossy cavity near the throat
        .with("branch2.on", 1.0)
        .with("branch2.position", 0.15)
        .with("branch2.length_m", 0.12)
        .with("branch2.area_cm2", 120.0)
        .with("branch2.neck_cm2", 2.0)
        .with("branch2.wall_loss_np_m", 30.0)
        .with("branch2.sections", 8)
        // the fleas: a light flap in the wall
        .with("flap.on", 1.0)
        .with("flap.mass_g", 0.15)
        .with("flap.tuning_hz", 90.0)
        .with("flap.gap_mm", 0.25)
        .with("flap.length_mm", 20.0)
        .with("flap.damping", 0.2)
        // a narrow throat for the air to hiss through, before the coat cavity
        // (behind it the coat's 1.4 litres would swallow the low-frequency flow)
        .with("bump1.position", 0.08)
        .with("bump1.width", 0.04)
        .with("bump1.depth", -3.4)
        .with("air.on", 1.0)
        .with("air.reynolds_onset", 400.0)
        .with("air.reynolds_full", 1200.0)
        .with("air.turbulence", 0.02)
        .with("drive.gain_m3s", 6.0e-4)
        .with("listener.pa_full_scale", 0.05)
}

/// The commander, as physics: a long narrow speaking tube (its 24 modes stop
/// near 4 kHz, so the band ends there), a loose flap that crackles when she is
/// loud, and the powered material behind it that sustains and compresses.
pub fn radio() -> BodySpec {
    BodySpec::new("radio", "A 1 m narrow speaking tube (dark above its 24th mode, near 4 kHz) with a hard narrow mouth, a crackling flap, and the powered string-brass material coupled at the far end for sustain and compression.")
        .with("bore.length_m", 1.0)
        .with("bore.sections", 72)
        .with("bore.entry_cm2", 3.0)
        .with("bore.exit_cm2", 1.2)
        .with("bore.flare", 1.0)
        .with("bore.wall_loss_np_m", 1.5)
        .with("bore.end_correction", 0.61)
        .with("flap.on", 1.0)
        .with("flap.mass_g", 0.05)
        .with("flap.tuning_hz", 400.0)
        .with("flap.gap_mm", 0.12)
        .with("flap.length_mm", 8.0)
        .with("flap.damping", 0.05)
        .with("flap.contact_stiffness", 8.0)
        .with("powered.on", 1.0)
        .with("powered.organization", 0.85)
        .with("powered.supply_w", 0.01)
        .with("powered.coupling", 0.7)
        .with("powered.headroom", 0.4)
        .with("powered.sustain", 0.5)
        .with("powered.sag", 0.5)
        .with("powered.couple_position", 0.95)
        .with("powered.coupling_hz", 300.0)
        .with("powered.level", 0.3)
        .with("drive.gain_m3s", 5.0e-4)
        .with("listener.distance_m", 0.3)
        .with("listener.ear_spacing_m", 0.0)
        .with("listener.pa_full_scale", 0.68)
}

/// A guard inside a cardboard box: a short mask into a big cardboard cavity
/// whose walls are a plate that flexes and radiates, with a slit to the outside.
pub fn guard_box() -> BodySpec {
    BodySpec::new("guard_box", "Her mouth into a 30 cm cardboard box (a wide chamber with lossy walls) that leaks through a narrow slit; the box's wall is a plate resonator that the trapped air drives and that radiates on its own.")
        .with("bore.length_m", 0.42)
        .with("bore.sections", 48)
        .with("bore.entry_cm2", 6.0)
        .with("bore.exit_cm2", 1.5)
        .with("bore.flare", 1.0)
        .with("bore.wall_loss_np_m", 6.0)
        .with("bore.end_correction", 0.85)
        // the box: a big chamber over the middle of the bore
        .with("bump1.position", 0.55)
        .with("bump1.width", 0.22)
        .with("bump1.depth", 3.5)
        // the box's wall: a plate the air drives, heard beside the slit
        .with("res1.on", 1.0)
        .with("res1.kind", 3)
        .with("res1.f1_hz", 140.0)
        .with("res1.t60_s", 0.25)
        .with("res1.t60_slope", 0.7)
        .with("res1.radiation_per_s", 60.0)
        .with("res1.radiation_slope", 1.5)
        .with("res1.touch", 0.37)
        .with("res1.coupling_hz", 900.0)
        .with("res1.couple_position", 0.55)
        .with("res1.level", 4.0)
        .with("res1.x_m", 0.15)
        .with("air.on", 1.0)
        .with("listener.distance_m", 1.5)
        .with("listener.pa_full_scale", 0.78)
}

/// Sir Edward Coke, aged 82, in 1634: a starched ruff (a lossy cavity around
/// the mouth), the listener at the narrow end of an ear trumpet she speaks
/// into, and a slack flap in the wall that leaks and slaps.
pub fn coke() -> BodySpec {
    BodySpec::new("coke", "She speaks into the bell of an ear trumpet (a 40 cm cone narrowing to the listener's ear 2 cm away), through a cloth ruff cavity that absorbs the mids, with a slack leaky flap in the wall.")
        .with("bore.length_m", 0.40)
        .with("bore.sections", 48)
        .with("bore.entry_cm2", 80.0)
        .with("bore.exit_cm2", 0.5)
        .with("bore.flare", 0.7)
        .with("bore.wall_loss_np_m", 1.0)
        .with("bore.end_correction", 0.61)
        .with("drive.position", 0.05)
        // the ruff: a wide, cloth-lined cavity around the mouth
        .with("branch1.on", 1.0)
        .with("branch1.position", 0.1)
        .with("branch1.length_m", 0.08)
        .with("branch1.area_cm2", 150.0)
        .with("branch1.neck_cm2", 40.0)
        .with("branch1.wall_loss_np_m", 40.0)
        .with("branch1.sections", 6)
        .with("flap.on", 1.0)
        .with("flap.mass_g", 1.5)
        .with("flap.tuning_hz", 45.0)
        .with("flap.gap_mm", 0.6)
        .with("flap.length_mm", 30.0)
        .with("flap.damping", 0.4)
        .with("listener.distance_m", 0.02)
        .with("listener.ear_spacing_m", 0.0)
        .with("listener.pa_full_scale", 0.071)
}

/// Wild: a voice spoken into a bottle.
pub fn bottle() -> BodySpec {
    BodySpec::new("bottle", "A wide-bellied bottle: her voice at the bottom of a 25 cm belly that narrows through a neck to a small mouth; one deep Helmholtz hoot and the belly's modes.")
        .with("bore.length_m", 0.30)
        .with("bore.sections", 48)
        .with("bore.entry_cm2", 40.0)
        .with("bore.exit_cm2", 2.0)
        .with("bore.flare", 4.0)
        .with("bore.wall_loss_np_m", 0.4)
        .with("bump1.position", 0.35)
        .with("bump1.width", 0.18)
        .with("bump1.depth", 0.8)
        .with("air.on", 1.0)
        .with("listener.pa_full_scale", 0.03)
}

/// Wild: a buzzing membrane and a hissing throat.
pub fn beehive() -> BodySpec {
    BodySpec::new("beehive", "A short pipe with a drumhead across its middle (a membrane resonator the air drives hard) and a light flap in the wall that rattles on every loud syllable; the narrow mouth hisses.")
        .with("bore.length_m", 0.20)
        .with("bore.entry_cm2", 6.0)
        .with("bore.exit_cm2", 1.0)
        .with("bore.wall_loss_np_m", 1.0)
        .with("res1.on", 1.0)
        .with("res1.kind", 2)
        .with("res1.f1_hz", 180.0)
        .with("res1.t60_s", 0.6)
        .with("res1.t60_slope", 0.5)
        .with("res1.radiation_per_s", 40.0)
        .with("res1.touch", 0.21)
        .with("res1.coupling_hz", 1200.0)
        .with("res1.couple_position", 0.5)
        .with("res1.level", 0.3)
        .with("flap.on", 1.0)
        .with("flap.mass_g", 0.05)
        .with("flap.tuning_hz", 220.0)
        .with("flap.gap_mm", 0.1)
        .with("flap.length_mm", 12.0)
        .with("flap.damping", 0.05)
        .with("air.on", 1.0)
        .with("air.reynolds_onset", 800.0)
        .with("air.reynolds_full", 2000.0)
        .with("air.turbulence", 0.01)
        .with("drive.gain_m3s", 5.0e-4)
        .with("listener.pa_full_scale", 0.51)
}

/// Wild: the room, and a bell on the wall.
pub fn cathedral() -> BodySpec {
    BodySpec::new("cathedral", "The neutral pipe opening into the World's shared delayed field with a long decay, and a bell (prime 220 Hz) hanging at the mouth that the field and her voice set ringing.")
        .with("bore.length_m", 0.19)
        .with("bore.entry_cm2", 4.0)
        .with("bore.exit_cm2", 8.0)
        .with("bore.flare", 1.0)
        .with("bore.wall_loss_np_m", 3.0)
        .with("drive.position", 0.0)
        .with("drive.corner_hz", 120.0)
        .with("room.port", 0.8)
        .with("room.coupling", 0.3)
        .with("room.decay", 0.7)
        .with("room.radiation", 0.2)
        .with("room.level", 2.0)
        .with("res1.on", 1.0)
        .with("res1.kind", 4)
        .with("res1.f1_hz", 220.0)
        .with("res1.t60_s", 6.0)
        .with("res1.t60_slope", 0.8)
        .with("res1.radiation_per_s", 8.0)
        .with("res1.touch", 0.29)
        .with("res1.coupling_hz", 300.0)
        .with("res1.couple_position", 0.95)
        .with("res1.level", 2.0)
        .with("res1.x_m", 0.3)
        .with("listener.distance_m", 2.0)
        .with("listener.pa_full_scale", 0.056)
}

/// Wild: sympathetic strings behind the voice.
pub fn sitar() -> BodySpec {
    BodySpec::new("sitar", "The neutral pipe with three sympathetic strings (A2, E3, A3; slightly inharmonic) touching it at the mouth, ringing where her harmonics meet theirs and pushing back on the air.")
        .with("bore.length_m", 0.19)
        .with("bore.entry_cm2", 4.0)
        .with("bore.exit_cm2", 8.0)
        .with("bore.flare", 1.0)
        .with("bore.wall_loss_np_m", 3.0)
        .with("drive.position", 0.0)
        .with("drive.corner_hz", 120.0)
        .with("res1.on", 1.0).with("res1.kind", 0).with("res1.f1_hz", 110.0).with("res1.inharmonicity", 0.0004)
        .with("res1.t60_s", 4.0).with("res1.radiation_per_s", 15.0).with("res1.touch", 0.13).with("res1.coupling_hz", 600.0)
        .with("res1.couple_position", 0.9).with("res1.level", 3.0).with("res1.x_m", -0.1)
        .with("res2.on", 1.0).with("res2.kind", 0).with("res2.f1_hz", 164.8).with("res2.inharmonicity", 0.0004)
        .with("res2.t60_s", 4.0).with("res2.radiation_per_s", 15.0).with("res2.touch", 0.13).with("res2.coupling_hz", 600.0)
        .with("res2.couple_position", 0.9).with("res2.level", 3.0).with("res2.x_m", 0.0)
        .with("res3.on", 1.0).with("res3.kind", 0).with("res3.f1_hz", 220.0).with("res3.inharmonicity", 0.0004)
        .with("res3.t60_s", 4.0).with("res3.radiation_per_s", 15.0).with("res3.touch", 0.13).with("res3.coupling_hz", 600.0)
        .with("res3.couple_position", 0.9).with("res3.level", 3.0).with("res3.x_m", 0.1)
        .with("listener.pa_full_scale", 0.63)
}
