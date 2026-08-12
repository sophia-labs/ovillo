# MUS trajectory v1 — continuous gesture over persistent bodies

**Status:** implemented foundation on `agent/gesture-body-unification`  
**Artifact:** `mus-trajectory/v1`  
**Schema:** `schemas/trajectory-program.schema.json`  
**Rust:** `mus-rs/crates/mus-trajectory`  
**Python:** `mus_analysis/trajectory.py`  
**First adapters:** Weave/Guitar and GARGANTA

## 1. The constitutional distinction

The repository previously used *gesture* for three different things:

1. a musician directly moving one or more controls through time;
2. an inverse request such as “darken this sound without changing pitch”;
3. the executable path ultimately applied to the body.

Those are now separate values:

```text
Trajectory
    an authored path through named physical control coordinates

GestureRequest
    a target, constraints, and objective whose compiler chooses a path

TrajectoryProgram
    the immutable sample-clock artifact executed by a body
```

A direct trajectory lowers trivially into a `TrajectoryProgram`. Ariadne's
inverse compiler, Threader, and Extension models may also produce a
`TrajectoryProgram`; none gets a privileged executor. This is the key
unification: **authored, optimized, and learned motion all cross the same
validated body boundary.**

A score is therefore not a list of disconnected render requests. It can be a
path through a persistent physical body whose state, work, and history survive
across samples and—where the body contract permits—across events.

## 2. V1 scope

V1 carries scalar, fixed-capacity control lanes:

```text
TrajectoryProgram
  format: mus-trajectory/v1
  sample_rate_hz
  duration_samples
  hold_final: true
  lanes[]

Lane
  target
  unit
  curve
  value_space
  optional min/max
  optional max_rate_per_second
  knots[] = (at_sample, value)
```

Interpolation laws:

| curve | law | knot continuity |
|---|---|---|
| `step` | left hold | discontinuous |
| `linear` | affine | C0 |
| `smoothstep` | `3t² - 2t³` | C1 |
| `smootherstep` | `6t⁵ - 15t⁴ + 10t³` | C2 |

`value_space=log2` interpolates ratios geometrically. Rate bounds are evaluated
in the declared value space. This lets a pitch-ratio client eventually state a
bound in octaves per second without changing the artifact.

V1 explicitly does **not** include:

- topology edits or rank changes;
- stochastic lanes;
- inferred baseline return;
- hidden clipping;
- perceptual objectives;
- a claim that a coordinate name has the same physics on every body.

Those are adapter or later-version responsibilities.

## 3. Validation and refusal

Validation occurs before audio exists. A program refuses when:

- its format, clock, or duration is invalid;
- `hold_final` is false (v1 has no lawful implicit reset);
- targets are empty or duplicated;
- a lane has no knots or does not begin at sample zero;
- knot samples are not strictly increasing or leave the horizon;
- values are non-finite, outside declared bounds, or non-positive in log space;
- a declared rate limit is exceeded;
- a finite rate is claimed for a discontinuous moving lane;
- an adapter does not bind a target.

Every refusal has a stable code. No adapter may silently clamp authored motion
and then issue a receipt for the unclamped program.

## 4. Receipts

A receipt records:

```text
program_sha256
sample rate and duration
lane count and target order
curve and continuity class per lane
value space and units
knot count
minimum and maximum authored value
observed peak rate (when finite)
whether each lane and the whole program are zero-motion
```

Rust and Python replay the same committed Weave fixture and digest. The digest
is over compact deterministic JSON after validation, not over the pretty file
bytes.

A trajectory receipt proves that a named path was valid and executed through a
particular adapter. It does not by itself prove perceptual effect, biological
faithfulness, or lawful state transport inside a body. Those remain separate
receipt columns and claim levels.

## 5. Direct MUS surface

The existing token parser already preserves bracket values as strings. V1 uses
arrow-valued physical parameters:

```mus
@instrument loom = synth=weave courses=11 couple=0.08 body=0.30

loom 1: A2h[
  couple=0.04->0.40->0.12,
  body=0.25->0.72,
  curvature=0.10->0.90,
  trajectory_curve=smootherstep
]
```

Scalar values retain their old meaning. An adapter compiles only target names
it owns; unrelated MUS-A arrows such as `pan=-1->1` remain in the ordinary
transform pipeline.

The string adapter replaces each arrow expression with its first value for
voice construction, then applies the validated path before every sample. Thus
construction state and sample-zero control agree.

**Current integration boundary:** the Rust trajectory-enabled render entry
points are live in `mus_dsp::trajectory`. The ordinary `mus-engine` source
resolver still calls the legacy scalar entry points until the source adapter is
switched after workspace CI proves the new layer. The syntax above is therefore
specified and parser-preserved, but not yet the default score renderer on this
commit.

## 6. Weave/Guitar adapter

Bound targets:

```text
couple          [0, 0.45] rad
chirality       [-1, 1]
orbit           [0, 20] Hz
orbit_depth     [0, 1]
curvature       [0, 1]
body            [0, 1]
buzz            [0, 1]
sing            [0, 1]
bow             [0, 1]
bow_speed       [0, 1]
sus             [0.05, 20] s
damp            [0, 0.95]
breath          [0, 1]
```

`StringTrajectoryVoice` applies these through the existing
`StringNetworkVoice::apply_control` method immediately before each sample. It
therefore inherits the current engine clamps only as a redundant defense; the
adapter's authored bounds must already have passed.

