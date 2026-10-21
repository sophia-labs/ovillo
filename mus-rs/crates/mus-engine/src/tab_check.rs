//! What a checker verifies on an annotated-tab score (`SPEC-TAB.md` §6), on
//! the same reading of the score the renderer plays: the same bar map, the
//! same token walk per hand, the same swing warp, the same open strings.
//!
//! 1. Every note of a tab instrument's hands has a position, and the position
//!    agrees with the pitch (§1).
//! 2. No string is struck twice at the same instant, across all hands.
//! 3. `ho`, `po` and `sl` land on a string that is sounding, and `sl` names a
//!    different fret.
//! 4. Fretting-hand reach: at every instant the fretted notes being held span
//!    at most four frets. The span is the distance between the lowest and the
//!    highest stopped fret (frets 2 and 6 span four): five or more is an error,
//!    four is reported as a stretch.
//! 5. Each hand's durations add up to the bar's length.
//!
//! Also refused, because the renderer would silently drop them: notes written
//! on the instrument's own track rather than on a hand, and a tab instrument
//! with no hands.
//!
//! A held note lasts its written duration (§5), or until its string is next
//! touched, or until its string is next used when it carries `[ring]`. Open
//! strings, harmonics (`[harm]`) and dead notes (`[x]`) don't occupy the
//! fretting hand.

use std::collections::BTreeMap;
use std::path::Path;

use mus_notation::{
    expand_pattern_repetitions, parse_token, split_attached_dynamics, tokenize_bar_content,
};
use serde::Serialize;

use crate::ariadne_tab::{self, midi_name, warp, TabInstrument};
use crate::{layout, rawbars, RenderError};

const EPS: f64 = 1e-6;
/// The widest stop a fretting hand holds without a stretch (SPEC-TAB §6.4).
const REACH_FRETS: u8 = 4;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TabDiagnostic {
    /// `"error"` or `"warning"`.
    pub severity: &'static str,
    pub code: &'static str,
    pub message: String,
    pub anchor: TabAnchor,
}

/// Where a diagnostic points, in the shape `mus check` anchors everything:
/// an event is a bar, a track and the index of the token in that track's
/// bar (after pattern repetitions are expanded and attached dynamics split
/// off), plus the token's text.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum TabAnchor {
    #[serde(rename = "event")]
    Event {
        bar: u32,
        track: String,
        #[serde(rename = "tokenIndex")]
        token_index: usize,
        token: String,
    },
    #[serde(rename = "instrument")]
    Instrument { abbrev: String },
}

