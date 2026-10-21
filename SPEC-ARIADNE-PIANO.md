# Ariadne piano: a grand played on one persistent world

*Draft 0.1, 2026-10-21. Anatomy `piano-v1`. It sits under the [pure-physics
constitution](ARIADNE-PURE-PHYSICS-CONSTITUTION.md), follows the
[steel-string](SPEC-ARIADNE-STEEL-STRING.md) in how an anatomy becomes a
world, and is played from an ordinary [MUS](SPEC.md) score.*

## 1. The idea in one paragraph

A score names an instrument with `anatomy=piano-v1`. For that score Ariadne
builds one persistent world shaped like a 190 cm grand: the soundboard with
its ribs and bridges, and, for every key the score plays, that key's strings,
its hammer and its damper. A note is a key going down. The damper lifts off
the strings, the hammer is thrown at the strings at the velocity of the
written dynamic, its felt is squeezed against them and lets go, and the
strings ring through the bridge into the soundboard, which radiates. When the
key comes up the damper falls back, unless the pedal holds it. Nothing makes a
note: every sound is what the strings and the board do after a hammer hits.
The ledger says where the energy of every throw went.

## 2. Three steps

```bash
cargo build --release -p mus-cli                                     # once, in mus-rs/
mus render ../examples/piano/prelude-c-major.mus -o prelude.wav --json      # play the score
mus render ../examples/piano/prelude-c-major.mus -o prelude.wav --energy e.json   # and keep every body's energy
cargo run --release -p mus-engine --example piano_world -- score.mus out/name     # the World's own output (§8)
```

## 3. The anatomy (what the piano is)

`mus_analysis/piano_anatomy.py` writes `profiles/ariadne-piano-v1.anatomy.json`
and its derivation receipt, which labels every number `manufacturer`,
`measured`, `law`, `fit` or `design-hypothesis`. `mus render` composes one
`ariadne-world-plan-v3` per score from it (§5).

### 3.1 The scale

The published scale of the Estonia "Parlour" grand (Engelbrecht, Mägi & Stulov,
*Proc. Estonian Acad. Sci. Eng.* 5, 1999, Table 1), transcribed from the scan for
all 88 keys: speaking length, distance of the strike point from the agraffe,
strings per choir (1 for A0–F#1, 2 for G1–A2, 3 above), linear density, core and
winding diameters. Each wire is tensioned so that its own first partial,
`f0 sqrt(1 + B)`, sounds equal temperament (A4 = 440 Hz):
`T = mu (2 L f1)^2 - pi^3 E d^4 / (64 L^2)`. These tensions agree with the
table's within 0.1 %. The three strings of a unison are tuned 0, +0.6 and
-0.5 cents (two strings: 0, +0.6), a tuner's unison (design-hypothesis).

### 3.2 The strings: `stiff_wire`

Each string is one `modal_space` body whose map is `stiff_wire`
(`mus-ariadne` `configuration/space.rs`). The map is given what a scale
designer writes down, length `L`, core diameter `d`, linear density `mu`,
Young's modulus `E` (2.0e11 Pa) and tension `T` (a coordinate scales it: the
tuning pin), and the runtime computes the rest:

```text
f0  = sqrt(T / mu) / (2 L)
B   = pi^3 E d^4 / (64 T L^2)          (the core's; a winding adds mass, not stiffness)
f_n = n f0 sqrt(1 + B n^2)             (Euler-Bernoulli with tension, pinned ends)
psi_n(x) = sqrt(2 / (mu L)) sin(n pi x / L)   (mass-normalized shapes)
```

The partials are therefore not authored: a piano part sounds whatever its
wires' material and geometry make of them (gate 3). Up to 24 partials below
21.6 kHz are kept (C4 keeps 24, C6 14, C7 7); the slots above hold no state.
Loss is `sigma(omega) = a + b omega^2` with `a = 0.25 /s`, `b = 6e-9 s`, the
form of Chaigne & Askenfelt's damped string (design-hypothesis values). The
wire radiates as a thin cylinder (a line dipole, `sigma_n = pi^2 rho
omega_n^3 a^4 / (4 c^2 mu)`): booked, but about 70 dB below the board, and not
heard.

### 3.3 The soundboard

