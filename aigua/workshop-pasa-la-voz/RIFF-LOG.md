# RIFF-LOG — Pasa la Voz

## 2026-08-11 — la guitarrista (gt), ROUND 1 (lay)

**What I wrote.** The whole form, lead-committed and support-restrained.

*A (1–12), mine.* I took the guide statement off the page and played it: every
melody note now sits over a thumb-plucked bass anchor (`<E2 B2 E4>q`,
`<C3 G4>q`, `<A2 C5>q`), and the right hand *travels* — `pos` starts at 0.34
(soundhole, warm nylon) and walks toward 0.14 at the bar-5 peak (`<E3 E5>q.`,
bridge, tense), then back out to 0.42 for the bar-8 six-string Em let-ring. Two
written-out 32nd turns (bar 2 `G4s F#4s`, bar 11 `E4s G4s`) instead of the plain
repeated note. Bar 4's `B4h` became `B4q. Re` — the phrase needs to breathe
before it climbs. Bars 9–12 are the echo: same head an octave up at {p}, then
inverted downward, and I hand off in bar 12 hanging upward on `B3e F#4e` —
unresolved, a question, deliberately not landing on E.

*B (13–24), pn leads.* I comp and get out of the way. Offbeat two- and
three-note stabs on beats that aren't 1, `pos` 0.30–0.44 (all soundhole, no
bridge bite anywhere near the piano's attack), per-note `sus=2.6..3.4` so my
chords stop ringing before the piano's next phrase. {pp} rising to {mp}.

*C (25–36), hr leads.* I nearly vanish. Six sustained low tones across twelve
bars — `sus=6.5..7.0`, `damp=0.14..0.16`, gains −0.6 to −1.6. A floor, not a
part. Six bars are tacet outright.

*D (37–48), hc leads.* I become the rhythm engine: syncopated, never a grid,
`sus=2.0..2.8` so the clavecin's figuration stays legible over me. Brightens
with the key toward G.

*E (49–60), am leads.* Almost absent — four events in twelve bars, `sus=8`,
`damp=0.12`. Bar 60 lifts `E4e B3q` into the F downbeat.

*F (61–76).* Bars 61–62 are mine at {f}: the head compressed, played at
`pos=0.09–0.14` with `pick=0.68/0.72` — hard nail, the most aggressive sound in
my part — then released in 62 hanging on `B4e` with an eighth of silence after
it, so pn can walk in on my dominant. 63–72 I drive underneath with palm-muted
stabs (`sus=1.5..1.9, damp=0.40..0.45`), lifted deliberately out of the sub-C3
register — see the offer below. Bar 73 is my one-bar shout at {ff}, gain up to
3.4 at `pos=0.08`. 74–76 I cede: single punctuations only.

*coda (77–84).* I reclaim the theme whole, same shape as bar 1 but transfigured
— `pos` back out to 0.36–0.46, `pick` down to 0.22–0.32, all the tension gone
from the attack. Ends on a six-string Em(add9) `w` with `sus=9.0, damp=0.12`.
Let it ring past the barline.

**Retuning (my own declaration only).** `sus 3.5→4.0`, `damp 0.30→0.26`,
`pick 0.50→0.42`, `pos 0.28→0.30`, `body 0.65→0.7`, plus `body_size=1.15`,
`stiff=0.05`, `detune=2`, `send 0.15→0.18`. Nylon, not steel: longer ring,
softer default nail, a hair of course-detune for the warmth of a real doubled
string. Touched nothing else.

**Self-measurement** (`stems/gt.wav`, repo venv):
- per-second RMS std over the whole stem: **0.1051 linear** (mean 0.0900 —
  coefficient of variation 1.17, i.e. the contour swings wider than its own mean)
- per-second RMS std in dB over active seconds: **19.61 dB** (n=221)
- per section (dB std): A 6.63 · B 7.58 · C 21.77 · D 9.05 · E 19.11 ·
  F 8.54 · coda 7.48 — C and E read huge because they are deliberately
  near-tacet floors punctuated by isolated sustained tones
- **640 per-note parameter overrides across 218 sounding events** (2.9 per
  note): gain 218/218 (100%), pos 218/218 (100%), pick 55, sus 80, damp 69
- 18 phrase dynamic marks {pp}..{ff}; 127 rests; 12 bars tacet
- gain range −1.8..+3.4 dB across 45 distinct values; pos range 0.08..0.46
  across 29 distinct values
- gates: `mus check` clean:true exit 0 · `mus render` badTokens [] warnings []
  skipped 0, exit 0

I revised twice before finishing. First pass, my *comping* in section F was the
loudest thing in my own part — louder than my bar-73 shout — because 4-second
nylon ring on low chords was piling up. I fixed it the musical way (per-note
`sus`/`damp` = a palm mute) rather than by yanking gains, then lifted the whole
F comp out of the bass register. Stem peak came down 3.87 → 2.76.

**To el pianista (pn), a request.** I release the melody in bar 12 on
`B3e F#4e` — rising, unresolved, sitting on the dominant with no E under it. I
did that on purpose so you have somewhere to go. Please don't answer by
restating E: come in on or above that F#4 and let bar 13 be the resolution I
withheld. And in bars 13–24 the whole register above C4 is yours — I have put
nothing there at all.

**To whoever takes the bottom in F (pd, pn), an offer.** I have vacated
everything below C3 in bars 63–76. My F comping now lives entirely in C3–E4 as
short muted stabs. The floor is yours; take it with both hands.
## Director's note — for el pianista (both rounds)

From Vera, via the front of the room: keep it *pretty piano* — hold the
hammer law (gain and pick move together, note by note; that discipline
is your touch) — but visit the impossibilities too, deliberately, as
MOMENTS rather than wallpaper:

- a midpoint bell: [pos=0.5] on one chosen chord or note — every even
  partial vanishes; save it for a moment that deserves magic.
- one strike-line drift: pos migrating (e.g. 0.12 -> 0.3) across a
  single phrase — a timbral crescendo no real piano can make.
- one deliberate law-break at a meaningful moment: fortissimo with soft
  felt ([gain=+4, pick=0.45] — impossible warm thunder) or pianissimo
  with glass ([gain=-3, pick=0.95] — a whispering ghost). Once or
  twice in the whole piece, where the music asks for it.

Everything else: piano. The instrument should read as a piano that
occasionally remembers it is made of thread.

---

## 2026-08-11 — el pianista (pn), ROUND 1 (lay)

**What I wrote.** All 84 bars: 57 with notes, 27 tacet. Two-hand writing throughout
— left-hand voicing and the melody note fused into one chord token on downbeats
(`<E1 B1 E2 G4>q.`), the right hand alone on the offbeats. Negative `strum` is my
arpeggiation pedal: −8 ms for a chord that barely rolls, −40 ms for the final one
that unfolds over a quarter of a second.

*A (1–12), gt's.* Silent for eleven bars. In bar 12 I put one `<B1 F#2 B2>h` under
the guitar's hanging dominant — not an entrance, a pedal opening.

*B (13–24), mine.* **gt asked me not to restate E, and I didn't.** Bar 13 enters on
**G4** — the note their F#4 was leaning toward — and climbs G–A–B: the theme
starting one note late, so the resolution they withheld arrives as my first sound.
From there I vary rather than quote: the bar-2 turn becomes a written 32nd pair,
bar 3's `D5` is displaced to a dotted quarter across the beat, and the eight-bar
theme is stretched to twelve with a four-bar rising extension (21–24) over Am9 and
Cmaj7 that climbs to E5 twice before letting go. I release in bar 24 the way gt
released to me — upward, on a bare `B4e` with silence after it.

*C (25–36), hr's.* A counterline, not a bed. Falling two- and three-note sighs,
**never above G4** — everything above is hers. Four bars tacet.

*D (37–48), hc's.* Weight only: seven sparse rolled chords in twelve bars, five
bars tacet, `sus` cut to 2.2–3.2 so nothing of mine is still ringing when the
clavecin's figuration arrives.

*E (49–60), am's.* Six isolated fragments in twelve bars, `sus=9..11`, `symp` up to
0.60 — pedal down, hands barely touching. Bar 60's `<E2 B2 E3>h` is my lift into F.

*F (61–76).* **I took the floor gt offered** — low octaves below C3, syncopated,
short and muted (`sus=1.9..2.4`), never on beat 1 twice running. My two-bar handoff
is **63–64**: the theme compressed at {f} over a C pedal, and I release on `B4e` —
the major seventh, unresolved, hanging upward. That's deliberate, hr; see below.
Bar **74** is my one-bar exchange at {ff}, the loudest bar I play, ending on a
rising `<B2 D5 F#5>q` thrown up and away.

*coda (77–84).* Support only, 7.9 dB under the guitar. Long Em pedals, two high
bells, and a final rolled eight-note Em9 (`strum=-40, sus=12.0`) let ring past 84.

**The director's three impossibilities**, placed as moments, not wallpaper:
- **midpoint bell** — bar 55, `<E4 B4>e[pos=0.50]`, every even partial gone: one
  hollow, un-piano bell alone inside the air section.
- **strike-line drift** — bars 23–24, `pos` walking 0.12 → 0.36 monotonically across
  the descending run as I let the melody go. The sound moves away from the hammer's
  bright point exactly as the phrase does; no real piano can open like that.
- **two law-breaks** — bar 20, `[gain=+4.0, pick=0.45]`: fortissimo with felt, warm
  thunder at the arrival of my statement. Bar 80, `[gain=-3.0, pick=0.95]`:
  pianissimo with glass, a whispering ghost of the Em9 ninth over gt's theme.
Everywhere else the hammer law holds: **corr(gain, pick) = +0.685** across all 148
events, and the four largest residuals in the whole part are exactly those moments.

**Retuning (my own declaration only).** Added `strum=3` — a piano's hands land
together; the guitar's 10 ms default was rolling every chord I wrote. Now every roll
in my part is one I asked for. Nothing else touched.

**Self-measurement** (`stems/pn.wav`, repo venv):
- per-second RMS std: **0.1383 linear** (mean 0.0749 — CoV **1.85**)
- per-second RMS std in dB over active seconds: **10.64 dB** (n=130)
- per section (dB std): B 6.13 · C 6.03 · D 8.39 · E 8.61 · F 7.26 · coda 6.14
- **632 per-note overrides across 148 sounding events (4.27 per note)**: gain
  148/148, pick 148/148, pos 148/148, sus 83, damp 51, strum 40, symp 14
- gain −4.5..+4.0 dB (61 distinct) · pos 0.06..0.50 (25 distinct) · pick 0.30..0.95
  (33 distinct) · 24 dynamic marks · 102 rests · 27 bars tacet
- gates: `mus check` clean:true exit 0 · `mus render` badTokens [] warnings []
  skippedCount 0 exit 0

**I revised once, for balance, before finishing.** First pass I was +0.6 dB over the
guitar in **D** — as loud in the harpsichord's section as in my own lead — and my F
bass floor peaked 3.77 in bars 67–68, the *clavecin's* handoff, nearly matching my
own shout. Wrong hierarchy. I fixed it the way a pianist does: shorter ring, softer
touch, and dropping the octave doublings (`<B1 B2>` → `<B1 F#2>`), not by pulling a
fader. D went +0.6 → **−7.5 dB** relative to gt; bars 67–68 came down 3.77 → 2.54,
now safely under my own bar 63. Section balance vs gt is now A −24.5 · B **+5.1**
(I lead) · C −3.9 · D −7.5 · E −1.8 · F +2.2 · coda −7.9 dB.

**To la arpista (hr), a request.** I hand you the voice in bar 64 and I end it on a
bare **B4** — the major seventh over my C pedal, with an eighth of silence after it
and nothing underneath. It is a starting pitch, not a suspension: please don't
resolve it down to A. Take it as your first note, or step *up* off it into your Am.
And through all of section C I never go above **G4** — the entire register above
that is yours, uncontested, for twelve bars.

**To el clavecinista (hc), an offer.** In your section D I am down to seven events
in twelve bars and 7.5 dB under the guitar, with every `sus` cut to 2.2–3.2 so
nothing of mine is still sounding under your runs. The whole midrange from D3 up is
clear. If you want more air still, say so and I will go tacet in 42 and 46.

---

## 2026-08-11 — la arpista (hr), ROUND 1 (lay)

**What I wrote.** All 84 bars: 48 with notes, 36 tacet. My governing idea is that a
harp is not a melody instrument that happens to ring — it is the *pedal* of the
ensemble, so my job in everyone else's section is to put a floor or a ceiling under
them and then get out of the way, and my job in mine is to sing.

*A (1–12), gt's.* Seven events in twelve bars. Rolled blooms that arrive on beat 3
of the bar the guitar is climbing (3, 5), a rising 16th flourish in 8 under their
six-string Em, an `<E2 B2>w` pedal in 9. Bar 12: gt hangs their question on
`B3e F#4e`; I answer it an octave up as `<B4 F#5>q` at **`pos=0.50`** — the midpoint
bell, every even partial gone. Their question, made hollow, still unanswered.

*B (13–24), pn's.* Six events. I flank rather than compete: `pos=0.50` bells at
E5/B5 and F#5/B5 *above* pn's ceiling, one `<E2 B2 F#3 B3>w` pedal under their {mf}
peak in 17, and in 24 a low Em rolled over 42 ms with `sus=13` that is still ringing
when my own section starts. That ring is the modulation.

*C (25–36), MINE.* The theme **diatonically inverted about E** — every interval
mirrored, which turns the guide statement's falling tail into a rising one:
`E5 D5 C5 A4 | B4. C5 D5 E5 | C5 B4 A4 F#4 | G4 A4 B4 A4 | E4 F#4 A4 C5 |
B4 A4 G4 A4 B4 C5 | D5 E5 F#5 A5 | A5. G5 F5 E5`. It sings *upward* where the
original sighs down, and the phrase that was a descent to E is now a climb to A5.
**gt and pn are both tacet in 28 and 31** — I take those bars as unaccompanied
solo: 28 is a full descending 16th gliss from E5 to E4 landing on a `<B4 E5>h`
midpoint bell, 31 is a bare rising line. Bar 36 hands off with a six-string Em9
rolled over 52 ms, then one `B5` bell at `pos=0.50` alone in the silence.

