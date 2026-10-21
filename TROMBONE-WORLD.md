# A trombone that is one Ariadne World

## v1: one N-D body (branch `ariadne/trombone-world-v1-20261020`, on `poc-3-20261020`)

*Needs `patches/mus-ariadne-tube-length.patch` applied to `mus-ariadne` (not
applied on this branch, as asked). On an unpatched runtime the v1 plan refuses
to load with a message naming the patch, and its tests skip. v0 (below)
still builds and plays.*

### What it is

The World has one `modal_space` body. Plan `profiles/ariadne-trombone-v1.plan.json`
(`ariadne-world-plan-v3`) is compiled by `mus_analysis/trombone_nd_anatomy.py`;
its receipt sits beside it.

- **Bore (map `tube_network`):** the declared tenor bore of v0 plus a declared
  mouthpiece, as five tubes in series, 124 volumes: mouthpiece (cup, throat,
  backbore), leadpipe, slide, bell tail, bell mouth. The runtime computes the 24
  lowest modes of the lossless Webster network at every point. Wall loss is
  Kirchhoff's per tube (the runtime's α is frequency independent; evaluated at
  150 Hz). The bell mouth is the radiating end, through the runtime's conductance
  law, booked as radiation. There is no cup body and no fitted throat weighting:
  the lips act on the bore's own air.
- **Slide:** the body's one coordinate, `slide_extension_m` (0 to 0.72 m per
  leg). The slide tube's length is 1.19 m + 2x, a `tube_length` term (the patch).
  Moves use the `mass_momentum` connection (the air state Cp, Lu is held; the
  work is billed; energy the 24 modes cannot hold is radiated). Positions are
  solved on the network modes: 0, 8.8, 18.0, 27.7, 38.0, 48.8, 60.1 cm.
- **Lips:** the runtime's flow valve (`law/valve.rs`): a mass on a tension spring
  with contact, and a Bernoulli + Poiseuille slit jet into the cup volume.
  - The lungs are the body's reservoir. Their refill is the only source work;
    mouth pressure is the valve's supply.
  - Lip tension is the valve's tuning, moved with `World::set_body_valve_tuning`
    and billed as control work.
- **What you hear:** the World's own emission (the bell volume's pressure, Pa)
  × `OUTPUT_SCALE` = 1/300 (set once).

### The patch (`patches/mus-ariadne-tube-length.patch`, 158 lines)

A trombone slide changes a tube's length. The tag's tube network fixes
`length_m` and lets only areas depend on the coordinates, and no area function
can remove travel time. The patch adds `AreaTermSpec::TubeLength { tube,
length_m: AffineSpec }`: the tube's sections stretch uniformly, and `dx` and the
column halves are recomputed per point. It is a new enum variant, so no existing
struct literal breaks, and specs without it serialize as before. Tests in
`configuration/network.rs`:
- `tube_length_term_is_a_slide_on_the_quarter_wave_series`: a uniform tube's
  modes equal the staggered grid's closed form at every length to 1e-10, and a
  non-positive length is refused by the band check;
- `a_slide_move_is_transported_and_old_specs_serialize_unchanged`.

With it applied: mus-ariadne 60/60 lib tests and ovillo-dsp 37/37 pass.

### How it was tuned (all `fit`, measured on this World)

