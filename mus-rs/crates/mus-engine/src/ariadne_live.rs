//! Live Ariadne Worlds: the tab instruments (`ariadne_tab`) and the piano
//! (`ariadne_piano`) built from the same anatomies, laws and constants that
//! `mus render` plays, with their touches arriving while the World runs (from
//! a game, a page, an offline render) instead of compiled from a score ahead
//! of time. `ovillo-portable` drives these.
//!
//! A score can still be played: [`LiveTab::from_score`] and
//! [`LivePiano::from_score`] build the World the score names and schedule the
//! score's own touches, computed by the same code as `mus render`. Played that
//! way, the World's observation is `mus render`'s World observation sample for
//! sample, before the mix bus (the tests below). Nothing here synthesizes a
//! note: every sample is a World's answer to a touch.
//!
//! What a live World adds is only *when*: a touch is scheduled at a World
//! sample, and the World's clock is the number of samples it has rendered.
//! A touch scheduled in the past happens now, as early as its mechanics
//! allow (a tab pluck that needs a new fret lands the fret and plucks on the
//! same sample; a piano key pressed now lifts its damper now and its hammer
//! reaches the strings `DAMPER_LEAD_SECONDS` later, as the action does).

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;
use std::path::Path;

use mus_ariadne::{
    BodyControls, BowControls, BowSpec, BowTotals, ConstitutiveBodyControls, SpaceBodyControls,
    StereoListener, StereoListenerSpec, World, WorldControls, WorldPlan,
};
use serde::Serialize;
use serde_json::{json, Value};

use crate::ariadne_piano::{
    self, Anatomy, KeyBodies, KeyPress, DAMPER_FALL_SECONDS, DAMPER_LAND_SECONDS,
    DAMPER_LEAD_SECONDS, DAMPER_LIFT_SECONDS, PIANO_V1, VELOCITY,
};
use crate::ariadne_tab::{
    self, apply_hands, box_radiation, world_plan_band, BowSpan, Hand, StringHandReceipt,
    TabInstrument, Touch, TouchKind, BOW_FORCE_UNIT, BOW_LANDING_SECONDS, BOW_SLIP_VELOCITY,
    BOW_VELOCITY_UNIT, FRET_LEAD_SECONDS, PLUCK_SCALE, PULSE_FLESH_SECONDS, PULSE_PICK_SECONDS,
    SLIDE_SECONDS, STANDARD_TUNING, STEEL_STRING_V0, STRING_RADIATION,
};
use crate::layout::BarMap;
use crate::mixbus::db_to_gain;
use crate::util::DYN_DB;
use crate::RenderStats;

/// Every Ariadne anatomy here is built at this rate (the plans' own).
pub const SAMPLE_RATE: f64 = mus_dsp::SR_F64;

/// The written dynamics and their dB (`mus_audio.DYN_DB`): what a tab touch's
/// `db` means.
pub fn dynamics_db() -> &'static [(&'static str, f64)] {
    DYN_DB
}

/// The written dynamics and the piano hammer velocity each throws (m/s).
pub fn piano_velocities() -> &'static [(&'static str, f64)] {
    VELOCITY
}

/// A score's timeline, as `mus render` lays it out.
struct Layout {
    proposal: mus_text::Proposal,
    bar_map: BarMap,
    bar_content: BTreeMap<u32, BTreeMap<String, String>>,
    n_bars: u32,
    swing_unit_ql: f64,
    swing_r: f64,
    n_total: usize,
}

fn layout(source: &str) -> Result<Layout, String> {
    let (proposal, _diags) = mus_text::parse_score_lossy(source);
    let headers = &proposal.headers;
    let (swing_unit_ql, swing_r) = crate::layout::parse_swing(headers.get("swing"));
    let abbrevs: Vec<String> = proposal
        .instruments
        .iter()
        .map(|i| i.abbrev.clone())
        .collect();
    let raw = crate::rawbars::scan(source, &abbrevs);
    let n_bars = headers
        .get("bars")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|&n| n != 0)
        .unwrap_or(raw.max_bar);
    let bar_map = crate::layout::build_bar_map(
        headers.get("tempo"),
        headers.get("time"),
        &raw.inline_tempo,
        &raw.inline_time,
        n_bars,
    )
    .map_err(|e| e.to_string())?;
    let n_total = (bar_map.total_s * SAMPLE_RATE) as usize;
    Ok(Layout {
        proposal,
        bar_map,
        bar_content: raw.bar_content,
        n_bars,
        swing_unit_ql,
        swing_r,
        n_total,
    })
}

/// What reading a score found, besides the World it built.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreRead {
    /// The instrument (abbrev) the World plays.
    pub instrument: String,
    /// Samples the score lasts (`mus render`'s length).
    pub samples: usize,
    pub notes: u32,
    /// Tokens the score walk did not understand (`mus render`'s `badTokens`).
    pub bad_tokens: Vec<String>,
}

// ---------------------------------------------------------------- tab instruments

/// How a hand touches a string, folded into that string's body when the
/// World is built (`SPEC-ARIADNE-STEEL-STRING.md`, Layer 2): `pos` (pluck
/// point, 0.02..0.5 of the length), `pick` (0 flesh .. 1 plectrum), `pm`
/// (palm loss, 0..1), `gain_db` (added to every touch's level).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabHand {
    pub pos: f64,
    pub pick: f64,
    pub pm: f64,
    pub gain_db: f64,
}

impl Default for TabHand {
    /// A hand that names none of its parameters (`ariadne_tab::collect`).
    fn default() -> Self {
        Self {
            pos: 0.18,
            pick: 0.4,
            pm: 0.0,
            gain_db: 0.0,
        }
    }
}

/// A tab instrument to build live.
#[derive(Debug, Clone, Default)]
pub struct TabSpec {
    /// The anatomy's name (`steel-string-v0`, or the name a `plan_json` goes by).
    pub anatomy: String,
    /// A tab anatomy's plan (`ariadne-world-plan-v2` JSON); `None` is the
    /// built-in `steel-string-v0`.
    pub plan_json: Option<String>,
    /// MIDI pitch of each course, lowest course first (as `tuning=` writes
    /// it); `None` is standard tuning (six courses only).
    pub tuning: Option<Vec<f64>>,
    pub capo: f64,
    /// One hand per string, string 1 (the highest course) first; a shorter
    /// list repeats its last hand; empty is the default hand on every string.
    pub hands: Vec<TabHand>,
}

/// How a touch meets a string (`SPEC-TAB.md`; `SPEC-ARIADNE-STEEL-STRING.md`, Layer 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TabTouch {
    Pluck,
    HammerOn,
    PullOff,
    Slide,
}

impl TabTouch {
    fn of(kind: TouchKind) -> Option<Self> {
        match kind {
            TouchKind::Pluck => Some(Self::Pluck),
            TouchKind::HammerOn => Some(Self::HammerOn),
            TouchKind::PullOff => Some(Self::PullOff),
            TouchKind::Slide => Some(Self::Slide),
            TouchKind::Bow => None,
        }
    }
}

/// A live player's bow (`SPEC-ARIADNE-STEEL-STRING.md`, "The bow"): which
/// string the hair rests on (1-based; 0 lifts it), the bow point (fraction of
/// the speaking length from the bridge, 0.02..0.49), speed (-1..1: its sign is
/// the direction, + a down-bow; `speed = 1` is the score's `speed=1`) and
/// force (0..1; 0 lifts). The hair lands and lifts over
/// `BOW_LANDING_SECONDS`; moving it to another string or point lifts it there
/// first and lands it again. One bow per string, at one point (the spec's limit).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveBow {
    pub string: usize,
    pub position: f64,
    pub speed: f64,
    pub force: f64,
}

impl Default for LiveBow {
    fn default() -> Self {
        Self {
            string: 0,
            position: 0.12,
            speed: 0.5,
            force: 0.0,
        }
    }
}