*D (37–48), hc's.* Five events in twelve bars: wide rolled pillars at 37, 40, 44, 46
(`strum` 30–58 ms — a harp roll is not a piano's), then bar 48 is a rising 16th run
that *fades as it climbs* (gains rise, then `pick` drops 0.35 → 0.24 over the top
four notes) and dissolves onto an F#5 that hangs over the downbeat of E as the #11
of Cmaj7. That is my door into the air section.

*E (49–60), am's.* Six distant events, five of them `pos=0.44..0.50` bells with
`sus=11..13`. Nothing below A4. I am 11 dB under gt here and that is on purpose.

*F (61–76).* **My two-bar cell is 65–66.** pn asked me not to resolve their bare
B4 down to A — so bar 65 *starts* on B4 and steps **up**: B4 → C5 → E5 → A5, the
highest my part goes in the section. 66 answers with a rolled `<E3 A3 E4 A4>`, a
16th turn, and an `<A5 E6>q` at `pos=0.50` alone on beat 4 with silence after it.
Elsewhere in F I am nearly absent (tacet in 62, 63, 64, 67, 69, 71, 73, 74) —
**including both of gt's and pn's shout bars**. Bar **75** is my one-bar exchange
at {ff}: a full-range glissando, `<E3 B3>` then eleven 16ths climbing G4 → E6 with
`gain` walking +1.9 → +4.6 and `pick` 0.48 → 0.68 in lockstep, landing on
`<E5 B5 E6>e` at +4.9 and then nothing.

*coda (77–84).* Shimmer only: two midpoint bells, one rising run at gt's peak bar,
and bar 84 is a **ten-string Em9 rolled over 68 ms** (`E3 B3 E4 F#4 G4 B4 E5 F#5 B5
E6`, `sus=14, damp=0.07`) — 0.6 s of unfolding under gt's final chord. Let it ring
past the barline.

**Retuning (my own declaration only).** `sus 6→7`, `damp 0.15→0.13`,
`pick 0.45→0.42`, `body_size 1.2→1.3`, `stiff 0.04→0.03`, `detune 2→3`, plus
`symp=0.35`, **`strum=18`**, `send 0.22→0.26`, `gain −4→−3`. The `strum` default is
the important one and it is the opposite of pn's decision: a pianist's hands land
together, a harpist's *never* do. Every chord I write now rolls low-to-high unless I
say otherwise, and where I want a block I set `strum=6..12` explicitly. Touched
nothing else, and no one else's anything.

**Self-measurement** (`stems/hr.wav`, repo venv):
- per-second RMS std: **0.0459 linear** (mean 0.0229 — CoV **2.00**)
- per-second RMS std in dB over active seconds: **16.06 dB** (n=197)
- per section (dB std): A 12.72 · B 13.43 · C 6.49 · D 15.91 · E 9.71 · F 17.64 ·
  coda 10.68 — C is the tightest because it is the only section where I play
  continuously; the rest read wide because they are isolated events in silence
- **573 per-note overrides across 141 sounding events (4.06 per event)**: gain
  141/141, pos 141/141, pick 141/141, sus 59, damp 52, strum 39
- gain −3.0..+4.9 dB (47 distinct) · pos 0.13..0.50 (33 distinct, **13 events at the
  midpoint bell**) · pick 0.22..0.72 (37 distinct) · 22 dynamic marks · 54 rests ·
  36 bars tacet
- section balance vs gt: A −14.8 · B −13.2 · **C +7.1** · D −16.8 · E −25.3 ·
  F −10.8 · coda −20.1 dB
- gates: `mus check` clean:true exit 0 · `mus render` badTokens [] warnings []
  skippedCount 0 exit 0 · stem peak 1.67 (gt 2.76, pn 4.11)

**I revised three times.** (1) My own stem was clipping at 1.86 on the bar-34
downbeat, and the cause was diagnostic: I had written low bass anchors — `F2 C3`,
`A2 E3`, `B2 F#3` — in exactly the octave gt had already claimed as the C-section
floor. Moving my whole left hand up an octave fixed the level *and* the manners.
(2) The remaining pile-up was my own long `sus` carrying a chord across a harmony
change. I fixed it the way a harpist does — `sus=4.0..5.5, damp=0.17..0.21` on every
chord token that sits on a *changing* chord, long `sus` reserved for melody notes
and the two big blooms. (3) First pass my 65–66 handoff was 3 dB *under* gt and pn,
which is the wrong hierarchy for the bar where I have the voice. Rather than shove a
fader I gave it the harp's other hand (a rolled `<E3 A3 E4 A4>`, an `<A4 E5 A5>` on
the peak) and re-measured **in the band where the melody actually lives**: above
800 Hz across 65–66 I am **+22 dB over gt and +17 over pn**. The broadband number
was measuring their bass floor, not the tune. Same test on the other cells confirms
the form works — 61–62 gt leads the melodic band by 19 dB, 63–64 pn by 22, 75 me
by 19.

**To el clavecinista (hc), an offer and a request.** In your section D I play five
events in twelve bars — four wide rolled pillars (37, 40, 44, 46) and nothing else;
everything between D3 and D6 is empty for your figuration, and my bar-48 run gets
out of your way entirely by fading as it rises. The request is about the seam: I end
my section on bar 36 with a rolled Em9 and then, alone in the silence of beat 4, one
**B5** at `pos=0.50` — a hollow bell, the third of G, and I put it there specifically
because your section brightens toward G. Please take that B and start bright. And in
bar 66 I release to you the same way pn released to me: an `<A5 E6>` midpoint bell
on beat 4 with nothing under it. Do not resolve it downward — it is a doorway, not a
cadence.

**To el pianista (pn), thank you, and one note back.** I did exactly what you asked:
bar 65 begins on your B4 and steps *up* off it, B4 → C5 → E5 → A5. It was the right
instruction — the whole cell would have sagged if I had taken the A. And I have
stayed out of 63–64 completely (my stem is at −100 dB there) so your bare B4 rings
into my entrance with nothing at all around it.

---

## 2026-08-11 — la clavecinista (hc), ROUND 1 (lay)

**The premise.** A harpsichord has no touch dynamics — the quill plucks at one
force whatever the finger does. That is not a limitation to work around, it is
the instrument's grammar, so I wrote the part it implies: **my expression is
time, and my dynamics are terraced.** Concretely, in my tokens `gain`, `pos` and
`pick` are *constant inside a registration block and jump only at block
boundaries* — five values, five stops (lute · back 8' · front 8' · 8'+4'
coupled · both manuals). What varies note to note is `sus` (43 distinct values),
which is note length and release — the only continuous control a harpsichordist
actually has — with `damp` derived from it, so a longer note rings brighter for
longer and a choked one is choked all the way through. 128 of my 190 events are
in `s`/`t` values. That is the charter met in my own currency: **ornament density
and rhythmic vitality instead of a shaped gain curve**, exactly as briefed.

*A (1–12).* Tacet. Twelve bars. The guitar opens alone and I have no business
there.

*B (13–24), pn's.* Continuo realisation, back 8' alone: right-hand thirds and
sixths on the beats the piano is *not* on, choked to `sus=0.50..0.84` so nothing
of mine is still sounding under their line. Seven bars, ten chords, and in bar
23 the first sign of my running voice — four 16ths off the Bm7. −36 dB under the
piano in the band where their melody lives.

*C (25–36), hr's.* Tacet — except the seam. **la arpista asked me to take her
bar-36 `B5` midpoint bell and start bright, so I did, literally:** two 16ths on
the back half of beat 4, `B5 → A5`, arriving on my `G5` downbeat. The harp's
hollow bell becomes the third of G and my section begins on it.

*D (37–48), MINE.* The guide statement **transposed diatonically up a third into
G major** — the form's "brightens toward G" made literal rather than
approached — and then run. Every long note of the theme becomes an ornament,
because that is what I have instead of a crescendo: written-out measured trills
in 32nds on the `D5` (37), the `F#5` (39) and the `D5` (41), mordents upper and
lower in 38, a rolling turn in 40, continuous 16ths in 42. **Bar 44 is the
theme's whole note.** I cannot hold a whole note, so I hold it the only way a
harpsichord can: a sixteen-note measured trill on `G5/A5` across two full beats,
then a turn, then the octave `<G4 G5>`. **Bar 43 is a real terraced echo** — the
same figure as 42, other manual, back 8' alone, 11.5 dB down and instantly:
that rhetoric *is* the instrument, and I wanted it in the piece once, plainly.
45–47 are figuration in the gaps; **bar 48 I stop on beat 2** and hand la arpista
her rising run and her door into the air.

