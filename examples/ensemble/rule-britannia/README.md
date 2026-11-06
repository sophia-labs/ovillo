# Rule, Britannia!

An arrangement of Thomas Arne's *Rule, Britannia!* (1740, public domain) for
Vera's farewell film, *Welcome Back, Mrs. Chambers*, played by relatives of
the *Before the Law* instruments (`../before-the-law/`): jazzy, poppy, with
one section that goes out and lands back in a big warm chorus. About 2:50.

Everything here is a World's response to a touch, a breath or a voice. No
recorded audio, no samples. The tune is Arne's; the harmony, the hook, the
form and every part are original (nothing is taken from any recording or
arrangement of the song). Mixing and mastering happen **outside** the Worlds,
labelled as such (`mix.py`), the way the film's PS1 colour does.

## The instruments

Three new tab anatomies, built with the same code and laws as the film's
(`build_instruments.py` imports `../before-the-law/build_instruments.py`),
and three World families reused as they are.

| instrument | relative of | World | played by |
|---|---|---|---|
| **The Walker** | the Waiting | new tab anatomy `walker.plan.json`: four plucked courses E1 A1 D2 G2, each fretted over an octave, on a big wooden body | `mus render` (`scores/rb-walker.mus`) |
| **The Vibes** | the Light's bells | new tab anatomy `vibes.plan.json`: twenty-two bars C4..A5, the Light's bar construction with a metal-like loss, over two boxes of tuned tubes | `mus render` (`scores/rb-vibes.mus`) |
| **The Kit** | the Fleas and the Slam | new tab anatomy `kit.plan.json`: kick, snare (brushed or struck), cross-stick, hi-hat closed and open, ride; the snare's wires a sympathetic radiator | `mus render` (`scores/rb-kit.mus`) |
| **The Brass** | the Doorkeeper | three trombone v1 Worlds (`TROMBONE-WORLD.md`), unchanged, one note list each | `trombone_nd_score` (`brass/rb-tb1..3.tsv`) |
| **The choir, doo-wop** | the Light's choir | the same twelve tract Worlds and calibration tables (`../before-the-law/choir/`), singing "bah", "doo", "dah", "wah", "ah", "ooh" | `tract_choir` (`choir/rb-choir.tsv`), with a new optional `glide_s` column |
| **The piano** | — | the Ariadne piano (`SPEC-ARIADNE-PIANO.md`, `anatomy=piano-v1`) | `mus render` (`scores/rb-piano.mus`) |

### The Walker