/// One tab instrument's World, touched live.
pub struct LiveTab {
    anatomy: String,
    plan: WorldPlan,
    capo: f64,
    open: Vec<f64>,
    hands: Vec<TabHand>,
    world: World,
    listener: StereoListener,
    bodies: Vec<String>,
    string_body: BTreeMap<usize, usize>,
    controls: Vec<BodyControls>,
    strings: Vec<StringHandReceipt>,
    /// (sample, string, target stop in frets, ramp samples), in time order.
    stops: Vec<(usize, usize, f64, usize)>,
    /// (sample, string, amplitude, width samples), in time order.
    pulses: Vec<(usize, usize, f64, usize)>,
    next_stop: usize,
    next_pulse: usize,
    current: BTreeMap<usize, f64>,
    ramps: BTreeMap<usize, (f64, f64, usize, usize)>,
    active: Vec<(usize, usize, f64, usize)>,
    /// The last stop scheduled on each string (what a new touch moves from).
    scheduled: BTreeMap<usize, f64>,
    /// [plucks, hammer-ons, pull-offs, slides, bowed notes] (the live bow's landings count as bowed notes).
    counts: [usize; 5],
    intonation_touches: usize,
    k: usize,
    // ---- the bow: the score's (`ariadne_tab::prepare`'s) and the live player's
    /// (body, bow point) of every bow attached, by bow index.
    bows: Vec<(usize, f64)>,
    score_spans: Vec<BowSpan>,
    next_span: usize,
    active_spans: Vec<BowSpan>,
    /// Per score bow: whether its hair was down last sample.
    score_bowing: Vec<bool>,
    score_bow_count: usize,
    bow_target: LiveBow,
    /// The live bow now: its binding (bow index) and the force on it (0..1).
    bow_binding: Option<usize>,
    bow_applied: f64,
}

fn validate_strings(plan: &WorldPlan) -> Result<usize, String> {
    let ids: Vec<&str> = plan
        .bodies
        .iter()
        .filter(|b| b.id.starts_with("string/"))
        .map(|b| b.id.as_str())
        .collect();
    let n = ids.len();
    let mut seen = vec![false; n];
    for id in ids {
        let number = id["string/".len()..]
            .parse::<usize>()
            .ok()
            .filter(|&k| k > 0 && k <= n && id == format!("string/{k}") && !seen[k - 1]);
        let Some(k) = number else {
            return Err(format!(
                "string body IDs must be unique and contiguous string/1 through string/{n}; found {id}"
            ));
        };
        seen[k - 1] = true;
    }
    if n == 0 {
        return Err("the plan has no string/N bodies".into());
    }
    Ok(n)
}

impl LiveTab {
    /// Build a tab instrument from its anatomy, tuning, capo and hands.
    pub fn new(spec: &TabSpec) -> Result<Self, String> {
        let text = spec.plan_json.as_deref().unwrap_or(STEEL_STRING_V0);
        let plan: WorldPlan =
            serde_json::from_str(text).map_err(|e| format!("anatomy {}: {e}", spec.anatomy))?;
        let n = validate_strings(&plan)?;
        let tuning = match &spec.tuning {
            Some(t) => t.clone(),
            None if n == STANDARD_TUNING.len() => STANDARD_TUNING.to_vec(),
            None => {
                return Err(format!(
                    "anatomy {} has {n} strings: give its tuning",
                    spec.anatomy
                ))
            }
        };
        if tuning.len() != n {
            return Err(format!(
                "tuning has {} courses, the anatomy has {n} strings",
                tuning.len()
            ));
        }
        let hand_at = |s: usize| -> TabHand {
            spec.hands
                .get(s)
                .or(spec.hands.last())
                .copied()
                .unwrap_or_default()
        };
        let hands: Vec<TabHand> = (0..n).map(hand_at).collect();
        // One hand per string, each touching only its own string: apply_hands
        // folds exactly that hand into that string's body.
        let inst = TabInstrument {
            abbrev: "live".into(),
            anatomy: spec.anatomy.clone(),
            hands: hands
                .iter()
                .enumerate()
                .map(|(s, h)| Hand {
                    abbrev: format!("string{}", s + 1),
                    pos: h.pos.clamp(0.02, 0.5),
                    pick: h.pick.clamp(0.0, 1.0),
                    pm: h.pm.clamp(0.0, 1.0),
                    gain_db: h.gain_db,
                })
                .collect(),
            plan,
            open: tuning.iter().rev().copied().collect(),
            capo: spec.capo,
            gain_db: 0.0,
            send: 0.0,
        };
        let touches: Vec<Touch> = (0..n)
            .map(|s| Touch {
                seconds: 0.0,
                string: s + 1,
                fret: 0.0,
                intonation_cents: 0.0,
                db: 0.0,
                hand: s,
                kind: TouchKind::Pluck,
                bow: None,
            })
            .collect();
        let hands = inst
            .hands
            .iter()
            .map(|h| TabHand {
                pos: h.pos,
                pick: h.pick,
                pm: h.pm,
                gain_db: h.gain_db,
            })
            .collect();
        Self::build(&inst, &touches, hands, 0)
    }

    /// Build the World a score's tab instrument names (its anatomy, tuning,
    /// capo and hands, folded as `mus render` folds them) and schedule the
    /// score's touches on it. `instrument` picks a tab instrument by abbrev
    /// (default: the first); `resolve` hands in anatomy plans by name (the
    /// `anatomy=` value) where there is no file system.
    pub fn from_score(
        source: &str,
        instrument: Option<&str>,
        resolve: &dyn Fn(&str) -> Option<String>,
    ) -> Result<(Self, ScoreRead), String> {
        Self::from_score_at(source, instrument, resolve, 0)
    }

    /// [`LiveTab::from_score`], the score's first instant at World sample `offset`.
    pub fn from_score_at(
        source: &str,
        instrument: Option<&str>,
        resolve: &dyn Fn(&str) -> Option<String>,
        offset: usize,
    ) -> Result<(Self, ScoreRead), String> {
        let l = layout(source)?;
        let (insts, _) =
            ariadne_tab::collect_with(&l.proposal.instruments, Path::new("."), resolve)
                .map_err(|e| e.to_string())?;
        let inst = match instrument {
            Some(a) => insts.iter().find(|i| i.abbrev == a),
            None => insts.first(),
        }
        .ok_or_else(|| match instrument {
            Some(a) => format!("the score has no tab instrument {a}"),
            None => "the score has no tab instrument (anatomy=)".into(),
        })?;
        let mut stats = RenderStats::default();
        let touches = ariadne_tab::performance(
            inst,
            &l.bar_map,
            &l.bar_content,
            l.n_bars,
            l.swing_unit_ql,
            l.swing_r,
            &mut stats,
        )
        .map_err(|e| e.to_string())?;
        let hands = inst
            .hands
            .iter()
            .map(|h| TabHand {
                pos: h.pos,
                pick: h.pick,
                pm: h.pm,
                gain_db: h.gain_db,
            })
            .collect();
        let mut live = Self::build(inst, &touches, hands, l.n_total)?;
        for t in &touches {
            let hand = &inst.hands[t.hand];
            let at = offset + (t.seconds * SAMPLE_RATE) as usize;
            let fret = if t.intonation_cents == 0.0 {
                t.fret
            } else {
                t.fret + t.intonation_cents / 100.0
            };
            match TabTouch::of(t.kind) {
                Some(kind) => live.schedule(
                    at,
                    t.string,
                    fret,
                    t.db,
                    kind,
                    hand.pick,
                    t.intonation_cents != 0.0,
                ),
                None => live.schedule_bowed(at, t, fret, offset + l.n_total)?,
            }
        }
        let read = ScoreRead {
            instrument: inst.abbrev.clone(),
            samples: l.n_total,
            notes: stats.notes,
            bad_tokens: stats.bad,
        };
        Ok((live, read))
    }