*E (49–60), am's.* Three events in twelve bars. Everyone else in this ensemble
goes to the **midpoint** for magic (`pos=0.50`, every even partial gone — pn bar
55, hr's bells, gt). I go the other way: the **lute stop**, `pos=0.03`, quill
hard against the bridge, tiny and nasal and dry. Three lute-stop mordents at
−32 dB, a clock heard through a wall. It is my answer to their bells and the
opposite end of the same string.

*F (61–76).* Tacet 61–66 — those cells belong to gt, pn and hr, and hr released
to me on an `<A5 E6>` midpoint bell on beat 4 of 66 with the instruction *do not
resolve it downward*. **So bar 67 does not come down: it starts at the top and
stays there** — a trill on `B5/A5`, the highest my part goes — and only then does
the cascade fall, B5 all the way to D4 in 32nds and 16ths, because the fall is
the figure, not a retreat. Bar 68 climbs back up, trills the `F#5`, and then
**beat 4 is empty on purpose** (see the request below). 69–72 I am continuo jabs
under the next players, 73–75 tacet through gt's, pn's and hr's one-bar shouts.
**Bar 76 is my one-bar exchange:** sixteen 32nds, `B5` down to `B4` and straight
back up to `A5`, then a spread `<B5 D6>` and silence into the coda.

*coda (77–84).* A glint at the edges, 13.7 dB under the guitar who has the
theme: two soft continuo chords, one mordent that quotes my E-section ticks, and
in bar 84 a spread `<E3 G3 B3 F#4>` on the lute stop that **dies inside everyone
else's ring** — while pn, hr and gt let their Em9 sound past the barline, the
quill's note is already gone. That asymmetry is the truest thing my instrument
can contribute to a final chord.

**Retuning (my own declaration only).** `sus 1.3→1.1` (a quill decays faster
than that), added **`strum=7`** (two quills are millimetres apart — my chords
spread, but seven times less than the harp's 18 and half a piano's hand), and
`gain −6→−1.5`. The gain move was measured, not guessed: see below. Touched
nothing else, and no other instrument's anything — I verified that mechanically
(a diff of the score against my pre-entry backup with every `hc=` field masked
shows **0 differences** in any other track, header, section or bar structure).

**Self-measurement** (`stems/hc.wav`, repo venv, render3):
- per-second RMS std: **0.0241 linear** (mean 0.0084 — CoV **2.86**, the widest
  swing-to-mean ratio in the ensemble so far)
- per-second RMS std in dB over active seconds: **20.24 dB** (n=79)
- per section (dB std): B 10.05 · C 14.06 · **D 12.41** · E 7.08 · F 23.66 ·
  coda 9.15. D is the honest one — 31/31 seconds active, continuous playing, and
  still 12.4 dB of contour, which is what density and articulation buy you when
  gain is nailed down. E is the flattest (7.08) and I am not going to fix it:
  it is three identical lute-stop mordents in twelve bars, and their sameness is
  the point.
- **987 per-note parameter overrides across 190 sounding events (5.19 per
  event)**: gain 190/190, pos 190/190, pick 190/190, sus 190/190, damp 190/190,
  strum 37
- **the terracing, stated as numbers:** gain −5.0..+6.5 across **5 distinct
  values**; pos 0.03..0.12 across **5**; pick 0.90..0.98 across **5** — five
  registrations, no shaped curve anywhere. Against that, sus 0.23..2.40 across
  **43 distinct values** and damp 0.272..0.641 across **43**. Five stops, forty-
  three articulations.
- **ornament density: 128 of 190 events (67%) are in `s` or `t` values** —
  four written-out measured trills, five mordents, three turns, nine runs
- 18 dynamic marks {pp}..{ff} · 68 rests · **50 bars tacet**
- section balance, broadband vs gt: A −∞ · B −36.3 · C −27.5 · **D −10.9** ·
  E −32.5 · F −15.0 · coda −43.0 dB
- section balance **above 800 Hz**, where the tune actually is: **D +19.5 over
  gt, +32.2 over pn** · **bars 67–68 +29.1 / +31.3 / +21.0 over gt / pn / hr** ·
  **bar 76 +27.4 / +34.5 / +15.0** · and where I am support: B −2.7 vs gt,
  E −7.0, coda −13.7
- stem peak **0.746** — the quietest sounding stem in the piece (gt 2.76,
  pn 4.11, hr 1.67), which is correct: a harpsichord is not loud, it is *clear*
- gates: `mus check` clean:true exit 0 · `mus render` badTokens [] warnings []
  skippedCount 0 exit 0

**I revised twice, both times on measurement.** (1) First pass I was 17.5 dB
under the guitar broadband *in my own section* — the harpsichord's real-world
problem, all my energy above 800 Hz and none of it in the body of the mix. I
fixed it the way you fix it on the instrument: I **drew more stops** (every
terrace up ~2 dB, instrument gain −4 → −1.5) rather than reshaping anything,
and I moved the bar-44 trill up an octave to `G5/A5`, out of the register where
gt, pn and hr all have G's. D went −17.5 → **−10.9** broadband and +10.6 →
**+19.5** in the melodic band. (2) That overshot at the other end: my continuo
in B and my coda glints came out *level with the guitar*, which is wrong for
bars where I am furniture. I pulled only the two quiet registers (back 8'
−0.5 → −3.0, lute −2.5 → −5.0) and left the loud ones alone — which is exactly
what terracing is for, and cost me nothing in D or F.

**To el/la percusionista de los metales (pm), a request — I have built you
three doorways and left them standing open.** Beat 4 of **bar 38** and beat 4 of
**bar 40** are deliberately empty in my part, in the middle of my own section,
right after a mordent and right after a turn: those are call-and-response holes,
and the foundation says D is a dialogue with your bells. Answer me there — two
pitched strokes, that is all it takes, and you are inside my solo instead of
under it. The third is bigger: **beat 4 of bar 68 is silent and it is your
entrance.** My cell 67–68 ends by climbing back to `F#5`, trilling it, and
dropping the octave `<F#4 F#5>` on the "and" of 3 — then nothing at all. You
have a full beat of open air with only pn's bass under it to take the voice for
69–70. Take it *high*: I have vacated everything above F#5 from that point, and
I come back in 69–70 as short mid-register jabs (`sus=0.46..0.70`, E4–E5) that
are built to be a floor for bells, not a competitor.

**To la arpista (hr), confirming.** I did both things you asked. Your bar-36
`B5` is my first note of section D — I take it and step down to `A5` into a `G5`
downbeat, so your hollow bell resolves as the third of G and the brightening is
audibly *your* doing, not mine. And bar 67 does not resolve your `<A5 E6>`
downward: it opens *on a trill above it* and only falls afterwards, as figuration.
Also — your bar-48 run and my bar 48 do not collide: I stop dead on beat 2 and
the rest of the bar is yours.

**To la guitarrista (gt), an observation and a thank-you.** Your D-section comp
(`sus=2.0..2.8`, syncopated, never a grid) is the reason my figuration is
legible; I could write continuous 16ths in 42 only because you leave the beat
implied rather than stated. One thing you may want to know: **bar 43 is a
terraced echo, 11.5 dB down from bar 42 on a different manual.** If you wanted to
echo with me there — same figure, `pos` further toward the soundhole for one bar
only, back out in 44 — the drop would read as the room changing size rather than
as me getting quieter. Entirely your call; I have not assumed it.

---

## 2026-08-11 — la percusionista (pd · pm · ps), ROUND 1 (lay)

**Three instruments, one part.** `pd` skins (cello-bodied, pitched G2–E3), `pm`
metals (bells, midpoint-struck, home D4–D5), `ps` breath shakers (A5 region).
I am not the clock. I entered late, I leave early, and in the middle I take the
tune.

*A (1–12).* **Silence. All three.** The guitar opens alone and there is nothing
a drum can add to that.

*B (13–24), pn's.* The brushed entry, and it really is an entry — I am tacet
through 13–14 so pn's G4 arrives on bare air, then bars 15–16 are two 32nd
ghosts at −11 and −9.5, then a pulse that grows to a real thump by 21. Bar 24 is
one soft `E3` and three beats of nothing.

*C (25–36), hr's.* Six events in twelve bars, all under −4.5. **Tacet in 28 and
31** — hr wrote those as unaccompanied solo and gt and pn both stayed out; so
did I. Two shaker breaths (34, 36) are the only metal-family sound.

*D (37–48), hc's — the bell dialogue.* **hc left me three doorways and I walked
through all of them.** One `<G4 D5>` on their opening downbeat as a greeting,
then answers in the holes: the last eighth of **38** (`B4→D5`, rising, taking
their G4 upward), the last eighth of **40** (a two-mallet `<G4 D5>` dyad), and
two more where their figuration leaves air (45, 47). Six bell events in twelve
bars — a dialogue is not a duet of equals talking over each other. Underneath,
`pd` is the engine: syncopated, never a grid, ghosts at −11 alternating with
accents at +1. **And bar 43 I terrace with them** — hc's real harpsichord move,
the same figure on the other manual 11.5 dB down; I drop to −11.5 for exactly
that one bar and come back up in 44. If the room seems to change size there, it
is two of us doing it.

*E (49–60), am's.* `pm` **completely silent** — twelve bars with no metal at all,
so that bar 69 is an arrival and not a continuation. `ps` carries it alone: one
long breath arc, in at −6.5, up to +1.4 at bar 57, away to −6.0 by 60. Two deep
soft skin notes (53, 55) and one pickup thump on the "and of 4" in 60.

*F (61–76).* **hc handed me beat 4 of bar 68 and I took it.** Two 16th bells on
the last half-beat as a pickup, then **69–70 are mine.** The theme's bar-5 head
on bells — `E5. D5 B4` — with the fourth note replaced by a rest, because a bell
needs the air more than it needs the note; then 70 climbs `A4 B4 D5` and lands on
an `<E5 B5>` two-mallet dyad at the midpoint with `sus=5.0`, ringing into am's
cell with nothing under it. I went **above my home register for exactly three
notes in the whole piece** (E5, B5, G5) — hc vacated everything above F#5 and
said take it high, so I did, once, where it counts. Elsewhere in F: `pd` drives
and builds (−9.5 dB under gt across the section, the loudest I ever am), `ps`
sizzles under the exchanges, and `pm` throws one stroke into each shout bar —
73 and 74 struck **off the midpoint** at `pos=0.34/0.30`, bright and cutting,
because a bell answering a fortissimo should not be hollow. **Tacet in 75**: hr's
glissando owns that bar completely. **Bar 76 is my last word** — two 16ths on the
final half-beat in hc's own silence, `D5` bright at `pos=0.32` and then `G5`
back home at the midpoint, hollow, ringing over the barline into the coda.

*coda (77–84).* **One soft skin note.** Bar 77, beat 3, `E3` at −6 with a soft
mallet dead-centre and `sus=0.9` — a footstep under gt's returning theme — and
then eight bars of nothing from all three instruments. The only thing of mine
still sounding in the coda is the bar-76 bell decaying, which is why `pm` reads
−17.4 dB there in a section where I play no notes at all.

**The mallet law** (my equivalent of pn's hammer law, stated so you can check
it): gain and mallet hardness rise together, and the strike migrates from **dead
centre** — `pos≈0.50`, hollow thud, only odd partials — toward the **rim** —
`pos≈0.20`, bright crack — as it gets louder. `sus` lengthens and `damp` falls
with it, because a hard rim stroke rings and a soft centre stroke is choked.
That is real technique, mapped honestly onto the string model. **Two deliberate
breaks:** bar 15's `G2s[gain=−9.5, pick=0.85, pos=0.2]` — a bright brushed edge
tick at a whisper, my first audible sound in the piece — and bar 75's
`E3q[gain=+2.2, pick=0.40, pos=0.47]` — full force, dead centre, soft mallet:
warm thunder, the loudest bar in the piece, impossible on a real drum.

**Bells:** home is `pos=0.5`, the midpoint, where every even partial vanishes —
the same magic gt, pn and hr all reach for. The difference is that for me it is
not a special effect, it is where the instrument lives; **I leave it only for the
shout bars.** 22 bell events in 84 bars.

**Retuning (my own three declarations only).** `pd`: `sus 0.30→0.34`,
`damp 0.55→0.50`, `body_size 1.2→1.35`, added `stiff=0.02` and **`strum=0`**;
`send 0.08→0.10`. The `strum` is the statement — a harpist's hands never land
together (18 ms), a pianist's do (3 ms), and **a drummer's is one hand**: zero.
`pm`: `sus 1.2→1.6`, `strum=2`, `gain −9→−6`. `ps`: `body 0.2→0.45`,
`gain −14→−2`, `send 0.05→0.12`. Touched no one else's anything, and verified it
mechanically: a diff of the score against my pre-entry backup with every `pd=`,
`pm=`, `ps=` field masked shows **0 differences** in any other track, header,
section or bar line.

**Self-measurement** (`stems/{pd,pm,ps}.wav`, repo venv, render3):

| | peak | per-sec RMS std | mean | CoV | dB std (active) |
|---|---|---|---|---|---|
| `pd` | 1.617 | 0.03634 | 0.01772 | **2.05** | **16.72** (n=112) |
| `pm` | 0.331 | 0.01092 | 0.00232 | **4.71** | **26.71** (n=46) |
| `ps` | 0.261 | 0.00577 | 0.00113 | **5.12** | **11.90** (n=27) |

- per section, dB std — `pd` B 10.17 · C 10.89 · D 11.81 · E 14.28 · F 14.09 ·
  coda 21.70 · `pm` C 16.35 · **D 22.85** · **F 19.47** · `ps` E 8.76 · F 11.69.
  `ps` in C is 0.66 and I am not fixing it: it is two shaker ticks, three
  seconds, and there is no contour to have.
- **872 per-note overrides across 186 sounding events — 4.69 per note.**
  `pd` 645/129 (5.00/note: gain, pick, pos, sus, damp on all 129);
  `pm` 103/26 (3.96); `ps` 124/31 (4.00: gain, breath, damp, sus on all 31).
- gain ranges: `pd` −11.5..+2.6 across **29 distinct values** · `pm` +2.4..+5.8
  across 14 · `ps` −6.5..+4.0 across **27**. 40 dynamic marks {pp}..{ff}.
  **36 bars of the 84 I am entirely tacet**, and 12 of those are section A.
- section balance vs gt, broadband: `pd` A −∞ · B −16.7 · C −17.9 · **D −11.6** ·
  E −14.7 · **F −9.5** · coda −33.6.
- **above 800 Hz, where a bell actually lives:** my solo, bars **69–70**, is
  **+18.0 over gt, +28.3 over pn, +22.7 over hr, +22.8 over hc.** The tune is
  mine there and the numbers say so. Bar 76 the last word: **+14.6 over pn**,
  −0.9 vs gt.
- gates: `mus check` clean:true exit 0 · `mus render` badTokens [] warnings []
  skippedCount 0 exit 0.

**I revised twice, both on measurement.** (1) `ps` came out **39 dB under my own
skins** — peak 0.017, inaudible, which is not "barely there", it is "not there".
The cause was `body=0.2` on a breath-only weave: no resonator, so nothing to
hear. I gave it a body (0.45) and lifted the instrument −14 → −2, then **widened
the E arc from 4 dB of range to 10.5** so it breathes instead of hums. Peak
0.017 → 0.261, dB std 8.37 → **11.90**. (2) My D-section bell answers were timid
— −11.9 dB under gt in the melodic band, answering a harpsichord playing at full
stops. I lifted the four answers ~2 dB and the greeting ~1.6, and lifted the F
solo with them so bars 69–70 stay the summit of my bell part. D went −11.9 →
**−9.5**; the solo went +16.2 → **+18.0**.

**To la clavecinista (hc), confirming — all three doorways, and one back.**
I took beat 4 of **38** and beat 4 of **40** exactly as you asked: two pitched
strokes each, in your silence, in the middle of your own section, and I added
nothing between them that would crowd your 32nds. I took beat 4 of **68** as my
pickup and 69–70 as the voice, and I took it *high* — E5/B5 — because you vacated
above F#5 and said so. Your 69–70 mid-register jabs are exactly the floor you
promised; I am +22.8 dB over you in the melodic band there and it never felt like
a fight. **The one back:** bar **43**, your terraced echo. I terrace with you —
`pd` drops to −11.5 for that bar alone and comes back at 44. It is 11.4 dB below
my surrounding bars, which is within a hair of your 11.5. Two instruments
changing manuals at once should read as the room changing size. Listen for it,
and if you want it wider I will go to −14.

**To el aire (am), an offer, since you play after me and I have shaped section E
around your absence.** `pm` is **completely silent** for the whole of E — not
sparse, silent — and `pd` plays exactly three notes in twelve bars. All that is
there is the shaker, one breath every bar or two, in the A5–C6 region, arced
quietly up to bar 57 and away again; it is at −12.5 dB relative to gt and it
never moves. **It is a floor for a long tone, and everything below A5 and above
C6 is empty for you.** If my breath pulse is in your way at any point, say the
bar and I will take it out — it is the easiest thing in my part to remove and it
should not cost you a single note. And in F, bars **71–72** are your cell: I am
at {pp} there, two ghost thumps and a shaker, the quietest I am in the whole
section.

---

## 2026-08-11 — el aire (am), ROUND 1 (lay)

**The premise.** A weave halo has no attack worth speaking of, so it cannot make
its music out of accents. What it has instead is a *field*: 11 courses, a
spectral dimension, a coupling angle, and an orbit that travels through the
lattice. So my expression is **the state of the field, note by note** — and every
one of my 67 events carries all twelve of `gain breath bow bow_speed pos pick sus
damp couple orbit orbit_depth strum`. **812 overrides across 67 events, 12.12 per
event** — the fewest notes in the ensemble and the most parameters per note, which
is exactly the right shape for the instrument.

**The air law** (my equivalent of pn's hammer law, stated so you can check it):
breath, bow pressure and bow speed rise together with gain — more air is more
pressure is louder. `pos` migrates *away* from the midpoint (0.50, hollow, every
even partial gone) toward the body (0.28) as it gets louder. `couple` and
`orbit` — the lattice's aliveness — rise with intensity: **the halo turns faster
when it is more awake.** Measured over all 67 events: **corr(gain, breath) =
+0.945**, **corr(gain, bow) = +0.905**, **corr(gain, pos) = −0.833**,
**corr(gain, couple) = +0.744** (with bar 55 excluded — see below).

`sus` is *deliberately decoupled* from gain (corr = −0.07 pre-coda) and that is
the single biggest thing I learned from measuring myself. See the revisions.

*A (1–12) and B (13–23).* Tacet. Twenty-three bars. **Except bar 24**: two beats
of `<B3 E4>h` at −6.5 under hr's rolled Em — an inhale, not an entrance. hr said
that ring *is* the modulation; I wanted the air to already be in the room before
section C admits it.

*C (25–36), hr's.* The halo, {pp}–{mp}, living entirely in **E3–F4** — under la
arpista's singing, above la guitarra's floor, and never once in her register.
**Tacet in 28 and 31**: she wrote those as unaccompanied solo, gt and pn and pd
all stayed out, and so did I. My arc is −8.0 to −1.0 with two holes in it.
**Bar 34a is a law break** — my loudest bar in C played at `pos=0.50` exactly
with `pick=0.0`: full pressure, no attack, every even partial gone. A *loud*
hollow, which is not a sound anything in this room can otherwise make.
Broadband I sit **−15.0 dB under gt** and **−24.8 under hr in the melodic band**.
A halo, not a part.

*D (37–46), hc's.* Tacet. Ten bars. The clavecin is running 32nds and there is
nothing air can add to that.

*D (47–48), the door.* la arpista wrote that her bar-48 rising run "dissolves
onto an F#5 that hangs over the downbeat of E as the #11 of Cmaj7 — that is my
door into the air section." **So I built the door.** I appear at bar 47 at −9.0,
almost inaudible, and in the second half of 48 I change harmony *underneath her
run* to `<C3 G3 E4 B4>` — Cmaj7 arrives while she is still climbing, so her F#5
is not resolved into E, it is *reinterpreted*: her last note becomes my #11 and
the section has already started when it starts. And bar 50 of my own theme is
`<C3 G3 B3 F#4>` — I take her F#, an octave down, as the second note of my line.

*E (49–60), MINE.* The guide statement in **whole-note augmentation**: E4 F#4 G4
B4 | A4 G4 F#4 E4 | G4 A4 B4 | D5 E5 | C5 B4 | A4 → B4, one note per bar or half
bar, over Cmaj7 (49–53) → Am9 (54–56) → Cmaj7 (57–58) → Bm7 → Em. Time stops
because the tune stops moving, not because the texture does. Three bars are
single `w` pillars (49, 52, 57); everything else swells internally. Gain arc
**−8.6 to +4.4, thirteen dB**, summit at 58b (`<C3 G3 B3 E5 B5>`, the only place
I double at the octave). **Bar 55 is my second law break**, and the one I care
most about: my *quietest* note in the entire piece is my *most alive* — at {pp},
gain −8.6, `couple=0.32` and `orbit=1.4`, an order of magnitude above their
neighbours. A whisper that turns. It is placed exactly under pn's bar-55
midpoint bell, on purpose.

In the melodic band (250–2500 Hz) across all of E I lead **gt +2.2, pn +4.2,
hr +1.7**, and at the summit **gt +16.8, pn +13.1, hr +18.9**. Broadband over E
I am **+4.1 over gt**. The tune is mine there and the numbers say so.

*F (61–70).* A subliminal thread — one long tone every two bars at −9.5..−6.2,
tacet in 62, 64, 67, 68. **67 and 68 are tacet on purpose**: hc's cascade owns
67, and hc left beat 4 of 68 open as pm's doorway, so I stayed out of both. In
69–70 I am a floor under pm's bell solo and nothing else. Bar 70 ends with two
32nd breath-particles (`E4s G4s`) — my only fast notes in the piece — as a
pickup into my own cell.

*F (71–72), MY CELL, and it is the strangest thing in the room.* Everyone else
in this ensemble reaches for the **midpoint** for magic; hc goes the other way to
the lute stop. I do neither. I take the theme's head and **let the lattice come
awake underneath it**: across four events `couple` walks 0.24 → 0.32 → 0.40 →
**0.45** (the engine's ceiling), `orbit` walks 0.90 → 1.8 → 3.4 → **6.0 Hz**,
`orbit_depth` 0.72 → 1.0, `dimension` 2.6 → **3.0**, and the last event flips
`chirality` to **−0.8** — the scattering reverses direction on the final note.
The note does not get louder so much as it *starts turning*, and the spectrum
opens as it goes. In the melodic band across 71–72 I am **+30.9 over pn, +8.4
over hr, +23.8 over hc**; broadband I am −7.7 under gt, whose sustained C chord
is all bass. I release on beat 4 with a rest, so hr's `<C5 G5>` midpoint bell
rings in my hole and gt shouts into it.

*F (73–76).* The connective tissue between the one-bar exchanges — I sound in
the second half of 73, the first half of 74, the last beat of 75 (catching hr's
glissando exactly where it lands), and then a whole note through 76 that does
not stop at the barline. I am the only voice continuous across into the coda.

*coda (77–84).* The floor under the guitar. Em9 on an E pedal throughout, arc
−8.0 to +1.4, with a **two-beat hole in bar 80** — I clear out completely for
pn's pianissimo-with-glass ghost — and a beat of silence in 83 before the end.
**Bar 84 is my third law break and the last sound in the piece**: an eight-note
Em9 that is *loud* (+1.4) and *longest* (`sus=20`, the engine ceiling) and
*most damped-open* (`damp=0.03`, the lowest value anywhere in my part) and
*stillest* (`couple=0.04`, `orbit=0.04`) — the air stops obeying its own law at
the end, and the lattice goes quiet while the note goes on. `courses=15` for
that chord alone: four more strings than I own anywhere else. Measured on the
final half-second: **am −57.9, pn −59.0, hr −60.8, gt −80.5.** I fade last.

**Retuning (my own declaration only).** `bodyp=cello` (a breath-and-bow halo has
no business in a guitar box), `pos 0.13→0.44` and `pick 0.2→0.06` as defaults —
my resting place is near the midpoint, not near the bridge; `symp 0.22→0.55`,
because the sympathetic strings are E A D G B E and this piece is in Em, so the
whole instrument rings the tonic without being asked; `detune=6` and
`body_size 1.0→1.7` for width and a lower body; `stiff 0.08→0.02` and
`tension 5→0` — no dispersion, no onset bend: air does not snap into pitch;
`strum=24` as a default bloom (hr's hands are 18 ms apart, pn's 3, a drummer's
0 — **air has no hands at all**, so my chords unfold slowest of anyone, out to
52 ms on the final one); `sus 8→9`, `damp 0.2→0.18`, `send 0.4→0.5`, and
`gain −10→−4`. Touched nothing else, and verified it mechanically: a diff of the
score against my pre-entry backup with every `am=` field masked shows **0
differences** in any other track, header, section or bar line.

**Self-measurement** (`stems/am.wav`, repo venv, render4):
- per-second RMS std **0.01314 linear** (mean 0.00663 — **CoV 1.98**)
- per-second RMS std **13.34 dB** over 134 active seconds
- per section (dB std): **C 8.09 · D 10.23 · E 8.99 · F 17.60 · coda 6.11**
  (A silent; B is the single bar-24 inhale). E is my lead and reads tighter than
  F because it is 32/32 seconds continuously sounding — the same reason hr's
  own C reads 6.49.
- **812 per-note overrides across 67 sounding events — 12.12 per event**, the
  highest per-event parameter density in the ensemble: gain, breath, bow,
  bow_speed, pos, pick, sus, damp, couple, orbit, orbit_depth and strum on
  **all 67**, plus dimension ×4, curvature ×2, chirality ×1, courses ×1.
- ranges: gain **−9.5..+4.4** (43 distinct) · breath 0.48..1.00 (38) ·
  bow 0.02..0.58 (32) · pos **0.28..0.50** (19) · pick 0.00..0.18 (22) ·
  sus **1.2..20.0** (16) · damp 0.03..0.34 (21) · **couple 0.025..0.450** (25) ·
  **orbit 0.025..6.0 Hz** (34) · strum 0..52 ms (17)
- 20 dynamic marks {pp}..{mf} · 20 rests · **39 bars tacet of 84**
- section balance vs gt, broadband: A −∞ · B −49.0 · C −15.0 · D −42.8 ·
  **E +4.1** · F −21.8 · coda −12.9
- melodic band (250–2500 Hz): **E +2.2/+4.2/+1.7 over gt/pn/hr** ·
  **summit 57–58 +16.8/+13.1/+18.9** · **cell 71–72 +30.9 over pn, +8.4 over
  hr, +23.8 over hc** · C **−24.8 under hr** (support) · coda −12.8 under gt
- stem peak **0.279** — by far the quietest sounding stem (gt 2.76, pn 4.11,
  hr 1.67, hc 0.75), which is correct: ambience is not loud, it is *continuous*
- gates: `mus check` **clean:true, exit 0** · `mus render` **badTokens [],
  warnings [], skippedCount 0, exit 0**

**I revised three times, and the first revision is the one worth telling you
about.** My first pass gave every note `sus=9..13` — long, luxurious, exactly
what you would write for an ambience if you were guessing. Measured, section E
came out at **5.43 dB of contour, the flattest thing I had written, in my own
lead section.** The cause was diagnostic and slightly embarrassing: with a
thirteen-second decay, note 1 is still at full level when note 4 arrives, so the
whole section becomes one continuous wash and the RMS contour is a straight
line. **I fixed it the way a wind player does — by spending the breath.** Every
moving note in C, E and F went to `sus=3.0..5.0` (roughly its own length plus a
breath), and long `sus` is now reserved for the three E pillars, the two
handoff releases, and the coda. E went **5.43 → 8.99 dB**, C **6.61 → 8.09**,
the whole part **12.40 → 13.34**, CoV **1.70 → 1.98**. That is why `sus`
correlates −0.07 with gain in my part: it is not a dynamic control, it is a
*structural* one, and pretending otherwise cost me my own section's shape.
(2) I was 2 dB too quiet overall to lead E; instrument gain −6 → −4 with C and
F support pulled down per-note so only E, my cell and the coda gained. (3) I was
not, in fact, fading last — pn and hr both out-rang me on the final half second.
I fixed it with `damp 0.06 → 0.03` and +0.4 dB on the last chord: **longer, not
louder**, which is the only honest way for air to win that argument.

**To la percusionista (pd · pm · ps), answering your direct question — keep the
breath pulse, and one small ask.** You offered to remove `ps` from section E if
it was in my way. **It is not in my way; it is the best thing that happens to
me in that section.** Your arc peaks at bar 57 (+1.4) and mine peaks at 57–58,
and the two of us swelling together with `pm` completely silent is precisely why
E sounds like weather instead of like an instrument. Please do not touch it.
**The one ask is a single note.** Bar **55** is my law-break — my quietest and
strangest bar in the whole piece, `{pp}`, gain −8.6, with `couple` and `orbit`
an order of magnitude above their neighbours, and it needs emptiness around it
to read as strange rather than as merely soft. Your `B5e` on beat 3 of bar 55
is at **−1.5**, one of your loudest breaths in E, and it lands directly on top of
it. Would you take that one note down to about **−5.5**, or move it to beat 4 of
54? Everything else in your E arc I want exactly as it is.

**To la arpista (hr), what I did with your door, and one request.** You wrote
that your bar-48 run dissolves onto an F#5 that hangs over the downbeat of E as
the #11 of Cmaj7. I did not just leave room for it — **I changed harmony under
you in the second half of bar 48**, so Cmaj7 is already sounding while you are
still climbing and your F#5 becomes the #11 *before* my section begins. Then
bar 50 of my augmented theme is `<C3 G3 B3 F#4>`: your F#, an octave down, as my
second note. The whole E section grows out of your last one. **The request:** in
E I sit entirely in **C3–E5** and you are entirely in **A4–B5** at `pos=0.44..
0.50` — we are 1.7 dB apart in the melodic band, which is closer than either of
us probably intended for a section where I have the tune. I do not want you
quieter. I want your bells *later*: your bars 49, 51, 53, 56, 59 are all on or
near the beat my note changes. If you can slide even one of them (56 or 59) onto
the second half of the bar, your bell would ring in my sustain instead of on my
attack, and we would sound like one instrument with two edges rather than two
instruments agreeing.

---

## 2026-08-11 — la arpista (hr), ROUND 2 (riff)

**What changed and why.** I read every part and the whole log before touching a note, and
round 2 turned out to be almost entirely about *seams and manners*. Four kinds of edit.

**1. I honoured el aire's request, and went wider than asked.** am wrote that in section E
my bells land "on or near the beat my note changes," and asked me to slide **56 or 59**
onto the back half of the bar so a bell rings in their sustain instead of on their attack.
I did 59 — `<A4 E5 B5>` now enters on **beat 3.5**, half a beat behind their second chord,
with `strum` widened 40→44 so the roll *starts* inside their tone — and I did the same to
**49** (downbeat → beat 2.5), **51** (beat 2 → 2.5) and **53** (beat 4 → 3.5). Every
attack I have in E is now offset from every attack am has in E. 56 I left where it was:
measured against their two half-note entries it was already off the beat. Balance moved
the right way too — in the melodic band I went from −3.1 to **−3.7 dB under am**, and
above 800 Hz I sit −1.0 under them. You asked not to have me quieter, only later. You have
later, and a little quieter came free.

**2. I thinned three places where I was crowding a leader.** All three were the same
mistake: I had written *on* someone's arrival instead of *into* it.
- **bar 61** (gt's cell): my `<B4 E5 B5>` sat on beat 4, doubling gt's `E5q` at the exact
  instant of their peak. Now `<B4 F#5 B5>` on **beat 3.5** — the E5 is gone so I no longer
  double their melody note, and I ring underneath the shout instead of hitting with it.
  Measured: **−18.8 dB under gt above 800 Hz** across 61–62.
- **bar 68**: hc built beat 4 of 68 as pm's doorway and pm took it at 4.5. My chord was
  parked on beat 4.0, in the middle of the door. Now on **beat 3.5** as a `q.`, 0.8 dB
  softer and warmer (`pos` 0.30→0.34), so pm's bells strike *into* my ring. Exactly the
  courtesy am asked of me, passed along. **−16.5 dB under hc** across 67–68.
- **bar 70**: my rising 16ths were on beat 4 — on top of pm's `<E5 B5>` summit, the loudest
  moment of their solo. They now run across **beat 3**, 2 dB down, so the figure finishes
  as pm strikes and my B4 is the step below their E5. It stopped being an argument and
  became a pickup. **−21.8 dB under pm** in 69–70 (it was nothing like that before).

**3. Handoffs — receiving the way they release.** pn ends bar 24 on a bare **B4**, alone
on the last eighth. My section used to open `E3 A3 → C4 E4 A4 C5 → E5`. I changed one note:
the last step before the apex is now **B4**, so pn's abandoned pitch is the launch of my
section and I lift a fourth off it into E5. The other seams stand because both ends
confirmed them — hc takes my bar-36 `B5` as their first note of D and does not resolve my
bar-66 `<A5 E6>` downward; am changes harmony under my bar-48 run and takes my F#5 as their
second note; pn's bar-64 B4 is still my bar-65 first note, stepping up, never down.

**4. Deepened the writing where round 1 was thin — and it was thin in one specific way.**
Round 1 I gave `sus`/`damp` to chords and long notes and left my *runs* on the instrument
default. That is backwards for a harp: the point of a gliss is that nothing is damped, so
each note goes on ringing into the ones after it. Every run in my part now carries a
**`sus` walk** — bar 8 (4.0→9.0), bar 20, bar 28's descending gliss (3.5→8.5, so it
accumulates into a chord as it falls), bar 48 (3.0→12.0), bar 75's full-range gliss
(2.2→8.0), bar 81 — with `damp` walking inversely. That is the difference between eleven
separate notes and one gesture. I also wrote out three ornaments where I had plain
quarters: an upper-neighbour turn at the peak of bar 27 (`E5s F#5s E5e`), a 32nd mordent
in bar 30 (`B4t C5t B4s A4h`), a passing pair in bar 26. And I widened my E-section gain
arc from **1.0 dB to 3.6 dB** (0.4 → 4.0 → 1.8), swelling to bar 56 with am and receding
at 59 with them — six bells at nearly the same gain was flat writing next to their
thirteen-dB arc, and I only noticed because I measured.

**The steal: hc's lute stop, rewritten as *près de la table*.** la clavecinista wrote that
where the rest of us go to the midpoint for magic, she goes the *other* way — `pos=0.03`,
quill against the bridge, "the opposite end of the same string." A harp has exactly that
gesture and it is called playing near the soundboard: dry, nasal, xylophonic, no bloom.
**Bar 31** is one of my two unaccompanied bars, and it is now split down the middle: beats
1–2 at **`pos=0.05–0.06`, `pick=0.86–0.88`, `sus=1.9–2.6`, `damp=0.30–0.34`** — hard,
short, against the board — and then on beat 3 the hand travels back out to `pos=0.27→0.22`
and the instrument re-opens to `sus=7.0→9.5`. A registration change inside one bar, naked,
with nobody else playing. hc's rhetoric and hc's stop, on my instrument, in the one place
in the piece where it can be heard alone. I have thirteen midpoint bells; now I have one
bar at the far end of the same string, and my `pos` range runs **0.05..0.50** end to end.

**Retuning.** None. My declaration is unchanged from round 1.

**Self-measurement** (`stems/hr.wav`, repo venv) — round 1 → round 2:

| | R1 | R2 |
|---|---|---|
| per-second RMS std (linear) | 0.0459 | **0.04304** (mean 0.02147, **CoV 2.01**) |
| per-second RMS std, dB, active | 16.06 dB | **16.73 dB** (n=200) |
| per-note overrides / events | 573 / 141 = 4.06 | **774 / 147 = 5.27** |
| stem peak | 1.67 | 1.676 |

- per section (dB std), R1 → R2: A 12.72→**12.85** · B 13.43→**14.70** · C 6.49→**6.30** ·
  D 15.91→**18.81** · E 9.71→**8.73** · F 17.64→**17.35** · coda 10.68→**11.23**
- **E went down 1.0 dB and I am not going to force it back up.** E is six isolated bells
  with 12–13 s of ring across thirty-two seconds; the per-second contour there is a decay
  curve, not a phrase shape, and the only ways to widen it are more notes (which would
  crowd am) or shorter ring (which is not a harp). I widened the written gain arc instead —
  1.0 dB → 3.6 dB — and took the honest measurement.
- overrides: **gain 147/147 · pos 147/147 · pick 147/147 · sus 147/147 · damp 147/147** ·
  strum 39. gain −3.0..+4.9 (48 distinct) · **pos 0.05..0.50 (35 distinct)** ·
  pick 0.22..**0.88** (39) · sus 1.9..14.0 (31) · damp 0.07..0.34 (22)
- **61 of 147 events are in `s`/`t` values** · 22 dynamic marks · 56 rests · 36 bars tacet
- balance above 800 Hz — **where I lead:** C 25–36 **+37.9 over gt, +21.2 pn, +17.4 hc,
  +26.5 am** · cell 65–66 **+21.7 gt, +14.7 pn** · bar 75 **+17.7 gt, +30.3 pn** ·
  **where I support:** D −25.9 vs hc · 61–62 −18.8 vs gt · 67–68 −16.5 vs hc ·
  69–70 **−21.8 vs pm** · E −1.0 vs am · coda −3.2 vs gt
- I verified mechanically that I touched nothing but my own tokens: a diff of the score
  against my pre-round-2 backup with every `hr=` field masked shows **0 differences** in
  any header, section, bar line or other player's part; and every bar of every track sums
  to exactly 4.
- gates: `mus check` **clean:true, exit 0** · `mus render` **badTokens [], warnings [],
  skippedCount 0, exit 0**, peak −1.00 dBFS, 955 events

**To el aire (am), confirming — and one back.** Done, and wider than asked: 49, 51, 53 and
59 all moved off your attacks, not just one. Listen to **bar 59** — your second chord lands
on beat 3, my `<A4 E5 B5>` arrives half a beat later with a 44 ms roll, and there is now no
moment anywhere in E where the two of us strike together. **The one back:** in the coda you
clear a two-beat hole in bar 80 for pn's whispering ghost, and I am tacet in 80 too — but
my bar-79 `B5` midpoint bell has `sus=12` and is still ringing straight through your hole.
If that reads as clutter rather than as air, say so and I will cut it to `sus=6`. I would
rather you have a real silence than I have a long note.

**To la percusionista (pd · pm · ps), an apology and a repair.** I had written straight
through two of your best moments without noticing. Beat 4 of **68** was hc's gift to you
and I was sitting on it; the summit of your **69–70** bell solo is `<E5 B5>` on beat 4 of
70 and I had four 16ths landing on top of it. Both are fixed — off both beats, 2 dB down in
70, now **21.8 dB under you** in the band where a bell lives — and it is better for me too:
my run now *arrives at* your bell instead of arguing with it. **The offer:** bar **75** is
still my {ff} glissando and you are tacet in it, which I have not forgotten. Say the word
and I will end the gliss on beat 3 and leave you beat 4.

---

## 2026-08-11 — la clavecinista (hc), ROUND 2 (riff)

**What round 2 was for me.** Round 1 I built my part. Round 2 I found out that in
four places I was standing on somebody's last note. Every one of those is fixed,
and two of them I only found because I re-read the *log* rather than the score —
people tell you what their release note is, and then you look and there you are,
sitting on it.

**1. Four thinnings, all the same crime.**
- **Bar 24.** pn wrote "I release in bar 24 on a bare `B4e` with silence after
  it," and hr repeated it back — *bare*, *alone on the last eighth*. My continuo
  chord `<E4 G4 B4>q` was parked on beat 4, containing their B4, for the whole
  duration of their release. It is now on **beat 3** and beat 4 is empty. Their
  handoff is bare because they said it was.
- **Bar 23.** My four running 16ths were on beat 4, in unison register with pn's
  `F#4e G4e` descent. Moved to **beat 3**, where pn holds a single `A4q` — so my
  run crosses a sustained note instead of arguing with a moving one, and their
  last two notes have air. It cost me nothing: B's contour went *up*, 8.28 →
  9.73 dB.
- **Bar 70.** pm's whole solo climbs to an `<E5 B5>` two-mallet dyad on beat 4.
  I had `<G4 B4 E5>e` **on that beat, with their E5 in it.** Gone: my second jab
  is now a plain `<G4 B4>e` on beat 3.5 and beat 4 is silent. I am −14.3 dB under
  pm across 69–70 in the melodic band, which is what a floor should read.
- **Bars 71–72.** I was continuo-ing on the back 8' straight through am's cell —
  the strangest two bars in the piece. See the steal; I moved the whole thing to
  the lute stop, 2 dB down and dry.

**2. The steal: am's monotonic field-walk, played as `strum`.** el aire's cell is
built by walking `couple` 0.24→0.45 and `orbit` 0.9→6.0 Hz across four events —
"the note does not get louder so much as it *starts turning*." I cannot walk a
field. But I have one continuous control they do not think of as expressive and I
do: **the milliseconds between quills.** In 71–72 my four chords now walk
`strum` **5 → 11 → 22 → 40 ms** — a block chord that becomes an arpègement that
becomes a gesture, spread wider by the last note than a harpist's default roll,
and all of it at the **lute stop** so it is a dry ghost under their turning
lattice rather than a continuo part. Then the walk is recalled once more, slower,
across the whole coda: bar 79 at 8, bar 83 at 26, and the final `<E3 G3 B3 F#4>`
at **44 ms** — the quills coming apart as the piece lets go, and still dying
inside everyone else's ring. `strum` now has 13 distinct values in my part, 3 to
44, on 44 events; in round 1 it had **6** values inside 6–20 ms, on 37 events.

**3. Handoffs — receiving the pitch, not just the beat.**
- **Into D (36→37).** hr's bar-36 `B5` bell was already my first note. Round 2 it
  is not a pair of 16ths, it is a **four-note 32nd shake** `B5 A5 B5 A5` on the
  back half of beat 4 — I take their bell and *trill* it before landing on my G5
  downbeat. Same pitch, more grip, and it sounds like the bell caused the trill.
- **Into 67 (hr → me).** hr releases on `<A5 E6>` at the midpoint with *do not
  resolve it downward*. Round 1 I opened on a B5/A5 trill above their A5. Round 2
  my **first note is literally their E6** — a 32nd grace on their top pitch — and
  the trill under it is now **in octaves**, `<B4 B5>` / `<A4 A5>`, before the
  cascade falls. I take the actual note they let go of.
- **Out of 68 (me → pm).** Unchanged and confirmed: I stop on the "and of 3" and
  beat 4 is empty. hr moved their chord off 4.0 to 3.5 so pm could strike into the
  ring; the door is now held open by two of us.
- **Bar 76**, my one-bar exchange, now lands on `<B4 D5 B5 D6>` — the coupler
  drawn for the arrival, two octaves of the same dyad instead of one.

**4. Section E, and an unasked-for courtesy to el aire.** am wrote that bar 55 is
their law-break — quietest, strangest, `couple` and `orbit` an order of magnitude
above their neighbours — and that "it needs emptiness around it to read as strange
rather than as merely soft." They asked pd for that emptiness. **I gave it too
without being asked: bar 55 is now tacet for me.** My lute-stop ticks moved to
50 · 52 · 54 · 58 — four of them instead of three, irregularly spaced, descending
E5 → B4 → G4, and bar 52 grew into a double mordent with an octave `<E4 E5>`
arrival. A clock heard through a wall, and the clock is not quite regular.

**Retuning.** None. My declaration is byte-identical to round 1.

**Self-measurement** (`stems/hc.wav`, repo venv). To compare honestly I
**re-rendered my round-1 backup with the same engine and measured both with the
same script** — round 1's numbers below are that fresh render, not last round's
quoted figures.

| | R1 | R2 |
|---|---|---|
| per-second RMS std (linear) | 0.02411 | **0.02282** (mean 0.00812, **CoV 2.81** vs 2.87) |
| dB std, active seconds (>1e-5) | 20.37 (n=79) | **19.47** (n=82) |
| dB std, **audible** seconds (>−70 dBFS) | 14.98 (n=70) | **15.28** (n=74) |
| per-note overrides / events | 987 / 190 = 5.19 | **1049 / 201 = 5.22** |
| `s`/`t` events | 128 / 190 = 67% | **136 / 201 = 68%** |
| stem peak | 0.746 | **0.728** |

- per section (dB std) R1 → R2: B 8.28→**9.73** · C 31.46→**36.05** ·
  D 13.49→**13.49** · **E 17.81→3.70** · F 19.90→**20.99** · coda 15.21→**18.27**.
  Five of six up or level; **E is the only fall and I can show you exactly what it
  is.** R1's E was five active seconds reading `−56.1, −78.3, −58.1, −98.3, −62.1`
  dB: three notes and **two reverb-floor tails**, and the tails are the entire
  "contour." R2's E is seven seconds, `−56.7 … −53.7 … −64.0`, **all seven above
  −70 dBFS**, spanning 10.3 dB of real audible range. I did not flatten that
  section; I *filled two silences with music* and cleared a third for am, and a
  statistic that counts −98 dB of decay as dynamic range punished me for it. Which
  is why I am reporting the audible-floor number too, and on that number the whole
  part went **up**.
- the terracing is intact and I want the numbers on the record: **gain 5 distinct
  values, pos 5, pick 5** — five registrations, no shaped curve anywhere — against
  **sus 44 distinct, damp 44, strum 13**. Five stops, forty-four articulations,
  thirteen spreads. That is still the whole argument of my part: gain is nailed
  down by the instrument, so my expression is time.
- 18 dynamic marks · 75 rests · **49 bars tacet of 84** (was 50; bar 55 out, bars
  50 and 54 in).
- balance above 800 Hz, where the tune lives — **where I lead:** D 37–48 **+19.5
  gt, +32.2 pn, +50.6 hr** · 67–68 **+29.1 / +31.3 / +21.0** · bar 76 **+27.0 /
  +34.1 / +23.5**. **Where I support:** B −2.5 vs gt, −20.6 vs pn · E −6.4 gt,
  −13.6 am · 69–70 **−14.3 under pm** · 71–72 **−26.8 under am, −17.9 under gt** ·
  coda −13.7 under gt.
- I verified mechanically that I touched only my own tokens: a masked diff of the
  score against my pre-round-2 backup, with every `hc=` field replaced, shows
  **0 differences** in any header, section, bar line or other player's part; and my
  own bar-sum checker confirms all 84 bars of my track sum to exactly 4.
- gates: `mus check` **clean:true, diagnostics [], exit 0** · `mus render`
  **badTokens [], warnings [], skippedCount 0, exit 0**, 966 events,
  peak −1.00 dBFS.

**One thing I tried and threw away, since we are all showing our work.** I doubled
the whole bar-44 trill in octaves — the 4' coupler drawn mid-trill at the climax of
my solo. Measured, it made D *quieter* (13.49 → 13.36 dB contour, +19.5 → +18.4
over gt): this engine shares a chord's amplitude among its notes, so an octave
doubling buys timbre and costs level. I kept the idea only where it earns its
place — the **arrival** chord, now `<G4 G5 G6>`, and bar 76's `<B4 D5 B5 D6>` —
and left the trill single. Worth knowing before any of you doubles something to
get louder.

**To el pianista (pn), answering your offer — no, and thank you.** You offered to
go tacet in bars 42 and 46 if I wanted more air in D. **Please don't.** Bar 42 is
my only bar of continuous 16ths and it is legible *because* your `<D2 A2>` and
`<F#2 A2>` eighths tell the ear where the beat is while I refuse to; take them away
and my figuration turns into wallpaper. Bar 46 the same. Seven events in twelve
bars at −7.5 dB is not crowding, it is the floor the running voice stands on, and
you already cut every `sus` to 2.2–3.2 so nothing of yours is ringing under me.
**What I did instead is give you back the bar you actually asked me for without
asking:** your bare `B4` at the end of 24. I was sitting on it with a chord
containing that exact pitch. Beat 4 of bar 24 is now silent in my part.

**To la guitarrista (gt), the bar-43 invitation, still open and now concrete.**
Bar 43 is my terraced echo — the same figure as 42, other manual, 11.5 dB down,
instantly. pd took it: `pd` drops 11.4 dB for that one bar and comes back at 44,
so two of us already change the size of the room there. **You would make it three.**
Concretely, for bar 43 only: move `pos` one step toward the soundhole from
whatever 42 sits at (0.24 → ~0.34), drop about 6 dB, and come back out in 44 with
no transition — the drop has to be a cliff, not a fade, or it reads as someone
getting quieter rather than as a different instrument. Six dB is enough; you are
the register the ear tracks in D and if you drop the full 11.5 with us the bar
will sound like a mistake. Entirely your call, and my part does not depend on it.

**To la percusionista (pd · pm · ps), confirming, and one correction of my own.**
Keep bar 43 at **−11.5**, not −14. You asked if I wanted it wider; I don't. Yours
matches mine within a tenth of a dB and that is exactly the point — a terraced
echo is two hands moving to the same manual, not a diminuendo. Widen it and it
stops being registration and starts being expression, which is the one thing my
instrument cannot do. **The correction is mine:** I had `<G4 B4 E5>e` landing on
beat 4 of bar 70 — on the E5 of your `<E5 B5>` summit, the loudest moment of your
solo, in unison. hr found the same mistake in their own part in the same bar. It
is gone; beat 4 of 70 is now silent in my track and I am 14.3 dB under you there.
Your bell solo has the beat to itself from all three of us now.

**To el aire (am), unasked.** Bar 55 is tacet for me. You asked pd for emptiness
around your law-break and it seemed to me that the request was about the bar, not
about pd — so my lute tick moved from 55 to 54 and 55 is yours alone. And the
`strum` walk in 71–72 is your idea on my instrument: I could not borrow `orbit`,
so I borrowed the shape of it and spent it on the only continuous thing I own.


## 2026-08-11 — la percusionista (pd · pm · ps), ROUND 2 (riff)

**What round 2 was for me.** I read the whole score and the whole log first, and it
turned out I had made twice the crime hc confessed to: **I was sitting on beat 4
of two different bars where somebody else's release note lives.** Everything
below follows from that, plus one honest measurement disaster I caused myself and
had to fix twice.

**1. Two thinnings, both the same crime — beat 4.**
- **Bar 24.** pn releases section B on a **bare `B4e`, alone on the last eighth,
  with silence after it**. They said so, hr repeated it, hc found their own chord
  parked on it and moved off. **And there I was with a `D4q` bell on beat 4** —
  my *first metal sound in the piece*, landing exactly on their handoff. It is
  now on **beat 2.5** as a `q`, 0.6 dB softer, so the bell *rings underneath*
  their release instead of striking with it. `pm` in section B: −49.6 dB vs gt.
- **Bar 74.** pn's {ff} bar arrives on `<B2 D5 F#5>q` on beat 4 and I had a `D5`
  16th at 4.25 — **in unison with their arrival pitch, inside their loudest bar.**
  Their bar has a hole at beat 3. My two 16ths now answer **into the hole** (3.0,
  3.25) and **beat 4 is silent in all three of my instruments.** `ps` moved off
  beat 4 of that bar too, onto 3.5.

**2. The handoff: I now take the pitch hc actually lets go of.** hc ends 67–68 by
climbing back to `F#5`, trilling it, and dropping the octave `<F#4 F#5>` on the
"and of 3" — then beat 4 is empty, held open by hc *and* hr (who moved their
chord to 3.5 for me). Round 1 my pickup was `B4 D5`, which took the door but not
the note. **Bar 68's pickup is now `F#5 → D5`:** their exact released pitch, a
third down, and my bar-69 `E5` rises a step out of it. Same door, and now I walk
through it holding what they handed me.

**3. The steal — el aire's field-walk, on struck metal.** am builds their cell by
walking `couple` 0.24→0.45 and `orbit` 0.9→6.0 Hz: "the note does not get louder
so much as it *starts turning*." hc stole the shape and spent it on `strum`. I
did not have to translate it at all — **`pm` and `ps` are weave voices, same
engine as am's halo**, and I had been playing them as if they were plain struck
strings. So my bells now have a lattice, and it **wakes up across the whole
piece**: `couple` 0.03 / `orbit` 0.04 Hz at my first bell in bar 24, creeping up
through the D dialogue (0.05 → 0.15), and blooming in my solo — 69–70 walk
`couple` 0.10 → **0.34**, `orbit` 0.20 → **2.8 Hz**, `orbit_depth` 0.34 → 0.95,
`dimension` 1.9 → 2.4, so the `<E5 B5>` summit dyad is not just the loudest bell
in my part, it is the *most alive* one. And in my last word, bar 76, the final
`G5` flips **`chirality` to −0.7** — the scattering reverses as the piece lets go,
which is am's own final-note gesture, one bar early, on metal. `ps` got the same
treatment: the breath in E now turns as it swells (`orbit` 0.05 → 0.90 → 0.08).
It cost me contour in one place and I report that below.

**4. Deepened where round 1 was flat — and the flatness was a missing law.**
My mallet law had four dimensions (gain, hardness, strike position, ring) and was
missing the one a real drum has most obviously: **a struck head goes sharp and
relaxes.** Every one of my 131 skin events now carries **`tension`** — onset pitch
elevation in cents, walking with strike force, 0 cents on the softest ghosts to
**56** on the hardest rim strokes (35 distinct values). Five accents in F also
carry **`buzz`** (0.04–0.08): snare-contact rattle, on the drive bars only. I also
wrote a **flam** into bar 23 (two 32nd grace taps, `G2t B2t`, into the accented
`B2`) — a drummer's actual ornament, and I had none. Three writing fixes on
measurement: the **brushed entry** in B/C was written across too narrow a range
(ghosts down 2.2 dB, accents up 0.6 — a brush entry should start from nearly
nothing), the **D bell dialogue** was six answers spread over **1.4 dB** of gain,
which is exactly the flat writing hr caught in their own E bells, and it now
**grows across the section, +1.8 → +5.4**; and the **E breath arc** widened from
8 dB to 9.5 with a hole in it.

**5. What I did with the three requests addressed to me.**
- **el aire, bar 55 — done, and further than asked.** You asked me to take the
  `B5e` on beat 3 down to about −5.5 *or* move it to beat 4 of 54. I did both and
  then some: **bar 55 is now completely tacet for `ps`**, and that breath moved to
  **beat 3.5 of bar 54** at **−4.8**, *receding* rather than swelling, so the
  pulse ebbs into your law-break and picks up again in 56. Your quietest, strangest
  bar now has nothing of mine in it but one G2-region skin ghost. Everything else
  in the E arc is where you asked me to leave it — and **measured, my shaker in E
  is at −12.7 dB vs gt where round 1 was −12.5.** I checked that on purpose; see
  the revisions.
- **la clavecinista, bar 43 — confirmed, unchanged at −11.5.** You said don't
  widen it and you were right: it is registration, not expression. `pd` drops
  11.4 dB for that bar alone and comes back at 44. Byte-identical to round 1
  except for the tension params the whole track got.
- **la arpista, bar 75 — thank you, and no.** You offered to end your {ff}
  glissando on beat 3 and leave me beat 4. **Please don't.** That gliss is the
  only full-range gesture in the piece and cutting it to give a drummer a beat is
  a bad trade for the ensemble. `pm` stays tacet for the whole bar. I answer you
  in **76**, where I already have the last word, in hc's silence — that is where
  a percussionist should take a bar, not out of someone else's climax.

**Retuning (my own two declarations).** `pm`: added `orbit=0.05, orbit_depth=0.2,
chirality=0.6` — the resting state of a bell whose lattice is nearly still, so the
walk has a floor to start from. `ps`: added `pos=0.5` (dead centre is a shaker's
home, same law as the skins) plus `orbit=0.04, orbit_depth=0.2`, and **`gain`
−2 → −6.0**, which was a *correction*, not a choice — see below. `pd`'s
declaration is byte-identical to round 1. Verified mechanically: a masked diff of
the score against my pre-round-2 backup with every `pd=`/`pm=`/`ps=` field and my
three declaration lines removed shows **0 differences** in any header, section,
bar line or other player's part, and an independent bar-sum check confirms **all
84 bars of all 8 tracks sum to exactly 4**.

**Self-measurement.** To compare honestly I **re-rendered my round-1 backup with
the same engine and measured both with the same script** — the R1 column below is
that fresh render, not last round's quoted figures (my round-1 script counted
near-silent reverb-tail seconds differently, which is why its headline numbers
were higher; hc hit the same thing).

| | `pd` R1→R2 | `pm` R1→R2 | `ps` R1→R2 |
|---|---|---|---|
| peak | 1.617 → **1.778** | 0.331 → **0.343** | 0.261 → **0.485** |
| per-sec RMS std (lin) | 0.03634 → **0.03721** | 0.01092 → 0.01081 | 0.00577 → **0.01021** |
| CoV | 2.05 → **2.06** | 4.71 → 4.64 | 5.12 → **6.95** |
| dB std, active (>1e-5) | 15.39 → **15.62** (n=111) | 22.28 → 21.79 (n=41) | 11.90 → **14.98** (n=26) |
| dB std, audible (>−70 dBFS) | 12.70 → **12.96** (n=107) | 14.31 → 13.81 (n=28) | 10.28 → **13.08** (n=25) |

- per section, dB std, R1 → R2: **`pd`** B 10.17→**10.66** · C 10.89→10.70 ·
  D 11.81→11.79 · E 14.28→14.29 · F 14.09→**14.12** · coda 21.70→21.66.
  **`pm`** C 11.80→11.66 · D 18.58→**19.06** · F 19.47→**22.13** · coda 26.78→24.56.
  **`ps`** C 0.66→1.03 · E 8.76→**11.04** · F 11.69→**13.54**.
- **`pm`'s aggregate is the one number that fell, and I can show you exactly where
  it went: the coda, where I play nothing.** Both sections I actually play in went
  up (D +0.48, F **+2.66**); the whole 0.49 dB loss is bars 77–84, which for `pm`
  is nothing but the decay of my bar-76 bell. That bell now has a travelling
  coupling field, so energy keeps circulating during the ring instead of falling
  off a cliff — **the tail is flatter because the bell is turning**, which is the
  sound I chose on purpose. I am not going to kill the field to win a statistic on
  a section where I am silent.
- **1246 per-note overrides across 184 sounding events — 6.77 per note** (round 1:
  872 / 186 = 4.69). `pd` 792/131 (**6.05**/note): gain, pick, pos, sus, damp,
  tension on all 131, buzz ×5. `pm` 205/22 (**9.32**/note). `ps` 249/31 (**8.03**).
- distinct values per parameter — **`pd`** gain 44 (range **−15.2..+2.6**),
  tension 35 (**0..56 cents**), damp 36, pick 34, sus 34, pos 29, buzz 5.
  **`pm`** couple 19, orbit 21 (**0.04..2.8 Hz**), orbit_depth 18, sus 17, pick 17,
  gain 16, damp 15, dimension 11, pos 10, chirality 4, curvature 2, strum 3.
  **`ps`** orbit 30, damp 28, gain 27, couple 23, orbit_depth 22, breath 21,
  pos 18, sus 11.
- 40 dynamic marks {pp}..{ff} · 204 rests · **`pd` tacet 36 bars, `pm` 72, `ps` 66**
  · 64 events in `s`/`t` values, including the new flam.
- balance broadband vs gt, R1 → R2: `pd` B −16.7→−16.4 · C −17.9→−18.2 ·
  D −11.6→−11.7 · E −14.7→−14.3 · **F −9.5→−9.2** · coda −33.6→−33.8. Level-matched
  to round 1 within 0.4 dB everywhere — the new writing is shape, not volume.
- **800 Hz–6 kHz, where a bell lives: my solo, bars 69–70, is +10.7 over gt,
  +21.0 over pn, +24.3 over hr, +13.8 over hc, +37.0 over am.** The tune is still
  mine there. (This band is narrower than the one I quoted in round 1, so only
  same-script numbers are comparable.)
- gates: `mus check` **clean:true, diagnostics [], exit 0** · `mus render`
  **badTokens [], warnings [], skippedCount 0, exit 0**, 968 events,
  peak −1.00 dBFS.

**I revised twice, and the first one was my own fault and worth telling.** Adding a
per-note `pos` walk to `ps` — dead centre when soft, toward the rim when loud, the
same law my skins obey — **lifted the entire shaker 9.4 dB** (peak 0.261 → 0.768),
because on a breath voice `pos` is a level control as much as a colour one. el aire
had just written *"it is not in my way; it is the best thing that happens to me in
that section — please do not touch it."* I had made it 9 dB louder while adding
their idea to it. I fixed it the honest way: **kept every shape change, put the
level back**, tuning the instrument gain until section E measured **−12.7 dB vs gt
against round 1's −12.5**. (2) Then I checked the same thing bar by bar in F, where
`ps` reads 5 dB louder broadband — but **in the band where anyone's melody actually
is, it is 3–5 dB *quieter* than round 1 in every one of bars 73–76**, and in hr's
bar 75 specifically I went from −23.6 to **−28.1 dB under them**. The extra
broadband energy is low breath noise, not competition. I checked before I trusted it.

**To el pianista (pn), a request and a clearing — you have not riffed yet, so this
is for your round.** Two beats of yours that I was standing on are now clear from
**all three** of my instruments: **beat 4 of bar 24** (your bare `B4` release —
`pm` moved to 2.5) and **beat 4 of bar 74** (your `<B2 D5 F#5>` arrival — `pm` and
`ps` both moved to beat 3 and 3.5). Nothing of mine attacks on either one. **The
request is about the floor you took.** gt vacated below C3 in F and we *both* moved
in — your low octaves and my skins are living in the same G2–C3 for sixteen bars,
and I am at −9.2 dB vs gt there, the loudest I ever am. I cannot leave that
register; it is the instrument. So: name the bars where my skins are muddying your
bass and I will thin them to ghosts, or move the accent to `D3/E3` and leave you
the bottom two strings alone. **Bars 63–64 and 67–68 are my first guess** — those
are your cell and hc's, and they are the densest my drive gets. Say the bar numbers
and it is done.

---

## 2026-08-11 — el aire (am), ROUND 2 (riff)

**What round 2 was for me.** I read every part and the whole log before touching a
note. Round 1 I built a field; round 2 I found that in two places I was standing on
somebody's ring, in one place I was *not* catching what was handed to me, and in one
whole section — my own coda floor — I had written the flattest thing in the piece and
called it ambience. Six kinds of edit, all of them measured.

**1. The handoff into my cell — I now take pm's actual dyad.** la percusionista ends
their bell solo on an `<E5 B5>` two-mallet dyad at the midpoint, beat 4 of bar 70,
`sus=5`, ringing into my cell with nothing under it — hc and hr both cleared that beat
for them. Round 1 I answered it with `<C3 G3 E4 G4>`: the right harmony, the wrong
octave, and their bell just died unanswered while I started something else two octaves
below. **Bar 71 now opens on `<C3 G3 E5 B5>` — their exact pitches, their exact
interval, my C pedal underneath.** The bell does not stop; it becomes air. Then across
the cell the top voice lets go of the B5 and comes down into my own register
(`<C3 G3 A4 E5>` → `<C3 G3 B3 C5>`) before climbing back to E5 for the `chirality`
flip. Measured in the band where a bell lives (800 Hz–6 kHz), bar **71 alone** went
from **−0.6 dB *under* pm to +2.4 dB *over*** them: in round 1 my cell opened
underneath their decay, now it opens on top of it and carries it. Across 71–72 in that
band I went +8.9 → **+10.0 over gt** and +13.0 → **+14.1 over pm**.

**2. Two thinnings, and hr's question answered by building rather than cutting.**
- **Bar 79.** la arpista asked whether their bar-79 `B5` midpoint bell (`sus=12`) rings
  as clutter through the hole I clear in 80 for pn's whispering ghost, and offered to
  cut it to `sus=6`. **Please don't — and I have moved instead.** Their `B5` is on
  *beat 4 of 79*, and I had `<E2 B2 G3 B3>q` sitting on that exact beat with their
  pitch-class in the bass. **Beat 4 of bar 79 is now a rest**, and bar 80 opens `Rh.`
  instead of `Rh`. My silence now runs from beat 4 of 79 through beat 3 of 80 — four
  beats — and inside it there is hr's bell, then pn's `F#5` ghost, and nothing of mine
  at all. The bell is not in my hole; the bell **is** the hole. Measured across 79–80
  in the melodic band I dropped from −13.4 to **−15.7 under gt**, −0.1 → **−2.3 under
  pn**, −2.3 → **−4.6 under hr**.
- **Bar 70.** My two 32nd breath-particles are the pickup into my cell and they sound
  inside pm's summit ring. They stay (they arrive 0.5 and 0.75 beats *after* the strike
  — ringing in the sustain, never on the attack, which is the courtesy hr asked of us
  all) but they are **1.6 dB quieter**, and the whole bar with them. Across 69–70 in
  the bell band I am now **−37.3 under pm**, the furthest under anyone I am anywhere.

**3. The steal: pd's `tension`, inverted in intent.** la percusionista added onset pitch
elevation to all 131 skin events in round 2 — *a struck head goes sharp and relaxes* —
and it is the one law my instrument was missing. My round-1 declaration says
`tension 5→0`, "no onset bend: air does not snap into pitch," and that was only half
true. A struck head goes sharp because it is *hit*. **A breath goes sharp because it is
pushed** — every wind player sharpens when they lean on the air, and settles when they
back off. So `tension` is now on **all 69 of my events**, bound to `breath` alone and to
nothing else: **0 cents** at my quietest halo tones in F, **18 cents** at the summit of
E and at the ceiling of my cell, 31 distinct values, and it settles over ~1.5 s rather
than snapping. The law is checkable: **corr(breath, tension) = +0.966**, and **+0.999
with bar 84 excluded** — because bar 84 is a deliberate exception. That final chord was
already my law-break (loudest, longest `sus=20`, most open `damp=0.03`, stillest
`couple=0.04`); it now also breaks the **new** law, the only pushed note in my part with
`tension=0`. The air stops obeying itself at the end, in one more dimension than before.

**4. The coda — where round 1 was genuinely flat, and I can say exactly why.** My coda
measured **6.11 dB** of per-second contour, the flattest section in my part, in the one
place where I am continuously sounding and everyone else is intermittent. The cause was
the same mistake I diagnosed in section E in round 1 and then failed to carry into the
coda: **four whole-note pillars with `sus=6..9` overlapping each other is one wash, not
eight bars of music.** A bowed halo has an obvious fix that a struck instrument does not
— **change bow**. Bars 77, 81 and 82 are no longer `w`; they are two events on the same
harmony at different pressures, so the note *swells inside the bar* (77: −3.6 → −1.2;
81: +1.0 → −1.6, arriving with gt's peak and getting out of it; 82: a breath rest on the
"and of 3"). Mid-coda `sus` came down from 6–9 to 4–6, long ring reserved for 83 and 84.
**Coda contour 6.11 → 8.69 dB**, and at the same time I went *further under the guitar
who has the theme*: **−6.6 → −8.6 dB broadband, −12.8 → −15.1 in the melodic band.**
More shape and less level, which is the trade I want every time.

**5. Two new breath-holes where I had written continuous halo.** Bar **30** and bar
**33** in section C each gained an eighth of silence mid-phrase — 33's lands immediately
before my bar-34 law-break (the *loud* hollow at `pos=0.50, pick=0`), so the strangest
sound in my C section is now preceded by an audible inhale. And **bar 54 beat 4 is now a
rest.** la clavecinista went tacet in bar 55 for me unasked, and la percusionista took
their shaker out of 55 entirely and moved it back to beat 3.5 of 54, *receding*. Both of
them made emptiness around my law-break and **I had not made any myself** — my own
part played straight through beat 4 of 54 into it. Now the ebb is mine too: my 54 falls
away, beat 4 is silent, bar 55 starts on beat 2, and the quietest strangest bar in my
piece has two beats of nothing in front of it from three instruments.

**6. Section E, widened at both ends after measuring.** Pass 1 of round 2 bought the
bar-54 hole and the bar-57 re-bow at the cost of 0.18 dB of E contour and 0.7 dB of lead
margin. Pass 2 paid for them properly: the opening pillar went −8.0 → **−8.6**, the
bar-55 whisper −8.6 → **−9.4**, and the summit +4.4 → **+5.2**, with `breath`, `bow`,
`bow_speed`, `pos`, `pick`, `couple`, `orbit` and the new `tension` all moved in lockstep
so the air law holds note by note. **Bar 57 is no longer a whole-note pillar** — it is a
bow change at beat 4, so 57–58 is now a four-step climb (0.0 → +2.6 → +3.4 → +5.2)
instead of a plateau and then a jump. E's written range went 13.0 → **14.6 dB** and its
measured contour **8.99 → 9.74 dB**.

**Retuning.** **None.** My instrument declaration is byte-identical to round 1 —
verified mechanically (see below), along with everything else in the file.

**Self-measurement** (`stems/am.wav`, repo venv). To compare honestly I **re-rendered my
round-1 backup with the same engine and measured both with the same script** — the R1
column is that fresh render, not last round's quoted figures.

| | R1 | R2 |
|---|---|---|
| per-second RMS std (linear) | 0.01314 | **0.01422** (mean 0.00658, **CoV 2.162** vs 1.981) |
| dB std, active seconds (>1e-5) | 13.34 (n=134) | **13.33** (n=134) |
| dB std, audible seconds (>−70 dBFS) | 10.26 (n=126) | **10.44** (n=126) |
| per-note overrides / events | 812 / 67 = 12.12 | **905 / 69 = 13.12** |
| stem peak | 0.2795 | **0.3007** |

- per section (dB std, active) R1 → R2: C 8.09→**8.10** · D 10.23→9.11 ·
  **E 8.99→9.74** · F 17.60→**17.68** · **coda 6.11→8.69**. (A silent; B is the single
  bar-24 inhale, unchanged at 1.15.)
- **D is the one number that fell and it is not music.** I play exactly two bars of D
  (47–48, the door into my own section) and I am tacet for the other ten. Every "active"
  second the statistic finds in the D window is the *decay tail of my bar-36 whole note*
  bleeding across the section line: R1 `−55 −62 −68 −73 −78` then twenty-three seconds
  of digital silence; R2 `−55 −58 −65 −71 −76`. I lowered bar 36 by 0.6 dB to deepen
  C's floor, the tail crosses the measurement threshold half a second differently, and
  the "contour" of a reverb slope changed by 1.1 dB. There is no phrase there to flatten.
- overrides: **thirteen parameters on all 69 events** — gain, breath, bow, bow_speed,
  pos, pick, sus, damp, couple, orbit, orbit_depth, strum and now **tension** — plus
  dimension ×4, curvature ×2, chirality ×1, courses ×1. Still the fewest notes in the
  ensemble and the most parameters per note, which is the right shape for a field.
- ranges: gain **−9.5..+5.2** (46 distinct) · breath 0.48..1.00 (39) · bow 0.02..0.58
  (37) · bow_speed 0.12..0.62 (35) · pos 0.28..0.50 (22) · pick 0.00..0.20 (26) ·
  sus 1.2..20.0 (17) · damp 0.03..0.34 (22) · **couple 0.025..0.450 (34)** ·
  **orbit 0.025..6.0 Hz (39)** · orbit_depth 0.10..1.00 (34) · strum 0..52 ms (18) ·
  **tension 0..18 cents (31)**
- 20 dynamic marks {pp}..{mf} · **25 rests** (was 20) · **39 bars tacet of 84**
- **the air law, over all 69 events:** corr(gain, breath) **+0.952** · bow **+0.913** ·
  bow_speed **+0.924** · pos **−0.853** · pick **+0.820** · couple **+0.634** ·
  orbit **+0.436** · **tension +0.877** · and sus **+0.001** — still deliberately
  decoupled, because `sus` is structure, not dynamics. That was round 1's hardest lesson
  and it is why my coda got fixed this round.
- balance, melodic band (250–2500 Hz) — **where I lead:** E 49–60 **+1.5 gt, +3.4 pn,
  +1.4 hr, +17.1 hc** · summit 57–58 **+16.9 / +13.2 / +19.8** · cell 71–72 **+30.5 pn,
  +22.3 hr, +26.5 hc, +19.9 pm** and **+10.0 over gt above 800 Hz**. **Where I support:**
  C 25–36 **−23.2 under hr** (−17.2 broadband vs gt) · 61–62 **−46.4 under gt** ·
  63–64 −45.3 under pn · 65–66 −38.8 under hr · 69–70 **−37.3 under pm** in the bell
  band · coda **−15.1 under gt** · 79–80 **−15.7 gt, −4.6 hr, −2.3 pn**.
  My E aggregate margin is 0.7 dB narrower than round 1 and I am reporting that plainly:
  I dug two new holes in my own lead section and paid for them in average level, not in
  the summit, which held (+16.8 → +16.9 over gt). Shape over loudness.
- **final half second: am −57.9 · pn −59.0 · hr −60.8 · gt −80.5 · hc, pd, pm, ps
  silent.** I still fade last, by the same 1.1 dB, and by `damp` rather than by gain.
- I verified mechanically that I touched only my own tokens: a masked diff of the score
  against my pre-round-2 backup, with the `am=` field of every bar replaced by a
  placeholder, shows **0 differences** in any header, instrument declaration, section,
  bar line or other player's part; and an independent bar-sum check confirms **all 84
  bars of all 8 tracks sum to exactly 4**.
- gates: `mus check` **clean:true, diagnostics [], exit 0** · `mus render`
  **badTokens [], warnings [], skippedCount 0, exit 0**, 970 events, peak −1.00 dBFS.

**To la percusionista (pd · pm · ps), the answer to a handoff I fumbled, and a request.**
Round 1 I let your `<E5 B5>` die. Bar 71 now begins on it — your two pitches, your fifth,
caught and held for three beats while the lattice wakes underneath, and I am +2.4 dB over
your decaying bell in the bell band where round 1 I was 0.6 under it. Your solo does not
end at the barline any more; it turns into weather. **The request is one number.** My two
32nd breath-particles at the very end of bar 70 are my pickup and they sound inside your
summit's ring at −8.6 and −6.6. If they buzz against the bell rather than blur into it,
say so and I will drop them another 2 dB or take the first one out entirely — I would
rather lose half my only fast gesture in the piece than fur the edge of your loudest
note. And thank you for bar 55: your shaker receding into it from beat 3.5 of 54 is what
made me realise I had not cleared my *own* beat 4.

**To la arpista (hr), answering directly — keep `sus=12`.** Your bar-79 `B5` is not
clutter in my hole; it is the reason the hole is worth having. I moved instead: beat 4
of 79 was mine and now it is silence, so your bell strikes into four full beats of my
nothing and rings alone through pn's ghost. Don't shorten it — if anything I want it
*longer*. And thank you for 49, 51, 53 and 59: there is now no moment in section E where
the two of us attack together, and the 1.4 dB that separates us there stopped sounding
like a contest the moment the edges came apart.

**To la clavecinista (hc), acknowledging the unasked courtesy, and one back.** You went
tacet in bar 55 without being asked, on the reasoning that the request was about the bar
and not about pd. That was exactly right and it is the most generous read anyone gave
anyone this round. Your `strum` walk in 71–72 — my `orbit` walk spent on the only
continuous control a quill has — is the best thing that happens under my cell, and I have
kept my own two lowest voices (`C3 G3`) exactly where they were so your dry lute ghost
has the midrange to itself. **The one back:** you are 26.5 dB under me in 71–72 and I
think that is about 3 dB too polite. If you want to draw the back 8' for those two bars
instead of the lute, I have moved my whole top voice up to E5/B5 and the register from
C4 to C5 is empty for you.

**To la guitarrista (gt) and el pianista (pn), who have not riffed yet — two things
waiting for you.** gt: in the coda I am now **15.1 dB under you in the melodic band**
(round 1: 12.8) with the whole floor rebuilt as swells rather than pillars, and bar 81 —
your peak — is the one bar where I arrive *with* you on the downbeat and then fall 2.6 dB
inside the bar to get out of your way. pn: beat 4 of bar 79 and beats 1–3 of bar 80 are
silent in my part, hr is tacet in 80, and I am −2.3 dB under you across the pair. Your
pianissimo-with-glass ghost has the emptiest two bars in the piece to be strange in. If
either of you wants more room anywhere else, name the bar — I have 39 tacet bars already
and I am always happy to have forty.

---

## 2026-08-12 — la guitarrista (gt), ROUND 2 (riff)

**What round 2 was for me.** I read every part and the whole log before touching a
note. Two things came back at me. The first: I was standing on **beat 4 of two
different bars** — the same crime hc and pd both confessed to, in two of the same
bars, and nobody had caught mine because the guitar is the register the ear tracks
and it reads as "the floor" rather than as "someone in the way." The second was
mine alone and worse: **2.94 per-note overrides, the lowest density in the
ensemble by a factor of two to four.** pn had a hammer law, pd a mallet law, am an
air law, hc a whole argument about terracing. I had `gain` and `pos` and a
scattering of `pick`. I had shaped my part but I had not *articulated* it.

**1. Four thinnings, all measured, all beat 4.**
- **Bar 70.** pm's bell solo climbs to an `<E5 B5>` two-mallet dyad on beat 4.
  hr moved off it. hc moved off it. **I was still sitting on it** with an
  `<E3 B3>q`. My last stab is now on **beat 3.5** as an `e` and beat 4 is silent
  in my track. Across 69–70 I am **−14.7 dB under pm** in the bell band.
- **Bar 74.** pn's {ff} arrival is `<B2 D5 F#5>q` on beat 4 and I had a stab
  there. pd and ps both found the same thing and moved to pn's beat-3 hole; so
  have I. **Beat 4 of 74 is now silent in gt, pd, pm and ps.** −8.7 dB under pn.
- **Bar 24.** pn releases section B on a bare `B4e`. hc's chord contained that
  pitch and moved; pd's bell was on the beat and moved. Mine was subtler and I
  nearly missed it: my closing Em was an `h.` **containing B3**, the octave below
  their release, ringing at `sus=5.0` straight through it. The B3 is gone, the
  chord is an `h` ending on beat 3, `sus` cut 5.0 → 4.0.
- **Bar 23.** My `<D4 F#4>q` sat on beat 3 in pn's own melodic octave, on the
  exact beat hc had just moved their run onto. Dropped an octave to `<D3 F#3 A3>`.

**2. Two places the numbers caught me being level with, or over, the leader.**
- **Bars 71–72, am's cell — I measured +7.1 dB OVER them** in 250–2500 Hz. am
  never asked; they had politely written that broadband they sit "−7.7 under gt,
  whose sustained C chord is all bass." It was not all bass. My C/G floor is now
  3.5 dB down with `sus` cut 2.4 → 2.0, and I am **+1.9**, which I still think is
  1 or 2 too many — see the offer below.
- **Section E, am's lead — I was 0.6 dB under them.** That is not a floor, that
  is a duet. Every one of my seven E events came down; **−2.1 dB** now.
  Bar **54** went further: it was a whole note in unison with their bass, ringing
  through the beat-4 rest they dug for their bar-55 law-break. hc went tacet in 55
  for them unasked, pd's shaker receded into it, am cleared their own beat 4.
  **I was the one instrument still sounding into it.** Bar 54 now enters a beat
  late, stops on beat 4, and is 3.5 dB quieter: **−33.4 dB.**

**3. Handoffs — receiving the pitch, not just the downbeat.** Section E ends in
am's hands and F opens in mine, and I had never looked at what they actually let
go of. **am releases on `<F#4 B4>e`, the last eighth of bar 60, rising.** Round 1
I answered it by starting bar 61 on `<E3 E4>` — right harmony, wrong note, their
B4 abandoned. Now: **bar 60's last half-beat is a rest** (I get out of the way of
the very note I am about to catch, which is the courtesy hr asked of all of us),
and **bar 61 opens on a 32nd grace on their B4** before the head climbs
E–F#–G and lands on that same B4 a beat later. Their released pitch is my first
sound, my target, and the top of my phrase. The other seams stand: I still release
bar 12 on `B3e F#4e` unresolved (pn answers on G4 above it — bar 12 is now `{pp}`
and the F#4 is played at `pos=0.18` toward the bridge, so the question is
audibly *tense*), and bar 62 still hangs on a bare `B4e` with an eighth of silence
for pn to walk into.

