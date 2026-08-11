# PW9 — El Telar: hardware honesty, the v2 surface, and the DAW arena

Status: keystone draft (2026-08-10, evening of the Ariadne landing).
Three layers: a hardware thought-experiment used as a design instrument,
the Observatory v2 interaction contract it produces, and how the same
work aims at the plugin market. The engine prerequisite for all three is
one seam: **within-note control automation** (§4).

## 1. The hardware thought-experiment (the design instrument)

A round loom you play: courses as physical rods with felt dampers
(course count = dampers lifted — state visible across a room);
`dimension` as a mechanical iris re-spacing the lattice; **orbit as a
hurdy-gurdy crank** (field speed = arm speed, reverse by cranking
backwards, depth = cam pressure); **couple as palm pressure on the
central bridge rosette**, which vibrates with the live holonomy defect
(δ as haptics); **chirality as a sprung shuttle bar** you must hold
against its spring; **curvature as tilting the bowl** (energy flows
downhill); **sing as a horn into the bridge** with a wah-style dry/wet
pedal; `sus` as a sustain pedal; and a thermal drum printing the
per-course energy braid — takes as tear-off strips, receipts as
objects, no preset menus anywhere.

None of this is for manufacture (yet). It is a forcing function: every
control must live on a body, cost an effort proportional to its
exoticism, and change something you can see or feel. Software surfaces
that skip this discipline become slider rails — which is precisely what
Observatory v1 did.

## 2. Observatory v2 — the interaction contract

What v1 got wrong: it *shows* geometry but *controls* scalars. v2 laws:

1. **The Ring is the controller, not the display.** Circular drag on
   the ring = cranking orbit at drag speed. Press-and-hold the center
   = couple (dwell = pressure). Off-axis drag = curvature. Click a
   node = damp/undamp that course (course count follows). The iris
   gesture (two-finger spread on the ring) = dimension. Sliders demote
   to an Advanced drawer.
2. **Held, not set.** Chirality springs to 0 on release unless pinned.
   Exotic states cost sustained gesture — an interaction law, not a
   hardware limitation.
3. **Feel the defect.** A barely-audible sub-click pulse at δ's beat
   rate while gesturing (haptics through sound).
4. **Continuous audition or nothing.** Cranking is meaningless against
   one-shot probes; v2 requires streaming render (§4).

## 3. The DAW arena — how we play

**The market read.** The hardware-surface plugin market renders
*remembered* hardware — photoreal 609s, Junos, LA-2As; a nostalgia
business with crowded shelves. The counter-school (FabFilter, Vital)
renders functional flatness. The open lane between them: **hardware
honesty without a referent** — instruments that never existed but obey
hardware logic (causal controls, visible state, effort, no menu
caverns), plus the things only software can do (telemetry-true
animation, printable braids, deterministic receipts). Closest cousins
are boutique pedal-world crossovers (Chase Bliss, Hologram) — strong
identity, instrument-like FX — not the NI catalog.

**Our unfair advantages, none decorative:**
- The physics is unduplicated: nobody ships a chirality knob because
  nobody has noncommuting coupling. Path memory (0.713 rel RMS between
  traversal orders, equal loudness) is a *feature class*, not a patch.
- `render_block()` was designed stateful for a host from day one (the
  handoff's own intent); the kernel is the product, the plugin is a
  window.
- The vocabulary IS the parameter schema — automation names, ranges,
  docs, and receipts stay coherent by construction; the manual can
  quote the claim table. Honesty as brand: the manual is a lab
  notebook with theorems and their limits.
- The score-language round trip nobody has: export a gesture as a mus
  event, import a score as a preset-with-provenance.
- Our own market research exists: the 80-principle VST doctrine
  (plans/vst-design-principles-20260810.md) from dissecting the OSS
  corpus — we wrote the checklist before entering the store.

**The entry product is the resonator, not the synth.** As an
*instrument* plugin, Ariadne competes with every synth ever shipped.
As an **FX plugin whose sidechain is the sing seam** — route any track
into the labyrinth, fret it with MIDI, crank the orbit — it enters the
boutique-resonator category (Mood/Microcosm territory) where its
physics is unanswerable and the surface IS the Telar. The instrument
version follows once the FX earns attention.

**Technical ladder (CLAP-first; design doc §8 stands: the wrapper
never reimplements synthesis):**
- P0: the automation seam (§4) + CPU pass (precompute body-coupling
  trig; budget target: extreme preset < 0.5× core realtime).
- P1: `ariadne-clap` via nih-plug (CLAP + VST3 from one codebase):
  params from the vocab dump, generic host UI, MIDI fretting,
  sidechain-as-sing. CLAP's polyphonic modulation maps exactly onto
  per-course/within-note control — our seam and CLAP's note
  expressions are the same shape, which is why CLAP first.
- P2: the Telar face (Ring-as-controller, GPU canvas in-plugin),
  braid strip export (PNG/receipt), score round-trip.

## 4. The one seam everything waits on: within-note automation

Motivated now five ways: E15's closed loop (A,B,−A,−B), the Braid's
replay, the plugin's host automation, hardware-style continuous play,
and streaming audition for v2. Sketch of the contract (its own
keystone when built, this is the shape):

- Engine: `StringNetworkVoice::apply_controls(&ControlFrame)` legal at
  block boundaries; a `ControlFrame` is a sparse set of (param, value)
  targets with declared smoothing (none | linear-over-block |
  coefficient-safe ramp). Every currently-per-event param declares its
  automation class in the vocabulary (adds a field to ParamSpec:
  `automatable: Static | Block | Smooth`).
- Determinism law: a render driven by a control *trajectory* is a pure
  function of (patch, notes, trajectory) — trajectories are content,
  they digest into receipts.
- Service: `renderWeaveProbe` grows an optional `trajectory` array of
  timestamped ControlFrames; the trace already reports what actually
  applied.
- Score language (later, its own ruling): within-event automation
  syntax is NOT invented ad hoc here; scores keep per-event params
  until a deliberate language ruling.

## 5. Order of work

1. Automation seam (engine + probe + trace) — unlocks everything.
2. Observatory v2 gestures on streaming probes (the software Telar).
3. E15 run + report to the mathematician (their §6 protocol).
4. `ariadne-clap` P1 skeleton (nih-plug, sidechain sing).
5. Telar face in-plugin (P2) only after the FX proves the surface.