/// One tab instrument as the checker and the renderer read it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabInstrumentSummary {
    pub abbrev: String,
    pub anatomy: String,
    /// Open courses, string 1 (highest) first.
    pub strings: Vec<String>,
    pub capo: f64,
    pub hands: Vec<TabHandSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabHandSummary {
    pub abbrev: String,
    pub pos: f64,
    pub pick: f64,
    pub palm_mute: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabReport {
    pub bars: u32,
    pub duration_seconds: f64,
    pub instruments: Vec<TabInstrumentSummary>,
    pub diagnostics: Vec<TabDiagnostic>,
}

impl TabReport {
    pub fn errors(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == "error")
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Technique {
    Pluck,
    HammerOn,
    PullOff,
    Slide,
}

impl Technique {
    fn flag(self) -> &'static str {
        match self {
            Technique::Pluck => "pluck",
            Technique::HammerOn => "ho",
            Technique::PullOff => "po",
            Technique::Slide => "sl",
        }
    }
}

/// One string touched by one note (a chord touches several).
#[derive(Debug, Clone)]
struct Touch {
    order: usize,
    bar: u32,
    track: String,
    token_index: usize,
    token: String,
    string: usize,
    fret: u8,
    onset: f64,
    end: f64,
    technique: Technique,
    stops: bool,
    ring: bool,
}

impl Touch {
    fn anchor(&self) -> TabAnchor {
        TabAnchor::Event {
            bar: self.bar,
            track: self.track.clone(),
            token_index: self.token_index,
            token: self.token.clone(),
        }
    }

    fn describe(&self) -> String {
        format!("{} ({} bar {})", self.token, self.track, self.bar)
    }
}

fn event(bar: u32, track: &str, token_index: usize, token: &str) -> TabAnchor {
    TabAnchor::Event {
        bar,
        track: track.to_owned(),
        token_index,
        token: token.to_owned(),
    }
}

fn diagnostic(
    severity: &'static str,
    code: &'static str,
    message: String,
    anchor: TabAnchor,
) -> TabDiagnostic {
    TabDiagnostic {
        severity,
        code,
        message,
        anchor,
    }
}

fn trim_number(value: f64) -> String {
    let text = format!("{value:.4}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() {
        "0".into()
    } else {
        text.into()
    }
}

/// Check every annotated-tab instrument in `source`. `score_dir` resolves an
/// `anatomy=` given as a path, exactly as the renderer does. An error here
/// (an unreadable anatomy, a tuning that doesn't fit it, a header the bar
/// map refuses) is one the renderer would refuse with too.
pub fn check(source: &str, score_dir: &Path) -> Result<TabReport, RenderError> {
    let (proposal, _) = mus_text::parse_score_lossy(source);
    let headers = &proposal.headers;
    let (swing_unit_ql, swing_r) = layout::parse_swing(headers.get("swing"));
    let abbrevs: Vec<String> = proposal
        .instruments
        .iter()
        .map(|i| i.abbrev.clone())
        .collect();
    let raw = rawbars::scan(source, &abbrevs);
    let n_bars = headers
        .get("bars")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|&n| n != 0)
        .unwrap_or(raw.max_bar);
    let bar_map = layout::build_bar_map(
        headers.get("tempo"),
        headers.get("time"),
        &raw.inline_tempo,
        &raw.inline_time,
        n_bars,
    )?;
    let (instruments, _) = ariadne_tab::collect(&proposal.instruments, score_dir)?;

    let mut diagnostics = Vec::new();
    let mut summaries = Vec::new();
    for inst in &instruments {
        summaries.push(TabInstrumentSummary {
            abbrev: inst.abbrev.clone(),
            anatomy: inst.anatomy.clone(),
            strings: inst.open.iter().map(|m| midi_name(*m)).collect(),
            capo: inst.capo,
            hands: inst
                .hands
                .iter()
                .map(|h| TabHandSummary {
                    abbrev: h.abbrev.clone(),
                    pos: h.pos,
                    pick: h.pick,
                    palm_mute: h.pm,
                })
                .collect(),
        });
        if inst.hands.is_empty() {
            diagnostics.push(diagnostic(
                "error",
                "tab-no-hands",
                format!(
                    "{} has an anatomy but no hands: declare {0}.p, {0}.v (SPEC-TAB §3); nothing would play",
                    inst.abbrev
                ),
                TabAnchor::Instrument { abbrev: inst.abbrev.clone() },
            ));
        }
        for b in 1..=n_bars {
            let Some(content) = raw.bar_content.get(&b).and_then(|m| m.get(&inst.abbrev)) else {
                continue;
            };
            let content = content.trim();
            if content.is_empty() || content == "-" {
                continue;
            }
            let token = tokenize_bar_content(content)
                .into_iter()
                .next()
                .unwrap_or_default();
            diagnostics.push(diagnostic(
                "error",
                "tab-notes-on-instrument",
                format!(
                    "{} is played by its hands: write these notes on {0}.p, {0}.v ... (SPEC-TAB §3); the World would not hear them",
                    inst.abbrev
                ),
                event(b, &inst.abbrev, 0, &token),
            ));
        }
        let touches = hand_touches(
            inst,
            &bar_map,
            &raw.bar_content,
            n_bars,
            swing_unit_ql,
            swing_r,
            &mut diagnostics,
        );
        instrument_checks(&touches, &mut diagnostics);
    }
    Ok(TabReport {
        bars: n_bars,
        duration_seconds: bar_map.total_s,
        instruments: summaries,
        diagnostics,
    })
}

