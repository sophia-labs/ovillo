# MUS-T lifetime extension — track-owned, rearticulated string bodies

**Status:** executable strict v2; Rust execution gates pending an available runner  
**Parent contract:** `SPEC-TRAJECTORY.md`  
**Surface:** instrument parameter `body_lifetime=event|track`

## 1. Lifetime law

A physical body has an explicit owner and clock.

```text
body_lifetime=event (default)
  one score event owns one body and one event-local trajectory clock

body_lifetime=track
  the track owns one body from its first pitched attack through the end of
  the final notated bar; one trajectory clock and one attack schedule cross
  every intervening score event
```

Absence is exactly `event`. The parity-locked historical renderer remains a
separate source-identical module; it is not emulated by constructing an empty
track session.

The score, excitation, and allocation ledgers are distinct:

```text
notated event       one note/chord token and score onset
physical attack     one scheduled injection of excitation work
body construction  allocation and initialization of physical state
```

For three repeated note tokens under `body_lifetime=track`, v2 therefore records
three score events, schedules three physical attacks, and constructs one body.

## 2. Fixed-topology rearticulation v2

The executable lifetime accepts:

```text
body       pluck | weave
attacks    one per pitched score event
pitch set  fixed for the complete session
gain       unity for every continuation attack in v2
members    repeated identical note/chord tokens and rests
controls   at least one registered MUS-T arrow trajectory
clock      first attack -> end of final notated bar
```

Each repeated note/chord onset becomes an `AttackFrame`:

```text
AttackFrame {
  at_sample
  gain
}
```

The engine reuses the constructor's content-keyed excitation template on every
attack. It does **not** clear or reconstruct:

- delay-line samples or write indices;
- damping and allpass memories;
- modal-body state;
- Weave wave variables;
- orbit phase;
- contact memory;
- breath or bow state;
- trajectory-control state.

V2 does not yet attach a different excitation identity, gain derived from a
later dynamic, per-attack articulation, or pitch target to a continuation. Those
belong to the next payload contract.

### 2.1 Attack scheduling law

Before allocating a `StringNetworkVoice`, the engine derives the exact attack
span of the fixed active course set:

```text
max over active courses (
  constructor strum offset
  + constructor excitation-template length
)
```

The schedule must satisfy:

```text
first attack sample = 0
attack samples strictly increase
0 < gain <= 4
attack onset < notated drive duration
successive constructor excitation windows do not overlap
```

Overlapping attacks refuse in v2. This is a narrow execution boundary, not a
claim that superposed pick contacts are physically invalid. It keeps exactly
one active template per course in the real-time kernel and gives the first
implementation a constant-memory, monotone cursor.

The ordinary MUS adapter emits `gain=1`. The public DSP schedule type carries
gain now so dynamics can later lower into a declared excitation coordinate
rather than being smuggled into mix gain.

### 2.2 First-event session envelope

The first pitched event may provide:

- construction/static string parameters admitted by `SYNTH_KEYS`;
- registered MUS-T coordinates and trajectory directives;
- post-source session transforms such as filters, envelope, distortion,
  time treatment, pan, Haas delay, level, and reverb send;
- an optional score gliss, subject to the existing pitch-owner conflict law;
- articulation flags implemented by the post-source transform/level stages.

Those post-source values apply to the complete session render. Later event
params, flags, glissandi, and dynamic changes refuse: v2 knows that a
continuation is an attack, but it does not yet have a typed per-attack payload
for those fields.

Controls belonging to ordinary source resolution—sample selection/offset,
quotation material, `reverse`, and similar—also refuse rather than being
silently ignored.

### 2.3 Constructor-owned subsystems

A subsystem whose deterministic state exists only at body construction must be
present at sample zero. A `breath` trajectory may vary only when its first value
is nonzero, causing the content-keyed noise channels to be allocated once. A
path rising from zero refuses before construction.

External `sing` and `string.sing` remain refused until external-source supply
and work are part of the session executor.

### 2.4 Refusal boundary

The following authoring cases refuse before physical-body allocation:

```text
pitch-set or active-course-topology change
later event params, flags, or gliss
dynamic change after the first attack
any score hairpin
unpitched event
missing physical trajectory
non-string synth
unsupported first-event source-stage control or flag
external sing source or string.sing trajectory
breath trajectory that rises from zero
non-increasing, out-of-window, non-finite, or overlapping attack schedule
traj_strict=1 with more than one attack while attack work is unreceipted
```

A post-construction `AttackSpanMismatch` or topology mismatch is an internal
invariant failure, not an authoring refusal.

## 3. Compilation and execution

The track executor:

1. walks and validates the complete track without allocating DSP state;
2. computes every absolute note onset under the existing tempo, meter, and
   swing laws;
3. compiles one `TrajectoryProgram` over the full session duration;
4. lowers note onsets to exact integer-sample `AttackFrame`s;
5. validates the fixed-topology attack schedule before body construction;
6. constructs one `StringNetworkVoice`;
7. moves each active course's constructor excitation into a fixed template bank;
8. consumes controls and attacks with monotone cursors while advancing that one
   body;