**Construction** (`walker.plan.json`, `walker.design.json`; room ×2, bridge
×0.8): four courses E1 A1 D2 G2, `f_n = n f1 √((1+Bn²)/(1+B))` with B = 2.5·10⁻⁵
(the Waiting's is 6·10⁻⁵), loss `σ = 0.9 + 6·10⁻⁷ ω²`: the fundamentals' T60 is
6.1–7.3 s and the top partials' 0.05–0.28 s (design receipt). The body is the
steel-string box scaled ×0.6 in frequency (lowest mode 63.8 Hz) with its T60
kept. The hand plucks at `pos = 0.30` with a fleshy contact (`pick = 0.05`)
and a light palm (`pm = 0.12`, which is what makes each quarter note give way
to the next; the tab engine has no note-off damping).

**Measured** (`scores/walker-audition.mus`, spectral peaks of the first 0.45 s
of each note): the open E1 speaks with its second partial strongest (82.4 Hz)
and the fundamental 41.0 Hz at −4.9 dB; A1 likewise (109.5 Hz, 54.9 at −11 dB);
D2 and G2 at 73.2 and 97.8 Hz as the strongest peak. The body's modes radiate
the second partial of the lowest courses more than the fundamental: that is
this body, and a limit (below). A note falls about 17 dB in its first 250 ms
(the palm), and the accented F2 is 20 dB down after 0.55 s.

### The Vibes

**Construction** (`vibes.plan.json`; room ×2.5, bridge ×0.08 / ×0.03 to the
two tube boxes and ×0.1 to the case): twenty-two bars C4..A5 with the Light's
mode families in the same index order (undercut bar tuned 1 : 4 : 10, then
lateral bending, then torsion at 1.35 × f1 × k), loss `σ = 0.30 + 1·10⁻⁸ ω²`
(fundamental T60 9–17 s against the Light's 0.9 + 1.6·10⁻⁸: a metal bar
rather than glass). Above A5 a bar has fewer than 24 modes under the runtime
band, which is why the range stops there. box/0 holds the eleven upper bars'
tubes and box/1 the lower eleven (closed-open: f, 3f, 5f; the eleven
fundamentals and the thirteen lowest odd harmonics each; Q 120); each bar
meets its own tube at weight 1/h and the others at 0.02. box/2 is the
steel-string box's high band ×1.0, ×0.8 in T60. The mallet strikes at
`pos = 0.42`, `pick = 0.80`, as on the Light.

**Measured** (`scores/vibes-audition.mus`): C4 sounds 261.1 Hz, F4 349.4,
A4 440.2, C5 523.3, E5 659.2, G5 784.1, A5 880.0 Hz as the strongest peak,
with every other peak 31–39 dB down. A chord falls about 9 dB per second
(−11.5 dBFS at 4.5 s to −39.9 at 7.5 s): the ring the Light does not have.

**How it got there**: with the tubes coupled at the Light's rate (×0.2) and
Q 200, the low bars split into a pair around their tube (C4 as 249.4 and
275.8 Hz, F4 as 336.2 and 361.8) and A4 sat 50 cents flat. That is mode
repulsion, a real coupled-resonator effect and the Light's earlier trouble
too; weakening the bridge to the lower tubes (×0.03) and lowering Q to 120
left the bars on pitch. The motor tremolo is **not** modelled (below).

### The Kit

**Construction** (`kit.plan.json`; room ×1.5, the kit is close):

| string | element | law | first mode | T60 first → last mode |
|---|---|---|---|---|
| 1 | cross-stick | free-free beam, two bending planes split 0.5 %, f1 chosen at F4 so that twelve modes fit under the band | 349.2 Hz | 0.15 → 0.018 s |
| 2 | hi-hat closed | two thin rings' in-plane flexural modes n = 2..13 (the second ring ×1.06, as the Gate's chain); f2 at C4 | 261.6 Hz | 0.115 → 0.083 s |
| 3 | hi-hat open | the same rings, less loss | 261.6 Hz | 0.77 → 0.26 s |
| 4 | ride | the same ring law at G3 | 196.0 Hz | 3.45 → 1.07 s |
| 5 | snare | ideal membrane (Bessel zeros), f01 at G3 | 196.0 Hz | 0.37 → 0.15 s |
| 6 | kick | ideal membrane, one head, f01 at F1 | 43.65 Hz | 1.12 → 0.61 s |

