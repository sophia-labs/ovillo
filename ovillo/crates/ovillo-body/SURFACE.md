# The surface of `ovillo-body`

A Body to speak through. Her recording is the energy: every sample becomes a
volume velocity into one volume of a tube network inside one
`mus_ariadne::World`, and the World books its exact work as source work (the
same door `ovillo-tract-world`'s `impact` uses for one pulse, held open). The
listener is a pair of ears in front of the Body's openings; it loads nothing.
Offline only: `ovillo-body render` runs 7 to 28 times faster than real time on
this machine (measured per preset, below).

```text
ovillo-body describe                      the surface as JSON (every knob: id, unit, range, default, physics, sound)
ovillo-body presets [--write <dir>]       the built-in bodies
ovillo-body render --body <name|file.json> --in dry.wav --out wet.wav [--receipt r.json] [--set id=value ...]
ovillo-body inspect --body ... [--set ...] [--seconds S] [--level L] [--json]   modes, openings, measured transfer
ovillo-body analyze a.wav [b.wav ...]     long-term third-octave spectra, centroid, level (a measurement aid)
ovillo-body plan --body ...               the ariadne-world-plan-v3 it compiles to
```

Every number quoted here was measured with these commands on this branch
(2026-11-05), on white noise (`inspect`) and on `say -v Daniel|Ralph|Samantha`
renders. Nothing is from memory.

## 1. What the World is

```text
air       one tube-network body: the bore (an area profile with up to three
          Gaussian bumps), up to three side branches (closed stubs, bottles,
          or open vents with their own radiating ends), a flap valve in the
          entry wall, POC-3's air (contact, jet loss, turbulent dipole,
          DC-correct wall loss) at the narrowest column. 24 retained modes,
          recomputed if the one live coordinate (the mouth) moves.
res1-3    passive modal resonators (string, bar, membrane, plate, bell:
          declared partial series), each touching one air volume through a
          reciprocal lossless direct coupling, each radiating mode by mode.
powered   the canonical string/brass material (`shared-wave-material-v1`, the
          guitar World's body) with its reservoir-paid active inclusion,
          touching one air volume the same way.
room      the canonical eight-course delayed field, ported at the mouth,
          with back-action; disconnected exactly at room.port = 0.
listener  two ears: each air opening a monopole p_end A / (4 pi r l) delayed
          r / c; resonators, the powered material and the room as spherical
          waves of their radiated power; her dry recording optionally added
          at the ears (an observer path: it changes no physical state, tested).
```

The anti-smuggling rule, item by item: one World with persistent distributed
state (the air's 24 modes, the resonators', the material's, the room's
delay lines); geometry that matters (where she enters, where the branches
join, where the openings are, where the ears are); back-action (the direct
couplings and the room port are reciprocal rotations; her own flow is pushed
back on: the receipt's `voice_pushed_back_fraction` is the share of samples
where the air did work on her); nonlinearity from contact and material law
(the flap's unilateral contact, the jet's separation loss, the powered
material's saturable inclusion); no serial chain (every part is a body in
the same World and the listener reads them all); exact limits (section 7);
no electrical anatomy anywhere.

## 2. Gain staging: from her microphone to a flow

A microphone hears pressure. A mouth's far field is `rho dU/dt / (4 pi r)`:
the time derivative of its volume velocity. The Body wants the flow, so the
recording is integrated before it becomes `U` (`drive.integrate`, a leaky
integrator with corner `drive.corner_hz`, unity at 200 Hz). Then
`U = drive.gain_m3s x flow coordinate`: at the default 3e-4 m3/s, a
full-scale 200 Hz tone pushes 0.3 L/s, a loud vowel's peak flow at the lips.
Daniel's recording (peak -1.6 dBFS) does 8e-4 J of work on the neutral body
in 10 s and reaches 0.023 Pa peak at 1 m: a quiet conversational level, in
the physical range, perhaps 2 to 3 times low. Raise `drive.gain_m3s` to
push the body harder (the jet and the flap answer to it, not just the
level); `listener.pa_full_scale` is only the output level.

Why the integrator is not an EQ box: without it (`drive.integrate = 0`) the
neutral body measures +6 dB per octave from 300 Hz up, the sound of treating
a pressure as a flow. With it, the neutral body is flat.

## 3. The knobs, and how they interact

`describe` is the authoritative list (133 knobs in 9 groups). This is the
map of what they do to each other.

### The bore

- `bore.length_m` sets the quarter-wave series `c/4L, 3c/4L, ...` when she
  drives the closed wall. The World keeps the 24 lowest modes of the whole
  network and refuses a network whose 24th mode is above 23 kHz: a closed-open
  pipe alone must be at least about 0.18 m; every branch adds modes, so with
  branches the bore can be shorter (the `v` preset is 0.15 m). The 24th mode
  is the receipt's `bandwidth_hz`: the Body is dark above it. Long bodies pay
  for it: the 1 m `radio` is dark above 3.8 kHz, the 0.42 m `guard_box` above
  8.6 kHz.
- `bore.entry_cm2`, `bore.exit_cm2`, `bore.flare`: the taper. A narrow entry
  raises the pressure her flow makes; a wide mouth radiates and damps.
- `bore.wall_loss_np_m` is frequency-flat (the runtime's law). It widens every
  resonance by about `109 x alpha` Hz, but it also makes the pipe resistive
  below about `109 x alpha` Hz: a felt-lined pipe (15 Np/m) loses its
  monopole rise below 1.6 kHz and the integrated drive then boosts the lows.
  `drive.corner_hz` is the honest place to take that back (neutral: 3 Np/m,
  corner 120 Hz).
- `bump{k}`: a Gaussian on ln(area). Negative is a constriction: it moves the
  formants and, with `air.on`, becomes the jet. Positive is a chamber.
- `mouth.open` is the live coordinate: the last volume's opening. Its live
  range is 0.3 to 1 (or down to where the preset starts). A move is a
  receipted mass-momentum reconfiguration (control work, disposal).

### The drive

- `drive.position` chooses the volume. At the wall every mode is driven;
  nearer the mouth the pipe behind becomes a closed stub and its quarter-wave
  notches appear. The lowering never puts her in the mouth volume itself
  (and says so in the receipt's notes): a volume velocity dumped into the
  volume the listener reads adds a one-sample, frequency-flat pressure term
  that the lumped volume cannot flow out of within a sample (its local
  Helmholtz rate is about 0.7 per sample); measured as a -6 dB per octave
  tilt with the integrator on. One volume back and it is gone.

### Branches

A closed branch of length `L` is an antiresonance at `c/4L` and its odd
multiples; its pole sits just above the notch, higher the wider the branch.
A narrow neck in front of a wide short branch is a Helmholtz bottle: one
deep notch at `c/(2 pi) sqrt(S / (V l))` with the pole above it. An open
branch is a second mouth with its own position for the ears
(`vent_x_m`) and a half-wave series. `wall_loss_np_m` on a branch turns a
notch into an absorber (the gatekeeper's fur coat: 120 cm2 x 12 cm at
30 Np/m). Up to four radiating ends in all (the mouth and three vents), up
to 128 volumes in all.

A branch's notch is deepest where the bore's pressure pattern is strong at
the junction; a branch at a mode's pressure node leaves that mode alone.
Measured on `v`: the 6.4 cm stub at 0.6 of the bore notches 1356 Hz by
-12 dB and the 3.6 cm stub at 0.85 notches 2416 Hz by -15 dB (relative to the
band mean); the same 6.4 cm stub at 0.8 notched 1356 Hz by -28 dB.

A big cavity swallows low-frequency flow: with the gatekeeper's coat cavity
between the drive and the throat, the throat's jet stayed at Reynolds 150; with
the throat moved in front of the coat it reaches 5200 and hisses.

### The air (`air.*`)

`air.on` adds contact at the bore's narrowest column, the free-jet
separation loss there (`rho |u|^3 / 2 A^2`, scaled by `air.jet_loss`), a
turbulent dipole `air.obstacle_offset` columns downstream whose efficiency
rises as Mach^3 and switches on between `air.reynolds_onset` and
`air.reynolds_full`, and the DC-correct wall loss. Off, the body is linear
acoustics bit for bit (tested: doubling the input doubles the output to
1e-6). The receipt reports `jet_loss_j`, `turbulence_injected_j`,
`max_reynolds`, `max_jet_speed_m_s`.

Honest numbers: speech flow through a 0.13 cm2 throat reaches 18 m/s and
Reynolds 5200 at `drive.gain_m3s = 6e-4`; the jet eats 3.4 % of her work
(level-dependent compression) but the dipole returns only 1e-6 J of it as
hiss at the law's constant (Curle's exponent, a design-hypothesis constant
from `ovillo-tract`). The air is a compressor first and a hiss second; to
hear the hiss, lower `air.reynolds_onset` and raise `air.turbulence`
(capped at 0.05).

### The flap (`flap.*`)

The runtime's flow valve with no supply: a mass on a spring with unilateral
contact in the entry wall, opening a slit to the outside. Positive pressure
pushes it open and air leaks out (a Bernoulli slit: the leak loss grows
with level); negative pressure sucks it shut and it slams the stop. Off, it
is a 1 kg plate on a 2 kHz spring adducted to a 1e-15 m gap, whose flow is
exactly zero (the valve cuts off below 1e-14 m2). On, it is a serious
mechanism: the gatekeeper's flap leaks 29 % of her work and dissipates 4.5 %
in its slams. The receipt has `flap_loss_j` (the fold's damping),
`flap_leak_loss_j` (the slit's jet) and `flap_reservoir_draw_j` (the
aerodynamic work on the fold, booked to the body's reservoir, as the
runtime declares).

### Resonators (`res{k}.*`)

A 24-mode passive body with a declared partial series (`kind`: string with
inharmonicity, free-free bar, circular membrane, rectangular plate 1:1.3,
bell), its first partial `f1_hz`, T60 and radiation rate at the first partial
with power-law slopes, touching one air volume (`couple_tube`,
`couple_position`) through a direct coupling: a Givens rotation at
`coupling_hz` between the air's pressure there and the resonator's motion
along the touch pattern `sin(n pi touch)`. Lossless and reciprocal: the
resonator takes from the air and gives back, and loads the air (strong
couplings split the air's modes near the partials). Its own radiation is
heard at `x_m`, scaled by `level`. A resonator on a side branch hears only
what reaches the branch.

The partial series are declared design profiles (the bell's especially),
not measurements of any instrument. The touch pattern is a pinned string's
for every kind: a hypothesis for membranes, plates and bells.

### The powered material (`powered.*`)

The guitar World's body, unchanged (`WorldPlan::string_brass_v1`): 24 modes
on a string-to-brass path (`organization`), a compliant boundary, ordered
internal transport (`interlace`, `memory`) and the reservoir-paid active
inclusion (`supply_w` refills a 2 mJ reservoir; `coupling`, `drive`,
`headroom`, `sustain`, `sag`, `bloom`). It touches one air volume through a
direct coupling into its source direction (where its pick or lips would
act). Its supply is the only source work besides her voice; the receipt
separates them (`voice_work_j`, `supply_work_j`). In `radio`, the supply
does 0.10 J against her 0.028 J over 10 s: the material is doing most of the
sustaining, and the books say so.

### The room (`room.*`)

The shared delayed field (eight courses, 7 to 90 ms) ported at the mouth
volume with weight `room.port` (0: never touched, tested). `coupling`,
`decay`, `radiation` are the field's own controls; `level` is the listener's.
It is a modal room with back-action, not a geometry.

### The listener (`listener.*`)

Ears at `distance_m`, `ear_spacing_m` apart (0: mono), `azimuth_deg` off the
mouth's axis, in the plane of the openings. The output is pascals over
`pa_full_scale`. `dry` adds her recording at the ears, delayed by the mouth
path, full scale to full scale: the band above the Body's 24th mode, and
her own sibilants, come back without touching the Body.

## 4. The presets, measured

Transfer: white noise at 0.1 rms through a fresh World, third-octave bands,
dB relative to the 150-8000 Hz mean unless stated. Speed and books: Daniel's
10.3 s read, Apple M-series, release build; every preset's maximum frame
residual is 4.4e-16 to 8.9e-16 J and no preset faulted.

| preset | what it is | measured | speed |
|---|---|---|---|
| `neutral` | her voice at the wall of a 19 cm flesh-lined pipe, 4 to 8 cm2 | ripple +/-6.3 dB over 150-8000 Hz; +4 dB at 100 Hz, flat 300 Hz-3 kHz within 3 dB, -9 dB at 8 kHz; quarter-wave peaks at 508/1280/2153 Hz, +4 dB; dark above 20.2 kHz. On Daniel the long-term spectrum follows the dry one within 4 dB from 100 Hz to 4 kHz | 21-27x |
| `v` | neutral at 15 cm with three closed side tubes on her formant medians | notches 359/1356/2416 Hz (-8/-12/-15 dB), peaks 678/1613/3044 Hz (+6/+1/+4); bandwidth 9.0 kHz. Daniel: 317-504 Hz down 4 dB, 1270-2540 Hz down 4 to 9 dB, 635-1008 Hz up 2 to 4 dB, 3200 Hz up 3 dB against neutral; 200-5000 Hz centroid 597 to 570 Hz (Ralph 550 to 520; Samantha 586 to 609) | 24x |
| `gatekeeper` | 26 cm throat, 0.13 cm2 jet, open 22 cm nose vented 4 cm left, 1.4 L fur-lined coat cavity, light flap, mouth half shut | jet 18 m/s, Reynolds 5200, jet loss 3.4 % of her work, flap leak 29 %, slams 4.5 %; +10 dB at 50 Hz, -6 dB/oct from 100 Hz to 1 kHz, dark above 6.2 kHz; two radiators in stereo | 21x |
| `radio` | 1 m narrow speaking tube, crackling flap, the powered material at the far end | dark above 3.8 kHz (its 24th mode), -6 dB/oct tilt, supply 0.10 J vs voice 0.028 J | 6.5x |
| `guard_box` | 42 cm bore into a 30 cm chamber (bump +3.5), 1.5 cm2 slit, a plate wall (140 Hz series) driven by the trapped air | boomy below 160 Hz, -19 dB at 800 Hz, -35 dB at 1.3 kHz; the plate's loss 4.7e-5 J | 16x |
| `coke` | she speaks into the bell of a 40 cm ear trumpet narrowing to 0.5 cm2 at an ear 2 cm away, through a cloth ruff cavity, with a slack leaky flap | +12 dB at 50 Hz, mid dips at 1 kHz and 2.5 kHz, flap leak 38 % of her work | 24x |
| `bottle` | a 40 to 2 cm2 bottle with a belly | one hoot and the belly's modes | 22x |
| `beehive` | a drumhead across a short pipe, a light flap, a 1 cm2 hissing mouth | Reynolds 1900 at the mouth; the membrane's loss 3.7e-4 J | 14-16x |
| `cathedral` | neutral opening into the room (decay 0.7) with a bell (prime 220 Hz) at the mouth | the room's halo with back-action; bell partials 110/220/264/330/440 Hz | 11x |
| `sitar` | neutral with three sympathetic strings (A2, E3, A3) at the mouth | strings ring at her harmonics and load the air | 10x |

`listener.pa_full_scale` is set per preset so Daniel peaks near -6 dBFS.

## 5. Recipes

- **A notch on a formant:** a closed branch of length `343 / (4 f)` m, area
  about half the bore's, near the mouth. Deeper: wider; broader: lossier.
- **A notch below 400 Hz without the odd multiples:** a bottle: neck 0.3 to
  0.6 cm2, 30 to 60 cm2 behind it, 3 to 4 cm long. A 26 cm stub would also
  notch 990 and 1650 Hz.
- **Reinforce just above a notch:** the pole comes free with the branch;
  widen the branch to push it higher.
- **A second voice from the side:** an open branch with `vent_x_m`; its
  half-wave series is the nose's own resonance (the gatekeeper's 22 cm nose
  at 780 Hz and multiples).
- **Eat the lows:** a wide, lossy cavity (30 Np/m) near the entry.
- **Hiss when she pushes:** a constriction of 0.1 to 0.3 cm2 *before* any
  big cavity, `air.on`, onset 400 to 800, `air.turbulence` 0.01 to 0.05,
  `drive.gain_m3s` up.
- **A rattle on loud syllables:** `flap.on`, mass 0.05 to 0.2 g, tuning 90 to
  400 Hz, gap 0.1 to 0.25 mm, damping low.
- **A box:** a long bore with a wide bump (+3 to +4 on ln area) and a narrow
  exit, and a plate resonator at the bump (`res.kind 3`) heard beside the slit.
- **A room:** `room.port` 0.5 to 1 with `room.decay` and `room.level`.
- **Sustain that is paid for:** `powered.on` at the mouth; `powered.supply_w`
  is the budget, `powered.headroom` the compression.
- **Her air back:** `listener.dry` 0.2 to 0.5.
- **Measure before you listen:** `inspect` (noise transfer, peaks, notches,
  modes) and `analyze dry.wav wet.wav` (long-term spectra, centroid).

## 6. Feminization: what the physics can and cannot do

Her first mic check: median F0 111 Hz, running-speech formant medians
330/1343/2466/3408 Hz, apparent tract about 18 cm. The size cue she wants
is a 14.5 to 15 cm tract: formants about 20 % higher (roughly
400/1600/2950/4100 Hz).

What a Body can do: it is a fixed filter made of air that she speaks
through. It can put antiresonances on her formant medians and resonances
just above them (`v` does: notches 359/1356/2416, peaks 678/1613/3044). On a
steady vowel whose formant sits on a notch, the energy moves up into the
pole and the peak reads higher. On running speech, her formants wander
+/-30 % with the vowel, the notches do not follow, and the net effect is a
re-weighting of the long-term spectrum, not a formant shift. Measured: the
200-5000 Hz centroid of two male synthetic voices went *down* through `v`
(the notches remove more than the poles add) and up only for the female
one. On her own take `s02-05/dry.wav` (the first in the studio folder),
`v` against `neutral`: 317-400 Hz down 4 to 6 dB, 635-800 Hz up 3 to 5 dB,
1270-2540 Hz down 3 to 8 dB, 3200 Hz up 3 dB; 200-5000 Hz centroid 548 Hz
dry, 577 through `neutral`, 589 through `v`. Expect a coloration ("through
a tube") with a lift of the 600-1000 Hz and 3 kHz bands, and tune by ear
with her takes: `--set` on the branch numbers and `mouth.open` is the whole
surface for it.

What it cannot do: move her formants with the vowel (that needs her tract,
or the gesture route in `film/audio/voice/DESIGN.md`), change her pitch (she
does), add breathiness (no aspiration source exists in the World), or shift
the band above its 24th mode. A fair expectation is a few percent of
apparent size on sustained vowels and a timbre on everything else.

## 7. Exact limits (tested in `tests/laws.rs`)

- `air.on = 0`: linear acoustics; jet loss and turbulence exactly 0; output
  scales exactly with input.
- `room.port = 0`: the shared space is never touched (stored energy exactly 0).
- `branch.on = 0`, `res.on = 0`, `powered.on = 0`: absent from the plan (the
  plan digest does not see their other knobs).
- `flap.on = 0`: the slit's flow is exactly zero.
- `listener.dry`: changes no ledger bit.
- `drive.integrate = 0`: the sample is the flow.
- Every preset: maximum frame residual under 1e-12 J (measured 4e-16 to
  9e-16), cumulative under 1e-9 J, source work equal to her work unless
  `powered.on`.

## 8. Where the edges are

- **24 modes.** The air's bandwidth is its 24th mode; every branch, bump and
  centimetre spends modes downward. Short simple bodies are refused (the
  error says what to lengthen).
- **Frequency-flat wall loss.** Resistive below `109 x alpha` Hz: lossy
  bodies boost the lows under the integrated drive. Real viscothermal loss
  goes as the square root of frequency; the runtime's does not.
- **The mouth volume cannot be driven** (the one-sample self term); the
  lowering moves the drive back one volume and notes it.
- **The lumped end** is a low-`ka` radiator: a 60 cm2 mouth is past the law's
  validity above about 1.2 kHz (`ka = 1`); keep mouths under about 10 cm2
  when the top octaves matter.
- **One jet**, at the narrowest column of the bore only; a branch neck does
  not hiss; the mouth constriction can be the jet.
- **The flap's aerodynamic work** is booked to the air body's reservoir (the
  runtime's declared approximation), so `flap_reservoir_draw_j` moves a
  reservoir that nothing refills; the 1 J capacity lasts.
- **Resonator radiation** is mode-diagonal and heard as a sum of modal
  coordinates, converted to pascals as a spherical wave of the radiated
  power: level, not directivity.
- **The room** is eight delay courses, not a geometry; its port direction is
  the mouth's modal shape at construction, so a moved mouth is ported a
  little off.
- **No aspiration, no glottal source:** nothing in the World makes noise
  except her and the jet.
- **Speed** is 7 to 28x real time offline; nothing here is tuned for latency.

## 9. The receipt

`render --receipt` writes `ovillo-body.receipt.v1`: input and output levels,
clipped samples, faults (a World error rebuilds the World silent and
counts), `voice_work_j`, `voice_pushed_back_fraction`, `supply_work_j`, the
World's ledger (source, control, internal loss, radiation, stored, the four
residuals), the air's books (wall loss, radiation per end booked and
physical, jet loss, turbulence, the flap's three entries, field energy,
maximum Reynolds and jet speed), per-body source and loss, the 24 modes
(Hz, wall T60, radiation rate per end, shapes at the valve and the mouth),
`bandwidth_hz`, the layout (volumes, drive, openings, radiators, couplings),
the resolved knobs, the speed, and notes.