    fn build(
        inst: &TabInstrument,
        touches: &[Touch],
        hands: Vec<TabHand>,
        _n_total: usize,
    ) -> Result<Self, String> {
        let mut plan = inst.plan.clone();
        if (plan.internal_sample_rate_hz - SAMPLE_RATE).abs() > 1e-6 {
            return Err(format!(
                "anatomy runs at {} Hz, the engine at {SAMPLE_RATE} Hz",
                plan.internal_sample_rate_hz
            ));
        }
        let strings = apply_hands(&mut plan, inst, touches);
        let listener_spec =
            StereoListenerSpec::all_bodies(&plan, 1.0).map_err(|e| e.to_string())?;
        let listener = StereoListener::bind(&listener_spec, &plan).map_err(|e| e.to_string())?;
        let mut world = World::from_plan(plan.clone()).map_err(|e| e.to_string())?;
        let bodies: Vec<String> = (0..world.body_count())
            .map(|i| world.body_id(i).unwrap_or("?").to_owned())
            .collect();
        let string_body: BTreeMap<usize, usize> = bodies
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                id.strip_prefix("string/")
                    .and_then(|n| n.parse().ok())
                    .map(|n| (n, i))
            })
            .collect();
        let controls = bodies
            .iter()
            .enumerate()
            .map(|(i, id)| BodyControls {
                radiation_fraction: if id.starts_with("string/") {
                    STRING_RADIATION
                } else {
                    box_radiation(&world_plan_band(&inst.plan, i))
                },
                ..BodyControls::default()
            })
            .collect();
        // The score's bows: one per bowed string, at the pos of the hand that
        // bows it most, attached in string order (`ariadne_tab::prepare`).
        let mut bows = Vec::new();
        for (&string, &body) in &string_body {
            let mut votes = vec![0_usize; inst.hands.len()];
            for t in touches
                .iter()
                .filter(|t| t.string == string && t.kind == TouchKind::Bow)
            {
                votes[t.hand] += 1;
            }
            let Some((h, _)) = votes
                .iter()
                .enumerate()
                .filter(|(_, &v)| v > 0)
                .max_by_key(|(_, &v)| v)
            else {
                continue;
            };
            let pos = inst.hands[h].pos.min(0.49);
            attach(&mut world, body, pos)?;
            bows.push((body, pos));
        }
        let score_bow_count = bows.len();
        Ok(Self {
            anatomy: inst.anatomy.clone(),
            plan,
            capo: inst.capo,
            open: inst.open.clone(),
            hands,
            world,
            listener,
            bodies,
            string_body,
            controls,
            strings,
            stops: Vec::new(),
            pulses: Vec::new(),
            next_stop: 0,
            next_pulse: 0,
            current: BTreeMap::new(),
            ramps: BTreeMap::new(),
            active: Vec::new(),
            scheduled: BTreeMap::new(),
            counts: [0; 5],
            intonation_touches: 0,
            k: 0,
            bows,
            score_spans: Vec::new(),
            next_span: 0,
            active_spans: Vec::new(),
            score_bowing: vec![false; score_bow_count],
            score_bow_count,
            bow_target: LiveBow::default(),
            bow_binding: None,
            bow_applied: 0.0,
        })
    }

    /// A score's bowed note (`ariadne_tab::prepare`, `TouchKind::Bow`).
    fn schedule_bowed(
        &mut self,
        at: usize,
        t: &Touch,
        fret: f64,
        n_total: usize,
    ) -> Result<(), String> {
        let g = t.bow.ok_or("a bowed touch carries its gesture")?;
        let fretted = self.capo + fret;
        let previous = *self.scheduled.get(&t.string).unwrap_or(&self.capo);
        if (fretted - previous).abs() > 1e-9 {
            let lead = at.saturating_sub((FRET_LEAD_SECONDS * SAMPLE_RATE) as usize);
            insert(
                &mut self.stops,
                self.next_stop,
                (lead, t.string, fretted, 1),
            );
        }
        let body = self.string_body[&t.string];
        let bow = self.bows[..self.score_bow_count]
            .iter()
            .position(|b| b.0 == body)
            .ok_or("a bowed string has its bow")?;
        let sign = if g.up { -1.0 } else { 1.0 };
        let end = at + ((g.seconds * SAMPLE_RATE) as usize).max(1);
        let span = BowSpan {
            start: at,
            end: end.min(n_total),
            bow,
            velocity: (
                sign * BOW_VELOCITY_UNIT * g.speed.0,
                sign * BOW_VELOCITY_UNIT * g.speed.1,
            ),
            force: (BOW_FORCE_UNIT * g.force.0, BOW_FORCE_UNIT * g.force.1),
        };
        let i = self.next_span
            + self.score_spans[self.next_span..].partition_point(|s| s.start <= span.start);
        self.score_spans.insert(i, span);
        self.counts[4] += 1;
        self.intonation_touches += usize::from(t.intonation_cents != 0.0);
        self.scheduled.insert(t.string, fretted);
        Ok(())
    }

    /// Set the live player's bow (it takes effect from the next sample).
    pub fn set_bow(&mut self, bow: LiveBow) -> Result<(), String> {
        let n = self.open.len();
        if bow.string > n {
            return Err(format!("no string {}", bow.string));
        }
        if ![bow.position, bow.speed, bow.force]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err("bow controls must be finite".into());
        }
        let old = self.bow_target;
        let was_down = old.string > 0 && old.force > 0.0;
        self.bow_target = LiveBow {
            string: bow.string,
            position: bow.position.clamp(0.02, 0.49),
            speed: bow.speed.clamp(-1.0, 1.0),
            force: bow.force.clamp(0.0, 1.0),
        };
        let new = self.bow_target;
        let moved = new.string != old.string || (new.position - old.position).abs() > 1.0 / 800.0;
        if new.string > 0 && new.force > 0.0 && (!was_down || moved) {
            self.counts[4] += 1;
        }
        Ok(())
    }

    /// The live player's bow as last set.
    pub fn bow(&self) -> LiveBow {
        self.bow_target
    }

    /// The live bow's binding for (string, point): an attached bow, or a new one.
    fn bow_for(&mut self, string: usize, position: f64) -> Result<usize, String> {
        let body = *self
            .string_body
            .get(&string)
            .ok_or_else(|| format!("no string {string}"))?;
        // Points within 1 mm-ish of each other (1/400 of the length) share a bow.
        let q = (position * 400.0).round() / 400.0;
        if let Some(i) = self.bows[self.score_bow_count..]
            .iter()
            .position(|b| b.0 == body && b.1 == q)
        {
            return Ok(self.score_bow_count + i);
        }
        let i = attach(&mut self.world, body, q)?;
        debug_assert_eq!(i, self.bows.len());
        self.bows.push((body, q));
        Ok(i)
    }

    /// One sample of the live bow: lift from a binding that is no longer
    /// wanted, land on the wanted one, follow speed and force.
    fn step_live_bow(&mut self) -> Result<(), String> {
        let b = self.bow_target;
        let landing = (BOW_LANDING_SECONDS * SAMPLE_RATE).max(1.0);
        let step = 1.0 / landing;
        let wanted = if b.string > 0 && b.force > 0.0 {
            Some((b.string, b.position))
        } else {
            None
        };
        let same = match (self.bow_binding, wanted) {
            (Some(i), Some((string, position))) => {
                let (body, point) = self.bows[i];
                Some(body) == self.string_body.get(&string).copied()
                    && (point - (position * 400.0).round() / 400.0).abs() < 1e-12
            }
            _ => false,
        };
        let target = if same || (self.bow_binding.is_none() && wanted.is_some()) {
            b.force
        } else {
            0.0
        };
        if self.bow_binding.is_none() {
            if let Some((string, position)) = wanted {
                self.bow_binding = Some(self.bow_for(string, position)?);
            }
        }
        let Some(i) = self.bow_binding else {
            return Ok(());
        };
        let d = target - self.bow_applied;
        self.bow_applied += d.clamp(-step, step);
        if self.bow_applied <= 0.0 {
            self.bow_applied = 0.0;
            self.world
                .set_bow(i, BowControls::default())
                .map_err(|e| e.to_string())?;
            self.bow_binding = None;
            return Ok(());
        }
        let velocity = BOW_VELOCITY_UNIT * b.speed;
        self.world
            .set_bow(
                i,
                BowControls {
                    velocity,
                    force: BOW_FORCE_UNIT * self.bow_applied,
                },
            )
            .map_err(|e| e.to_string())
    }

    /// Strings, string 1 (the highest course) first.
    pub fn string_count(&self) -> usize {
        self.open.len()
    }

    /// Open pitch (MIDI) of string `n` (1-based), capo included.
    pub fn open_midi(&self, n: usize) -> Option<f64> {
        self.open.get(n.checked_sub(1)?).map(|m| m + self.capo)
    }

    /// The highest stop the anatomy's configuration path reaches, in frets
    /// above the nut (fret k is amount k/12).
    pub const MAX_STOP: f64 = 12.0;

    /// The World's clock: samples rendered so far.
    pub fn now(&self) -> usize {
        self.k
    }

    /// Touch string `string` (1-based) at fret `fret` above the capo
    /// (fractional for fine intonation, SPEC-TAB §4.1) at World sample `at`,
    /// `db` loud (the written dynamic's dB plus accents and gains; mf is
    /// -11 dB), with the string's own hand. A sample already rendered means now.
    pub fn touch(
        &mut self,
        at: usize,
        string: usize,
        fret: f64,
        db: f64,
        kind: TabTouch,
    ) -> Result<(), String> {
        let hand = *self
            .hands
            .get(string.wrapping_sub(1))
            .ok_or_else(|| format!("no string {string}"))?;
        if !fret.is_finite() || fret < 0.0 || self.capo + fret > Self::MAX_STOP + 1e-9 {
            return Err(format!(
                "fret {fret} with capo {} is outside the stopped-length path [0, 12]",
                self.capo
            ));
        }
        if !db.is_finite() {
            return Err("level is not finite".into());
        }
        self.schedule(
            at,
            string,
            fret,
            db + hand.gain_db,
            kind,
            hand.pick,
            fret.fract() != 0.0,
        );
        Ok(())
    }

    /// `ariadne_tab::prepare`'s per-touch arithmetic, at World samples.
    #[allow(clippy::too_many_arguments)]
    fn schedule(
        &mut self,
        at: usize,
        string: usize,
        fret: f64,
        db: f64,
        kind: TabTouch,
        pick: f64,
        intoned: bool,
    ) {
        let sr = SAMPLE_RATE;
        let now = self.k;
        let width =
            ((PULSE_PICK_SECONDS + PULSE_FLESH_SECONDS * (1.0 - pick)) * sr).max(3.0) as usize;
        let amp = PLUCK_SCALE * db_to_gain(db + 11.0); // mezzo-forte is a unit gesture
        let fretted = self.capo + fret;
        let previous = *self.scheduled.get(&string).unwrap_or(&self.capo);
        let pulse_at = at.max(now);
        match kind {
            TabTouch::Pluck => {
                if (fretted - previous).abs() > 1e-9 {
                    let lead = at
                        .saturating_sub((FRET_LEAD_SECONDS * sr) as usize)
                        .max(now);
                    insert(&mut self.stops, self.next_stop, (lead, string, fretted, 1));
                }
                insert(
                    &mut self.pulses,
                    self.next_pulse,
                    (pulse_at, string, amp, width),
                );
                self.counts[0] += 1;
            }
            TabTouch::HammerOn => {
                insert(
                    &mut self.stops,
                    self.next_stop,
                    (pulse_at, string, fretted, 1),
                );
                insert(
                    &mut self.pulses,
                    self.next_pulse,
                    (pulse_at, string, 0.3 * amp, (0.0003 * sr) as usize),
                );
                self.counts[1] += 1;
            }
            TabTouch::PullOff => {
                insert(
                    &mut self.stops,
                    self.next_stop,
                    (pulse_at, string, fretted, 1),
                );
                insert(
                    &mut self.pulses,
                    self.next_pulse,
                    (pulse_at, string, 0.5 * amp, width),
                );
                self.counts[2] += 1;
            }
            TabTouch::Slide => {
                insert(
                    &mut self.stops,
                    self.next_stop,
                    (pulse_at, string, fretted, (SLIDE_SECONDS * sr) as usize),
                );
                self.counts[3] += 1;
            }
        }
        self.intonation_touches += usize::from(intoned);
        self.scheduled.insert(string, fretted);
    }

    /// Render `left.len()` samples of the World's stereo observation (the
    /// listener's, in the World's units: `mus render` multiplies it by
    /// [`LiveTab::OBSERVATION_GAIN`]).
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let k = self.k;
            // ---- `WorldRun::step`, one sample
            while self.next_stop < self.stops.len() && self.stops[self.next_stop].0 <= k {
                let (_, string, to, len) = self.stops[self.next_stop];
                let from = *self.current.get(&string).unwrap_or(&self.capo);
                self.ramps.insert(string, (from, to, k, len.max(1)));
                self.next_stop += 1;
            }
            let mut done = Vec::new();
            for (&string, &(from, to, s0, len)) in &self.ramps {
                let x = ((k - s0 + 1) as f64 / len as f64).min(1.0);
                let frets = from + (to - from) * x;
                if let Some(&body) = self.string_body.get(&string) {
                    self.world
                        .set_body_configuration_index(body, (frets / 12.0).clamp(0.0, 1.0))
                        .map_err(|e| e.to_string())?;
                }
                self.current.insert(string, frets);
                if x >= 1.0 {
                    done.push(string);
                }
            }
            for s in done {
                self.ramps.remove(&s);
            }
            while self.next_pulse < self.pulses.len() && self.pulses[self.next_pulse].0 <= k {
                self.active.push(self.pulses[self.next_pulse]);
                self.next_pulse += 1;
            }
            for c in self.controls.iter_mut() {
                c.source_wave = 0.0;
            }
            let (string_body, controls) = (&self.string_body, &mut self.controls);
            self.active.retain(|&(s0, string, amp, width)| {
                let i = k - s0;
                if i >= width {
                    return false;
                }
                if let Some(&body) = string_body.get(&string) {
                    controls[body].source_wave +=
                        amp * 0.5 * (1.0 - (2.0 * PI * i as f64 / width as f64).cos());
                }
                true
            });
            // The score's bows (`WorldRun::step`): every span whose note is on
            // sets its bow's arm; a bow whose note has ended is lifted once.
            while self.next_span < self.score_spans.len()
                && self.score_spans[self.next_span].start <= k
            {
                self.active_spans.push(self.score_spans[self.next_span]);
                self.next_span += 1;
            }
            if self.score_bow_count > 0 {
                let landing = ((BOW_LANDING_SECONDS * SAMPLE_RATE) as usize).max(1);
                let mut down = vec![false; self.score_bow_count];
                self.active_spans.retain(|span| k < span.end);
                for span in &self.active_spans {
                    self.world
                        .set_bow(span.bow, span.controls_at(k, landing))
                        .map_err(|e| e.to_string())?;
                    down[span.bow] = true;
                }
                for (bow, (was, is)) in self.score_bowing.iter_mut().zip(&down).enumerate() {
                    if *was && !is {
                        self.world
                            .set_bow(bow, BowControls::default())
                            .map_err(|e| e.to_string())?;
                    }
                    *was = *is;
                }
            }
            if self.bow_binding.is_some() || self.bow_target.force > 0.0 {
                self.step_live_bow()?;
            }
            let frame = self
                .world
                .process_sample(&self.controls, WorldControls::default())
                .map_err(|e| e.to_string())?;
            let heard = self
                .listener
                .observe(&self.world, &frame)
                .map_err(|e| e.to_string())?;
            *l = heard.left as f32;
            *r = heard.right as f32;
            self.k += 1;
        }
        if self.next_span > 256 {
            self.score_spans.drain(..self.next_span);
            self.next_span = 0;
        }
        // Forget what has happened (the schedule only grows forward).
        if self.next_stop > 256 {
            self.stops.drain(..self.next_stop);
            self.next_stop = 0;
        }
        if self.next_pulse > 256 {
            self.pulses.drain(..self.next_pulse);
            self.next_pulse = 0;
        }
        Ok(())
    }

    /// `ariadne_tab`'s listening gain: the listener's observation times this
    /// is what `mus render` mixes into its bus.
    pub const OBSERVATION_GAIN: f64 = ariadne_tab::OBSERVATION_GAIN;

    /// The World again from its plan, at rest, its clock and books at zero.
    pub fn reset(&mut self) -> Result<(), String> {
        self.world = World::from_plan(self.plan.clone()).map_err(|e| e.to_string())?;
        self.stops.clear();
        self.pulses.clear();
        self.next_stop = 0;
        self.next_pulse = 0;
        self.current.clear();
        self.ramps.clear();
        self.active.clear();
        self.scheduled.clear();
        self.counts = [0; 5];
        self.intonation_touches = 0;
        self.k = 0;
        // The score's bows again, at their points; the live bow's are re-attached when it lands.
        self.bows.truncate(self.score_bow_count);
        for &(body, pos) in &self.bows {
            attach(&mut self.world, body, pos)?;
        }
        self.score_spans.clear();
        self.next_span = 0;
        self.active_spans.clear();
        self.score_bowing.fill(false);
        self.bow_target = LiveBow::default();
        self.bow_binding = None;
        self.bow_applied = 0.0;
        Ok(())
    }

    /// Bows now resting with force on a string (the score's and the player's).
    pub fn bows_down(&self) -> usize {
        self.score_bowing.iter().filter(|b| **b).count() + usize::from(self.bow_binding.is_some())
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn bodies(&self) -> &[String] {
        &self.bodies
    }

    /// Where each string's stop is now (frets above the nut, capo included).
    pub fn stop(&self, string: usize) -> f64 {
        *self.current.get(&string).unwrap_or(&self.capo)
    }

    /// The World's receipt, in `ariadneWorlds[]`'s shape (`AGENTS.md`).
    pub fn receipt(&self) -> Value {
        let r = self.world.receipt();
        json!({
            "anatomy": self.anatomy,
            "planDigest": self.world.plan_digest(),
            "listenerDigest": self.listener.listener_digest(),
            "bodies": self.bodies,
            "plucks": self.counts[0],
            "hammerOns": self.counts[1],
            "pullOffs": self.counts[2],
            "slides": self.counts[3],
            "intonationTouches": self.intonation_touches,
            "bowedNotes": self.counts[4],
            "bow": self.bow_receipt(),
            "simulatedSeconds": self.k as f64 / SAMPLE_RATE,
            "capo": self.capo,
            "strings": self.strings,
            "sourceWork": r.source_work,
            "controlWork": r.control_work,
            "internalLoss": r.internal_loss,
            "radiation": r.radiation,
            "initialEnergy": r.initial_energy,
            "currentEnergy": r.current_energy,
            "cumulativeClosureResidual": r.cumulative_closure_residual,
            "maximumFrameResidual": r.maximum_frame_residual,
            "globalClosureResidual": r.global_closure_residual,
            "sampleFrames": r.sample_frames,
        })
    }
}