9. applies the first event's admitted post-source session envelope once;
10. records every pitched score onset for event and sidechain accounting;
11. emits one trajectory receipt joined to the first onset and full session
    span.

The semantic digest commits:

```text
lifetime_scope=track
lifetime_track=<track abbreviation>
member_event_count=<notes, chords, and rests from first attack onward>
notated_event_count=<pitched score-event count>
physical_attack_count=<pitched score-event count>
attack_samples=<comma-separated exact sample indices>
attack_gain_policy=unity-v2
attack_excitation_identity=constructor-content-keyed-template
attack_overlap_policy=refuse-v1
body_construction_count=1
event_local_body_construction_baseline=<pitched score-event count>
avoided_body_constructions=<baseline - 1>
session_tail_count=1
event_local_tail_baseline=<pitched score-event count>
ordinary_trace_allocation=none
lifetime_end_seconds=<absolute score time>
execution_strategy=prevalidated-monotone-attack-cursor+
                   network-observer-off-v2
```

The execution receipt promotes only to:

```text
claim_level    = score-spanning-preallocated-rearticulation
evidence_class = validated-numerics+engine-execution+
                 single-construction+preallocated-attacks+
                 network-observer-off-specialization
```

Every multi-attack receipt also carries the unresolved obligation:

```text
rearticulation-excitation-work-ledger
```

An attack is an energy source. The schedule declares where and how the source is
applied, but the ordinary observer-off render does not yet integrate its work.
`traj_strict=1` therefore refuses multi-attack execution.

The existing two-knot linear `pitch_ratio` subset still uses native persistent
retuning and retains its delay-rank and sideband obligations. Reattacks do not
reset the onset-tension clock or restart native pitch transport.

## 4. Performance law

The v2 session path is a dedicated ordinary-render specialization:

- N event-local body constructions become one;
- N event-local tails become one;
- attack frames and excitation templates allocate once;
- attack lookup is a monotone cursor, not a per-sample search;
- no excitation buffer proportional to the score duration is allocated;
- phase offsets `2πi/N` are precomputed once;
- active-course and whole-network square-root normalizers are precomputed once;
- the network-level hot path omits gesture execution, external-drive dispatch,
  backbone dispatch, edge telemetry, body-energy observation, trace allocation,
  and observer-ledger writes;
- canonical control frames remain a monotone cursor;
- output capacity is reserved exactly and filled by append;
- all user-facing schedule refusals occur before body construction.

The per-course `DelayString::read` and contact functions still contain their
observer conditionals. Full monomorphization of those inner kernels remains a
separate performance gate.

### 4.1 Zero-limit law

For one unity attack at sample zero and identical canonical control frames,
`RearticulatedStringVoice` must produce byte-identical audio to the historical
voice. Permanent unit tests compare:

```text
one attack, no controls  <-> StringNetworkVoice::render
one attack, controls     <-> render_traced_with(...).audio
```

A second attack must leave the entire prefix before its onset byte-identical and
change the suffix.

### 4.2 Measurement

```text
mus-rs/tools/benchmark_track_lifetime.py --mus <target/release/mus>
```

The v2 harness compares:

```text
event-local    three notated events, three attacks, three bodies, three tails
track lifetime three notated events, three attacks, one body, one tail
```

Both sides now have the same attack count. Audio parity is still not expected:
the track-owned body retains physical state between attacks while event-local
bodies begin from zero. No speed, memory, or energy number is promoted until the
harness runs.

## 5. Acceptance matrix

| Gate | Permanent evidence | Claim |
|---|---|---|
| one attack byte identity | DSP unit test | specialization preserves zero limit |
| controlled one-attack byte identity | DSP unit test | no-trace controls preserve audio |
| second attack prefix law | DSP unit test | no effect before authored onset |
| overlap refusal | DSP unit test | one active template in v2 |
| three score events / three attacks / one body | engine + CLI tests | score and physics ledgers stay distinct |
| cross-bar deterministic receipt | CLI test | content-addressed lifetime |
| fixed topology refusal | engine + CLI tests | no hidden body reconstruction |
| strict work refusal | engine test | unreceipted source work not promoted |
| measured speedup | not yet run | withheld |

## 6. Next lifetime gate

The next payload extends `AttackFrame` without changing body capacity:

```text
RearticulationFrame {
  at_sample
  excitation identity
  excitation gain
  optional articulation/contact controls
  optional fixed-capacity native retune target
}
```

Required work:

1. define and receipt per-attack excitation work;
2. map score dynamics and accents to physical gain rather than post-mix gain;
3. admit overlapping attacks through a bounded superposition law;
4. give pitch changes a certified fixed-capacity state connection;
5. preserve the one-attack byte law and the no-allocation sample kernel.

Changing course capacity remains a topology edit with an explicit state
disposition.
