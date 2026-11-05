# 1812 Overture, for guts

Tchaikovsky's *The Year 1812*, Op. 49 (1880), played by seven guts: six voices
(`v1` high to `v6` low) and a cannon. Each part is its own physical World
(the `TractNdAnatomy::fart_v1` family: a self-oscillating valve, a canal of
28 sections whose 24 modes follow the cheeks, a reservoir), with its own
ledger. `score.json` names each voice's anatomy: guts I-II `soprano` (1 g
valve, 20 cm canal), III-IV `alto` (`fart_v1` itself: 1.3 g, 20 cm), V-VI
`bass` (1.6 g, 28 cm), the cannon `cannon` (2.2 g, 40 cm, a 2.5 J reservoir).
`fart_v1` replaced `fart_v0` after the first run: its valve is heavier with
a softer contact, so a held note's pulses are wider (`fart_v0` clicked once
per cycle); the parts were retuned on it in a second pass (below). The
numbers behind the choice are in `TractNdAnatomy::fart_v1`'s documentation.

`score.json` also carries a `direction`: the house settings every part starts
with (wobble 0.5, vibrato 6 Hz / 0.25 st, jitter 1 st, rasp 0.6), applied as
if they were the part's first tokens; a part that sets its own wins. They are
the director's, not the composers'.

Source: the OpenScore edition (CC0), transcription by Mike Nelson,
<https://musescore.com/openscore/scores/4145221>. `source/` and the baseline
parts are written from its MusicXML by
`../../../crates/ovillo-fart/tools/musicxml_parts.py`; `source/` and `out/`
are not committed (they are reproducible).

## For composers

You are a composer **and** the player. Nothing is tuned or fed for you: you
command the valve's tension, push, cheeks and tone, you decide what the gut
eats and when it digests, and the World tells you what happened.

### Your unit

You own a few consecutive sections. Edit only the parts in your sections:
`<section>/v1.notes` ... `v6.notes` and `cannon.notes`. Do not touch
`score.json`, other sections, `source/`, or any tool. Keep a diary at
`diaries/<your unit>.md` (below).

### What you have

- `source/<section>/<part>.notes`: every orchestral part's top line in your
  section, at sounding pitch (`r` rests, `X` unpitched/cannon). Reference
  only. `source/sections.json` lists, per section, each part's note count,
  range and chords (chords were reduced to their top note).
- Your parts as the baseline left them: the six busiest parts of the
  section, moved by octaves into 36-300 Hz, tensions naive, nothing eaten.

### The part format

Tokens in time order, separated by spaces; `|` bar lines (checked: each
must fall on a whole bar); `#` starts a comment when it starts a token (C#3
is a note).

```
G2@84:1      note: target G2 (what it should sound as), valve tension commanded at 84 Hz, 1 beat
G2:1         tension = the target's own frequency (the valve lands sharp of it)
Bb2@101:1/3  beats as fractions or sums (1+1/2); trailing ~ ties to the same note next (one attack)
r:2          rest (no push)            X:1  cannon shot (cheeks clench, pressure builds, release)
push=0.7  cheeks=0.4  tone=0.75        supply 0..1, cheek aperture 0..1, valve adduction 0..1
wobble=0.3  vibrato=5.5/0.4            slow tension wander 0..1; vibrato rate Hz / depth semitones
jitter=1.5  rasp=0.6                   tension jittered +-semitones every 10 ms; a partial clench near the exit
                                       while sounding (the jet there sputters), 0..1
tongue=on|off                          on: clench 50 ms between consecutive notes (each note starts on a release pop); off: slur
eat=beans  eat=beans*3  eat=soda,egg   into the stomach
digest=on|off                          the stomach pays the reservoir while on
pre=4                                  seconds of silence before beat 1 (trimmed): eat and digest there
```