impl LiveTab {
    /// The bow's part of the books (`ariadne_tab::BowReceipt`'s fields), or
    /// null when no bow has been attached.
    fn bow_receipt(&self) -> Value {
        if self.bows.is_empty() {
            return Value::Null;
        }
        let totals: Vec<BowTotals> = (0..self.bows.len())
            .map(|b| self.world.bow_totals(b).unwrap_or_default())
            .collect();
        let bowed: u64 = totals.iter().map(|t| t.bowed_samples).sum();
        let stuck: u64 = totals.iter().map(|t| t.stuck_samples).sum();
        json!({
            "bows": self.bows.iter().map(|(body, pos)| json!({"body": self.bodies[*body], "pos": pos})).collect::<Vec<_>>(),
            "sourceWork": totals.iter().map(|t| t.source_work).sum::<f64>(),
            "frictionLoss": totals.iter().map(|t| t.friction_loss).sum::<f64>(),
            "bowedSeconds": bowed as f64 / SAMPLE_RATE,
            "stuckFraction": if bowed == 0 { 0.0 } else { stuck as f64 / bowed as f64 },
            "releases": totals.iter().map(|t| t.slip_onsets).sum::<u64>(),
            "peakForce": totals.iter().map(|t| t.peak_force).fold(0.0, f64::max),
        })
    }
}