An orthotropic spruce plate on the Parlour's outline (traced from the same
paper's Fig. 1, 1480 x 1320 mm, 1.45 m²), 9 mm, with the wood, rib spacing and
loss of Ege, Boutillon & Rébillat (*J. Sound Vib.* 332, 2013): `E_L` 11.5 GPa,
`E_R` 0.47 GPa, `G_LR` 0.5 GPa, 392 kg/m³, ribs every 13 cm, loss factor 2 %.
Glued 25 x 25 mm spruce ribs run across the grain (each bending with the
strip of board it carries); maple bridges, 32 x 35 mm, run along the string
ends; the rim is clamped. Its bending energy is discretized by finite
differences on a 10 x 13 mm grid (11,142 points) and its 720 lowest modes
solved (74 Hz to 7.4 kHz). All modes below 3.0 kHz are kept; above, every
4th mode stands for its neighbours (its bridge shape carries `sqrt(4)`, so the
band's mean bridge admittance and radiated power are kept): 336 modes, 14
Ariadne bodies of 24. This split is exact for a linear plate (its modes meet
only through the bridges).

Each mode loses `pi f eta` to the wood and radiates at its own rate, the
diagonal of the baffled Rayleigh integral over its shape
(`sigma = rho c k^2 / (4 pi) sum_ij phi_i phi_j sinc(k r_ij) dA^2`), booked as
radiation (`SpaceBodyLaws::Passive { radiation_rates_per_s }`: mode-diagonal,
the modal approximation the tube network also makes). One projection cannot
radiate such a band: driven together at one frequency, the modes' forced
responses cancel in it (measured: 1-3 % of their loss radiated instead of
20-25 %).

Mean driving-point impedance at the bridge pins: about 1300 (C3) to 1950 kg/s
(C6) (receipt and §9.8), inside the 1000-10000 kg/s Giordano measured on real
boards (quoted by Bank 2000, §2.3).

### 3.4 The bridge

A string's end is not fixed: it moves with the board at its bridge pin. With
the end displaced by `u_b` the string is `w_s + (x / L) u_b`; for pinned
partials the tension and bending cross terms vanish and the coupling is
kinetic, `u_b' sum_n G_n eta_n'` with `G_n = sqrt(2 mu L) (-1)^(n+1) / (n pi)`.
At resonance partial `n` and board mode `m` then exchange as two resonators
split by `omega_n G_n Phi_m(pin)`, with
`omega_n G_n = (-1)^(n+1) sqrt(2 T / L) sqrt(1 + B n^2)`
(`StiffWireSpec::moving_end_weights`). That is exactly a rank-1 momentum
exchange between the string and a board body: one Ariadne direct coupling per
string and board body, string direction `omega_n G_n`, board direction the
body's mass-normalized shapes at the pin, rate `|omega G| |Phi| / (2 pi)`.
Nothing is fitted. It reproduces the transmission-line decay
`sigma_n = (T / L)(1 + B n^2) Re Y(f_n)`: measured partial by partial on C4
(§9.3).

The three strings of a unison end on pins 4.5 mm apart: their symmetric
motion drives the board strongly, their antisymmetric motions barely. That
is the piano's two-stage decay (prompt sound, aftersound), and the unison
detuning mixes the two.

### 3.5 The hammers

A `hammer` body (`law/hammer.rs`) is a head mass with one coordinate. Its felt
presses on each string of its choir separately, with compression
`delta_i = y - w_i` at the strike point (the string's shape averaged over the
felt's width):

```text
F_i = K delta_i^p + K alpha d(delta_i^p)/dt      (clamped at zero: the felt never pulls)
```

(Chabassier, Chaigne & Joly, *ESAIM: M2AN* 48, 2014, eqs. 4.5-4.8). One step
holds each force constant over the sample and solves the strings (exact
rotation about the shifted equilibrium) and the hammer (exact ballistics)
under it; the elastic force is the discrete gradient of the felt's potential,
so strings, hammer and felt are conserved to roundoff and the relaxation term
is the felt's only loss, booked. The step happens in the World's coupling
stage, after the strings' own propagation, from the inverse rotation of their
state. Masses and head radii are Stulov's measurements (*Proc. Estonian Acad.
Sci. Eng.* 1, 1995, Table 1: 13, 10.6, 8.2 g at keys 1, 37, 73), scaled so C4
is the 8.9 g hammer whose felt law that table quotes from Chaigne & Askenfelt:
`K = 142.3 N/mm^2.5`, `p = 2.5`. Elsewhere `p` rises 0.5 every three octaves
toward the treble (2.2 to 3.2) and the force at 1 mm follows the Hertz-like
geometry factor of Stulov's eq. 18; `alpha = 10 us` (design-hypotheses).

A throw places the hammer 2 mm (letoff) below the nearest string at the
velocity the action gave it and books `1/2 M v^2` as source work. After the
rebound the backcheck catches it 8 mm below the strings and books its kinetic
energy as loss. Velocity (dynamics) changes the timbre only through the felt:
a harder throw compresses the felt further, where it is stiffer, so the
contact is shorter and its force pulse richer (gate 4).

### 3.6 The dampers

A `damper` (`law/damper.rs`) is a viscous felt pad on a string: 15 mm long,
its centre at 1.32 x the strike distance (on A#4 that puts a node under it
every 7th partial, which rings through, as Bank 2000 §6.1 measured), its
resistance set so that it damps the fundamental at 10 /s (time constant
0.1 s, the middle of Bank's measured 0.05-0.15 s). Its engagement (how fully
the felt rests, 0 to 1) is a World control; a dashpot only dissipates, so
nothing is billed, and its loss is booked. The top 19 keys (above F6) have no
dampers.

### 3.7 What is heard

Two far-field points 1 m from the board, 30 degrees off its normal, leaning to
the bass and to the treble side: each board mode's far-field pressure there
from the Rayleigh integral over its shape (`D_k = sum phi_k e^(-i k u.x) dA`,
`p = -(rho / 2 pi r) sum_k (omega_k^2 Re D_k eta_k + omega_k Im D_k eta_k')`),
a linear non-loading readout of the board's state (`StereoStateTap`). There
is no room: the World is heard dry. `mus render` multiplies the pressure (Pa)
by 0.1 and mixes it into the dry bus like any voice; the engine's bus then
applies its 28 Hz high-pass and its master (RMS target, look-ahead limiter,
soft clip, trim) to every render (§8).

## 4. The score

```text
# instruments:
#   pn   = grand piano (treble, anatomy=piano-v1)
#   pn.l = left hand (bass)
#   pn.r = right hand (treble)
#   pn.ped = sustain pedal (perc, pedal=sustain)
bar 1: pn.l={mp} C4h C4h  pn.r=Re G4s C5s E5s G4s C5s E5s Re G4s C5s E5s G4s C5s E5s  pn.ped=Xw
```

- Any dot-voice of the piano is a staff (as many as the music needs; the
  demo uses three to hold Bach's overlapping left-hand notes).
- A pitch is a key (A0 to C8). A tab position (`@s:f`) or a pitch the piano
  has no key for is a bad token.
- The dynamic sets the hammer velocity: ppp 0.34, pp 0.5, p 0.74, mp 1.09,
  mf 1.6, f 2.35, ff 3.45, fff 5.1 m/s (Chabassier et al. play p at 0.5 and ff
  at 4 to 4.5 m/s; Bank puts a piano's largest near 6 to 7). Accents and
  `[gain=]` scale it: +12 dB doubles it.
- A note's duration is how long its key is held (ties extend it, `stac`
  shortens it). Its damper lifts 15 ms before the hammer arrives and falls
  8 ms after release, settling over 12 ms.
- The pedal voice: `X<duration>` holds the pedal down from its onset plus
  80 ms (`lag=` sets it: the foot catches the new harmony, syncopated
  pedalling) to its end; a rest lets it up. While the pedal is down every
  damper is up.

## 5. The world, per score

Only the keys a score plays are strung. The world is the 14 board bodies,
then per key its strings, its hammer, and one bridge edge per string and
board body (the demo: 54 bodies, 420 edges). Every strike, contact time and
damper fall is in the receipt; the composition is the `planDigest`.

## 6. Receipts and the energy trace

`ariadneWorlds[]` in the render receipt: `anatomy`, `planDigest`,
`listenerDigest`, `bodies`, `keys` (each with its strings, retained partials,
`f0`, `B`, strike fraction, hammer mass, whether it is damped), `strikes`,
`refusedStrikes` (a throw while the felt still presses), `damperFalls`,
`pedalSpans`, the shortest and longest felt contact, and the ledger:
`sourceWork` (the hammers' throws), `controlWork`, `internalLoss`,
`radiation` (board and strings separately), and the closure residuals.

`--energy` writes `mus.ariadne.energy-trace.v1` with one world per piano:
every body's stored energy each 10 ms (board bodies, strings, hammers, then
the room), what each body took in, radiated and lost in each hop, the energy
the bridge moved into each string, every strike (time, key, velocity,
release, contact, peak force) and the pedal spans.

## 7. What it does not model, said plainly

- **One transverse polarization.** No horizontal string motion, so no
  polarization part of the two-stage decay; no longitudinal waves, no
  phantom partials, no tension modulation (the strings are linear).
- **No sympathetic strings beyond the score's keys.** Only the keys a score
  plays are strung; with the pedal down the rest of the scale does not ring
  along.
- **The action is a throw.** No key, wippen, shank, escapement dynamics,
  repetition lever, key bounce or action noise; no hammer-shank vibration;
  gravity on the hammer's few millimetres of flight is ignored. A key cannot
  be re-thrown while its felt still presses (refused and counted).
- **The board is flat and the frame rigid.** No crown, no downbearing, no
  board taper, no rim or case motion, no lid, no iron frame resonances; the
  bridges are beams along the string ends with no torsion; board modes
  above 3 kHz are sampled every 4th.
- **Radiation is baffled and diagonal in the modes,** no cross-radiation
  between modes; heard at two far-field points, dry, with no room.
- **The felt** is a power law with a relaxation term; its parameters away
  from C4 are design-hypotheses, and so is the damper away from A#4.
- **Pitch is set per string, alone**: where a string partial sits on a board
  resonance the coupled pitch moves off it (C4 is the worst: -5.6 cents);
  a tuner would correct it, this piano is not re-tuned after it is built.
- It is a piano-shaped Ariadne world, not a model of the Estonia or of any
  particular piano.

## 8. The World's own output and the bus

`mus render` mixes the listener's pressure (x 0.1) into the engine's bus, so a
piano render carries the bus's 28 Hz Butterworth high-pass and its master
stage, as every render does. `examples/piano_world.rs` writes the same
performance without them (stereo f32), with its receipt, energy trace and a
notes TSV; all measurements in §9 are made on it.

## 9. Gates

Measured on the World's own output (§8) of `examples/piano/probe-*.mus` and
the demo, with `mus_analysis/piano_analyse.py` (both listening points, powers
added). 48 kHz throughout.

1. **Nothing else moves.** Steel-string tour and *Wayfaring Stranger*:
   WAV and `--energy` trace byte-identical to a `9bed930` build (tour
   `renderDigest sha256:fa3d615f...`), tour receipt equal but for wall time.
   mus-ariadne lib 61, nd_configuration 6, struck_wire 6; mus-notation 77,
   mus-engine 3; mus-cli ariadne_tab 2, check_report 2, ariadne_piano 2,
   render_receipt 2; ovillo-dsp 37 + 2, ovillo-tract 6 + 7,
   ovillo-tract-world 1 + 3 + 6 + 3.
2. **The ledger closes.** Demo, 832,000 frames: largest frame residual
   1.04e-17 J, cumulative -1.08e-14 J; source work (64 throws) 0.2952 J,
   internal loss 0.2759 J, radiation 0.01926 J (board 0.019257 J, strings
   1.6e-6 J). Every probe: frame residual at most 4.2e-17 J.
3. **The partials are the stiff string's.** Range probe (C3 to C6 every
   third semitone, mf), partials above -45 dB against `n f0 sqrt(1 + B n^2)`
   with each wire's own `f0` and `B`: median deviation 0.8-1.1 cents per
   note; largest 0.9-2.2 cents except where a board mode pulls one partial
   (C3 partial 2: 7.6 cents; C4 partial 1: 6.1). A least-squares fit of
   `(f0, B)` to the measured partials returns `B` = 1.005 x the wire's for C3
   (wire: 1.50e-4), 1.116 x for C4 (3.29e-4; the pulled first partial biases
   it), 0.969 x for C6 (2.80e-3). The bridge: the C4 partials' decay through
   it equals `(T / L)(1 + B n^2) Re Y(f_n)` from the board's own modes within
   about 10 % for partials 1-16 (measured on the board with projection
   radiation, before `77db03d`).
4. **A harder blow is brighter, through the felt.** C3 / C4 / C5 at
   pp 0.5, mf 1.6, ff 3.45 m/s: felt contact 3.15 / 2.77 / 2.60 ms,
   2.58 / 2.19 / 1.98 ms, 2.08 / 1.56 / 1.42 ms; spectral centroid of the
   first 0.5 s 369 / 426 / 439 Hz, 336 / 383 / 398 Hz, 550 / 610 / 667 Hz;
   energy above 1 kHz -20.8 / -12.7 / -11.2 dB, -17.6 / -13.1 / -12.6 dB,
   -14.0 / -10.2 / -7.8 dB. C4 partials 9-14 come up about 20 dB from pp to ff
   against its fundamental.
5. **Pitch against equal temperament.** First partial, range probe:
   C3 +0.40, Eb3 +0.06, F#3 -1.15, A3 -1.42, C4 -6.07, Eb4 +2.22, F#4 +0.81,
   A4 +1.37, C5 +0.02, Eb5 +0.07, F#5 +0.01, A5 +1.49, C6 +1.36 cents (median
   |1.15|); C2 -4.26. The wires are tuned alone (§3.1); the offsets are the
   board's (C4 sits 2.3 Hz below a board mode at 263.9 Hz).