Settings hold until changed. Every part starts on a fresh gut: reservoir
full (1.5 J; the cannon's 2.5 J), stomach empty, digestion off, push 0.7,
cheeks 0.4, tone 0.75, tongued, then the score's `direction` (above). Beat
counts per part must equal the section's.

Foods (gas J / seconds to digest; invented numbers): beans 0.60/8,
broccoli 0.35/5, cabbage 0.40/6, onion 0.30/4, egg 0.20/3, soda 0.25/1,
cheese 0.15/5, salad 0.03/2. Each food pays gas/seconds watts while
digesting; all digestion together is capped at 1 W, and nothing is paid
while the reservoir is full. A part that runs dry goes silent: the report
says `gas` at every note and `notesStartedDry`.

### Tools (all from the repository's `ovillo/` directory)

```
F=./target/release/ovillo-fart ; D=scores/fart/1812
$F check   $D --section S                 free: parse errors by line, beats, bars, tension range, feeding
$F render  $D --section S --no-mix --jobs 2   renders your parts (cached when unchanged); per part: notes within
                                          50 cents of target (by the World's measured F0), silent notes, notes
                                          started dry, gas at the end, rms, sharpness, centroid, attack pops,
                                          clipping, and the worst notes by line
$F analyze $D --section S --voice V       the full per-note table of a rendered part
$F render  $D --section S --jobs 2        render and mix the section (out/sections/S.wav)
$F analyze $D --section S                 the section mix's timbre per half second (sharpest first)
$F probe --push P --cheeks C --tone T     the instrument's answer: tension -> measured pitch, rms, sharpness, brightness
```

What the numbers mean: **cents** are measured F0 against the target; the
valve runs sharp of its commanded tension by an amount that depends on push,
cheeks and tone (probe it). **sharpness** is how impulsive the valve's
pulses are (the 99.9th percentile sample-to-sample step over its RMS): a
sine is 1.4, held notes measured 6.8 to 10.3, single-sample clicks at 85 Hz
would be about 17. **popDb** is
an attack's peak over the note's level: tongued notes start on a release
burst. **centroid** is brightness. **clipped** samples are the listener
clamping at full scale.

Measured in this session on a held note commanded at 85 Hz (probe to confirm
at your own settings): at push 0.7, sharpness 6.8 at tone 0.2 with cheeks
0.15, 9.4 at tone 0.75 with cheeks 0.4, 9.6-9.9 with cheeks 0.8; wider
cheeks are brighter (centroid 260 to 700 Hz) and quieter; at push 0.4 a
tone of 0.2 does not swing at all.
At push 0.7, cheeks 0.4, tone 0.4 the valve lands about 100 cents sharp of its
tension around 80-110 Hz and less above 150 Hz; at tone 0.75 about 240.
An earlier baseline's tongued fast notes clipped (354 samples in one viola
part at m240); the same part slurred did not clip.

### What good is

The 1812 recognizable: the tune where the ear expects it, a bass that is a
bass, inner voices in register order, the cannon where Tchaikovsky wrote it.
Then, measured: at least 85% of sounding notes within 50 cents, no notes
started dry, no silent notes you meant to sound, no clipping, a section mix
whose sharpness is not a click train. Thin a run the valve cannot tune
rather than leave it smeared. Keep every part's beats.

### How to work (compute is not free)

1. Read your sections' `source/sections.json` entries, the source lines you
   need, and your parts.
2. Probe once or twice for the settings you mean to play with.
3. Edit; `check` until ok (free).
4. `render --no-mix` your section; read the report; fix the worst. At most
   four renders per section; `analyze` a part only when the summary is not
   enough.
5. Render with the mix and `analyze` the section once at the end.

### The diary

`diaries/<unit>.md`, first person, an entry per pass: what you meant (the
music), what you set (the physics: tensions, push, cheeks, tone, what you
fed and when), what the World measured (quote the numbers), what you heard
in them, and what you changed. Reviewers add their own entries below yours.
Do not invent a measurement: every number in the diary comes from a report.

## The retune pass (second run)

After the first run the instrument changed, for the listener's sake, and
every part you wrote on it must be checked again on the one it now plays:

- The valve is heavier (`fart_v1`: 1.3 g, a softer contact; the first run's
  `fart_v0` was 1 g and clicked once per cycle at low tensions). A heavier
  valve lands on a different pitch for the same tension and needs more push
  to start: on the first run's parts unchanged, notes within 50 cents fell
  from 86.7% to 57.5% and 359 notes went silent (`render` reports).
- Each voice has its own anatomy now (`score.json`, `voices.<v>.anatomy`):
  guts I-II `soprano` (1 g valve, 20 cm canal), III-IV `alto` (`fart_v1`,
  20 cm), V-VI `bass` (1.6 g, 28 cm canal, wider), the cannon `cannon`
  (2.2 g, 40 cm, a 2.5 J reservoir; its shots are not pitched). The bass
  guts moved most (31-35% in tune on the old tensions).
- The score has a `direction` (the house style: wobble 0.5, vibrato 6/0.25,
  jitter 1.0, rasp 0.6), applied before your first token. It is on in every
  render and in `probe --score`. You may override any of it in a part with
  the same tokens (`wobble=`, `vibrato=`, `jitter=`, `rasp=`), but the
  director asks you not to without a musical reason written in the diary.

So: probe the instrument your voice plays (`$F probe --score $D --voice v5
--push P --cheeks C --tone T`, from/to around your register), re-derive
tensions, raise push or tone where notes went silent (the report's
`silentNotes`, `worstTuning`), re-plan feeding if you raised push (more
push spends more gas), keep every part's beats, same render limits. Where a
note the valve cannot play now was a rest in the first run, it may stay a
rest. Write a diary entry headed `## Retune (fart_v1)`: what the new
instrument did to your parts, what you changed, the numbers before and
after. The first run's diary stays above it.
