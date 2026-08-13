# MUS-T — trajectories through persistent physical bodies

**Status:** executable v1 contract; ordinary MUS event dispatch landed
**Selected repository theme:** certified continuous gesture over persistent bodies
**Base:** `agent/weave-string-network@352bb2d25bb093bb84a11a9740051b0fa8e73a1b`

## 1. Thesis

A score event is not merely a request to synthesize a buffer. It is a path
through a body whose state persists while the path is traversed. MUS-T makes
that path an authored, content-addressed value.

The repository uses three deliberately different words:

- **control** — one physical coordinate, such as coupling, pitch ratio, jaw
  opening, tract area, lung pressure, or velum aperture;
- **trajectory** — an explicitly authored time path through one or more
  controls;
- **gesture** — an effect-level target whose inverse compiler chooses a
  trajectory and, where needed, an operator word.

The distinction is load-bearing. Direct trajectories let musicians work before
perceptual and effort metrics are trustworthy. `SPEC-GESTURE.md` remains the
higher inverse layer and must eventually lower into this exact IR rather than a
private automation tensor.

## 2. The shared IR

A `TrajectoryProgram` contains:

```text
schema_version
sample_rate_hz
duration_samples
control_stride_samples
metadata: sorted string map
tracks[]
  coordinate
  interpolation
  coordinate contract
    unit
    hard domain
    optional rate and acceleration limits
    optional declared quadratic metric weight
    fixed-capacity or topology-edit state semantics
  knots[]
    exact sample
    value
```

The transport schema is `schemas/trajectory-program.schema.json`. Rust owns the
runtime-neutral implementation in `mus-rs/crates/mus-trajectory`; Python owns a
line-for-line scientific oracle in `mus_analysis/trajectory.py`.

### 2.1 State semantics

`fixed_capacity` means the state coordinates retain their identity while the
propagation law moves. Authored discontinuous changes are refused.

`topology_edit` means the graph changes at a declared knot. It is legal only as
a step and must name one disposal policy:

```text
radiate | dissipate | retire | reject
```

There is no unqualified removal of state.

### 2.2 Interpolation

V1 supports:

```text
step | linear | smoothstep | smootherstep | cosine
```

The compiler derives segmentwise maximum velocity and, where the interpolation
is differentiable, acceleration. A linear path may carry a rate limit but not a
finite acceleration certificate at its corners. The declared quadratic metric
cost is reported as **path effort**, not silently promoted to physical work.

### 2.3 Deterministic lowering

Compilation:

1. validates finite values, exact endpoints, strict knot order, coordinate
   domains, state semantics, and optional kinematic limits;
2. samples at the declared control stride while also including every authored
   knot and exact endpoint;
3. emits only changed coordinate values after the initial frame;
4. records a conservative zero-order-hold execution error bound;
5. emits a receipt with analytic bounds and evidence class
   `validated-numerics`.

No body-specific clamp participates in acceptance. The generic compiler and
each body adapter reject out-of-domain values before audio or anatomy states
exist.

## 3. Cross-runtime identity

JSON number spelling is not a scientific identity. The semantic digest instead
hashes a binary canonical form containing:

- big-endian integer fields;
- UTF-8 strings with explicit lengths;
- sorted metadata and tracks;
- IEEE-754 float bits;
- stable enum names and option markers.

Compiled frames have a second digest under the same discipline. The committed
fixture pins both runtimes to:

```text
program sha256:f863212b5f40a0158719a4298d766bd4da41ccfdfb8430cbfd7654d03a7efc6d
frames  sha256:9604d346ac8ad35e338a3bffef75cd45ee887f57a774536bdc333207b0e23a0e
count   21
```

A Python or Rust implementation that disagrees does not share the IR.

## 4. MUS inline surface

Registered string-network controls may use arrows inside the existing bracket
surface:

```mus
A3w[
  couple=0.12@0->0.42@0.60->0.18@1,
  body=0.25->0.72->0.35,
  curvature=0.20->0.82->0.28,
  pitch_ratio=1->1.0594630943592953,
  traj_curve=cosine,
  traj_hz=200
]
```

`value@time` uses normalized event time in `[0,1]`. When `@time` is omitted,
knots are evenly spaced. One track may not mix timed and untimed terms.

Directives:

```text
traj_curve = linear | smoothstep | smootherstep | cosine
traj_hz = positive control-frame rate, at most the audio rate
traj_strict = 0 | 1
<control>_curve = per-track interpolation override
```

Only registered physical controls consume arrows. Existing MUS-A expressions
such as `pan=-1->1` and `lpf=70->900` remain untouched. The split function
returns a static patch containing each physical trajectory's initial value and
a separate immutable program.

The ordinary `mus-engine` source resolver now invokes this split automatically
for `synth=pluck` and `synth=weave` events. A non-empty program executes inside
one persistent `StringNetworkVoice` for the entire event; it is not rendered as
a succession of reconstructed endpoint notes. When no registered trajectory is
present, source resolution calls the historical scalar entry points directly.
That structural-absence law preserves the old path rather than emulating it
through an empty automation layer.

A trajectory attached to a non-string synth refuses. An event that combines a
trajectory with the external `sing` source, or authors a `string.sing` lane,
also refuses until that source path has an explicit supply/work account. These
are typed execution boundaries, not silent feature loss.

## 5. String-network adapter

`mus_dsp::trajectory` lowers the shared program into the existing persistent
`StringNetworkVoice`:

```text
TrajectoryProgram
  non-pitch tracks -> existing ControlFrame executor
  pitch track      -> existing native f0 -> f1 retuning path
```

There is no second automation loop and no downstream pitch effect.

Supported coordinates are:

```text
string.couple
string.chirality
string.orbit_hz
string.orbit_depth
string.curvature
string.body
string.buzz
string.sing
string.bow
string.bow_speed
string.sus
string.damp
string.breath
string.pitch_ratio
```

### 5.1 Pitch claim boundary

V1 accepts exactly two linear full-event `string.pitch_ratio` knots. It scales
the initial course frequencies and passes the endpoint ratio into the voice's
existing native persistent retune. Multi-knot or nonlinear pitch paths refuse;
they do not become framewise oscillator commands.

The execution is useful but not fully certified. Its receipt carries:

```text
integer-delay-state-transport
pitch-sideband-certificate
```

The allpass state may use the current neutral transport, while active delay-line
rank changes remain unresolved. `traj_strict=1` therefore refuses pitch before
rendering. Bow, breath, and externally driven source paths similarly carry
their open supply-ledger obligations.

### 5.2 Zero and determinism laws

The acceptance battery asserts:

- a constant path equal to the static patch is byte-identical;
- a moving path is deterministic and nontrivial;
- native pitch transport is finite and receipt-qualified;
- strict mode refuses before audio;
- unrelated arrows remain in the old MUS-A pipeline.

### 5.3 Ordinary render receipt

`mus-engine::source` returns a distinct `RenderedTrajectory` result containing
the adapter receipt. The event walker joins that immutable body receipt to:

```text
bar
track
onset_seconds
slot_seconds
```

without changing the trajectory program or frame digests. `mus render --json`
exposes the resulting list as optional `trajectoryReceipts`. The field is
structurally absent on a scalar-only score. The score event placement uses
camelCase JSON; the nested adapter and trajectory receipts retain their existing
snake_case schemas.

`mus check` remains an effect-free structural report and does not mint an
execution receipt. It preserves the raw event parameters, while executable
trajectory evidence belongs to render output. The warm `renderScore` service
currently returns its historical envelope; carrying the same optional receipt
list through that cache is a separate surface task.

## 6. GARGANTA adapter

The Python adapter consumes the same program while holding one
`AnatomyGenome` fixed. At each sample it evaluates:

```text
garganta.control.jaw_open
garganta.control.tongue_front
garganta.control.tongue_back
garganta.control.lip_round
garganta.control.apical_constriction
```

through the existing `area_function_cm2` map. Source coordinates remain
separate:

```text
garganta.source.pitch_ratio
garganta.source.lung_pressure
garganta.source.velum_aperture
```

A program may also address a section directly as
`garganta.tract_area_cm2.<index>` inside the established positive area cone
`[0.025, 18.0] cm²`. This is useful for controlled closure-oracle experiments;
it is not yet the G2 derived topology law.

The adapter receipt says `anatomy_static: true`, lists articulation, source, and
area-override coordinates separately, and retains the generic path receipt.
Representation has improved; evidence has not been inflated. G1 fast passage,
G2 closure/burst, G3 glottal supply, and G5 stochastic work remain separate
mathematical gates.

## 7. Relationship to Ariadne and Threader

Ariadne's compiled unitary words remain `GestureFrame`s. They may execute beside
a scalar `TrajectoryProgram`; order is content. A future inverse compiler may
return:

```text
GestureTarget
  -> TrajectoryProgram
  + optional compiled operator word
```

Threader should serialize posterior means and samples as trajectory programs
rather than unnamed frame tensors. Anatomy remains episode-persistent;
trajectory programs remain utterance-local. This preserves the current latent
factorization while giving its outputs score, engine, and receipt meaning.

## 8. Acceptance matrix

| Gate | Python | Rust | Claim |
|---|---:|---:|---|
| schema-valid program and receipt | yes | yes | deterministic transport object |
| pinned semantic/frame digests | yes | unit fixture | cross-runtime identity |
| analytic kinematic bounds | yes | yes | validated numerics |
| explicit topology disposal | yes | type-level | no silent deletion |
| MUS registered-arrow split | yes | yes | authoring surface |
| unrelated MUS-A arrows untouched | yes | yes | compatibility |
| string ControlFrame lowering | oracle | live voice | production execution |
| two-knot native pitch lowering | oracle | live voice | experimental, qualified |
| nonlinear pitch refusal | yes | yes | typed refusal |
| GARGANTA area-map motion | yes | n/a | anatomy-static causal control |
| whole score-engine dispatch | n/a | committed; gate not executed | event-local persistent execution |
| ordinary CLI trajectory receipt | n/a | committed; gate not executed | path identity joined to placement |
| certified variable-rank pitch | no | no | explicit obligation |
| G1/G2/G3/G5 vocal laws | no | no | explicit separate programs |

The two committed whole-render rows have permanent end-to-end tests, but the
normal GitHub Actions workspace job for their landing commit was created with
zero steps and refused by GitHub before execution because the organization
account's payment/spending limit currently blocks hosted runners. They remain
**implemented, not CI-verified** until the ordinary repository gate actually
runs.

## 9. Next gates

1. Execute `cargo fmt`, warnings-denied workspace clippy, and the complete
   release workspace against the ordinary-renderer landing commit once a Rust
   execution environment is available; do not promote the two committed rows
   above before that evidence exists.
2. Carry `trajectoryReceipts` through the warm `renderScore` service cache.
3. Add score-spanning programs whose timebase crosses note events while one
   body persists.
4. Implement and compare an integer-delay state connection against the dense
   variable-rank oracle; report state, waveform, sideband, pitch, and work
   errors separately.
5. Run GARGANTA P2 gesture-bandwidth sweeps in this representation.
6. Make Threader S1 emit posterior trajectory JSON.
7. Make `SPEC-GESTURE.md` lower effect-level targets into this IR.