/// The renderer's per-hand token walk (`ariadne_tab::performance`), keeping
/// what a checker needs and reporting what can't be played.
#[allow(clippy::too_many_arguments)]
fn hand_touches(
    inst: &TabInstrument,
    bar_map: &layout::BarMap,
    bar_content: &BTreeMap<u32, BTreeMap<String, String>>,
    n_bars: u32,
    swing_unit_ql: f64,
    swing_r: f64,
    diagnostics: &mut Vec<TabDiagnostic>,
) -> Vec<Touch> {
    let mut touches = Vec::new();
    let mut order = 0;
    for hand in &inst.hands {
        let abbr = hand.abbrev.as_str();
        for b in 1..=n_bars {
            let Some(content) = bar_content.get(&b).and_then(|m| m.get(abbr)) else {
                continue;
            };
            if content.trim().is_empty() || content.trim() == "-" {
                continue;
            }
            let Some(timing) = bar_map.bars.get(&b) else {
                continue;
            };
            let tokens = split_attached_dynamics(&expand_pattern_repetitions(
                &tokenize_bar_content(content),
            ));
            let at = |q: f64| timing.start_s + warp(q, swing_unit_ql, swing_r) * timing.spq;
            let mut q = 0.0_f64;
            let mut readable = true;
            let mut last_timed = None;
            for (ti, tk) in tokens.iter().enumerate() {
                let Some(ev) = parse_token(tk) else {
                    if !matches!(tk.trim(), "" | "-") {
                        readable = false;
                        diagnostics.push(diagnostic(
                            "error",
                            "unparseable-token",
                            format!("{tk} is not a MUS token"),
                            event(b, abbr, ti, tk),
                        ));
                    }
                    continue;
                };
                match ev.kind.as_str() {
                    "dynamic" | "hairpin" => continue,
                    "note" | "chord" => {}
                    _ => {
                        q += ev.ql;
                        last_timed = Some(ti);
                        continue;
                    }
                }
                let (onset, end) = (at(q), at(q + ev.ql));
                q += ev.ql;
                last_timed = Some(ti);
                if ev.tab.len() != ev.midis.len() {
                    diagnostics.push(diagnostic(
                        "error",
                        "tab-missing-position",
                        format!("{tk}: a tab instrument's notes need @string:fret (in a chord, every member or none)"),
                        event(b, abbr, ti, tk),
                    ));
                    continue;
                }
                let flag = |f: &str| ev.flags.iter().any(|x| x == f);
                let technique = if flag("ho") {
                    Technique::HammerOn
                } else if flag("po") {
                    Technique::PullOff
                } else if flag("sl") {
                    Technique::Slide
                } else {
                    Technique::Pluck
                };
                for (midi, pos) in ev.midis.iter().zip(&ev.tab) {
                    let string = pos.string as usize;
                    if string == 0 || string > inst.open.len() {
                        diagnostics.push(diagnostic(
                            "error",
                            "tab-no-string",
                            format!(
                                "{tk}: {} has no string {string} (it has {})",
                                inst.abbrev,
                                inst.open.len()
                            ),
                            event(b, abbr, ti, tk),
                        ));
                        continue;
                    }
                    let expected = inst.expected_midi(string, pos.fret, flag("harm"));
                    if (midi - expected).abs() > 0.01 {
                        diagnostics.push(diagnostic(
                            "error",
                            "tab-position-disagrees",
                            format!(
                                "{tk}: string {string} ({}) at fret {} sounds {}, not {} (SPEC-TAB §1)",
                                midi_name(inst.open[string - 1]),
                                pos.fret,
                                midi_name(expected),
                                midi_name(*midi),
                            ),
                            event(b, abbr, ti, tk),
                        ));
                    }
                    touches.push(Touch {
                        order,
                        bar: b,
                        track: abbr.to_owned(),
                        token_index: ti,
                        token: tk.clone(),
                        string,
                        fret: pos.fret,
                        onset,
                        end,
                        technique,
                        stops: pos.fret > 0 && !flag("harm") && !flag("x"),
                        ring: flag("ring"),
                    });
                    order += 1;
                }
            }
            if readable && (q - timing.qlen).abs() > EPS {
                let ti = last_timed.unwrap_or(0);
                diagnostics.push(diagnostic(
                    "error",
                    "tab-voice-duration",
                    format!(
                        "{abbr}'s durations in bar {b} add up to {} quarters; the bar holds {} (SPEC-TAB §6.5)",
                        trim_number(q),
                        trim_number(timing.qlen)
                    ),
                    event(b, abbr, ti, tokens.get(ti).map(String::as_str).unwrap_or("")),
                ));
            }
        }
    }
    touches.sort_by(|a, b| a.onset.total_cmp(&b.onset).then(a.order.cmp(&b.order)));
    touches
}