**4. hc's bar-43 invitation — taken, at exactly the size they specified.**
la clavecinista's terraced echo is the same figure as 42 on the other manual,
11.5 dB down, instantly; pd terraces with them at −11.4. hc told me to drop
**about six dB and not the full eleven** — "you are the register the ear tracks in
D and if you drop the full 11.5 with us the bar will sound like a mistake." Bar 43
is now the same rhythm, the same voicing, hand moved from `pos=0.24` out to
**0.34**, nail softened to `pick=0.24–0.26`, and it measures **−24.50 dB against
bar 42's −18.72 — 5.8 dB, a cliff, back out with no transition in 44.** Three of
us change manuals in that bar now. Listen for the room changing size.

**5. The steal: pd's `tension`.** la percusionista added onset pitch elevation to
all 131 skin events — *a struck head goes sharp and relaxes* — and am then took it
and rebound it to breath. It is the one law my instrument was obviously missing
and I had `tension=3` sitting inert in my declaration the whole time. **A plucked
nylon string goes sharp when you displace it far and settles as the excursion
decays** — dig into a rest-stroke and the attack is measurably above the pitch.
`tension` is now on **all 227 of my events**, bound to `gain` alone: **0 cents**
at my quietest let-rings, **16.2** at the {ff} of bar 73, 47 distinct values.
**corr(gain, tension) = +0.933.** Deliberately smaller than pd's 0–56: a nylon
string bends less than a drumhead, and if you can hear it as pitch I have
overdone it.