There is no law here for a cymbal's own modes; the ring is a stand-in, and
it is said so in the design file. The radiators are the shells (the
steel-string box ×0.5 in frequency, ×0.6 in T60, as the Slam's riser), the
snare wires (an **invented** cluster of 24 modes from 700 Hz to 5 kHz, T60
0.22–0.27 s, that rings in sympathy: linear, so no rattle and no contact) and
a cymbal stand (the box's high band ×1.8). Each piece reaches the three
radiators with its own weights (`reach` in the script): the kick the shells,
the snare the wires, the cymbals the stand. The brush is a second hand on the
snare head with a soft, round contact (`pick = 0.15`, `pos = 0.30`); a sweep
is written as a run of pianissimo strokes every sixteenth-triplet, because
the tab engine has no sliding contact. The brush hand carries an observation
gain of −10 dB in the score, because its longer pulse put more energy into
the head's low modes than a stick's and sounded louder than a mf stroke.

**Measured** (`scores/kit-audition.mus`): the kick's strongest peaks are 93.0,
69.6 and 104.4 Hz (the membrane's (2,1), (1,1) and the shells), its own 43.2 Hz
at −18 dB: the thump sits an octave above the head's fundamental. The snare
gives 195.2 Hz with 312, 449, 684 and 768 Hz (the last two in the wires'
band) and is 35 dB down after 250 ms. The cross-stick's strongest peak is
6.5 kHz, gone in 100 ms. The hi-hat's peaks are 277, 784, 1418, 1504 and
2431 Hz; the ride's 196, 554, 587, 1063 and 1719 Hz: darker than a real
ride's ping (below).

### The Brass

The trombone v1 World, unchanged, played three times from three note lists:
`tb1` leads (the tune, in verse 2 and the choruses, an octave below the
vibes), `tb2` and `tb3` take the chord tones nearest below it. The hairpins,
rips, falls, smears and vibrato are all the player's (`trombone_nd_score`).
The instances share no air: they are three Worlds, as the twelve singers are.

**Measured** (the piece, `out/rb-tb1.trombone-notes.tsv` against
`measure.py f0`): verse 2's F3 F3 ... C4 G3 A3 sound 53.10, 53.11, 60.01,
55.12, 57.11 MIDI; chorus 1's F3 F3 D3 sound 53.14, 53.14, 50.10. The player
chose partial 4 for F3–A3, 5 for Bb3–C4, 3 for C3–D3.

### The choir, doo-wop

The same twelve singers, anatomies, manners and calibration tables as the
film's choir. One addition to the driver (`tract_choir.rs`): an optional
`glide_s` column on a note, the time the target articulation takes to move
from the previous vowel (default `ART_GLIDE_S`, 0.2 s, as before). A
consonant is a hum's sealed lips released fast: "bah" and "dah" are an
`m` of 50 ms then `a` with `glide_s = 0.02`, "doo" is `m` then `o`, "wah" is
`m` then `a` over the default 0.2 s. The calibration heard only `a`, `o` and
`m`, so those are the vowels. In the out section the singers slide: one
short note per quarter tone (the fold retuned, every move booked), 0.16 s
each, a tritone up over four seconds.

### The piano

`anatomy=piano-v1`, comping: four-note voicings in the right hand on the
beat and the swung "and" of the next (the Charleston), root and a shell in
the left, the pedal changing with each chord; in octaves on the hook; low
clusters in the out section.

## The melody

Two public-domain transcriptions, cross-checked bar by bar:

1. ABC: "Rule Britannia", C: Thomas Augustine Arne (1740), N: Words by James
   Thomson, D major, 4/4, L:1/8 — abcnotation.com's mirror of
   `trillian.mit.edu/~jc/music/abc/mirror/code.google.com/Rule_Britannia_1`
   (tune page `abcnotation.com/tunePage?a=trillian.mit.edu/~jc/music/abc/mirror/code.google.com/Rule_Britannia_1/0000`).
2. MIDI: "Rule, Britannia (pg).mid" on Wikimedia Commons (melody by Thomas
   Augustine Arne, setting by Rabanus Flavus / Peter Gerloff, CC0 1.0),
   G major, 384 ticks per quarter; sha256
   `5ca2b69f8327468087b190a0e5ae5bfd2584a6eba7723c785eafe83a334ab8fa`,
   read with a stdlib MIDI parser.

They agree on the pitches of all fourteen bars (six of verse, eight of
chorus) except at two spots, where the ABC was taken: the fourth eighth of
bar 5 (C4 in F; the MIDI has E4) and the pickup into bar 9 (C4; the MIDI has
F4). Bar 10's turn (G A G F in sixteenths) is the ABC's; the MIDI simplifies
it to two quarters. The tune is in `compose.py` (`MELODY`), in F major.

IMSLP has the Henry Waylett first edition (file 284937) as a scan only; it
was not consulted.

## The arrangement

F major, 108 bpm, swung eighths (triplet tokens on a 96-tick bar), 74 bars.

| bar | time | section | what happens |
|---|---|---|---|
| 1 | 0:00 | intro | the Walker and the piano in unison on the hook, brushes; the vibes join at bar 5, the men and altos "doo" |
| 9 | 0:17.8 | verse 1 | the tune on the vibes over the changes; brushes; soft trombone pads from bar 11 |
| 15 | 0:31.1 | chorus 1 | the tune on the vibes with the three trombones under it; ride time; the choir "bah" on the Charleston |
| 23 | 0:48.9 | hook | everybody on the hook; "bah ... wah" |
| 27 | 0:57.8 | verse 2 | the first trombone sings the tune; vibes dyads; "doo ... wah" pads |
| 33 | 1:11.1 | chorus 2 | tutti |
| 41 | 1:28.9 | out | a lone low D and a ride wash; then an ostinato in seven eighths (D D A D F E D) across the 4/4 bars, the kit in seven, four-note semitone clusters planing down on the vibes, low clusters on the piano, the trombones smearing and falling in free time, the twelve singers humming a chromatic cluster, opening one by one, and sliding up a tritone; a snare fill |
| 53 | 1:55.6 | chorus 3 | big and warm: the sopranos and children sing the tune ("dah"/"ah"), the vibes and first trombone with them (a rip in), the men and altos "bah" |
| 61 | 2:13.3 | chorus 4 | up a semitone, G-flat; the vibes in octaves |
| 69 | 2:31.1 | hook | in G-flat |
| 73 | 2:40.0 | coda | two hits on G♭maj9(♯11); the third trombone falls off the last chord (the Doorkeeper's joke); the vibes and the choir ring |

**Harmony** (verse): F△9 – D–9 | G–9 – G♭13(♯9) [tritone sub for C7] |
F△9 – A–9 | D–9 – G–9 C7♭9 | G–13 – C7alt | A♭△7(♯11) – G♭13(♯11) [a chromatic
plane down into the chorus]. **Chorus**: F6/9 – B♭△13 | G–9 – G♭13(♯11) | F△9
– B♭△9 | G–9 C13♭9 – F6/9 | A–9 D7♭9 – G–9 | G–11 – G♭13(♯11) | F△9 – B♭△9 |
G–9 C7alt – F△9. **Hook**: F13 | A♭13 – G♭13(♯11). The voicings are in
`compose.py` (`CH`); the bass walks root, chord tone, chromatic approach.

## Receipts

`python3 render.py` checks every score (`mus check`, must be clean), renders
every World and writes `out/validation.json`: per World `sourceWork`,
`controlWork`, `internalLoss`, `radiation`, `cumulativeClosureResidual`,
`maximumFrameResidual`, the touch counts and the first touch time. It fails
on bad tokens, non-finite ledgers or a frame residual above 1·10⁻⁹ J. The
numbers of the delivered render are in `NOTES.md` next to the delivered
audio (`~/dev/verabot/film/audio/score/rule_britannia/`).

## What these instruments do not model

Everything in the steel-string's "Limits of v0" applies to the three tab
anatomies: no note-off damping, one touch point per cell, 24 modes per body,
a linear model. In addition:

- **The Walker**: plucked only, no bow, no slap, no fingerboard contact;
  the "walk" is a pluck per quarter note and the previous note rings under
  the next until the palm takes it (there is no left-hand damping). The body
  is a scaled guitar box, not a double bass: on the two lowest courses its
  second partial radiates more than the fundamental.
- **The Vibes**: no motor tremolo (the rotating discs that open and close the
  tubes would be a time-varying coupling the score cannot drive), no damper
  pedal (every bar rings until struck again), no mallet choice per note, one
  strike point. The bar tuning 1:4:10 and everything above the third bending
  mode are the Light's design hypotheses.
- **The Kit**: no cymbal law (a thin ring stands in; the ride is darker than
  a real ride and has no wash, because there is no nonlinear mode coupling);
  the wires are a linear sympathetic radiator, not snares rattling on a head;
  no second heads; no brush contact (a sweep is a run of strokes); the hand's
  `pos` is a string law applied by mode index, not a strike point on a
  membrane; the pluck pulse is at most 0.36 ms, shorter than a stick's or a
  beater's contact.
- **The Brass**: everything in `TROMBONE-WORLD.md`'s Limits, in particular
  no brightening with loudness and no room; no mute (a cup or plunger would
  be a change to the declared bell and a recalibration of the player; not
  done here); the three are three Worlds, not a section breathing together.
- **The choir**: everything in the film's list (no aspiration noise, the
  adult's area function throughout, unfitted `o`, no shared room); no
  consonant is a consonant: "b" and "d" are the same released hum, there is
  no "sh", no "w" beyond a slow glide, no words. The slide is a staircase of
  quarter tones, not a glissando.
- **The piano**: everything in `SPEC-ARIADNE-PIANO.md` §7.
- **The ensemble**: eight kinds of World mixed by gain, sharing no space.
  Pans, high-pass filters, the limiter and the loudness normalization are
  the studio's, outside every World (`mix.py`), and so is the PS1 colour.

## Reproduce

From the repository root:

```sh
(cd mus-rs && cargo build --release -p mus-cli)
(cd ovillo && cargo build --release -p ovillo-dsp --no-default-features --example trombone_nd_score)
(cd ovillo && cargo build --release -p ovillo-tract-world --example tract_choir)
python3 examples/ensemble/rule-britannia/build_instruments.py
python3 examples/ensemble/rule-britannia/compose.py
python3 examples/ensemble/rule-britannia/render.py          # ~25 minutes; the piano is the slow World (the rest finish in ~3)
uv run examples/ensemble/rule-britannia/mix.py              # out/mix/rule_britannia.wav, stems/, mix-report.json
```

`out/` is a render directory, not source, and is in `.gitignore`.