- **Bell:** Nelder-Mead on (tail, flare, s0) against this network's modes 2..8
  (numpy port of the runtime's solve, identical to the printed digit). Four
  starts agree: tail 0.223 m, flare exponent 0.703, s0 0.0252 m. Modes 2..8 sit
  at -15, +3, +25, +18, -2, -13, -18 cents from n × B-flat 1.
- **Lips:** lip-tension sweeps at fixed slide and pressure
  (`examples/trombone_nd_probe.rs`).
  - With the copied glottis numbers, mode 4 captured a 2x range of tension.
  - A 1 ms mucosal delay made the sweep lock partial after partial (2..11).
  - Mass 0.4 g with 1.5 mm thickness and an open embouchure (adduction 0.1)
    kept low tensions from being blown open, so partial 2 locks in every
    position.
- **Player:** `examples/trombone_nd_calibrate.rs` plays 7,938 tongued notes
  (partials 2-10 × 7 positions × 21 lip ratios × 6 pressures);
  `mus_analysis/trombone_nd_player_fit.py` fits them into
  `profiles/ariadne-trombone-v1.player.json`.
  - Every partial 2-10 speaks in all 7 positions, at 3-6 pressures each.
  - A written dynamic is a target level, and the player blows the pressure that
    gives it on that cell (`ovillo-dsp/src/trombone_nd_player.rs`).
  - Articulation is v0's: tonguing as pressure, slurs as lip moves under a
    legato tongue, smears/scoops/falls/vibrato as slide transport.

### Gates (renders `ovillo/renders/trombone-nd-*.wav`, made in a tree with the patch applied)

- **T1 lawful: pass.**
  - `trombone_nd::tests::audio_is_exactly_the_world_emission`: a second World
    from the same plan, given the same moves and controls, emits the same
    samples bit for bit (`to_bits`).
  - A source scan finds no filter, oscillator or noise in the audio path.
  - The WAV is `tick`'s output, as f32.
- **T2 ledger: pass.** Solo, 1,629,529 frames:
  - source (lungs) 28.116, control -0.00942 (slide -0.00941, lips -0.000009),
    internal loss 26.626, radiation 2.137;
  - slide disposal 2.5e-6, radiated.
  - Bound as in v0: ≤ 1e-12 per frame and 1e-15 per frame summed. Measured:
    largest frame 3.55e-15, cumulative -5.3e-12. Asserted in
    `trombone_nd::tests::the_ledger_closes`.
- **T3 it plays: pass.**
  - Solo (v0's 36 notes): 36 of 36 on the intended partial; median |error|
    5.7 cents, mean 6.5, max 13.7; 29 within 10 cents, all 36 within 25.
    Partials 2-10 used.
  - Étude (partials 2-3-4-5-6-5-4-3-2 slurred in each position): 63 of 63 on
    the intended partial; median 8.5, max 53.3; 39 within 10, 60 within 25,
    61 within 50.
- **T4 brightening with loudness: fail.** Dynamics ladder, centroid/f0 over the
  steady part as the level rises about 9 dB:

  | note | centroid/f0 | Spearman |
  |---|---|---|
  | F3 | 3.70 → 3.22 | -1.0 |
  | Bb3 | 2.64 → 1.82 | -0.9 |
  | D4 | 2.35 → 1.66 | -1.0 |
  | F4 | 1.91 → 1.83 | -0.1 |

  The tone is bright (solo median centroid/f0 1.97), but it does not brighten
  with loudness. Held 1.5 s, Bb3 stays at about 2.2 across a 15 dB range. The
  decline in the table is attack brightness decaying faster at loud levels.
  Neither embouchure firmness nor a harder lip contact changed the direction:
  firmness pulled the pitch off the partial, and harder contact darkened loud
  notes further. Real brass brightens at ff mostly through finite-amplitude wave
  steepening in the bore. This runtime is linear acoustics (`aero` is jet loss
  and turbulence), so that piece of runtime is what's missing.
- **T5 cost: pass.** 48 kHz, one thread, release (load average 4-6):
  - solo 2.44x real time (13.9 s wall / 14.4 s CPU for 33.9 s; slide moves
    recompute modes every 16 samples while the slide moves);
  - étude 19.0x;
  - dynamics 41.6x.
- **T6 musicality: partly.**
  - Solo note-to-note level spread at the same written dynamic: median 0.7 dB,
    max 3.5 dB (target < 12); 5.8 dB across all notes. Pass.
  - Partials 2-6 in all 7 positions: all 35 étude cells sound the intended
    partial; 34 of 35 are within 50 cents (partial 2 in seventh position is
    53). Pass.
  - Brightening on F3, Bb3, D4, F4: fail (T4).
  - On the coordinator's measure, 50 ms frames below -60 dBFS: v0 71 %, v1
    18 % (12 % without the final 2.5 s tail, i.e. the rests); median frame
    -16 dBFS.

### Limits

- **No brightening with loudness** (above): it needs nonlinear propagation in
  the runtime.
- **Grid:** 124 volumes, dx 7 mm (mouthpiece) to 35 mm (bell tail) and up to
  43 mm in a stretched slide. The grid's dispersion flattens the upper modes.
- **Series junctions:** each joins through the downstream half-column only (the
  runtime's branch element).
- **Slide:** a rubber slide (all slide sections stretch), not a telescope.
- **Lips:** copied glottis mechanics with fitted mass, thickness and delay. The
  aerodynamic work on the lips is booked to the lungs (the runtime's declared
  approximation).
- **Partial 3** has the narrowest windows.
- **Room:** none; the World is heard dry.

---

## v0 (history): lips and bore as two bodies (commit `6db9d41` on `ariadne/trombone-world-20261020`)


*Branch `ariadne/trombone-world-20261020`, 2026-10-20. Under the
[pure-physics constitution](ARIADNE-PURE-PHYSICS-CONSTITUTION.md). It replaces
nothing: the old `ovillo_dsp::trombone` (additive buzz into a passive
mouthpiece, bell high band filtered outside the World) still compiles beside it
and is what this work retires.*

## What it is

Two Ariadne bodies and one reciprocal edge in one `mus_ariadne::World`
(plan `profiles/ariadne-trombone-v0.plan.json`, compiled by
`mus_analysis/trombone_anatomy.py`, derivation receipt beside it):

| part | in the World | the player moves it by |
|---|---|---|
| lips | `lips` body: its compliant aperture (`law/aperture.rs`) is the lip valve. Mouth pressure is the source wave; the opening modulates the flow it lets into the cup mode, and the lips' motion sweeps volume into it. No reservoir-paid activity (`active_rate_per_second = 0`): every joule comes in as mouth pressure | **lip tension** = the lips body's configuration path (aperture natural frequency, slack 30 Hz at amount 0 to taut 900 Hz at amount 1); a change is booked as control work by `CompliantApertureBoundary::set_configuration` |
| mouthpiece | one field mode of the `lips` body (the cup, 600 Hz) and the `throat` edge: a lossless Givens exchange of the cup's momentum with the bore's momentum projected on its input (`coupling.rs`) | nothing |
| bore | `bore` body: 24 input-impedance resonances computed from a declared tenor-trombone-shaped bore. Wall loss is each mode's T60 (internal loss); the bell's radiation is the body's radiation direction and fraction (booked as radiation) | **slide** = the bore's configuration path, first position (amount 0) to seventh (amount 1), position k at (k-1)/6; transport is booked by `ModalMetricWaveField::set_configuration` |
| bell | the bore's `radiate()`: mode n loses amplitude at sigma_rad,n = pi f_n / Q_rad,n, so radiation efficiency rises with frequency (receipt `bore.radiated_share_first`: 3-9 % for modes 1-4, 20-50 % for 5-8, 60-80 % above) | nothing |

What you hear is `StereoListener::observe` of the World: the bore's emitted
coordinate (its radiation, sqrt(fraction) times its momentum projection on the
radiation direction). `TromboneWorld::tick` returns that observation and the
example writes it to the WAV sample for sample. The only gain is the listener
spec's `output_scale` (`LISTENER_SCALE = 0.5`, set once, part of the listener
digest). The room is off (space ports zero), so the World is heard dry.

A note is (slide, partial). The lips do not choose a pitch; their tension
chooses which bore resonance the lip-bore feedback locks to, the way a player
lips a partial. Tonguing is a mouth-pressure gesture. A slur is a lip-tension
move under a legato tongue: the air closes smoothly over 10 ms and is released
as in a tongued attack. A smear, scoop, fall or vibrato is slide transport, and
a rip is a 150 ms lip-tension ramp up through the partials below the target.
The old solo's growl and breath noise have no physical mechanism in this World
and are not played.

## How the bore is derived

`mus_analysis/trombone_anatomy.py`, labels in its `TromboneGeometry` and in the
receipt's `evidence`:

1. Profile, input end first: leadpipe cone 9.2 -> 13.34 mm over 0.24 m, two
   slide legs of 0.52 m (+ extension), a 0.15 m crook, a 0.201 m bell tail to
   14.5 mm, and a Bessel flare r(s) = r_mouth (s0/(s+s0))^1.406 with
   s0 = 0.194 m to an 8.25 in (209.6 mm) bell, 1.10 m long. Bore and bell
   diameters are `manufacturer` (class nominals), the tail, flare and s0 are
   `fit`, the rest `design-hypothesis`.
2. Input impedance by plane-wave transfer matrices over 4 mm cylinders,
   Kirchhoff visco-thermal wall attenuation, Norris-Sheng unflanged radiation
   load at the mouth.
3. Each of the 24 lowest |Z| peaks gives f_n, Q_wall (walls + only the
   reactive part of the bell load), Q_rad (lossless walls + the full load) and
   the modal input weight A_n = |Z|_peak w_n / Q_n.
4. The slide: each position k lowers the mean (log) of resonances 2..8 by k-1
   equal-tempered semitones; the leg extension is solved, not chosen. It comes
   out 8.4, 17.2, 26.4, 36.1, 46.3, 57.1 cm for positions 2..7
   (receipt `slide.leg_extension_m`). The World's path interpolates each mode
   in log frequency between first and seventh position; against the computed
   resonances at positions 2-6 that is off by at most 17.2 cents on modes 1-12
   (`slide.path_error_max_cents`).
5. Loss and radiation: T60_n = ln 1000 / (pi f_n / Q_wall,n); the radiation
   direction is r_n ~ sqrt(pi f_n / Q_rad,n) and the fraction
   4 sum(sigma_rad)/sr (0.1295 in first position, 0.0514 in seventh; the
   instrument interpolates it with the slide). This uses
   `field.rs::radiate`: removing fraction phi of the projection on unit r each
   sample takes amplitude from mode n at phi r_n^2 sr / 4 when modes are well
   separated.
6. The throat: bore mode n joins it with weight sqrt(A_n / f_n). The World's
   only reciprocal coupling exchanges momentum with momentum, and through that
   exchange the lips see bore peak n grow about as f_n (`trombone_fit.py
   seen`): 4.5 at mode 2 against 21 at mode 6 with weights sqrt(A_n). A real
   mouthpiece passes low frequencies freely. Dividing by f_n (`throat_tilt`,
   a `fit`, the counterpart of steel-string's `bridge_rise`) makes the peaks
   the lips see 2.6-3.2 for modes 2-10.
7. The bell `fit`: Nelder-Mead on (total length, flare, s0) so that the coupled
   resonances of cup + throat + bore (eigenvalues of the World's linearized
   map, `mus_analysis/trombone_fit.py`) sit on n x B-flat 1: -46, -13, -8, -2,
   -11, -10, -12 cents for n = 2..8.

## How the lips were found

The canonical brass aperture cannot be used as it is. Its momentum saturates at
0.334, so its displacement is about 3e-4 against an opening bias of 0.055: the
lips never close. Its active gain (38/s x pressure x opening) is below its own
damping. A numpy port of the World's per-sample map
(`mus_analysis/trombone_fit.py`, checked against the Rust World) was
linearized about its blowing fixed point; its unstable eigenvalues say what
sounds. That showed:

- The sweep's geometry factor tanh(2.5 x + 0.2 e) is odd in lip displacement.
  With the lips centred at zero the lip-bore coupling is only parametric, at
  twice the lip frequency. Giving the lips a rest offset
  (`equilibrium_displacement = -0.2`, with `opening_bias` set so the lips are
  barely open at rest) makes it linear.
  The sign of the offset decides whether the valve regenerates above or below
  a bore resonance.
- With no active term, the only power is mouth pressure entering through the
  opening-modulated flow. The lip-bore loop gain is then proportional to that
  pressure, and above threshold the most unstable mode steps through the bore
  resonances as lip tension rises.

- With the lips' rest opening at 0.01 every note was a sine: harmonics 60 to
  110 dB down, because near threshold the lips never closed. At 0.0005 (lips
  nearly closed at rest, as buzzing lips are) they close each cycle on the
  stronger notes, and the 2nd and 3rd harmonics come up to within a few dB of
  the fundamental.

The lip coefficients are `fit`: rest offset -0.2, rest opening 0.0005, damping
300/s slack to 40/s taut, mouth-to-lip 5 Hz, mouth-to-cup 3 kHz, sweep 1 kHz,
throat 2 kHz with tilt 1. Quartic stiffness and asymmetry are inherited from
the canonical aperture.

The player's knowledge (which lip tension speaks which partial, the pressure
range, and the measured pitch offset from the bore resonance) was learned by
playing the World. `examples/trombone_world_calibrate.rs` plays 7,560 tongued
0.5 s notes: partials 2-10 x the 7 positions x 20 lip ratios (0.28-1.04 of
bore resonance n) x 6 pressures. The result is
`profiles/ariadne-trombone-v0.calibration.tsv`, which
`mus_analysis/trombone_player_fit.py` turns into
`profiles/ariadne-trombone-v0.player.json`. Per partial and position, that file
holds whether it speaks, its lip ratio, its pressure range and its pitch offset.
The lips sit at 0.3-0.85 of the sounding resonance, and higher partials need
more air (partials 2-4 speak at 1,000-2,400, partials 6-9 at 3,500-7,000).
The player places each note on a partial and position that speaks (falling back
to the right pitch where none did), and solves the slide for the target pitch,
as a trombonist moves the slide to tune.

Speaking cells per position 1..7 (from the fit's printout):

| partial | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
|---|---|---|---|---|---|---|---|---|---|
| positions that speak | 1-7 | 1-7 | 1-7 | 1-7 | 1-7 | 1-7 | 3-7 | 5-7 | none |

## Runtime: no change needed

Two bodies with one 1-D path each, one edge, today's `mus-ariadne` (not edited
on this branch). `checkpoint-2a-20261020` did not exist when this was built.
It appeared near the end (`81774e0` on `ariadne/nd-configuration-20261020`)
and is not used. Its `modal_space` body (a `tube_network` bore with slide and
lip tension as two coordinates of one body) and its flow-valve law are the
direct route to a v1. The lips would sit on the bore itself, which removes the
cup intermediary and the throat tilt below. What today's runtime makes awkward:

- Lips and bore in one body would let the valve sit directly on the bore modes
  (no cup intermediary, no throat tilt). That needs two coordinates, lip tension
  x slide, on one body: exactly the N-D configuration.
- A plan cannot declare an aperture's initial state. The lips start at
  displacement 0, off their rest offset, so the World begins with a little
  stored lip energy that rings out (booked as initial energy).
- A body's radiation fraction is a per-frame control, not plan data, so the
  instrument reads first/seventh fractions from the derivation receipt.
- A coupling edge's directions are not on a configuration path, so the throat
  uses first-position input weights throughout.
- The sweep geometry's constants (2.5, 0.2) are hard-coded in
  `law/aperture.rs`; the lip rest offset is how this plan works with them.

## Gates

Renders in `ovillo/renders/` (git-ignored), from
`cargo run --release -p ovillo-dsp --example trombone_world_solo -- ovillo/renders <solo|etude|dynamics>`;
analysis by `uv run --no-project --with numpy --with scipy --with soundfile python -m mus_analysis.trombone_world_analyse <wav> <notes.tsv> [--brightness]`.
World plan digest `sha256:be3ebf7e508321ed7cb82fd9d9305a8e84250f6b464ed8c89bdf6dddec5f8919`,
listener digest `sha256:583520c323335bfacde743827f8897fe299a2c24b50c66bcc5cd7197ff6fec62`.

### T1 lawful: pass

- Nothing in the audio path is synthesized or filtered. Mouth pressure is a
  slowly varying source wave; lip tension and slide are configuration amounts;
  the World does the rest.
- `trombone_world::tests::audio_is_exactly_the_world_listener_output` builds a
  second World and listener from the same plan, drives it with the controls
  and configuration amounts the instrument applied, and asserts every output
  sample is bit-identical (`to_bits`).
- `trombone_world::tests::audio_path_names_no_oscillator_filter_or_noise` greps
  the module for `Biquad`, `filter`, `noise`, `phase`, `.sin()`, `.cos()`,
  `buzz`, `additive`, `mix`.
- The example writes `tick`'s left/right to the WAV as f32, with no
  normalization (`examples/trombone_world_solo.rs`).
- `cargo test --release -p ovillo-dsp`: 33 of 34 passed on the first run. The
  failure was this branch's own ledger test, whose closure bound was ad hoc; it
  was replaced by the bound below and now passes. The other 31 tests,
  including the old `trombone` test, pass unchanged.

### T2 ledger: pass

Solo, 1,629,529 frames (World receipt via `TromboneWorld::ledger`):

| initial energy | source work (mouth) | control work: lip / slide | internal loss | radiation | final energy |
|---|---|---|---|---|---|
| 0.07079 | 3.78140 | 0.73800 / 0.00019 | 4.40951 | 0.18087 | 0.000002 |

The initial energy is the lips' potential: a plan cannot declare their initial
displacement, so they start off their rest offset.

The closure bound is roundoff: each frame's books are O(1)-O(10) energies in
f64. Every frame closes to ≤ 1e-12, and the sum over frames to ≤ 1e-15 per
frame (1.6e-9 here). Measured: cumulative 1.137e-9, global 1.135e-9, largest
frame 4.26e-13, largest local law 4.26e-13. Étude (40.2 s): cumulative
5.7e-11, frame ≤ 1.7e-13. Dynamics (24.4 s): -3.5e-10, frame ≤ 2.0e-13. The
bound is asserted in `trombone_world::tests::the_ledger_closes`.

### T3 it plays: partly

**Solo** (the old example's 36 notes and timing): median |error| 8.1 cents,
mean 35.8, 23 notes within 10 cents, 30 within 25, 33 within 50. 34 of 36
sounded the intended partial; partials 2-9 were used.

Misses:
- the low F2 (partial 2, 6th position) sounded partial 3 (+714 cents);
- the Db4 at 9.32 s sounded partial 6 (+216);
- one Bb4 (partial 8, first position: no speaking cell there) is -58 cents;
- the C5 (partial 9, first position: no speaking cell) is voiced in only 3 % of
  its frames.

**Étude** (partials 2-3-4-5-6-5-4-3-2 slurred in each position): median 8.4
cents, 33 of 63 notes within 10 cents, 46 within 25, 49 within 50; 53 of 63 on
the intended partial.

- Partials 3, 4, 5 and 6 sound the intended partial in all seven positions
  (28 of 28 cells). Four of those notes are 65-115 cents sharp: partial 5 in
  positions 2 and 5, partial 3 in position 7.
- Partial 2 sounds as intended only in positions 1, 2 and 7. Elsewhere the
  World gives the pedal or partial 3, as the calibration predicted for the
  middle positions.

So "partials 2-6 across positions 1-7" holds for partials 3-6, and not for
partial 2 in positions 3-6.

### T4 it sounds like brass: partly

Dynamics piece: each note tongued five times, velocity 0..1 spanning its
calibrated pressure range. Centroid over 20 Hz-8 kHz of the steady part:

| note | level dBFS | centroid / f0 | Spearman(level, centroid) |
|---|---|---|---|
| F3 (3rd partial) | -59.6, -34.3, -25.3, -22.8, -22.4 | 1.00, 1.00, 1.26, 1.51, 1.63 | 1.0 |
| Bb3 (4th) | -33.8, -17.5, -13.5, -13.5, -19.4 | 1.00, 1.33, 1.64, 1.80, 1.52 | 0.8 |
| D4 (5th) | -75.8, -74.5, -69.6, -69.9, -66.2 | 1.00 at every level | (0.9, but there are no harmonics to rise) |

- On partials 3 and 4 the centroid rises with loudness, as brass does, because
  the lips close harder. Bb3's loudest note drops back.
- On partial 5 and above the World stays nearly sinusoidal and much quieter.
  In the solo the median level is -29.5/-23.1 dBFS for partials 3/4 and -60 to
  -71 dBFS for partials 5-9.
- Nothing beyond these measurements is claimed about how it sounds to a
  listener.

### T5 cost: pass

Native, 48 kHz, one thread, release build:
- solo: 33.9 s of audio in 7.98 s wall (4.26x real time), 8.13 s user CPU (4.2x);
- étude: 40.2 s in 10.8 s (3.7x);
- dynamics: 24.4 s in 5.45 s (4.5x).

The machine was shared (load average 15-22 during these runs; at load ~120 the
calibrator ran at about 1.2x per core). Most of the cost is the two 24-mode
bodies and the per-sample reconfiguration while lips or slide move.

## Limits, said plainly

- Partials 5 and up are quiet and nearly sinusoidal, partials 3-4 are full and
  brighten with loudness. The lip-bore gain falls off above about 300 Hz,
  where the bell radiates most and the lips' compliance drops. The throat and
  lip fits trade low partials against high; this plan favours 2-4.
- A pure lip slur (air kept going) leaves the new partial about 28 dB under a
  tongued one: it grows only from the old note's remnant. So slurs here are
  legato-tongued (median slurred level -37.4 against tongued -19.8 dBFS in the
  étude with the tongue; -48.3 without it).
- High-pressure notes run away (+17 dBFS) if the air is released slowly. The
  player releases in 8 ms (a tongue stop); falls keep 120 ms.
- Partial 2 speaks only in positions 1, 2 and 7; partial 8 only in 3-7;
  partial 9 only in 5-7; partial 10 nowhere. So the solo's Bb4 and C5 in first
  position are weak.
- 24 bore modes: nothing above about 1.5 kHz in first position.
- The calibration pressure grid is coarse (about x1.46 steps), so a cell's
  dynamic range is a few grid points. The player stays inside the pressures
  it heard speak.

## Files

| piece | path |
|---|---|
| anatomy compiler | `mus_analysis/trombone_anatomy.py` |
| plan + derivation receipt | `profiles/ariadne-trombone-v0.plan.json`, `.receipt.json` |
| linear map, seen impedance, bell fit | `mus_analysis/trombone_fit.py` |
| calibration (evidence) + player skill | `profiles/ariadne-trombone-v0.calibration.tsv`, `.player.json`, `mus_analysis/trombone_player_fit.py` |
| the instrument (World + listener) | `ovillo/crates/ovillo-dsp/src/trombone_world.rs` |
| the player (pressure, lips, slide) | `ovillo/crates/ovillo-dsp/src/trombone_player.rs` |
| renders, calibration | `ovillo/crates/ovillo-dsp/examples/trombone_world_solo.rs`, `trombone_world_calibrate.rs` |
| measurement | `mus_analysis/trombone_world_analyse.py` |