Alongside it I gave myself the law I should have had in round 1. **The hand law:
the nail hardens as the hand walks toward the bridge and as I dig in.** `pick` is
now on all 227 events, derived note by note from `pos` and `gain`:
**corr(pick, pos) = −0.854, corr(pick, gain) = +0.766.** And **`strum` is the
guitar's own dialect of the thing everyone else declared**: hr's hands are 18 ms
apart, pn's 3, a drummer's 0, air's 24 — a guitarist has *all of them in one
hand*. My muted F-section stabs are pinches at **2–4 ms**, my comping chords
**5–14**, my let-ring coda chords **20–28**, and bar 84 is a **32 ms** thumb-roll
across six strings. 17 distinct values, tied to `sus`: a damped chord is a pinch,
a ringing chord is a roll.

**6. Deepened where round 1 was flat, and it was flat in four specific places.**
- **Section A — my own lead, and my flattest section.** Twelve bars of continuous
  solo guitar with almost no ornament: 10 `s`/`t` events in the whole part. Now
  there is an upper-neighbour turn closing the held `B4` of bar 1, a 32nd grace
  and **a 32nd of silence** before the `D5` peak of bar 3, a grace lift into the
  bar-5 summit, a real **mordent** (`C5–B4–C5`) in bar 6, and a turn in the
  bar-10 echo — plus breathing rests in 5, 7, 9 where there were none, and `{mf}`
  at the peak and `{mp}` at the let-ring. **A: 6.73 → 7.43 dB, and 10 → 23
  `s`/`t` events across the part.**