fn attach(world: &mut World, body: usize, pos: f64) -> Result<usize, String> {
    world
        .attach_bow(
            body,
            pos,
            BowSpec {
                slip_velocity: BOW_SLIP_VELOCITY,
                ..BowSpec::default()
            },
        )
        .map_err(|e| e.to_string())
}

/// Insert keeping time order, after every entry at the same sample (as a
/// stable sort of the schedule would place it), never before `from`.
fn insert<T: Copy>(v: &mut Vec<(usize, T, f64, usize)>, from: usize, e: (usize, T, f64, usize)) {
    let at = from + v[from..].partition_point(|x| x.0 <= e.0);
    v.insert(at, e);
}

// ---------------------------------------------------------------- the piano

#[derive(Debug, Clone, Copy, PartialEq)]
enum PianoAction {
    Launch { key: usize, velocity: f64 },
    Hold { key: usize, delta: i32 },
    Pedal { delta: i32 },
}

#[derive(Debug, Clone, Copy)]
struct LiveDamper {
    engagement: f64,
    falling_since: Option<usize>,
    held: i32,
}

/// The piano (`anatomy=piano-v1`), its World composed for a set of keys,
/// played live.
pub struct LivePiano {
    anatomy: Anatomy,
    keys: BTreeMap<usize, KeyBodies>,
    key_receipts: Value,
    plan: mus_ariadne::ConstitutiveWorldPlan,
    world: World,
    listener: StereoListener,
    bodies: Vec<String>,
    controls: Vec<ConstitutiveBodyControls>,
    is_board: Vec<bool>,
    is_string: Vec<bool>,
    actions: Vec<(usize, PianoAction)>,
    next_action: usize,
    dampers: BTreeMap<usize, LiveDamper>,
    pedal: i32,
    /// Downs minus ups scheduled so far, per key and for the pedal (a live
    /// release of a key that is not down does nothing).
    key_scheduled: BTreeMap<usize, i32>,
    pedal_scheduled: i32,
    pending: Vec<(usize, usize)>,
    strikes: usize,
    refused: usize,
    damper_falls: usize,
    pedal_downs: usize,
    contact_ms: (f64, f64),
    board_radiation: f64,
    string_radiation: f64,
    k: usize,
}

/// The key number (1 = A0 .. 88 = C8) of a MIDI pitch.
pub fn piano_key(midi: f64) -> Option<usize> {
    let key = midi.round() as i64 - 20;
    ((midi - midi.round()).abs() <= 1e-9 && (1..=88).contains(&key)).then_some(key as usize)
}

impl LivePiano {
    /// The keys `piano-v1` has (MIDI 21..108).
    pub fn all_keys() -> Vec<usize> {
        (1..=88).collect()
    }

    /// Compose the World for these keys (numbers 1..88: MIDI - 20). Only
    /// these keys are strung (`SPEC-ARIADNE-PIANO.md` §5).
    pub fn new(keys: &BTreeSet<usize>) -> Result<Self, String> {
        let anatomy: Anatomy = serde_json::from_str(PIANO_V1).map_err(|e| e.to_string())?;
        Self::compose(anatomy, keys)
    }

