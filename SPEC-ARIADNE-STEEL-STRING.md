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
is a piecewise-linear warp, so 16ths ride inside the swung pair.

- **pluck:** the fretting finger lands 8 ms early (configuration amount = fret/12).
  Then a raised-cosine pulse drives the string's contact port. Its loudness comes
  from the dynamic, accents, and the hand's and note's `gain`.
- **`ho` hammer-on:** the stop jumps under a ringing string, with a small, sharp tap.
- **`po` pull-off:** the stop jumps, and the leaving finger plucks at half strength.
- **`sl` slide:** the stop travels from the old fret to the new one over 60 ms. Energy
  already in the string is carried along by transport, and the receipt bills the work.

Then the world steps, a stereo listener observes all bodies, and the observation
is mixed into the engine's dry bus. The strings radiate almost nothing
(radiation fraction 2·10⁻⁶) and the box radiates 0.02, so what you hear is
mostly the top and the air.

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
- It is a guitar-*shaped* Ariadne body, not a model of any particular guitar.

## Where to touch the code

| piece | path |
|---|---|
| notation (`@string:fret`) | `mus-rs/crates/mus-notation/src/lib.rs` (`split_tab`, `TabPosition`), `mus_audio.py` (`split_tab`) |
| anatomy compiler | `mus_analysis/steel_string_anatomy.py` |
| plan | `profiles/ariadne-steel-string-v0.plan.json` (+ `.receipt.json`) |
| hands + performance + world | `mus-rs/crates/mus-engine/src/ariadne_tab.rs` |
| probe (pitch, decay, ledger per string) | `cargo run --release -p mus-ariadne --example steel_string_probe -- profiles/ariadne-steel-string-v0.plan.json` |
| a full score | `examples/tab/wayfaring-stranger-doc.mus` |