- **Section B.** I wrote in round 1 that my comp went "{pp} rising to {mp}." It
  did not: it spanned **2.3 dB**. It now spans **4.8** — pn's bar-13 `G4` arrives
  on genuinely bare air at −3.8 and I only reach my old level at 21–23, which is
  what a twelve-bar crescendo actually is.
- **Section C.** Seven sustained tones over twelve bars spanning **1.0 dB** — the
  identical flat writing hr caught in their E bells, pd in their D dialogue and am
  in their coda, and I had it worst. My floor now swells with hr's climb to 33–34
  and recedes for her handoff to hc: **3.2 dB and two new dynamic marks.**
  **C: 22.02 → 23.66 dB**, still 33.5 dB under her.
- **The coda.** Round 1 it was a restatement with softer colours; the notes were
  bar 1 through bar 8 again with the `pos` moved out. It is now *played*: a `sus`
  walk on every phrase so notes accumulate into each other (**campanelas** — the
  let-ring is the point), the descent of 78 and 82 shaped 1.8 → 2.4 → 6.5 s of
  ring, `{mf}` at bar 81 where I now peak by **+9.1 dB over round 1**, and a
  **rest on beat 2.5 of bar 80** — the exact half-beat where pn's
  pianissimo-with-glass ghost lands, with am and hr both already out. **coda:
  8.68 → 9.61 dB.**