The runtime owns a program clock independent of host blocks. Rendering one
whole buffer or any partition of buffers is required to be byte-identical.
An empty program and a zero-motion program are required to be byte-identical to
the scalar voice.

### 6.1 The pitch refusal

`pitch_ratio` is deliberately recognized and refused as
`string-trajectory-pitch-transport-pending`.

The existing note gliss changes delay length. Continuous pitch therefore
crosses the unsolved active-line state-transport seam already named by the
Ariadne work ledger. Resampling the output, reconstructing one voice per frame,
or silently retaining reinterpreted delay state would create a counterfeit.
The artifact supports log-ratio lanes now; the string adapter will bind the
name only when the delay-state connection has a defensible implementation and
receipt.

This is the highest-priority next engine theorem/implementation seam.

## 7. GARGANTA adapter

The Python adapter binds:

```text
jaw_open
tongue_front
tongue_back
lip_round
apical_constriction
gain
aspiration
f0_ratio | f0_semitones
```

It samples one validated program onto the current eight-anchor genome used by
the P0-certified diagnostic renderer. Missing lanes retain the paired-campaign
neutral start. `f0_ratio` is converted to semitones only at this adapter
boundary.

This is useful unification, not claim inflation. The current renderer still
uses a frequency-domain tract construction and a diagnostic source. The
adapter does not satisfy G1 fast-passage or G3 stateful glottal supply. It makes
those future implementations consume the same path artifact rather than invent
a third control language.

## 8. Ariadne, Threader, and Extension

### Ariadne

A compiled geometric word remains a distinct operator program. Continuous
physical controls and operator gates can coexist on the same body, but neither
is recast as the other. An inverse MUS-F request may emit:

```text
operator word
trajectory program
or a typed mixed program with an explicit stage order
```

The stage order belongs in the receipt because overlapping operations need not
commute.

### Threader

Threader should decode anatomy separately from a `TrajectoryProgram`. Its local
frame controls become spline/trajectory knots; posterior trust regions and
physical priors act before validation. A model output that the body adapter
refuses remains refused—it does not become valid because it came from a model.

### Extension-v1

Extension learns closed gestures over a latent state field. The staged
production lowering target is now explicit:

```text
learned latent gesture
  -> named body-coordinate path or operator word
  -> TrajectoryProgram / existing gate program
  -> adapter validation
  -> persistent body execution
```

Extension does not own a second audio callback or bypass body-specific bounds.
Its morphology may choose which coordinates exist in a later fixed-capacity
body, but topology changes remain outside trajectory v1.

## 9. Energy and work

The trajectory layer never declares control work zero. It merely places control
values on the sample clock.

- Norm-preserving Weave scattering remains booked as roundoff-level scatter
  work.
- Changing allpass coefficients uses the existing retune policy and work
  receipt.
- Changing active delay length remains measured as line reinterpretation until
  the pitch transport law is solved.
- GARGANTA articulatory work becomes meaningful only when the tract state and
  metric work law are implemented.
- Source controls (`bow`, future folds, breath/turbulence) remain supply terms,
  not passive geometry.

The same path can therefore receive different lawful work receipts on different
bodies. The path artifact is portable; the physical claim is not.

## 10. Acceptance ladder

### T0 — artifact and replay — implemented

- Rust and Python v1 types;
- JSON Schema;
- deterministic digest and receipt;
- committed cross-runtime fixture;
- interpolation, bounds, rate, and refusal tests.

### T1 — real-time Weave binding — implemented

- pre-sample application through the existing control seam;
- no allocation on the sample path;
- empty and zero-motion byte laws;
- arbitrary host-block partition invariance;
- nontrivial moving path differs from both static endpoints;
- typed pitch refusal.

### T2 — GARGANTA binding — implemented at diagnostic-renderer claim level

- same artifact and clock;
- tract/source target map;
- range refusals;
- eight-anchor lowering and tests.

### T3 — default MUS engine source integration — next

- switch only pluck/weave events containing bound arrow lanes to the new entry
  point;
- preserve every scalar corpus render byte-for-byte;
- typed bad-event reporting for rejected trajectory authorship;
- add corpus and CLI fixtures.

### T4 — lawful pitch-ratio transport — open, highest priority

- fixed-capacity material coordinate where possible;
- explicit state connection when delay rank changes;
- rate, reversal, repetition, sideband, pitch, waveform, and ledger receipts;
- split-event and output-resampling counterfeits as controls.

### T5 — dynamic GARGANTA physics — open

- G1 fast-passage certificate or validated production bound;
- G3 stateful glottal source;
- topology-accounted closure/release and stochastic supply laws;
- one `/ba/` transition whose trajectory receipt and forensic receipt share a
  digest chain.

### T6 — inverse gesture lowering — open

- MUS-F target compiler emits v1 programs;
- Threader and Extension emit the same artifact;
- direct, inverse, and learned paths replay through one acceptance battery.

## 11. Definition of the first unified vertical slice

The vertical slice is defensible when one content-addressed packet contains:

1. a direct MUS-authored Weave trajectory;
2. the same program replayed independent of host-block partition;
3. an audible render and a static-endpoint counterfeit pair;
4. a GARGANTA transition program using the same schema;
5. adapter-specific receipts with no hidden clamping;
6. a typed refusal for pitch until its state transport is lawful;
7. one inverse or learned producer lowering into the artifact without owning a
   new executor.

T0–T2 are the implementation in this branch. T3–T6 are now narrow, named seams
rather than a vague split between music and engine work.
