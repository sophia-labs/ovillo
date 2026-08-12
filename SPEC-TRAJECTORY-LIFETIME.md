# MUS-T lifetime extension — work-receipted rearticulation

**Status:** implemented strict v3; Rust execution gates pending an available runner  
**Parent contract:** `SPEC-TRAJECTORY.md`  
**Surface:** instrument parameter `body_lifetime=event|track`

## 1. Lifetime law

```text
body_lifetime=event (default)
  one score event owns one body and one event-local trajectory clock

body_lifetime=track
  one track owns one fixed-capacity body, one trajectory clock, and one
  physical attack schedule from its first pitched event through the end of
  the final notated bar
```

Structural absence is exact: the historical event renderer remains a separate
source-identical path. Track lifetime is not emulated by placing empty session
objects around event-local voices.

The score, source, and allocation ledgers are distinct:

```text
notated event       a note/chord token and score onset
physical attack     declared excitation injected into existing state
body construction  allocation and initialization of physical state
```

Three repeated notes under track lifetime therefore remain three notated
events, become three physical attacks, and use one body construction and tail.

## 2. Fixed-topology attack contract

The executable v3 scope is:

```text
body       pluck | weave
pitch set  fixed for the complete session
attacks    one per pitched score event
members    repeated identical notes/chords and rests
controls   at least one registered MUS-T trajectory
clock      first attack -> end of final notated bar
```

The DSP artifact is:

```rust
AttackFrame {
    at_sample: usize,
    gain: f64,
}
```

`gain` is an amplitude multiplier on the constructor's content-keyed physical
excitation. The engine currently derives it from score dynamics and gain
articulations:

```text
gain = 10^((dynamic_db + articulation_db) / 20)
```

Dynamic values use the existing MUS `DYN_DB` table. Gain articulations use the
existing `ART_GAIN` table (`acc`, `marc`, `sfz`, `stress`, `unstress`). This is
a change of physical ownership: dynamics and gain articulations now scale the
attack that enters the body, rather than scaling the completed session buffer.

Voice trim and explicit first-event `gain=` remain post-body placement controls.
They do not retrospectively alter the source-work receipt.

### 2.1 Schedule validation

Before allocating a `StringNetworkVoice`, `AttackSchedule::compile` derives the
exact constructor excitation span over the fixed active course set:

```text
max(active course strum offset + one-period excitation length)
```

It includes detuned companion courses and the pitch-scaled initial frequencies.
The schedule must satisfy:

```text
first attack sample = 0
attack samples strictly increase
0 < gain <= 4
attack onset < notated drive duration
constructor excitation windows do not overlap
```

Overlapping attacks refuse in v3. This is a bounded implementation contract,
not a claim that simultaneous contacts are physically impossible. A later gate
must define superposition and its source-work law.

### 2.2 Persistent state

At construction, each active course's original excitation is moved into a
fixed template bank. Every later attack reads that template at:

```text
sample - attack.at_sample - constructor_strum_offset
```

No attack clears or reconstructs:

- delay-line cells or write positions;
- one-zero and allpass memories;
- integer-delay interpretation;
- modal-body state;
- Weave waves;
- orbit phase;
- contact memory;
- breath PRNG/filter state;
- bow state;
- continuous trajectory controls.

Reattacks also do not restart native pitch transport or the onset-tension
relaxation clock. Those are explicit v3 semantics.

## 3. Exact attack-work receipt

For course wave value `w` immediately before attack excitation `e`, the declared
source work is:

```text
Delta E = w e + e^2 / 2
```

This is the same identity already used by the Phase-0 burst observer. It is
measured before external drive, breath, bow, and contact, preserving the
existing ordered source attribution.

The ordinary rearticulation kernel accumulates, per attack and active course:

```text
signed work
positive work
extracted work
peak absolute course-sample work
nonzero source-sample count
```

The execution receipt is content-addressed by a canonical binary SHA-256 over the schedule and per-attack work fields, and contains:

```text
RearticulationReceipt {
  schema_version
  work_digest
  schedule
  work_law
  work_unit
  total_signed_work
  total_positive_work
  total_extracted_work
  peak_abs_course_sample_work
  attacks[]
}
```

One-attack work must be bit-identical to the existing Phase-0 observer's final
`inject_burst` total. The reduction is deliberately course-major to preserve
that law.