fn instrument_checks(touches: &[Touch], diagnostics: &mut Vec<TabDiagnostic>) {
    // The touch that follows each one on the same string.
    let mut next_on_string = vec![None; touches.len()];
    let mut last: BTreeMap<usize, usize> = BTreeMap::new();
    for (i, t) in touches.iter().enumerate() {
        if let Some(&p) = last.get(&t.string) {
            next_on_string[p] = Some(i);
        }
        last.insert(t.string, i);
    }
    // When each note lets go of its string: its duration, or the string's next touch, or ([ring]) only that.
    let release = |i: usize| -> f64 {
        let t = &touches[i];
        let taken = next_on_string[i]
            .map(|n| touches[n].onset)
            .unwrap_or(f64::INFINITY);
        if t.ring {
            taken
        } else {
            t.end.min(taken)
        }
    };

    let mut previous: BTreeMap<usize, usize> = BTreeMap::new();
    for (i, t) in touches.iter().enumerate() {
        if let Some(&p) = previous.get(&t.string) {
            let prev = &touches[p];
            if (prev.onset - t.onset).abs() < EPS {
                diagnostics.push(diagnostic(
                    "error",
                    "tab-string-twice",
                    format!(
                        "{}: string {} is already struck at this instant by {} (SPEC-TAB §6.2)",
                        t.token,
                        t.string,
                        prev.describe()
                    ),
                    t.anchor(),
                ));
                previous.insert(t.string, i);
                continue;
            }
        }
        if t.technique != Technique::Pluck {
            let flag = t.technique.flag();
            match previous.get(&t.string).map(|&p| (p, &touches[p])) {
                None => diagnostics.push(diagnostic(
                    "error",
                    "tab-technique-needs-sounding-string",
                    format!(
                        "{}: [{flag}] needs string {} sounding just before, and nothing has played it yet (SPEC-TAB §4)",
                        t.token, t.string
                    ),
                    t.anchor(),
                )),
                Some((p, prev)) => {
                    if !prev.ring && prev.end < t.onset - EPS && release(p) < t.onset - EPS {
                        diagnostics.push(diagnostic(
                            "error",
                            "tab-technique-needs-sounding-string",
                            format!(
                                "{}: [{flag}] needs string {} sounding, but {} has ended by then (SPEC-TAB §4)",
                                t.token,
                                t.string,
                                prev.describe()
                            ),
                            t.anchor(),
                        ));
                    }
                    if t.technique == Technique::Slide && prev.fret == t.fret {
                        diagnostics.push(diagnostic(
                            "error",
                            "tab-slide-same-fret",
                            format!(
                                "{}: [sl] slides to fret {} from fret {} of {}: a slide names a different fret (SPEC-TAB §6.3)",
                                t.token,
                                t.fret,
                                prev.fret,
                                prev.describe()
                            ),
                            t.anchor(),
                        ));
                    }
                }
            }
        }
        previous.insert(t.string, i);
    }

    // Reach: only an instant where a new stop begins can widen the hand.
    let mut i = 0;
    while i < touches.len() {
        let t0 = touches[i].onset;
        let mut j = i;
        while j < touches.len() && (touches[j].onset - t0).abs() < EPS {
            j += 1;
        }
        let starting: Vec<usize> = (i..j).filter(|&k| touches[k].stops).collect();
        if !starting.is_empty() {
            let held: Vec<usize> = (0..j)
                .filter(|&k| {
                    touches[k].stops && touches[k].onset <= t0 + EPS && release(k) > t0 + EPS
                })
                .collect();
            let low = held.iter().map(|&k| touches[k].fret).min().unwrap_or(0);
            let high = held.iter().map(|&k| touches[k].fret).max().unwrap_or(0);
            let span = high - low;
            if span >= REACH_FRETS {
                let culprit = &touches[starting[0]];
                let others: Vec<String> = held
                    .iter()
                    .filter(|&&k| {
                        touches[k].token != culprit.token || touches[k].track != culprit.track
                    })
                    .map(|&k| format!("{} on string {}", touches[k].token, touches[k].string))
                    .collect();
                let (severity, code, verdict) = if span > REACH_FRETS {
                    ("error", "tab-reach", "more than a hand can stop")
                } else {
                    ("warning", "tab-stretch", "a stretch")
                };
                diagnostics.push(diagnostic(
                    severity,
                    code,
                    format!(
                        "{}: frets {low} to {high} held at once, a span of {span} frets, {verdict} (SPEC-TAB §6.4){}",
                        culprit.token,
                        if others.is_empty() { String::new() } else { format!("; held with {}", others.join(", ")) }
                    ),
                    culprit.anchor(),
                ));
            }
        }
        i = j;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "# score: t\n# tempo: 120\n# time: 4/4\n# instruments:\n\
        #   gt   = steel-string guitar (treble, -12, anatomy=steel-string-v0, tuning=E2 A2 D3 G3 B3 E4, capo=0)\n\
        #   gt.p = thumb (treble, pos=0.10, pick=0.85)\n\
        #   gt.v = fingers (treble, pos=0.20, pick=0.45)\n";

    fn codes(body: &str) -> Vec<&'static str> {
        let report = check(&format!("{HEAD}{body}"), Path::new(".")).unwrap();
        report.diagnostics.iter().map(|d| d.code).collect()
    }

    #[test]
    fn a_clean_bar_is_clean() {
        assert!(codes("bar 1: gt.p=A2q@5:0 E3q@4:2 A2q@5:0 E3q@4:2  gt.v=A3e@3:2 B3e@3:4[ho] C4e@2:1 D4e@2:3[ho] C4e@2:1[po] B3e@2:0[po] A3e@3:2 B3e@3:4[sl]\n").is_empty());
    }

    #[test]
    fn position_must_agree_with_pitch() {
        let report = check(
            &format!("{HEAD}bar 1: gt.v=C4e@2:3 Re Rh. gt.p=Rw\n"),
            Path::new("."),
        )
        .unwrap();
        let d = &report.diagnostics[0];
        assert_eq!(d.code, "tab-position-disagrees");
        assert_eq!(d.severity, "error");
        assert!(d.message.contains("sounds D4, not C4"), "{}", d.message);
        assert_eq!(d.anchor, event(1, "gt.v", 0, "C4e@2:3"));
        assert_eq!(report.errors(), 1);
    }

    #[test]
    fn every_note_needs_a_position_and_a_real_string() {
        assert_eq!(
            codes("bar 1: gt.v=C4w gt.p=Rw\n"),
            vec!["tab-missing-position"]
        );
        assert_eq!(
            codes("bar 1: gt.v=<A3@3:2 C4>w gt.p=Rw\n"),
            vec!["tab-missing-position"]
        );
        assert_eq!(
            codes("bar 1: gt.v=C4w@7:0 gt.p=Rw\n"),
            vec!["tab-no-string"]
        );
    }

    #[test]
    fn one_string_is_struck_once_per_instant_across_hands() {
        assert_eq!(
            codes("bar 1: gt.p=A2w@5:0  gt.v=A2w@5:0\n"),
            vec!["tab-string-twice"]
        );
    }

    #[test]
    fn techniques_need_a_sounding_string_and_slides_move() {
        assert_eq!(
            codes("bar 1: gt.v=B3w@3:4[ho] gt.p=Rw\n"),
            vec!["tab-technique-needs-sounding-string"]
        );
        // A3 ends at beat 1; the pull-off two beats later finds string 3 silent.
        assert_eq!(
            codes("bar 1: gt.v=A3q@3:2 Rq G3h@3:0[po] gt.p=Rw\n"),
            vec!["tab-technique-needs-sounding-string"]
        );
        assert!(codes("bar 1: gt.v=A3q@3:2[ring] Rq G3h@3:0[po] gt.p=Rw\n").is_empty());
        assert_eq!(
            codes("bar 1: gt.v=A3h@3:2 A3h@3:2[sl] gt.p=Rw\n"),
            vec!["tab-slide-same-fret"]
        );
    }

    #[test]
    fn reach_four_is_a_stretch_five_an_error() {
        assert_eq!(
            codes("bar 1: gt.p=C3w@5:3  gt.v=Eb4w@2:4\n"),
            Vec::<&str>::new()
        );
        assert_eq!(
            codes("bar 1: gt.p=B2w@5:2  gt.v=E4w@2:5\n"),
            Vec::<&str>::new()
        );
        // frets 2 and 6: a stretch; 2 and 7: out of reach. Open strings don't count.
        assert_eq!(
            codes("bar 1: gt.p=B2w@5:2  gt.v=F4w@2:6\n"),
            vec!["tab-stretch"]
        );
        assert_eq!(
            codes("bar 1: gt.p=B2w@5:2  gt.v=F#4w@2:7\n"),
            vec!["tab-reach"]
        );
        assert_eq!(
            codes("bar 1: gt.p=E2w@6:0  gt.v=F#4w@2:7\n"),
            Vec::<&str>::new()
        );
        // A stop that has ended no longer holds the hand.
        assert_eq!(
            codes("bar 1: gt.p=B2q@5:2 Rh.  gt.v=Rh F#4h@2:7\n"),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn each_hand_fills_its_bar() {
        let report = check(
            &format!("{HEAD}bar 1: gt.p=A2h@5:0  gt.v=A3w@3:2\n"),
            Path::new("."),
        )
        .unwrap();
        let d = &report.diagnostics[0];
        assert_eq!(d.code, "tab-voice-duration");
        assert!(
            d.message.contains("add up to 2 quarters; the bar holds 4"),
            "{}",
            d.message
        );
        assert_eq!(d.anchor, event(1, "gt.p", 0, "A2h@5:0"));
    }

    #[test]
    fn notes_on_the_instrument_track_or_without_hands_are_refused() {
        assert_eq!(
            codes("bar 1: gt=A2w@5:0 gt.p=Rw gt.v=Rw\n"),
            vec!["tab-notes-on-instrument"]
        );
        let lonely = "# score: t\n# tempo: 120\n# time: 4/4\n# instruments:\n\
            #   gt = steel-string guitar (treble, anatomy=steel-string-v0)\nbar 1: gt=A2w@5:0\n";
        let report = check(lonely, Path::new(".")).unwrap();
        let got: Vec<_> = report.diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(got, vec!["tab-no-hands", "tab-notes-on-instrument"]);
    }

    #[test]
    fn swing_and_dynamics_and_patterns_follow_the_renderer() {
        let swung =
            format!("# swing: 8 64\n{HEAD}bar 1: gt.p={{p}} (A2e@5:0 E3e@4:2 ×4)  gt.v=Rw\n");
        let report = check(&swung, Path::new(".")).unwrap();
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        assert_eq!(report.instruments[0].hands.len(), 2);
        assert_eq!(report.instruments[0].strings[0], "E4");
    }

    #[test]
    fn the_steel_string_tour_is_clean() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../examples/tab/steel_string_tour.mus");
        let text = std::fs::read_to_string(&path).unwrap();
        let report = check(&text, path.parent().unwrap()).unwrap();
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        assert_eq!(report.bars, 9);
    }
}
