//! Body-profile registry (PW10 §A).
//!
//! `pluck.rs` originally hard-coded one shared modal body (`BODY_MODE_SPECS`)
//! for every string voice: guitar, weave, whatever the patch dialed in — one
//! resonant body in different costumes. This module turns that single table
//! into a small registry of named profiles, selected by the `bodyp` param;
//! `make_body` (`pluck.rs`) picks a [`BodyProfile`] from here and still
//! scales it by `body_size` exactly as before.
//!
//! **These are design profiles, not measurements.** Nothing here is fit to
//! a captured admittance curve or impulse response of a real instrument.
//! Each table is a literature-informed sketch — plausible main-resonance
//! placement, plausible relative T60s and radiation balance for the
//! instrument's size class — built so the five voices are audibly and
//! structurally distinct inside this engine's shared body-coupling
//! machinery (norm-preserving Givens scattering against a bank of damped
//! second-order sections, `pluck.rs::BodyMode`). Round 3's passive-fit body
//! projection is the path to measurement-grounded tables; until then, take
//! the frequencies as "in the right neighborhood for the instrument family"
//! and nothing more precise.
//!
//! Table shape is unchanged from the original: `(frequency_hz, t60_s,
//! coupling_angle_at_body_1, radiation_weight)`, consumed by
//! `pluck.rs::BodyMode::new` exactly as `BODY_MODE_SPECS` always was.

/// One named body: a profile identifier and its modal table.
#[derive(Debug, Clone, Copy)]
pub struct BodyProfile {
    pub name: &'static str,
    /// `(frequency_hz, t60_s, coupling_angle_at_body_1, radiation_weight)`
    /// per mode — the same shape `BODY_MODE_SPECS` always used.
    pub modes: &'static [(f64, f64, f64, f64)],
}

/// Guitar: the original ten modes, **VERBATIM** — this is the compat-law
/// anchor (PW10 §A hard law). Do not edit these numbers; `bodyp` absent or
/// `"guitar"` must stay byte-identical to the pre-PW10 render, and a test
/// in `tests/pluck_invariants.rs` asserts this table against the historical
/// literal values as an independent check that nothing drifted here.
///
/// Steel-string flattop guitar, main air (Helmholtz) resonance near 100 Hz,
/// main top-plate mode a bit below 200 Hz, decreasingly-coupled and
/// decreasingly-radiating higher modes climbing toward 1.6 kHz — the
/// original design sketch, unexamined and unchanged by this round.
pub const GUITAR_MODES: [(f64, f64, f64, f64); 10] = [
    // frequency, T60, coupling angle at body=1, radiation weight
    (98.0, 1.25, 0.0065, 0.78),
    (186.0, 0.95, 0.0058, 0.72),
    (221.0, 0.82, 0.0053, 0.66),
    (286.0, 0.72, 0.0047, 0.58),
    (365.0, 0.58, 0.0042, 0.50),
    (455.0, 0.50, 0.0038, 0.43),
    (610.0, 0.40, 0.0032, 0.34),
    (835.0, 0.32, 0.0027, 0.27),
    (1175.0, 0.25, 0.0022, 0.20),
    (1580.0, 0.18, 0.0018, 0.14),
];

/// Violin: small, light, thin-plated body. Fletcher & Rossing-style modal
/// surveys place the main air resonance (A0, Helmholtz) around 270-280 Hz
/// and the main wood/top-plate resonance (T1) around 440-470 Hz, often with
/// a nearby second corpus mode around 540-560 Hz — hence the 275/460/550 Hz
/// trio below. Above that sits the "bridge hill" region (roughly 2-4 kHz)
/// that gives violins their carrying, present upper-mid brilliance; the
/// higher entries here climb toward it. Small body mass means short T60s
/// (guitar's longest mode rings 1.25 s; violin's longest here rings 0.55 s)
/// and, in contrast to the guitar table's steep radiation rolloff (0.78
/// down to 0.14, a ~5.6x drop), the violin's radiation weight stays much
/// flatter across the climb (0.55 down to 0.30, ~1.8x) — "brighter
/// radiation" per the PW10 brief: a violin projects across its whole
/// register, not mainly through the fundamental.
pub const VIOLIN_MODES: [(f64, f64, f64, f64); 9] = [
    (275.0, 0.55, 0.0060, 0.55), // A0, main air (Helmholtz)
    (460.0, 0.42, 0.0072, 0.72), // T1, main top-plate wood mode
    (550.0, 0.36, 0.0065, 0.68), // second corpus mode, close to T1
    (700.0, 0.30, 0.0055, 0.60),
    (950.0, 0.25, 0.0048, 0.55),
    (1300.0, 0.20, 0.0042, 0.50),
    (1900.0, 0.16, 0.0036, 0.46), // entering the bridge-hill region
    (2800.0, 0.12, 0.0030, 0.38), // bridge-hill core: strong projection
    (3600.0, 0.09, 0.0024, 0.30),
];