Because attack work is now measured and nested in `StringTrajectoryReceipt`,
a multi-attack session no longer carries the former
`rearticulation-excitation-work-ledger` obligation. `traj_strict=1` may pass when
no independent pitch, bow, breath, or external-source obligation remains.

## 4. Score lowering

The planner tracks the dynamic state at every pitched onset. Standalone and
attached dynamics use the existing lexical rule, so:

```mus
{p} A3q Rq {f} A3q[acc] Rq
{ff} A3h[marc] Rh
```

lowers to three attacks with gains:

```text
p                  10^(-20/20)
f + acc            10^((-7+4)/20)
ff + marc           10^((-3.5+6)/20)
```

Continuation notes may carry only gain-articulation flags in v3. Later params,
glissandi, duration/contact articulations, and arbitrary flags require a typed
`RearticulationFrame` payload and therefore refuse. Score hairpins also refuse:
they need either a continuous excitation-gain law or an authored physical
trajectory, not retrospective interpolation over a rendered buffer.

The semantic trajectory digest commits:

```text
attack_samples
attack_dynamics
attack_articulation_db
attack_gain_db
attack_gains
attack_gain_law
attack_excitation_identity
attack_overlap_policy
physical_attack_count
body_construction_count
avoided_body_constructions
session_tail_count
execution_strategy
```

The state-dependent work receipt is not part of the authored program digest; it
is execution evidence produced by replaying that content-addressed program.

## 5. Refusal boundary

The following refuse before body allocation:

```text
pitch-set or active-course-topology change
later event params or gliss
a continuation flag outside ART_GAIN
score hairpin
unpitched event
missing physical trajectory
non-string synth
unsupported first-event source-stage control or flag
external sing source or string.sing trajectory
breath trajectory rising from zero
nonmonotone, out-of-window, nonfinite, or overlapping attacks
attack gain outside (0, 4]
strict transport with any remaining independent adapter obligation
```

Post-construction schedule/topology mismatches are internal invariant failures,
not user-authorship refusals.

## 6. Performance law

The v3 ordinary kernel is specialized for the observer-off, fixed-topology
session:

- N event-local bodies and tails become one body and one tail;
- attack and control schedules use monotone cursors;
- excitation storage is `O(active courses × one period)`, not score length;
- phase offsets and square-root normalizers are precomputed;
- no gesture, external-drive, backbone, trace, edge-telemetry, or network
  observer branch runs;
- `DelayString::read` and contact have explicit unobserved variants, removing
  the remaining per-course ledger branches;
- output capacity is reserved exactly and filled by append;
- work accumulators are fixed-size, allocated before sample zero.

Work accounting is not optional in this path: it is a small fixed set of scalar
accumulators required by the physical claim. It replaces the much larger
Phase-0 trace/ledger allocation and does not allocate per sample.

### 6.1 Zero-limit and causality laws

Permanent tests require:

```text
one unity attack, no controls
  byte-identical to StringNetworkVoice::render

one unity attack, canonical controls
  byte-identical to render_traced_with(...).audio

one unity attack, measured work
  bit-identical to Phase-0 inject_burst

second attack
  no change before onset; nontrivial change after onset
```

## 7. Receipt claim

The lifetime receipt promotes only to:

```text
claim_level = score-spanning-work-receipted-rearticulation

evidence_class = validated-numerics+engine-execution+
                 single-construction+physical-gain-lowering+
                 exact-attack-work+fully-unobserved-course-kernel
```

This does not certify overlapping contacts, pitch-set changes, arbitrary pitch
transport, perceptual equivalence to event-local rendering, or measured speedup.

## 8. Benchmark

```bash
python mus-rs/tools/benchmark_track_lifetime.py \
  --mus mus-rs/target/release/mus \
  --runs 9
```

The comparison holds notated event and attack counts fixed:

```text
event-local    3 events, 3 attacks, 3 bodies, 3 tails
track lifetime 3 events, 3 attacks, 1 body, 1 tail, exact attack receipt
```

Audio parity is not expected because persistent state is the intervention. No
wall-time, memory, or energy result is promoted until the harness executes.

## 9. Next gate

The next typed payload is:

```text
RearticulationFrame {
  at_sample
  excitation identity
  excitation gain
  duration/contact articulation
  optional fixed-capacity native retune target
}
```

Its obligations are:

1. bounded overlapping-source superposition;
2. duration/contact articulation without post-render truncation;
3. a continuous gain law for hairpins;
4. certified fixed-capacity pitch-change transport;
5. preservation of the one-attack byte and work laws.
