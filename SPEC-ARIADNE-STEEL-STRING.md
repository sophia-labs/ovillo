# Ariadne steel-string: playing a guitar-shaped world from a tab

*Draft 0.1, 2026-10-14. Profile `steel-string-v0`. It sits under the [pure-physics
constitution](ARIADNE-PURE-PHYSICS-CONSTITUTION.md) and reads scores written in
[MUS-T](SPEC-TAB.md).*

## The idea in one paragraph

You write a score as an annotated tab: which string, which fret, which hand,
when. Ariadne builds one persistent physical world shaped like an acoustic
guitar:

- six strings;
- a bridge;
- a spruce top with a soundhole;
- the air inside the box;
- a small room.

Then it plays your tab by **touching** that world. A pluck pushes a string
through a fingertip-sized contact. A fret moves the string's speaking length. A
slide moves it continuously. Nothing in the renderer makes a note. Every sound
you hear is energy that a touch put into a string, flowed through the bridge into
the top and the air, and radiated into the room. The ledger at the end says how
much energy went in, where it was lost, how much was radiated, and that the
books balance.

## Three steps

```bash
cargo build --release -p mus-cli                       # once, in mus-rs/
mus render my-tune.mus -o my-tune.wav                   # play the tab on the world
mus render my-tune.mus -o my-tune.wav --energy e.json   # also keep every body's energy, 100 times a second
```

The receipt that `mus render` prints has an `ariadneWorlds` entry. It records the
plan digest, which hand played each string and how, how many plucks, hammer-ons,
pull-offs and slides there were, and the energy ledger.

## Layer 1: the anatomy (what the guitar is)

`mus_analysis/steel_string_anatomy.py` compiles the guitar from declared geometry
into an ordinary `ariadne-world-plan-v2`, `profiles/ariadne-steel-string-v0.plan.json`.
It is built into `mus` as `anatomy=steel-string-v0`. Next to the plan, a derivation
receipt (`...receipt.json`) labels every number as `manufacturer`, `fit` or
`design-hypothesis`.

**The box** is one Ariadne body whose 24 modes are computed, not chosen:

1. The top is a thin orthotropic plate, spruce stiff along the grain and soft across
   it, on a dreadnought outline with the soundhole cut out. It is clamped at the
   rim and the rosette, and the bridge mass sits on it. Its bending energy
   `½∫ D_x w_xx² + 2D₁ w_xx w_yy + D_y w_yy² + 4D_k w_xy²` is discretized on an
   8.5 mm grid (1,866 points) and solved for its lowest modes.
2. The air in the box is a Helmholtz resonator: a plug of air in the soundhole on
   the spring of the cavity's volume (129 Hz alone).
3. The two talk through **volume**. When the top bulges out, the cavity grows and
   the pressure drops, and that pressure pushes on both the top and the air plug.
   This gives one coupled eigenproblem, top modes plus air, and its lowest
   solutions are the guitar's signature pair:
   - **A0 at 104.5 Hz**, mostly air;
   - **T(1,1) at 194.7 Hz**, the top breathing.

   Above those come a cross-dipole at 296.5 Hz and 21 more plate modes up to 2.2 kHz.
   These are where steel-string literature puts them. The bracing is the only fit:
   two stiffness factors that fold the X-brace into the plate.
4. The solve keeps **72 coupled modes** (to 6.15 kHz). An Ariadne body holds 24, so
   the box is three bodies (`box/0` to `box/2`). This split is exact: a linear plate's
   modes don't couple to one another, only through the bridge.
5. Each coupled mode carries two numbers onward:
   - how much it moves the saddle, which is the bridge coupling;
   - how loudly it radiates. Ariadne emits a body's momentum projected on its
     radiation direction, which is a volume velocity, and far-field pressure is its
     time derivative. So the weight is (net pumped volume + an edge/corner term that
     grows toward the 4 kHz coincidence frequency) × frequency. A0 and T(1,1) pump air,
     the higher modes mostly cancel their own volume, and that is why a guitar sounds
     the way it does.

**The strings** are six Ariadne bodies, each the first 24 partials of a stiff
steel string with real gauges and tensions (103–120 N, light phosphor bronze):