    fn compose(anatomy: Anatomy, keys: &BTreeSet<usize>) -> Result<Self, String> {
        if keys.is_empty() {
            return Err("a piano needs at least one key".into());
        }
        if (anatomy.sample_rate_hz - SAMPLE_RATE).abs() > 1e-6 {
            return Err(format!(
                "anatomy runs at {} Hz, the engine at {SAMPLE_RATE} Hz",
                anatomy.sample_rate_hz
            ));
        }
        let c = ariadne_piano::compose(&anatomy, keys)?;
        let listener =
            StereoListener::bind_constitutive(&c.listener, &c.plan).map_err(|e| e.to_string())?;
        let world = World::from_constitutive_plan(c.plan.clone()).map_err(|e| e.to_string())?;
        let bodies: Vec<String> = (0..world.body_count())
            .map(|i| world.body_id(i).unwrap_or("?").to_owned())
            .collect();
        let controls = c
            .radiation_fraction
            .iter()
            .map(|&fraction| {
                ConstitutiveBodyControls::ModalSpace(SpaceBodyControls {
                    body: BodyControls {
                        radiation_fraction: fraction,
                        ..BodyControls::default()
                    },
                    ..SpaceBodyControls::default()
                })
            })
            .collect();
        let n = bodies.len();
        let is_board = (0..n).map(|i| c.board_bodies.contains(&i)).collect();
        let is_string = (0..n).map(|i| c.string_bodies.contains(&i)).collect();
        let dampers = c
            .keys
            .iter()
            .filter(|(_, k)| k.damped)
            .map(|(&key, _)| {
                (
                    key,
                    LiveDamper {
                        engagement: 1.0,
                        falling_since: None,
                        held: 0,
                    },
                )
            })
            .collect();
        let mut live = Self {
            key_receipts: serde_json::to_value(&c.receipts).unwrap_or(Value::Null),
            keys: c.keys,
            plan: c.plan,
            anatomy,
            world,
            listener,
            bodies,
            controls,
            is_board,
            is_string,
            actions: Vec::new(),
            next_action: 0,
            dampers,
            pedal: 0,
            key_scheduled: BTreeMap::new(),
            pedal_scheduled: 0,
            pending: Vec::new(),
            strikes: 0,
            refused: 0,
            damper_falls: 0,
            pedal_downs: 0,
            contact_ms: (f64::INFINITY, 0.0),
            board_radiation: 0.0,
            string_radiation: 0.0,
            k: 0,
        };
        live.rest_dampers()?;
        Ok(live)
    }