6. **The same score twice gives the same bytes.** The demo rendered twice:
   identical WAV (`renderDigest sha256:997ce53b...`); also asserted in
   `mus-cli/tests/ariadne_piano.rs`.
7. **A released key is damped.** C4, mf: released after 0.5 s it falls
   25 dB in the next 0.3 s; held, its early decay is about 9 dB in 0.3 s;
   released under the pedal, 5 dB.
8. **Decay by register** (held 4 s, mf; T60 from the first 10 dB, then from
   10 to 30 dB below the peak): C2 6.7 / 13.1 s, C3 1.5 / 7.7 s, C4 1.8 /
   4.8 s, C5 2.4 / 1.9 s, C6 2.1 / 9.0 s. Across C3-A5 (range probe) the early
   T60 runs 1.6-5.5 s from note to note. The bass is long and every note
   decays in two stages. The prompt sound is faster than the one published
   example at hand (Weinreich's Eb3, about 8 dB/s; this Eb3 about 14 dB/s),
   and the treble's aftersound is long (C6), held up only by the strings'
   own loss: the board sits at the soft end of the measured impedances
   (§3.3), and the antisymmetric unison motions barely reach it.

The demo (`ovillo/renders/piano-demo*`): 17.33 s rendered, 13.33 s of
music and the last chord held to 15.83 s; 64 strikes, none refused, 37
damper falls, 4 pedal spans, felt contact 1.50-2.58 ms. `mus render`'s
output equals the World's output times one gain after the bus's 28 Hz
high-pass to within -45 dB (the rest is the master's soft clip and 24-bit
quantization); the two listening points correlate at 0.03. 40 s of wall time
for the 17.3 s on a shared laptop.

## 10. Where to touch the code

| piece | path |
|---|---|
| stiff wire (partials, shapes, bridge weights, dipole) | `mus-rs/crates/mus-ariadne/src/configuration/space.rs` |
| hammer (felt contact law and exact step) | `mus-rs/crates/mus-ariadne/src/law/hammer.rs`, `src/shared_world/hammer.rs` |
| damper | `mus-rs/crates/mus-ariadne/src/law/damper.rs` |
| passive bodies, modal radiation | `mus-rs/crates/mus-ariadne/src/space_body.rs`, `src/plan/space.rs` |
| state taps | `mus-rs/crates/mus-ariadne/src/observation/listener.rs` |
| anatomy compiler | `mus_analysis/piano_anatomy.py` → `profiles/ariadne-piano-v1.anatomy.json` (+ `.receipt.json`) |
| score → keys, pedal, dampers, world | `mus-rs/crates/mus-engine/src/ariadne_piano.rs` |
| the World's own output | `mus-rs/crates/mus-engine/examples/piano_world.rs` |
| measurement | `mus_analysis/piano_analyse.py` |
| tests | `mus-rs/crates/mus-ariadne/tests/struck_wire.rs`, `mus-rs/crates/mus-cli/tests/ariadne_piano.rs` |
| demo and gate probes | `examples/piano/*.mus` |