- `f_n = n·f₀·√(1 + B n²)`, where the inharmonicity `B` comes from the core diameter.
- Each partial decays at `σ = 0.55 + 1.9·10⁻⁷ ω²`, so high partials die first.
- Each string's configuration path runs from **open** (amount 0) to the
  **twelfth fret** (amount 1). Ariadne interpolates frequency in log space, so
  **fret k is amount k/12, exactly**, and a fret change is lawful transport with
  a work receipt, not a new voice.

**The bridge** is eighteen reciprocal edges, each string ↔ each box body. Each is a
lossless Givens exchange of momentum projections. Edges to the low body run at
45 Hz. Edges to the higher bodies run faster (302 and 757 Hz), because the saddle's
mobility rises across the plate's high band (`bridge_rise`, a fit):

- On the string side, the direction is the slope at the saddle. In
  energy-normalized coordinates, every partial weighs the same there, alternating in sign.
- On the box side, the direction is each mode's motion at the saddle.

Energy leaks from string to top slowly, because a bridge is stiff, and the top
spends it on radiation. Make the low edge stronger and the strings start to pull on
the body: a 120 Hz edge drags the open low E 14 cents flat. That is mode
repulsion, and real guitars do it too. Each box body radiates at a rate that grows
with its band, again because radiation efficiency rises toward coincidence. The
extra damping a real top shows at high frequency is therefore booked as radiation,
not as wood loss.

## Layer 2: the hands (how the score touches it)

The `gt.*` dot-voices are hands (MUS-T §3). Before the world is built, each
string is folded together with the hand that plays it most often. This is
geometry, not EQ:

| hand parameter | what changes in the string body |
|---|---|
| `pos` | source direction becomes `sin(nπ·pos)/n`, the shape of a string released at that point. Near the bridge (0.10), a thumb thump is bright and thin; over the soundhole (0.20), a finger is round |
| `pick` | the contact port's stiffness (380 + 900·pick Hz) and damping, and the contact time of the pluck pulse (0.06 + 0.3·(1−pick) ms). The string slips off a fingertip in a few tenths of a millisecond; a longer pulse would low-pass the pluck itself |
| `pm` | a resting palm: extra loss `pm·(24 + 1.2·10⁻⁵ ω²)` per second on every partial. The bass thumps, around 0.4–0.7 s, instead of ringing |

A tuning other than the anatomy's (`tuning=D2 A2 D3 G3 B3 E4`) retunes the
courses. A capo shifts the base amount.

## Layer 3: the performance (what happens each sample)

The tab compiles to **touches**, timed with the engine's bar map and swing. Swing
is a piecewise-linear warp, so 16ths ride inside the swung pair. A touch is a
pluck, a hammer-on, a pull-off, a slide, or a bowed note.

- **pluck:** the fretting finger lands 8 ms early (configuration amount = fret/12).
  Then a raised-cosine pulse drives the string's contact port. Its loudness comes
  from the dynamic, accents, and the hand's and note's `gain`.
- **`ho` hammer-on:** the stop jumps under a ringing string, with a small, sharp tap.
- **`po` pull-off:** the stop jumps, and the leaving finger plucks at half strength.
- **`sl` slide:** the stop travels from the old fret to the new one over 60 ms. Energy
  already in the string is carried along by transport, and the receipt bills the work.

- **`arco` bow:** the hair rests on the string for the note's duration, at the
  bowing hand's `pos`; see "The bow" below.

Then the world steps, a stereo listener observes all bodies, and the observation
is mixed into the engine's dry bus. The strings radiate almost nothing
(radiation fraction 2·10⁻⁶) and the box radiates 0.02, so what you hear is
mostly the top and the air.

## The bow

*Added 2026-11-05. The law is `mus-ariadne/src/law/bow.rs`; the World binds it in
`shared_world/bow.rs`; the score reaches it through `ariadne_tab.rs`. It brings
the older engine's bow (`SPECS/playable/PW10-bodies-bow-breath.md` §B, "a friction
port that is honestly a source") into the World and its ledger.*

A bowed note is not a pluck that lasts. The hair is a friction port on the
string: it drags the string along while static friction holds (**stick**), the
string tears free and snaps back when the restoring force wins (**slip**), and
the bridge-reflected wave re-captures it one period later. That is Helmholtz
motion, and it is what the World does here; nothing synthesizes it.