    fn rest_dampers(&mut self) -> Result<(), String> {
        for (key, d) in &self.dampers {
            for &body in &self.keys[key].strings {
                self.world
                    .set_body_damper(body, d.engagement)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Build the World for the keys a score's piano plays and schedule the
    /// score's key presses and pedal on it, as `mus render` times them.
    pub fn from_score(source: &str, instrument: Option<&str>) -> Result<(Self, ScoreRead), String> {
        Self::from_score_at(source, instrument, 0)
    }

    /// [`LivePiano::from_score`], the score's first instant at World sample `offset`.
    pub fn from_score_at(
        source: &str,
        instrument: Option<&str>,
        offset: usize,
    ) -> Result<(Self, ScoreRead), String> {
        let l = layout(source)?;
        let (pianos, _) = ariadne_piano::collect(&l.proposal.instruments, Path::new("."))
            .map_err(|e| e.to_string())?;
        let inst = match instrument {
            Some(a) => pianos.iter().find(|i| i.abbrev == a),
            None => pianos.first(),
        }
        .ok_or_else(|| "the score has no piano (anatomy=piano-v1)".to_string())?;
        let mut stats = RenderStats::default();
        let (presses, pedal) = ariadne_piano::performance(
            inst,
            &l.bar_map,
            &l.bar_content,
            l.n_bars,
            l.swing_unit_ql,
            l.swing_r,
            &mut stats,
        );
        if presses.is_empty() {
            return Err("the score's piano plays no key".into());
        }
        let used: BTreeSet<usize> = presses.iter().map(|p| p.key).collect();
        let mut live = Self::compose(inst.anatomy.clone(), &used)?;
        live.schedule_presses(&presses, &pedal, offset);
        // `play`: a damper rests at the start unless its key (or the pedal) is
        // already down. The live World starts at sample 0; a score's span that
        // starts there is down at the start.
        let mut down: BTreeMap<usize, i32> = BTreeMap::new();
        let mut pedal_down = 0;
        for (sample, action) in &live.actions {
            if *sample > offset {
                break;
            }
            match *action {
                PianoAction::Hold { key, delta } => *down.entry(key).or_default() += delta,
                PianoAction::Pedal { delta } => pedal_down += delta,
                PianoAction::Launch { .. } => {}
            }
        }
        for (key, d) in live.dampers.iter_mut().filter(|_| offset == 0) {
            if pedal_down > 0 || down.get(key).copied().unwrap_or(0) > 0 {
                d.engagement = 0.0;
            }
        }
        live.rest_dampers()?;
        let read = ScoreRead {
            instrument: inst.abbrev.clone(),
            samples: l.n_total,
            notes: stats.notes,
            bad_tokens: stats.bad,
        };
        Ok((live, read))
    }

    /// `ariadne_piano::play`'s timing of presses and pedal, at World samples.
    fn schedule_presses(&mut self, presses: &[KeyPress], pedal: &[[f64; 2]], offset: usize) {
        let at = |seconds: f64| offset + (seconds.max(0.0) * SAMPLE_RATE).round() as usize;
        let mut launches: Vec<(usize, usize, f64)> = presses
            .iter()
            .map(|p| {
                (
                    at(p.onset_s - self.letoff_m(p.key) / p.velocity),
                    p.key,
                    p.velocity,
                )
            })
            .collect();
        launches.sort_by_key(|l| (l.0, l.1));
        for (sample, key, velocity) in launches {
            self.push(sample, PianoAction::Launch { key, velocity });
        }
        for p in presses {
            if self.dampers.contains_key(&p.key) {
                self.push(
                    at(p.onset_s - DAMPER_LEAD_SECONDS),
                    PianoAction::Hold {
                        key: p.key,
                        delta: 1,
                    },
                );
                self.push(
                    at(p.release_s),
                    PianoAction::Hold {
                        key: p.key,
                        delta: -1,
                    },
                );
            }
        }
        for span in pedal {
            self.push(at(span[0]), PianoAction::Pedal { delta: 1 });
            self.push(at(span[1]), PianoAction::Pedal { delta: -1 });
            self.pedal_downs += 1;
        }
    }

    fn push(&mut self, sample: usize, action: PianoAction) {
        let at =
            self.next_action + self.actions[self.next_action..].partition_point(|x| x.0 <= sample);
        self.actions.insert(at, (sample, action));
    }

    fn letoff_m(&self, key: usize) -> f64 {
        self.anatomy
            .notes
            .iter()
            .find(|n| n.key == key)
            .map_or(0.002, |n| n.hammer.letoff_m)
    }

    /// The keys this World has strings for (MIDI pitches).
    pub fn strung_midi(&self) -> Vec<f64> {
        self.keys.keys().map(|k| (k + 20) as f64).collect()
    }

    /// Press `midi` so its hammer reaches the strings at World sample `onset`
    /// with `velocity` m/s (the damper rises `DAMPER_LEAD_SECONDS` before).
    /// An onset too soon for that lead is moved later, to now plus the lead.
    /// `Ok(false)`: the key is not strung in this World.
    pub fn key_down(&mut self, onset: usize, midi: f64, velocity: f64) -> Result<bool, String> {
        let Some(key) = piano_key(midi) else {
            return Err(format!("midi {midi} is not a piano key"));
        };
        if !self.keys.contains_key(&key) {
            return Ok(false);
        }
        if !(velocity.is_finite() && velocity > 0.0) {
            return Err(format!("hammer velocity {velocity} m/s"));
        }
        let lead = (DAMPER_LEAD_SECONDS * SAMPLE_RATE).round() as usize;
        let onset = onset.max(self.k + lead);
        let launch = (onset as f64 - self.letoff_m(key) / velocity * SAMPLE_RATE)
            .round()
            .max(self.k as f64) as usize;
        self.push(launch, PianoAction::Launch { key, velocity });
        if self.dampers.contains_key(&key) {
            self.push(onset - lead, PianoAction::Hold { key, delta: 1 });
            *self.key_scheduled.entry(key).or_default() += 1;
        }
        Ok(true)
    }

    /// Let `midi` up at World sample `at` (its damper falls unless the pedal holds it).
    pub fn key_up(&mut self, at: usize, midi: f64) -> Result<(), String> {
        let Some(key) = piano_key(midi) else {
            return Err(format!("midi {midi} is not a piano key"));
        };
        if let Some(n) = self.key_scheduled.get_mut(&key).filter(|n| **n > 0) {
            *n -= 1;
            self.push(at.max(self.k), PianoAction::Hold { key, delta: -1 });
        }
        Ok(())
    }

    /// The sustain pedal down (every damper up) or up, at World sample `at`.
    pub fn pedal(&mut self, at: usize, down: bool) {
        let at = at.max(self.k);
        if down && self.pedal_scheduled == 0 {
            self.pedal_scheduled = 1;
            self.pedal_downs += 1;
            self.push(at, PianoAction::Pedal { delta: 1 });
        } else if !down && self.pedal_scheduled > 0 {
            self.pedal_scheduled = 0;
            self.push(at, PianoAction::Pedal { delta: -1 });
        }
    }

    pub fn now(&self) -> usize {
        self.k
    }

    /// The listener's pressure (Pa at 1 m from the board) times this is what
    /// `mus render` mixes into its bus.
    pub const OBSERVATION_GAIN: f64 = ariadne_piano::OBSERVATION_GAIN;

    /// Render `left.len()` samples of the two far-field listening points (Pa).
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
        let dt = 1.0 / SAMPLE_RATE;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let k = self.k;
            // ---- `ariadne_piano::play`, one sample
            while self.next_action < self.actions.len() && self.actions[self.next_action].0 <= k {
                match self.actions[self.next_action].1 {
                    PianoAction::Launch { key, velocity } => {
                        let hammer = self.keys[&key].hammer;
                        if self
                            .world
                            .strike_hammer(hammer, velocity)
                            .map_err(|e| e.to_string())?
                        {
                            let strikes = self.world.hammer_totals(hammer).map_or(0, |t| t.strikes);
                            self.pending.push((hammer, strikes));
                            self.strikes += 1;
                        } else {
                            self.refused += 1;
                        }
                    }
                    PianoAction::Hold { key, delta } => {
                        if let Some(d) = self.dampers.get_mut(&key) {
                            d.held += delta;
                        }
                    }
                    PianoAction::Pedal { delta } => self.pedal += delta,
                }
                self.next_action += 1;
            }
            let world = &self.world;
            let contact = &mut self.contact_ms;
            self.pending.retain(|&(hammer, strikes)| {
                let Some(totals) = world.hammer_totals(hammer) else {
                    return false;
                };
                if world.hammer_active(hammer) == Some(true) && totals.strikes == strikes {
                    return true;
                }
                if totals.strikes == strikes {
                    let ms = totals.last_contact_samples as f64 / SAMPLE_RATE * 1e3;
                    if ms > 0.0 {
                        contact.0 = contact.0.min(ms);
                        contact.1 = contact.1.max(ms);
                    }
                }
                false
            });
            let pedal_down = self.pedal > 0;
            for (key, state) in self.dampers.iter_mut() {
                let lifted = state.held > 0 || pedal_down;
                let before = state.engagement;
                if lifted {
                    state.falling_since = None;
                    state.engagement = (state.engagement - dt / DAMPER_LIFT_SECONDS).max(0.0);
                } else {
                    let since = *state.falling_since.get_or_insert(k);
                    if k == since && before < 1.0 {
                        self.damper_falls += 1;
                    }
                    if (k - since) as f64 * dt >= DAMPER_FALL_SECONDS {
                        state.engagement = (state.engagement + dt / DAMPER_LAND_SECONDS).min(1.0);
                    }
                }
                if state.engagement.to_bits() != before.to_bits() {
                    for &body in &self.keys[key].strings {
                        self.world
                            .set_body_damper(body, state.engagement)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            let frame = self
                .world
                .process_constitutive_sample(&self.controls, WorldControls::default())
                .map_err(|e| e.to_string())?;
            let heard = self
                .listener
                .observe(&self.world, &frame)
                .map_err(|e| e.to_string())?;
            *l = heard.left as f32;
            *r = heard.right as f32;
            for i in 0..self.bodies.len() {
                if !(self.is_board[i] || self.is_string[i]) {
                    continue;
                }
                let radiated: f64 = self
                    .world
                    .body_radiation(i)
                    .map_or(0.0, |es| es.iter().map(|e| 0.5 * dt * e * e).sum());
                if self.is_board[i] {
                    self.board_radiation += radiated;
                } else {
                    self.string_radiation += radiated;
                }
            }
            self.k += 1;
        }
        if self.next_action > 256 {
            self.actions.drain(..self.next_action);
            self.next_action = 0;
        }
        Ok(())
    }

    /// The World again, at rest, its clock and books at zero (same keys).
    pub fn reset(&mut self) -> Result<(), String> {
        self.world = World::from_constitutive_plan(self.plan.clone()).map_err(|e| e.to_string())?;
        self.actions.clear();
        self.next_action = 0;
        for d in self.dampers.values_mut() {
            *d = LiveDamper {
                engagement: 1.0,
                falling_since: None,
                held: 0,
            };
        }
        self.pedal = 0;
        self.key_scheduled.clear();
        self.pedal_scheduled = 0;
        self.pending.clear();
        self.strikes = 0;
        self.refused = 0;
        self.damper_falls = 0;
        self.pedal_downs = 0;
        self.contact_ms = (f64::INFINITY, 0.0);
        self.board_radiation = 0.0;
        self.string_radiation = 0.0;
        self.k = 0;
        self.rest_dampers()
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    /// (strikes, refused strikes, damper falls) so far.
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.strikes, self.refused, self.damper_falls)
    }

    /// Keys strung in this World.
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Whether the pedal is down now.
    pub fn pedal_down(&self) -> bool {
        self.pedal > 0
    }

    /// How many hammers are in flight or on the strings now.
    pub fn hammers_active(&self) -> usize {
        self.keys
            .values()
            .filter(|k| self.world.hammer_active(k.hammer) == Some(true))
            .count()
    }

    /// The World's receipt, in `ariadneWorlds[]`'s piano shape (`SPEC-ARIADNE-PIANO.md` §6).
    pub fn receipt(&self) -> Value {
        let r = self.world.receipt();
        let (lo, hi) = self.contact_ms;
        json!({
            "anatomy": "piano-v1",
            "planDigest": self.world.plan_digest(),
            "listenerDigest": self.listener.listener_digest(),
            "bodies": self.bodies.len(),
            "keys": self.key_receipts,
            "strikes": self.strikes,
            "refusedStrikes": self.refused,
            "damperFalls": self.damper_falls,
            "pedalSpans": self.pedal_downs,
            "contactMsMin": if lo.is_finite() { json!(lo) } else { Value::Null },
            "contactMsMax": if hi > 0.0 { json!(hi) } else { Value::Null },
            "simulatedSeconds": self.k as f64 / SAMPLE_RATE,
            "sourceWork": r.source_work,
            "controlWork": r.control_work,
            "internalLoss": r.internal_loss,
            "radiation": r.radiation,
            "boardRadiation": self.board_radiation,
            "stringRadiation": self.string_radiation,
            "initialEnergy": r.initial_energy,
            "currentEnergy": r.current_energy,
            "cumulativeClosureResidual": r.cumulative_closure_residual,
            "maximumFrameResidual": r.maximum_frame_residual,
            "globalClosureResidual": r.global_closure_residual,
            "sampleFrames": r.sample_frames,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOUR: &str = include_str!("../../../../examples/tab/steel_string_tour.mus");

    /// A score played on a live tab World gives `mus render`'s World
    /// observation bit for bit (before the bus), from the World's first
    /// sample of activity; before it the live World is silent.
    #[test]
    fn a_score_on_a_live_tab_world_is_mus_renders_world() {
        let mut staged =
            crate::StagedRender::begin(TOUR, Path::new("../../../examples/tab")).unwrap();
        let n = 4 * 48_000;
        staged.step(n).unwrap();
        let run = staged.worlds.remove(0);
        let (mut live, read) = LiveTab::from_score(TOUR, None, &|_| None).unwrap();
        assert!(read.bad_tokens.is_empty(), "{:?}", read.bad_tokens);
        let start = run.start;
        let (mut l, mut r) = (vec![0.0f32; start + n], vec![0.0f32; start + n]);
        live.process(&mut l, &mut r).unwrap();
        assert!(l[..start].iter().all(|v| *v == 0.0));
        let done = run.done();
        assert!(done > 48_000);
        for i in 0..done {
            assert_eq!(
                l[start + i].to_bits(),
                run.left[i].to_bits(),
                "left sample {i}"
            );
            assert_eq!(
                r[start + i].to_bits(),
                run.right[i].to_bits(),
                "right sample {i}"
            );
        }
        assert_eq!(live.world().plan_digest(), run.world.plan_digest());
    }

    const BOWED: &str = include_str!("../../../../examples/tab/bowed-note.mus");

    /// A bowed score on a live World is `mus render`'s World, bit for bit.
    #[test]
    fn a_bowed_score_on_a_live_tab_world_is_mus_renders_world() {
        let mut staged = crate::StagedRender::begin(BOWED, Path::new(".")).unwrap();
        let n = 2 * 48_000;
        staged.step(n).unwrap();
        let run = staged.worlds.remove(0);
        let (mut live, _) = LiveTab::from_score(BOWED, None, &|_| None).unwrap();
        let start = run.start;
        let (mut l, mut r) = (vec![0.0f32; start + n], vec![0.0f32; start + n]);
        live.process(&mut l, &mut r).unwrap();
        for i in 0..run.done() {
            assert_eq!(
                l[start + i].to_bits(),
                run.left[i].to_bits(),
                "left sample {i}"
            );
            assert_eq!(
                r[start + i].to_bits(),
                run.right[i].to_bits(),
                "right sample {i}"
            );
        }
        assert!(live.receipt()["bow"]["sourceWork"].as_f64().unwrap() > 0.0);
    }

    /// The live bow lands, holds a note, moves, lifts; its work is a source
    /// and its friction a loss, and the books close.
    #[test]
    fn a_live_bow_sounds_and_the_books_close() {
        let mut live = LiveTab::new(&TabSpec {
            anatomy: "steel-string-v0".into(),
            ..TabSpec::default()
        })
        .unwrap();
        let (mut l, mut r) = (vec![0.0f32; 4800], vec![0.0f32; 4800]);
        live.set_bow(LiveBow {
            string: 3,
            position: 0.12,
            speed: 0.5,
            force: 0.5,
        })
        .unwrap();
        let mut late = 0.0f64;
        for i in 0..20 {
            live.process(&mut l, &mut r).unwrap();
            if i >= 15 {
                late = late.max(l.iter().fold(0.0f32, |m, v| m.max(v.abs())) as f64);
            }
        }
        assert!(late > 1e-6, "a held bowed note sounds: {late}");
        live.set_bow(LiveBow {
            string: 2,
            position: 0.1,
            speed: -0.6,
            force: 0.6,
        })
        .unwrap();
        live.process(&mut l, &mut r).unwrap();
        live.set_bow(LiveBow {
            force: 0.0,
            ..live.bow()
        })
        .unwrap();
        live.process(&mut l, &mut r).unwrap();
        assert_eq!(live.bows_down(), 0);
        let rec = live.receipt();
        let bow = &rec["bow"];
        assert!(
            bow["sourceWork"].as_f64().unwrap() > 0.0
                && bow["frictionLoss"].as_f64().unwrap() > 0.0
        );
        assert_eq!(rec["bowedNotes"], 2);
        assert!(
            rec["maximumFrameResidual"].as_f64().unwrap().abs() < 1e-12,
            "{rec}"
        );
        assert!(
            rec["cumulativeClosureResidual"].as_f64().unwrap().abs() < 1e-12,
            "{rec}"
        );
    }

    /// Live touches close the books, and a touch in the past happens now.
    #[test]
    fn live_touches_play_and_the_books_close() {
        let mut live = LiveTab::new(&TabSpec {
            anatomy: "steel-string-v0".into(),
            ..TabSpec::default()
        })
        .unwrap();
        let (mut l, mut r) = (vec![0.0f32; 2400], vec![0.0f32; 2400]);
        live.touch(0, 6, 0.0, -11.0, TabTouch::Pluck).unwrap();
        live.touch(1200, 1, 3.0, -7.0, TabTouch::Pluck).unwrap();
        live.process(&mut l, &mut r).unwrap();
        live.touch(0, 1, 5.0, -11.0, TabTouch::Slide).unwrap();
        for _ in 0..20 {
            live.process(&mut l, &mut r).unwrap();
        }
        assert!((live.stop(1) - 5.0).abs() < 1e-12);
        let rec = live.receipt();
        assert_eq!(rec["plucks"], 2);
        assert_eq!(rec["slides"], 1);
        assert!(rec["sourceWork"].as_f64().unwrap() > 0.0);
        assert!(rec["maximumFrameResidual"].as_f64().unwrap().abs() < 1e-12);
        assert!(live.touch(0, 7, 0.0, 0.0, TabTouch::Pluck).is_err());
        assert!(live.touch(0, 1, 13.0, 0.0, TabTouch::Pluck).is_err());
    }

    const PRELUDE: &str = include_str!("../../../../examples/piano/prelude-c-major.mus");

    /// The first bars of the prelude on a live piano World: `play_score`'s
    /// World observation, bit for bit.
    #[test]
    fn a_score_on_a_live_piano_is_mus_renders_world() {
        let lines: Vec<&str> = PRELUDE.lines().collect();
        // The header and the first two bars keep the test short.
        let mut short = String::new();
        let mut bars = 0;
        for line in &lines {
            if line.starts_with("bar ") || line.starts_with("bars ") {
                bars += 1;
                if bars > 2 {
                    continue;
                }
            }
            if line.starts_with("# bars:") {
                short.push_str("# bars: 2\n");
                continue;
            }
            short.push_str(line);
            short.push('\n');
        }
        let reference = crate::ariadne_piano::play_score(&short, Path::new("."))
            .unwrap()
            .remove(0);
        let (mut live, read) = LivePiano::from_score(&short, None).unwrap();
        assert!(read.bad_tokens.is_empty(), "{:?}", read.bad_tokens);
        let total = reference.start + reference.left.len();
        let (mut l, mut r) = (vec![0.0f32; total], vec![0.0f32; total]);
        live.process(&mut l, &mut r).unwrap();
        assert!(l[..reference.start].iter().all(|v| *v == 0.0));
        for i in 0..reference.left.len() {
            assert_eq!(
                l[reference.start + i].to_bits(),
                reference.left[i].to_bits(),
                "left sample {i}"
            );
            assert_eq!(
                r[reference.start + i].to_bits(),
                reference.right[i].to_bits(),
                "right sample {i}"
            );
        }
        let rec = live.receipt();
        assert_eq!(rec["strikes"], reference.receipt.strikes);
        assert_eq!(rec["damperFalls"], reference.receipt.damper_falls);
        assert_eq!(rec["pedalSpans"], reference.receipt.pedal_spans);
        assert_eq!(
            rec["boardRadiation"].as_f64().unwrap().to_bits(),
            reference.receipt.board_radiation.to_bits()
        );
    }

    #[test]
    fn live_keys_strike_damp_and_pedal() {
        let keys: BTreeSet<usize> = [60.0, 64.0, 67.0]
            .iter()
            .map(|m| piano_key(*m).unwrap())
            .collect();
        let mut live = LivePiano::new(&keys).unwrap();
        assert!(live.key_down(0, 60.0, 1.6).unwrap());
        assert!(!live.key_down(0, 61.0, 1.6).unwrap(), "C#4 is not strung");
        let (mut l, mut r) = (vec![0.0f32; 4800], vec![0.0f32; 4800]);
        live.process(&mut l, &mut r).unwrap();
        live.key_up(0, 60.0).unwrap();
        live.pedal(0, true);
        live.key_down(live.now() + 2400, 67.0, 2.35).unwrap();
        for _ in 0..10 {
            live.process(&mut l, &mut r).unwrap();
        }
        live.pedal(0, false);
        live.process(&mut l, &mut r).unwrap();
        let rec = live.receipt();
        assert_eq!(rec["strikes"], 2);
        assert!(rec["damperFalls"].as_u64().unwrap() >= 1);
        assert!(rec["maximumFrameResidual"].as_f64().unwrap().abs() < 1e-12);
        assert!(rec["boardRadiation"].as_f64().unwrap() > 0.0);
    }
}