/// Cello: same family as the violin but scaled up roughly an octave lower
/// and with a heavier, larger plate. Cello main air resonance is commonly
/// cited close to 95-100 Hz (Fletcher & Rossing put a full-size cello's A0
/// near 100 Hz — coincidentally close to the guitar's, though the two
/// bodies respond very differently above that), with the main wood mode a
/// bit under an octave above, around 190-235 Hz. Larger, heavier plates
/// store more acoustic energy per cycle than the violin's, so T60s here sit
/// closer to (a little under) the guitar's than the violin's; coupling and
/// radiation weights fall off with frequency in the same qualitative shape
/// as the guitar table, just re-anchored to the cello's lower register.
pub const CELLO_MODES: [(f64, f64, f64, f64); 10] = [
    (100.0, 1.40, 0.0068, 0.80), // A0, main air
    (190.0, 1.05, 0.0060, 0.74), // T1, main wood mode
    (235.0, 0.90, 0.0055, 0.68), // second corpus mode
    (300.0, 0.78, 0.0050, 0.60),
    (380.0, 0.65, 0.0044, 0.52),
    (480.0, 0.55, 0.0038, 0.45),
    (620.0, 0.45, 0.0033, 0.37),
    (800.0, 0.36, 0.0027, 0.30),
    (1050.0, 0.28, 0.0022, 0.23),
    (1400.0, 0.21, 0.0018, 0.17),
];

/// Double bass: the family's largest, heaviest body. Reported main air
/// resonances for full-size double basses cluster in the 55-70 Hz range;
/// 60 Hz is used here per the PW10 brief. The main wood/top-plate mode sits
/// roughly an octave to a fifth above, and 100 Hz is used per the brief's
/// explicit hint. A body this large and massive stores the most acoustic
/// energy of the family, so its low modes get the longest T60s in the
/// whole registry (longer even than the guitar's) — "long low T60s" per
/// the brief — decaying toward guitar-scale durations only well up into
/// the higher modes. Radiation weight starts highest of any profile here
/// (deep, room-filling low end) and falls the most steeply, mirroring how
/// little of a bass's character lives in its upper partials.
pub const BASS_MODES: [(f64, f64, f64, f64); 10] = [
    (60.0, 1.80, 0.0070, 0.85),  // main air resonance
    (100.0, 1.50, 0.0064, 0.78), // main wood/top-plate mode
    (150.0, 1.20, 0.0057, 0.68),
    (210.0, 1.00, 0.0050, 0.58),
    (290.0, 0.82, 0.0043, 0.48),
    (390.0, 0.65, 0.0037, 0.40),
    (520.0, 0.50, 0.0031, 0.32),
    (700.0, 0.38, 0.0025, 0.25),
    (950.0, 0.28, 0.0019, 0.18),
    (1300.0, 0.20, 0.0014, 0.12),
];

/// Mandolin: the family's smallest, lightest, most tautly built body (flat-
/// or carved-top, small air volume). Surveyed main air resonances for
/// mandolins tend to sit noticeably higher than the guitar's — around
/// 480-500 Hz — reflecting the small enclosed air volume; 490 Hz is used
/// here per the PW10 brief. Small, stiff, lightly damped top plates ring
/// short and bright: every T60 in this table is the shortest of the five
/// profiles (topping out at 0.28 s, versus the guitar's 1.25 s), while
/// coupling and radiation stay comparatively strong into the upper modes —
/// a small bright body that projects its whole (compressed, upward-shifted)
/// register rather than favoring the fundamental the way the guitar table
/// does.
pub const MANDOLIN_MODES: [(f64, f64, f64, f64); 8] = [
    (490.0, 0.28, 0.0062, 0.55), // main air resonance
    (620.0, 0.22, 0.0070, 0.62), // main top-plate wood mode
    (780.0, 0.18, 0.0062, 0.58),
    (980.0, 0.15, 0.0053, 0.52),
    (1250.0, 0.12, 0.0045, 0.46),
    (1600.0, 0.095, 0.0038, 0.40),
    (2050.0, 0.075, 0.0031, 0.34),
    (2650.0, 0.060, 0.0025, 0.28),
];

/// The registry `bodyp` selects over, in declaration order (guitar first —
/// the default and the fallback target for an unrecognised name).
pub const PROFILES: &[BodyProfile] = &[
    BodyProfile {
        name: "guitar",
        modes: &GUITAR_MODES,
    },
    BodyProfile {
        name: "violin",
        modes: &VIOLIN_MODES,
    },
    BodyProfile {
        name: "cello",
        modes: &CELLO_MODES,
    },
    BodyProfile {
        name: "bass",
        modes: &BASS_MODES,
    },
    BodyProfile {
        name: "mandolin",
        modes: &MANDOLIN_MODES,
    },
];

/// Look up a profile by name, falling back to guitar (`PROFILES[0]`) for
/// anything unrecognised. This is the engine's own clamp for an enum-typed
/// param: exactly the "invalid input degrades to a safe default rather
/// than erroring" discipline `PluckPatch::from`'s numeric `.clamp()` calls
/// apply to out-of-range numbers, applied here to an out-of-domain string.
/// `bodyp` is new engine-only surface (there is no `mus_audio.py` oracle
/// entry to match), so this fallback *is* the domain law, not a port of
/// one.
pub fn profile_for(name: &str) -> &'static BodyProfile {
    PROFILES
        .iter()
        .find(|profile| profile.name == name)
        .unwrap_or(&PROFILES[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrecognised_name_falls_back_to_guitar() {
        assert_eq!(profile_for("banjo").name, "guitar");
        assert_eq!(profile_for("").name, "guitar");
        assert_eq!(
            profile_for("Guitar").name,
            "guitar",
            "case-sensitive, falls back rather than panicking"
        );
    }

    #[test]
    fn every_registered_name_resolves_to_itself() {
        for profile in PROFILES {
            assert_eq!(profile_for(profile.name).name, profile.name);
            assert!(!profile.modes.is_empty());
        }
    }
}