**The law.** The bow is a point contact at fraction `beta` of the speaking length
(the hand's `pos`), with the string's partials there, `psi_n = sin(n pi beta)`.
Each sample, the force `F` is held constant over the step. A modal oscillator
under a constant force is solved exactly (the piano's hammer uses the same step),
so the string's field changes by exactly `F (w' − w)`, with `w` the bow-point
displacement. `F` is found the way McIntyre and Woodhouse draw it (1979): the
friction curve `F = F_N mu(dv)`, with `mu(dv) = sign(dv) (mu_d + (mu_s − mu_d) /
(1 + |dv| / v0))`, intersected with the string's load line `v(F) = (dw_free +
g F) / dt` (`g`: the bow point's compliance to a force held over the step). If
the force that carries the bow point exactly with the hair is within `mu_s F_N`,
the string sticks; otherwise it slips on the falling branch. Where both are
possible, the hysteresis rule keeps the regime the string was in. `mu_s = 0.8`,
`mu_d = 0.3` are PW10's starting points; `v0` is where this string's bowed
notes speak (`BOW_SLIP_VELOCITY`), all three design-hypotheses.

**The books.** The arm moves the bow at `v_bow` against `F`: its work `F v_bow dt`
is the bow's **source work**, inside the World's `sourceWork`. The string receives
`F (w' − w)`; the difference, `F dv dt`, is zero while sticking and positive while
slipping, and is booked as **friction loss** (heat in the hair and rosin), inside
`internalLoss`. Source minus loss is the string's gain to roundoff, so the ledger
closes with the bow in it. No passivity is claimed for a bowed string: the falling
friction branch is negative resistance, and the arm pays for it. A bow with no
force steps and books nothing; a World with no bow attached runs the code path it
ran before.

**Units.** The bow's velocity and force are in the string body's own units, and
the scale is measured, not derived (`mus-ariadne/tests/bowed_string.rs`):
`speed = 1` is `1.6·10⁻⁵`, `force = 1` is `0.16`, `v0 = 10⁻⁶`. A mezzo-forte
bow, `speed = force = 0.5`, holds G3 at the loudness of a mezzo-forte pluck's
first half second (heard RMS 4.13·10⁻⁵ against 3.73·10⁻⁵ in the World's own
output), sticking 85 % of each period and releasing once per period. Loudness
follows speed; `force = speed` is the centre of the region where the string
speaks (Schelleng's region, measured below).

### How a score opts in

```text
#   gt.b = bow (treble, pos=0.12)            # a hand whose pos is the bow point
bar 1: gt.b={mf} G3w@3:0[arco]               # a down-bow for a whole note, mf speed
bar 2: gt.b=G3w@3:0[upbow,speed=1.0->0.2]    # an up-bow, slowing: a diminuendo
```

- Any hand can bow: `pos` is the bow point (0.08–0.15 is a bow's place; a plucking
  hand's 0.18–0.20 bows too, rounder). One bow rests on each bowed string, at the
  `pos` of the hand that bows it most.
- `[arco]`, `[downbow]`, `[upbow]` mark a bowed note (SPEC-TAB §4). Its duration is
  how long the hair stays; the force lands and lifts over 10 ms. At the end the
  string rings down on its own (there is still no damping at note-off).
- The dynamic sets the speed (`mf` 0.5, `p` 0.18, `f` 0.79, `ff` 1.0); `speed=`
  and `force=` override it, as a value or a linear ramp `a->b` across the note.
  A crescendo is a rising speed; a swell is a down-bow rising and an up-bow
  falling, as in `examples/tab/bowed-swell.mus`. Hairpins (`{<}`) do not drive the
  bow yet; write the ramp on the note.
- A plucked score gains a bow by adding `[arco]` to the notes that should sustain,
  and nothing else changes: an unbowed score renders the bytes it rendered before.
  To bow the film's ostinato on `verabot/score-instruments`, give its sustained
  notes `[arco]` (alternating `[downbow]`/`[upbow]` for long lines), keep their
  `@string:fret`, and write `[...]` without spaces.
- The receipt's `ariadneWorlds[].bow`: `bowedNotes`, `strings[]` (string, hand, pos),
  `sourceWork` and `frictionLoss` (the bow's part of the ledger), `bowedSeconds`,
  `stuckFraction`, `releases` (stick-to-slip transitions; one per period in
  Helmholtz motion, plus the attack's), `peakForce`. The energy trace gains
  `bows`, `bowColumns` and `bowFrames` (per hop and per bow: stuck fraction,
  releases, force, source work, friction loss), and each bowed touch carries its
  direction, speed, force and seconds.

### Gates (`mus-ariadne/tests/bowed_string.rs`, `mus-cli/tests/ariadne_bow.rs`)

Measured on the World's own output, G3 (`string/3`) bowed at `pos = 0.12`, 48 kHz.

1. **Nothing else moves.** A World with a bow attached and no force plucks the
   same bytes as one without (`assert_eq!` on the samples and the ledger). The tour
   renders `renderDigest sha256:fa3d615f…` and the same `--energy` trace as before
   the bow existed; its receipt gains no key.
2. **The ledger closes with the bow in it.** The mf note, 2.5 s: bow source work
   5.33·10⁻⁷ J, friction loss 4.92·10⁻⁷ J (92 %), the rest into the string, the
   box and the room; world source work equals the bow's; cumulative closure
   residual −3.2·10⁻²⁰ J, largest frame residual 6.8·10⁻²¹ J. The demos:
   `bowed-note` 12 s, cumulative 2.5·10⁻¹⁷ J; `bowed-swell` 28 s, −1.7·10⁻¹⁷ J.
3. **A bowed note holds.** Heard RMS 4.13·10⁻⁵ at 0.5–1.0 s and 4.13·10⁻⁵ at
   1.5–2.0 s (the same note plucked mf: 3.73·10⁻⁵ in its first half second,
   4.7·10⁻⁷ at 1.5–2.0 s, 88× below the bowed). The demo's envelope is flat within
   0.2 dB for six seconds.
4. **It is Helmholtz motion.** Stuck 0.845 of the period, releases 198/s against
   `f0` 196.0 Hz; pitch 195.8 Hz (−1.4 cents). Through the swell (0.2 → 1.0 → 0.2),
   stuck 0.80–0.85 throughout, 196–198/s in every half-second window after the
   attack's down to speed 0.4 (222/s in the last, at 0.2), pitch within −2.3 cents. The demo trace: 196–198 releases/s on G3 from 0.5 s on;
   146–150/s on D3 (146.8 Hz) through both bows of the swell.
5. **The bow lifts.** Lifted after 1.5 s: 0.5 s later the heard level is 64 % of
   the held level, 1.5 s later 3.3 %; a lifted bow books nothing more.
6. **An up-bow mirrors a down-bow** exactly (worst `|down + up|` 0 at peak
   6.6·10⁻⁵), with the same stuck count and source work.
7. **The region, measured** (`the_speaking_region_is_declared`, 2 s, last half
   second): with `force = speed`, releases/s and pitch at speeds 0.2 / 0.5 / 1.0:
   234 (−1.4 c) / 196 (−1.4 c) / 196 (−2.3 c); at a third of that force 694 / 486 /
   446 (too light: multiple slipping); at three times 224 / 372 (−30 c) / 358
   (+51 c) (too heavy: the pitch flattens, then breaks). Heard level is proportional
   to speed (1.7·10⁻⁵, 4.1·10⁻⁵, 8.1·10⁻⁵).
8. **A swell follows the bow** (`examples/tab/bowed-swell.mus`): −13.9 dB at the
   start, 0 dB at the bow change, −13.6 dB at the end of the up-bow, then the ring-down
   (−27.8 dB two seconds after the lift); pitch within ±1.4 cents throughout.

### What the bow does not model, said plainly

- **The attack is slow.** From the landing, the string slips several times a period
  for 0.25–0.7 s before the Helmholtz regime locks (548–684 releases/s in the first
  quarter second of the mf note; the demo's first half second averages 640/s). No
  landing or acceleration law shortens it (measured in `sweep_attack`); a real
  attack settles in tens of milliseconds, helped by the hair's compliance and the
  string's torsion, neither of which is here.
- **Secondary slips below speed 0.4.** With `force = speed`, the period carries
  10–20 % extra releases at speeds 0.1–0.3 (ripple on the sticking phase of a
  24-partial string); the pitch and the stick fraction hold. The default force law
  is not tuned around it.
- **Bow force flattens the pitch** by tens of cents at three times the default
  force and the tone breaks beyond: the heavy edge of the region is usable only
  as an effect, and is not offered as one.
- The bow is a velocity source: no bow mass, no hair compliance, no hair width (a
  point contact), no torsional string motion, no thermal rosin law. The friction
  curve and its constants are design-hypotheses.
- One bow per string, at one point; a bow on two strings at once is two bows.
- Speed and force ramp linearly across a note; hairpins do not reach the bow.
- The bowed tone's spectrum is the 24-partial string's through this box; the
  high-frequency bridge limit above applies.

## Receipts

`ariadneWorlds[]` in the render receipt: `planDigest`, `listenerDigest`, `bodies`,
counts of each touch, `strings[]` (hand, pos, pick, palm per string), and the
ledger. The ledger lists `sourceWork` (what the hands put in), `controlWork`
(what the fretting hand paid), `internalLoss`, `radiation`, and the closure
residuals. The first full render, a 98.7 s Doc Watson–style *Wayfaring
Stranger* with 417 touches, closed its books to 9·10⁻¹⁶.

`--energy` writes `mus.ariadne.energy-trace.v0`: per body, plus the room, stored
energy every 10 ms. It is the raw material for a picture of a pluck travelling
string → bridge → top/air → room.

## Limits of v0, said plainly

- **No damping at note-off yet.** A note rings until its string is touched again,
  or until its own losses and the palm take it. A world control for contact
  damping, a finger or palm landing on a ringing string, is the next step.
  Without it, `[stac]` and lifted fingers can't be heard.
- **Each string has one pluck point,** its hand's. Moving the pluck point per note needs
  a per-touch source direction.
- **24 partials per string.** The low E reaches about 2 kHz and the high E about 8 kHz.
  This is plenty for a palm-muted thumb, thin for a bright open bass.
- The back, ribs and neck are rigid. Bracing is folded into the plate's effective
  stiffness. Radiation is monopole-weighted, with no directivity.
- The model is linear. There is no tension modulation and no fret buzz; those are
  constitutive nonlinearities the constitution invites (§3.3).
- **High frequencies cross the bridge weakly.** On its own, the strings' spectrum is a
  plausible steel-string's. With a single-direction momentum exchange per edge, a
  partial above about 1.5 kHz meets each box mode only through a thin slice of the
  24 × 24 pairs. So the band-rising bridge rate is a fit, not a law. The open
  question for the coupling law is a per-band or multi-rank bridge edge whose
  exchange follows the measured bridge mobility. Today the guitar is warm: the tour's
  spectral centroid is about 520 Hz, with 1–2.5 kHz at −20 dB and 2.5–5 kHz at −26 dB
  against 80 Hz–5 kHz.
- Harmonics (`[harm]`) are checked against pitch but played as ordinary plucks.
- The bow has its own list: "What the bow does not model", above.
- It is a guitar-*shaped* Ariadne body, not a model of any particular guitar.

## Where to touch the code

| piece | path |
|---|---|
| notation (`@string:fret`) | `mus-rs/crates/mus-notation/src/lib.rs` (`split_tab`, `TabPosition`), `mus_audio.py` (`split_tab`) |
| anatomy compiler | `mus_analysis/steel_string_anatomy.py` |
| plan | `profiles/ariadne-steel-string-v0.plan.json` (+ `.receipt.json`) |
| hands + performance + world | `mus-rs/crates/mus-engine/src/ariadne_tab.rs` |
| the bow (law, World binding, gates) | `mus-rs/crates/mus-ariadne/src/law/bow.rs`, `src/shared_world/bow.rs`, `tests/bowed_string.rs`; `mus-cli/tests/ariadne_bow.rs` |
| bowed demos | `examples/tab/bowed-note.mus`, `examples/tab/bowed-swell.mus` |
| probe (pitch, decay, ledger per string) | `cargo run --release -p mus-ariadne --example steel_string_probe -- profiles/ariadne-steel-string-v0.plan.json` |
| a full score | `examples/tab/wayfaring-stranger-doc.mus` |