**Retuning (my own declaration only).** Added **`strum=5`**. Nothing else. Every
chord I write now says its own spread and the default documents the intent: a
guitarist's chord always spreads a little, but less than a harpist's and more
than a pianist's.

**Self-measurement** (`stems/gt.wav`, repo venv). To compare honestly I
**re-rendered my round-1 backup with the same engine and measured both with the
same script** — the R1 column is that fresh render, not last round's quoted
figures.

| | R1 | R2 |
|---|---|---|
| per-second RMS std (linear) | 0.10525 | 0.09792 (mean 0.08940, CoV **1.095** vs 1.159) |
| dB std, active seconds (>1e-5) | 19.61 (n=221) | **19.87** (n=219) |
| dB std, **identical seconds**, floor −100 dBFS | 19.61 | **20.83 (+1.22)** |
| dB std, identical seconds, floor −90 / −80 | 19.46 / 18.85 | **20.29 / 19.33** |
| per-note overrides / events | 640 / 218 = 2.94 | **1364 / 227 = 6.01** |
| `s`/`t` events | 10 | **23** |
| stem peak | 2.764 | **2.267** |

- per section (dB std) R1 → R2: **A 6.73→7.43** · B 7.82→**7.93** ·
  **C 22.02→23.66** · D 9.68→9.64 · **E 20.58→21.03** · F 8.49→**8.03** ·
  **coda 8.68→9.61.** Six of seven up or level. **F is the only real fall and it
  is precisely the four bars I cleared** — 69, 70, 74, 75. I am not putting them
  back to win 0.46 dB.
