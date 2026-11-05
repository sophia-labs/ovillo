# Before the Law

Six instruments for a short film: Vera as a PS1-era low-poly character waiting
before a door, after Kafka's "Before the Law". A guard at a door, endless
waiting, a radiance from the gate, fleas in the doorkeeper's fur collar, and a
final slam. The score should sound like a 1998 PlayStation stealth game.

Everything here is original. No melody, theme or motif is taken from any game
soundtrack, and no recorded or ripped audio is used: every sound is a World's
response to a touch, a breath or a voice.

## Physics and period colour

The instruments are Ariadne Worlds and are played as such. The PS1 sound (SPU
ADPCM storage, Gaussian playback, the SPU's canned reverb) is **not** in this
directory and not in any World. It is a listening chain applied to finished
renders, outside Ovillo, in `~/dev/verabot/film/audio/tools/ps1_chain.py`.
Mixing several Worlds into one cue is also outside the Worlds: they don't share
a room or hear one another. The mix is a studio decision, and its gains are
listed in `~/dev/verabot/film/audio/instruments/manifest.json`.

## The instruments

| instrument | in the film | World | played by |
|---|---|---|---|
| **Gate** | the door: clangs, a rattling chain, the final slam | new tab anatomy `gate.plan.json`: six struck iron parts driving a door leaf, in a stone room | `mus render` |
| **Doorkeeper** | the guard: low brass swells, stabs, a rip, a comic fall | the existing trombone v1 World (`TROMBONE-WORLD.md`), unchanged | `trombone_nd_score` (new driver: a note list) |
| **Waiting** | the endless wait: a low string ostinato over a sub pedal | new tab anatomy `waiting.plan.json`: five low plucked courses on a large body | `mus render` |
| **Light** (bells) | the radiance from the gate | new tab anatomy `light.plan.json`: eight undercut glass bars over tuned tubes | `mus render` |
| **Light** (choir) | the radiance, sung: hum and "aah" pads, the children at its brightest | twelve tract Worlds (`TRACT-WORLD.md`), one per singer, each with its own anatomy built from the public tract's declared fields; runtime unchanged | `tract_choir` (new driver: a cast, a voice list, a pitch table each singer learned) |
| **Fleas** | the fleas in the doorkeeper's fur collar | new tab anatomy `fleas.plan.json`: six tiny hard clicks in a fur collar with a brass clasp | `mus render` |
| **Slam** | timpani, rolls, the weight under the slam | new tab anatomy `slam.plan.json`: two pedal kettledrums, a bass drum, a field drum | `mus render` |

The five new tab anatomies are **fictional designs**, built the way Sunskip's
are (`examples/ensemble/sunskip/`). Each is an ordinary canonical `WorldPlan`
with the steel-string plan as its structural template, and every changed number
is a design hypothesis recorded in the adjacent `*.design.json` file (baseline
hash, plan hash, every cell's 24 mode frequencies, loss law, tuning token). Each
`string/N` body is a struck or plucked element, each `box/N` body a radiator
it drives through reciprocal `bridge/N/band` edges, and the plan's shared space
is the room. The tab engine makes boxes radiate and cells almost not at all
(`SPEC-ARIADNE-STEEL-STRING.md`, Layer 3), so what you hear is the radiators
and the room.

Where a mode family follows a textbook law (free-free beam, simply supported
plate, ideal membrane, thin ring), the law is named in the design file. The
dimensions and losses fed into it are chosen, not measured. None of these is a
reconstruction of a real gate, contrabass, celesta, drum or flea.

## Playing them from MUS

Use the tuning the design file gives as `scoreTuning`. The engine retunes any
cell whose rounded first-mode pitch differs from the score's token, which would
move an inharmonic cell's modes. Each cell or course gets its own hand, as
Pocket does in Sunskip, so `pos`/`pick`/`pm` describe how that element is
touched. The instrument lines from `scores/`:

```text
#   gt = The Gate (perc, anatomy=../gate.plan.json, tuning=D2 Ab2 Eb3 F5 Bb4 A6)
#   wt = The Waiting (bass, anatomy=../waiting.plan.json, tuning=D1 A1 D2 G2 C3)
#   lt = The Light, bells (treble, anatomy=../light.plan.json, tuning=D4 F4 G4 A4 C5 D5 F5 A5)
#   fl = The Fleas (perc, anatomy=../fleas.plan.json, tuning=C6 D6 E6 F#6 Ab6 Bb6)
#   sl = The Slam (perc, anatomy=../slam.plan.json, tuning=A1 D2 A2 G3)
```

- **Gate, Light, Fleas** and the Slam's bass drum and field drum are fixed
  geometry (both configuration ends equal): use `@string:0` only. Pitch tokens
  address cells. Gate: `D2@6:0` panel, `Ab2@5:0` long bar, `Eb3@4:0` cross bar,
  `F5@3:0` bolt, `Bb4@2:0` lock plate, `A6@1:0` chain. Light: string 1 = A5 ...
  string 8 = D4. Fleas: string 1 = Bb6 ... string 6 = C6.
- **Waiting** courses are fretted over an octave (fret k = amount k/12, the
  steel-string convention). The checker's reach rule applies: a sub held high
  on the D1 course plus an ostinato low on the D2 course is refused.
- **Slam** kettledrums: the configuration path is the pedal (membrane tension,
  every mode ×2 at amount 1). Use frets 0–7, a fifth. `[sl]` is a pedal
  glissando (60 ms, the tab engine's slide time).
- Durations only place touches in time. There is no note-off damping
  (steel-string "Limits of v0"), so each element rings out by its own losses.

`compose.py` writes every score here from one 32nd-note grid, so the MUS bars
and the seconds in the trombone and choir lists agree. In `mus render`, bar 1
starts at 0 s; `render.py` records each World's first touch time in
`out/validation.json` (0.0 s throughout).

## Gate

**Construction** (`gate.plan.json`, `gate.design.json`; room delays ×4.5, bridge rates ×1.2):

| string | element | law | first mode | T60 first → last mode |
|---|---|---|---|---|
| 6 | riveted panel, 0.62 × 0.45 m | simply supported plate, 24 lowest (m,n) | 73.42 Hz (D2) | 5.75 → 4.93 s |
| 5 | long grille bar | free-free beam, two bending planes split 0.4 % (12 pairs) | 103.83 Hz (Ab2) | 4.60 → 0.79 s |
| 4 | cross bar | same | 155.56 Hz (Eb3) | 4.59 → 0.39 s |
| 3 | the bolt, flattened | free-free beam (two planes split 1.5 %) + longitudinal (5.1 kHz ×k) + torsional (3.2 kHz ×k); heavy contact loss in its keeper | 698.46 Hz (F5) | 0.38 → 0.020 s |
| 2 | lock plate, 0.30 × 0.22 m | simply supported plate | 466.16 Hz (Bb4) | 1.69 → 0.22 s |
| 1 | chain | eight thin rings (n = 2 in and out of plane, n = 3), sizes 0.93–1.18 | 1773.73 Hz (A6 +13.5 c) | 0.26 → 0.17 s |

The three radiators are one door leaf, a 2.4 × 1.4 m simply supported plate
(f11 ≈ 9.8 Hz, so far more modes than 72). It is sampled evenly in log
frequency in three bands, 39–400 Hz, 433–2219 Hz and 2.37–8.74 kHz, as the
piano samples its board above 3 kHz. Radiation weights follow the steel-string
box's rule: net pumped volume (odd-odd modes) plus an edge term rising to
2.5 kHz, times frequency. Each part joins the leaf at its own point, and the
edge's box-side direction is the leaf's mode shape there.

**Measured** (the audition, spectral peaks): the panel speaks at 73.4 Hz with
plate partials at 149, 276 and 449 Hz. The bars' fundamentals are weak against
their 2.756 and 5.40 modes (long bar 286, 562, 930 Hz), because radiation rises
with frequency. The bolt rings as a 697/707/720 Hz cluster, the chain as a
1.77–2.37 kHz cluster. After the slam the level falls about 11 dB/s.

## Waiting

**Construction** (`waiting.plan.json`; room ×3, bridge ×0.9): five courses D1
A1 D2 G2 C3, each `f_n = n f1 √((1+Bn²)/(1+B))` with B = 6·10⁻⁵, loss
`σ = 1.1 + 3.5·10⁻⁷ ω²` (fundamental T60 5–6 s, top partial 0.05–0.57 s), each
fretted over an octave. The body is the steel-string box scaled ×0.754 in
frequency (its air mode lands at 80 Hz, the top's main mode at 147 Hz) and
×0.45 in T60.

**Measured**: the sub sounds at 36.6 Hz (D1) and the D2 course at 73.2 Hz. In
the first 0.4 s after a pluck, the body's modes near 80–87 Hz and 147–152 Hz
are as strong as the course's fundamental, or stronger. Later the course
dominates. That is the body's character and a rough edge (see Limits).

## Light (bells)

**Construction** (`light.plan.json`; room ×3.5, bridge ×0.35 to the tubes,
×0.15 to the case): eight glass bars D4 F4 G4 A4 C5 D5 F5 A5. Each bar's 24
modes are the lowest of three families, held in index order:

1. vertical bending of an undercut bar tuned 1 : 4 : 10 (the vibraphone and
   marimba tuning), continued above with the free-free series scaled from 10
   (an invented continuation);
2. lateral bending (width/thickness 3.2);
3. torsion (1.35 × f1 × k).

Loss is `σ = 0.9 + 1.6·10⁻⁸ ω²`. The index order matters, because the engine
weights a touch by mode index as sin(nπ·pos)/n. The hand plays at
`pos = 0.42`, a mallet just off the bar's centre, where torsion has its nodal
line and the antisymmetric second mode is weak. box/0 holds the eight tuned
tubes (closed-open: f, 3f, 5f each, Q 150), and each bar meets its own tube.
box/1 and box/2 are a case (the steel-string box ×1.35 in frequency, ×0.8 in
T60).

**Measured**: every bar sounds its written pitch as the strongest peak (A4
440.2, D5 587.6, G4 392.0, C5 523.3, F5 698.5, A5 880.0 Hz), with other modes
30–39 dB down: a pure, celesta-like tone. Notes fall about 50 dB in 1–1.5 s.

**How it got there** (three earlier versions, all measured and discarded; kept
here so nobody repeats them):

- Unordered modes, Q-40 tubes, a strong tube edge: the tube drained the
  fundamental, and the bars spoke at their torsion (an A4 bar's strongest peak
  was 704 Hz).
- With the strike point moved, the fundamental split into a pair around its
  tube (431.6/447.5 Hz). That is real mode repulsion, but it beat and died in
  0.5 s.
- With the bars untuned (2.756 for the second mode), the untuned partial led.

## Fleas

**Construction** (`fleas.plan.json`; room ×1, the fleas are close): six cells
C6 D6 E6 F#6 Ab6 Bb6, a whole-tone flea scale. Each is an **invented** dense
cluster of 24 modes from f1 up to about 10.5 × f1, T60 0.03 s down to 0.008 s.
There is no law here. The radiators are a fur collar (180 Hz–4.5 kHz, T60
0.026–0.063 s), a brass clasp (1.65–15.6 kHz, T60 up to 1.5 s) and the coat's
leather. The fleas on strings 2 and 4 (Ab6, E6) reach the clasp most, so they
"tink".

**Measured**: clicks at 1.6–6 kHz, each gone within about 100 ms. The World
radiates 0.13 % of the work put in (1.8·10⁻¹² J of 1.4·10⁻⁹ J in the audition),
so in a mix the Fleas need a large observation gain (+17 dB in the cue).

## Slam

**Construction** (`slam.plan.json`; room ×4, a big stone hall): string 4 is a
bass drum and string 1 a field drum, both ideal membranes (Bessel zeros
j_{m,n}, 24 lowest) at A1 and G3. Strings 3 and 2 are kettledrums at D2 and A2.
Index 0 is the principal (1,1); (2,1)..(5,1) sit at the air-loaded ratios
1.50, 1.98, 2.44, 2.94 commonly quoted from Fletcher & Rossing; (6,1) at 3.42
and (0,1) at 0.85 (heavily damped, T60 0.25 s) are design hypotheses. The
remaining modes are ideal-membrane ratios to (1,1). The radiators are a wooden
riser: the steel-string box ×0.5 in frequency.

**Measured**: the D2 kettledrum gives 73.1 Hz with 109.7 Hz (×1.5) and
145.4 Hz (×1.98), and the pedal glissando lands at 111.6 Hz (A2). The bass drum
gives 55.7, 87.5 and 117.6 Hz (×1.594, ×2.136: the ideal membrane). The rolls
are 32nd-note single strokes; their crescendo is written dynamics.

## Doorkeeper (trombone v1, unchanged)

`ovillo/crates/ovillo-dsp/examples/trombone_nd_score.rs` plays the v1 World
from a TSV note list (`doorkeeper/*.tsv`; columns `start_s end_s midi vel
vel_end art fx`). Placement and blowing are `trombone_nd_player::place` and
`perform`, exactly as in the solo. The one addition is a **hairpin**: when
`vel_end` differs from `vel`, the mouth pressure the player blows is moved
sample by sample, log-interpolated, between the calibrated pressures for the
two dynamics of that cell. It is still the player's breath, and nothing is
applied to the sound.

**Measured**: every note within about 0.2 semitone. The audition sounds D3
50.09, Eb3 51.10, A2 45.04, Bb3 (rip) 58.19, A3 57.15 and F3 53.13 MIDI; the
cue sounds 50.13, 51.01, 50.06, 45.11, 46.05 and 50.11. The player chose
partial 3 for D3/Eb3/Bb2, 2 for A2, 5 for Bb3 and 4 for A3/F3. A swell from
`vel` 0.15 to 0.85 spans about 6 dB (−18.7 → −12.4 dBFS), because each cell's
calibrated pressure range is narrow (`TROMBONE-WORLD.md`, Limits: "a cell's
dynamic range is a few grid points"). The raw emission peaks at +2.6 dBFS
(1.35) on an attack, which a float WAV keeps. The audition copy is
peak-normalized by one gain.

## Light (choir: twelve tract Worlds)

`ovillo/crates/ovillo-tract-world/examples/tract_choir.rs` makes each singer
its own `TractWorldNd`. The cast is `choir/singers.tsv`; the parts are
`choir/*.tsv` (`singer start_s end_s midi vowel breath vib_depth`; vowels
`a i u m o`). Breath is the valve's supply, vibrato the World's booked fold
retuning. The choir is a listener's mix: one gain and one equal-power pan per
singer. The library and runtime are unchanged.

**Twelve anatomies.** Two singers per voice type: bass, baritone, tenor, alto,
soprano, child. A voice type starts from `TractNdAnatomy::public_v1()` and
changes only its declared anatomy fields (`VOICES` in the driver):

| voice | oral / nasal tract | fold length × thickness, mass | fold tuning range | lungs |
|---|---|---|---|---|
| bass | 0.170 / 0.118 m | 16 × 3.3 mm, 0.18 g | 45–340 Hz | 3.4 J, 0.32 W |
| baritone | 0.163 / 0.114 m | 15 × 3.1 mm, 0.16 g | 55–400 Hz | 3.2 J, 0.30 W |
| tenor | 0.155 / 0.108 m | 13 × 2.9 mm, 0.13 g | 70–520 Hz | 3.0 J, 0.30 W |
| alto | 0.142 / 0.099 m | 11 × 2.6 mm, 0.095 g | 110–760 Hz | 2.4 J, 0.25 W |
| soprano | 0.136 / 0.095 m | 9.5 × 2.4 mm, 0.075 g | 150–1100 Hz | 2.2 J, 0.24 W |
| child | 0.118 / 0.082 m | 7.5 × 2.1 mm, 0.05 g | 170–1000 Hz | 1.4 J, 0.16 W |

For comparison, `adult_v0` is 0.157 / 0.11 m, 14 × 3 mm, 0.15 g, 60–420 Hz,
3 J and 0.3 W. The other types are design hypotheses scaled in the commonly
reported directions (shorter tracts and lighter, shorter folds for higher
voices); none is a measurement. Each singer also scales the tract lengths by
`tract_scale` (0.965–1.027 here) and the fold by `fold_scale` (0.950–1.048;
mass × fold_scale³).

**Each singer's own manner**, all World controls:

- a constant lateness (−7.8 to +24.5 ms) and a per-note timing scatter (12.9
  to 31.7 ms);
- an intonation offset (−5.4 to +5.9 cents) and a per-note scatter (2.2 to
  5.3 cents): the pitch the singer aims for;
- vibrato rate (4.9–6.0 Hz) and wobble;
- a breath scale (0.94–1.07);
- a personal vowel shape (tongue index and lip offsets of ±0.03 at most).

The scatters are deterministic (a hash of singer and note), so a render
repeats exactly. The individual numbers were drawn once from a fixed seed in
`compose.py` and are written out, editable, in `choir/singers.tsv`. When a
singer changes vowel, the target articulation moves over 0.2 s
(`ART_GLIDE_S`). A jumped target is reached in about 25 ms, which out of a
hum's sealed lips released like a plosive (a −4.7 dBFS spike over a −30 dBFS
choir; with the glide, −12.5 dBFS).

**Each singer's skill.** The fold tuning is not the pitch a World sings. As the
trombone player learned its cells by playing its World,
`tract_choir --calibrate choir/singers.tsv choir/skill` plays every singer's
own World. For each vowel it holds every tuning in the voice's range in
quarter-tone steps and reads the fold's cycle rate (`Observers::measured_f0_hz`,
non-loading). That singer then looks up which tuning sounded each pitch (one
table per singer in `choir/skill/`). Pitches the calibration never heard are
refused. Above its range a voice stops voicing; it is not clamped. The
singers' contiguous sung ranges:

| singer | /a/ | /o/ | /m/ (hum) |
|---|---|---|---|
| bass1, bass2 | 42–341 Hz (E1–F4) | 48–348 Hz | 67–360 Hz |
| bari1, bari2 | 52–390 Hz (Ab1–G4) | 49–393 Hz | 62–271 / 64–414 Hz |
| ten1, ten2 | 74–521 Hz (D2–C5) | 72–485 Hz | 68–513 Hz |
| alto1, alto2 | 127–704 Hz (C3–F5) | 121–566 Hz | 117–720 Hz |
| sop1, sop2 | 171–771 Hz (F3–G5) | 175–616 Hz | 164–804 Hz |
| child1, child2 | 195–868 Hz (G3–A5) | 197–708 Hz | 189–937 Hz |

`o` (the midpoint of `a` and `u`, **not fitted**) tops out lower than `a` in
every high voice. On a shorter tract the same articulation gives higher
formants; that is the anatomy, not a fit.

**Measured**: a monophonic probe of all twelve singers on 36 notes, from A1
(bass1) to F5 (sop2, child2), sings within a median 4.6 cents of each
singer's own target (max 13.6 cents). The target includes that singer's
intonation offset and scatter. The singers were calibrated at breath 0.55;
other breaths move the pitch a little, which is part of the scatter.

In the parts, men and altos hum and then open to "aah"; the sopranos join on
B-flat (bar 8 of the cue); the children enter only on the A major of bar 9,
the radiance at its brightest. Voice-type mix gains are soprano +1 dB and
child +2 dB (their Worlds radiate about 2–4 dB less); everyone else 0.

## Receipts

`python3 render.py` checks every score (`mus check`, must be clean), renders
every World and writes `out/validation.json`. For each World that file holds
`sourceWork`, `controlWork`, `internalLoss`, `radiation`,
`cumulativeClosureResidual`, `maximumFrameResidual`, the touch counts and the
first touch time. It fails on bad tokens, non-finite ledgers or a frame
residual above 1·10⁻⁹ J. The latest run has 12 pieces, no failures, and these
largest residuals:

| World family | largest frame residual | largest cumulative residual |
|---|---|---|
| tab anatomies (Gate, Waiting, Light, Fleas, Slam), all pieces | 8.3·10⁻¹⁷ J | 2.8·10⁻¹⁶ J |
| trombone v1 (Doorkeeper) | 3.6·10⁻¹⁵ J | 9.5·10⁻¹² J |
| tract (each of 12 singers, per piece) | 8.9·10⁻¹⁶ J | 4.9·10⁻¹³ J |

Per piece, `out/` also has `mus render --json` receipts (`*.tab.receipt.json`),
check reports, energy traces (`*.energy.json`, `mus.ariadne.energy-trace.v1`),
the trombone's receipt and notes with partial and slide
(`*.doorkeeper-receipt.json`, `-notes.tsv`) and the choir's per-singer books
(`*.choir-receipt.json`). A tab World's `planDigest` changes from piece to
piece, because the engine folds the hands into the plan before building the
World; the plan file's own hash is in `*.design.json`.

The cue's instrument gains (Slam −20, Light −10, Fleas +17 dB against the
Waiting; Gate −2) were set from each World's radiated power per 10 ms in the
cue's energy trace. They are observation gains, not physics, and not a
listening judgement.

## What these instruments do not model

Everything in the steel-string's "Limits of v0" applies to the five tab
anatomies: no note-off damping, one touch point per cell, 24 modes per body, a
linear model. Read that section before blaming the score. In addition:

- **Gate**: no nonlinearity. A real slammed gate buzzes, rattles against its
  frame and changes pitch with amplitude; this one is linear, so a fff slam is
  a louder pp clang. No contact between parts (the chain links do not actually
  collide; their high loss stands in for it), no hinge, no scrape (there is no
  friction or bowing in the tab engine). The leaf is sampled at 72 of its
  thousands of modes. The bars' fundamentals radiate weakly.
- **Waiting**: plucked, not bowed. There is no bow, so no sustained string
  tone: the "sub drone" is a long-decaying D1 course re-plucked each bar. The
  body is a scaled guitar box, not a contrabass. Its 80–87 Hz and 147–152 Hz
  modes colour the attack of every low note.
- **Light (bells)**: the undercut 1:4:10 tuning is a design choice (real
  celesta bars were not measured), and everything above the third bending mode
  is invented. A touch can't choose its point on the bar per note. The tone is
  nearly pure, and the 1–1.5 s decay is shorter than a vibraphone's.
- **Fleas**: no law at all, an invented click. The fur is a lossy modal body,
  not a material model. Nothing hops: a flea "landing on the clasp" is a fixed
  coupling weight.
- **Slam**: no snare wires, no noise source, so the field drum's roll is a
  fast tom-like roll, not a snare buzz. No second head on the bass drum. The
  kettle's air is a ratio table, not a cavity. The engine's pluck pulse is at
  most 0.36 ms long, much shorter than a felt mallet's contact, so the strikes
  are brighter than real timpani. The hand's `pos` is a string law applied by
  mode index, not a strike point on a membrane.
- **Doorkeeper**: everything in `TROMBONE-WORLD.md`'s Limits, in particular no
  brightening with loudness (a linear bore) and no room: it is heard dry.
  Swells span about 6 dB.
- **Light (choir)**: the singers' anatomies differ in tract length, fold size
  and lungs, but every one keeps the adult's cross-sectional area function
  (the anatomy has no area scale). A child's tract is a shortened adult one,
  not a child's shape. The voice types are design hypotheses, not measured
  singers. No aspiration noise (`TRACT-WORLD.md` Gaps), so no breathiness: a
  real choir's air is missing, and nothing outside the Worlds adds it either.
  `o` is unfitted. The top of every voice is where its World stops voicing
  (soprano about G5, child about A5 on "aah"), not a trained singer's top. The
  singers share no room and don't listen to one another: their timing and
  intonation scatter is fixed, not a blend or a mutual retuning.
- **The ensemble**: seven kinds of World mixed by gain. They share no space.
  The SPU colour is applied after the mix, outside every World.
- Nobody has listened critically to these yet. The checks are scores, ledgers,
  pitches, spectra and levels.

## Reproduce

From the repository root (Rust 1.99.1 as in CI: `cargo +1.99.1`):

```sh
(cd mus-rs && cargo build --release -p mus-cli)
(cd ovillo && cargo build --release -p ovillo-dsp --no-default-features --example trombone_nd_score)
(cd ovillo && cargo build --release -p ovillo-tract-world --example tract_choir)
python3 examples/ensemble/before-the-law/build_instruments.py
python3 examples/ensemble/before-the-law/compose.py
ovillo/target/release/examples/tract_choir --calibrate examples/ensemble/before-the-law/choir/singers.tsv \
    examples/ensemble/before-the-law/choir/skill    # after editing singers.tsv; the twelve tables are checked in (~15 s)
python3 examples/ensemble/before-the-law/render.py
```

A full render takes about two minutes on this machine (all Worlds in
parallel). `out/` holds about 136 MB of WAVs and energy traces. It is a
render directory, not source, and is in `.gitignore`. The audition folder for the
film is built from it by `~/dev/verabot/film/audio/tools/build_auditions.py`.