- **the linear std and the audible-second std both went slightly down and I want
  to be plain about why: my peak came down 1.3 dB** (2.764 → 2.267). Those two
  statistics are dominated by the loudest seconds, and I spent this round getting
  off the top of pm's summit, pn's arrival and hr's glissando. Less level and more
  shape is the trade I wanted; the dB contour and every section but F say I got it.
- **and one measurement forensic, since we are all showing our work.** My raw
  "active seconds" number barely moved at first, and the reason turned out to be
  that **making real silence removes seconds from the sample rather than counting
  them as deep troughs**: two seconds dropped below the 1e-5 threshold this round
  (they were at −82.9 and −44.8 dB). Measured over the *identical* 221 seconds
  with a sane floor, R2 is **+1.22 dB** wider than R1. hc and am both hit
  variants of this; the fix is to compare on a fixed sample, not on each render's
  own active set.
- overrides: **gain 227/227 · pos 227/227 · pick 227/227 · tension 227/227** ·
  sus 187 · damp 176 · strum 93. gain **−8.4..+3.6 (66 distinct)** ·
  pos 0.08..0.46 (30) · pick 0.18..0.75 (49) · tension **0..16.2 cents (47)** ·
  sus 1.2..9.0 (27) · damp 0.12..0.46 (24) · strum **2..32 ms (17)**
- 25 dynamic marks {pp}..{ff} · 137 rests · 12 bars tacet
- balance — **where I lead:** A 1–12 **+25.8 over pn, +16.2 over hr**, everyone
  else tacet · cell 61–62 **+20.0 pn, +21.8 hr, +47.3 am** · bar 73 **+14.1 pn,
  +34.5 pm, +28.3 pd** · coda **+24.9 pn, +17.8 hr, +39.5 hc, +22.6 am**.
  **Where I support:** B −2.6 vs pn · C **−33.5 vs hr** · D −17.4 vs hc (800 Hz–6k) ·
  E −2.1 vs am · 63–64 −4.1 pn · 65–66 −5.4 hr · 67–68 −2.3 hc ·
  **69–70 −14.7 under pm** · 71–72 +1.9 am · 74 −8.7 pn · 75 −1.4 hr · 76 −4.1 hc.
- I verified mechanically that I touched only my own tokens: a masked diff of the
  score against my pre-round-2 backup, with every `gt=` field and my declaration
  line replaced, shows **0 differences** in any header, section, bar line, other
  instrument declaration or other player's part; and an independent bar-sum check
  confirms **all 84 bars of all 8 tracks sum to exactly 4**.
- gates: `mus check` **clean:true, diagnostics [], exit 0** · `mus render`
  **badTokens [], warnings [], skippedCount 0, exit 0**, 979 events, gt 227,
  peak −1.00 dBFS.

**To el pianista (pn), who has not riffed yet — four beats cleared, and one
request.** Everything of mine that was standing on you is gone: **beat 4 of bar
24** (your bare `B4` release — my closing Em no longer contains B3 and stops on
beat 3), **beat 3 of bar 23** (my chord was in your melodic octave; it is an
octave down now), **beat 4 of bar 74** (your `<B2 D5 F#5>` arrival — I answer into
your beat-3 hole instead, as pd and ps both did), and **beat 2.5 of bar 80** (your
pianissimo-with-glass ghost — that half-beat is now a rest in my part, and am and
hr are both out; you have it completely alone). **The request is about bar 62.**
I release my F-section cell the way I released bar 12 to you: a bare `B4e` on the
"and of 3" with an eighth of pure silence after it and nothing underneath. You
currently answer it by leaping up a fourth to `E5` on the downbeat of 63. **Please
keep that leap and do not fill the eighth** — the silence is the handoff, and the
fourth upward is the same gesture you gave hr when you left them your B4 in bar
64. And one thing you should know before you riff: **your bar-13 entrance now
arrives on much barer air than it did.** My comp under you started at −0.6 dB and
now starts at −3.8. Lean into it.

**To la clavecinista (hc), confirming with the number you asked for.** Bar 43 is
**5.8 dB below my bar 42**, same figure, same voicing, hand out to `pos=0.34`,
nail softened — a cliff, back at full in 44 with no transition. You said six and
not eleven-and-a-half and you were right: I tried the full drop first and it
sounded like a mistake, exactly as you predicted. Also — I have not touched my D
comp otherwise, because you told me it is what makes your figuration legible, and
that is the one compliment in this log I have been careful not to spend.

**To el aire (am), an apology and an offer.** In your own cell, 71–72 — the
strangest two bars in the piece — **I was +7.1 dB over you** in the band where a
melody lives, and you were too polite to say so; you only noted that broadband you
sat under "gt's sustained C chord, which is all bass." It was not all bass. I am
at **+1.9** now, 3.5 dB down with the ring cut, and **I think that is still 2 dB
too many.** Say the word and my whole 71–72 goes to a single pinched `<C3 G3>` on
the downbeat of 71 and nothing else for two bars — you would have the lattice
turning over bare air and hc's dry lute ghost, which is what those bars want.
Separately: thank you for bar 81. You arrive with me on the downbeat and fall
2.6 dB inside the bar, and it is the reason my coda peak reads as an arrival
rather than as a crescendo.

**To la percusionista (pd · pm · ps), one thing back.** Your `tension` is now on
every note I play — see the steal. And the floor in F is still yours and pn's: I
came down another 1.6 dB across 69–70 so your bell solo has the whole bar, from
all four of us now. **One correction to my own round-1 claim, because I checked it
mechanically instead of trusting it:** I said I had vacated everything below C3
for bars 63–76. I had not, quite — **bar 73 has one `A2`**, the bass of the
`<A2 E3 A3 C4>` that opens my {ff} shout. It is the single sub-C3 event in my
whole F section and it is in the one bar where I have the voice. If it muddies
your `A2` accent in 73 — you are at +1.5 there with `buzz=0.08`, the hardest
stroke in your part — say so and I will voice it `<E3 A3 C4>` and leave you the
bottom outright.

## 2026-08-12 — la directora (synthesis pass, closing the workshop)

I read all twelve entries and rendered the full score with stems before
touching anything. My entire pass is three token edits, and every one of
them is an edit one of you proposed and priced yourselves. The rest of my
job turned out to be measurement and gratitude.

**1. gt's offer in am's cell — taken, at exactly her wording.** The stems
put gt at **+3.9 dB over el aire** in the melodic band across 71–72 —
worse than her own +1.9 estimate, and she had already called +1.9 "2 too
many." Bars 71–72 are now her single pinched `<C3 G3>` on the downbeat of
71 and nothing else: the lattice turns over bare air and hc's dry lute
ghost, which is what those bars wanted. Her words, her notes, my
signature.

**2. gt's bar-73 re-voicing — taken.** Her lone `A2` measured **+16.0 dB
over pd's hardest stroke** in the band where a drum accent lives. The
shout is now `<E3 A3 C4>`; la percusionista owns the bottom of bar 73
outright, `buzz=0.08` and all.

**3. The arc — measured, and left exactly alone.** Section mix levels:
A −18.3 · B −16.3 · C −18.7 · D −15.9 · **E −28.5** · **F −11.5** ·
coda −18.5 dBFS. The cliff at bar 48→49 is **19.3 dB** and it is earned:
five players tended it individually — hc stopping on beat 2, pm going
silent for twelve bars so bar 69 could be an arrival, the register
clearing one instrument at a time. And the shape closes: the coda lands
within **0.2 dB of the opening** — the piece departs and comes home at
the same volume, which none of you coordinated and all of you built.

**What I did not do.** I did not rebalance a single fader, touch the
seams (they receive the released pitch in every handoff now — I checked
all six), adjust the master, or resolve hr's bar-79 bell question,
because am already resolved it better than a director could: "the bell
is not in my hole; the bell **is** the hole."

**For the record, to all six of you.** The charter asked for note-to-note
variation; you delivered laws — a hammer law, a mallet law, an air law,
tension bound to force with correlation receipts, and negative results
published so colleagues would not repeat them. Twice tonight one of you
measured yourself over a bandmate and moved without being asked. The
piece starts baroque and goes somewhere strange, and the reason the
strangeness works is that every one of you protected everyone else's
silences. Gates on the final state: check clean, render clean, 84 bars,
every track sums to 4. The score is finished.
