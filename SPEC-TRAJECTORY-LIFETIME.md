# MUS-T lifetime extension — track-owned string bodies

**Status:** executable strict v1; Rust execution gates pending an available runner  
**Parent contract:** `SPEC-TRAJECTORY.md`  
**Surface:** instrument parameter `body_lifetime=event|track`

## 1. Lifetime law

A physical body has an explicit owner and an explicit clock.

```text
body_lifetime=event (default)
  one score event owns one body and one event-local trajectory clock

body_lifetime=track
  the track owns one body from its first attack through the end of the
  final notated bar; one trajectory clock crosses intervening score events
```

Absence is exactly `event`. The historical event renderer remains a separate,
source-identical module; it is not emulated by constructing an empty track
session.

The score layer, attack layer, and allocation layer remain distinct:

```text
notated event       a note/chord token and its score onset
physical attack     work injected into an already-declared body
body construction  allocation and initialization of the body state
```

Three repeated note tokens therefore remain three score events even when the
strict v1 session executes one attack on one constructed body.

## 2. Track-session v1

The first executable track lifetime accepts:

```text
body       pluck | weave
attacks    exactly one, at the first pitched event
pitch set  fixed for the whole session
members    repeated identical note/chord tokens and rests
controls   at least one registered MUS-T arrow trajectory
clock      first attack -> end of final notated bar
```

Later note/chord tokens delimit notation and sidechain onsets but do not
re-excite or reconstruct the network. The delay lines, allpass memories,
frequency-dependent damping state, modal body, Weave waves, orbit phase, and
control state remain those of the first construction.

### 2.1 First-event envelope

The first pitched event may provide:

- construction/static string parameters already admitted by `SYNTH_KEYS`;
- registered MUS-T coordinates and trajectory directives;
- post-source session transforms such as filters, envelope, distortion,
  time treatment, pan, Haas delay, level, and reverb send;
- an optional score gliss, subject to the existing pitch-owner conflict law;
- articulation flags that the post-source transform/level stages actually
  implement.

Controls that belong to ordinary source resolution refuse instead of being
accepted and silently dropped. V1 therefore refuses, among others, sample
selection/offset, raw or pack gesture quotation, and `reverse`. Later event
params, flags, and glissandi also refuse: they require an explicit
rearticulation/control schedule rather than retrospective reinterpretation of a
session already in flight.

### 2.2 Constructor-owned subsystems

A subsystem whose deterministic state exists only when the body is constructed
must be present at sample zero. In particular, a `breath` trajectory may vary
only when its first value is nonzero, causing the content-keyed noise channels
to be allocated once. A path that rises from zero refuses before body
construction. External `sing` and `string.sing` remain refused until the
external-source supply/work ledger is part of the session executor.

### 2.3 Refusal boundary

The following refuse before audio or anatomy state exists:

```text
pitch-set or course-topology change
later event params, flags, or gliss
dynamic change after the first attack
any score hairpin
unpitched event
missing physical trajectory
non-string synth
unsupported first-event source-stage control or flag
external sing source or string.sing trajectory
breath trajectory that rises from zero after construction
```

These are capability boundaries, not claims that the final instrument cannot do
those things.

## 3. Session compilation and execution

The executor:

1. walks and validates the complete track without allocating DSP state;
2. computes absolute first-onset and final-bar times under the existing swing,
   tempo, and meter rules;
3. compiles one `TrajectoryProgram` against the complete session duration;
4. constructs one `StringNetworkVoice` with the first pitch set;
5. consumes canonical `ControlFrame`s using a monotone cursor while calling
   `next_sample` on that same voice;
6. applies the first event's admitted post-source transform and placement
   envelope once, over the session render;
7. records every pitched score onset for event and sidechain accounting;
8. emits one trajectory receipt joined to the first onset and full notated
   session span.

The generic semantic digest commits the following metadata:

```text
lifetime_scope=track
lifetime_track=<track abbreviation>
member_event_count=<notes, chords, and rests from first attack onward>
notated_event_count=<pitched score-event count>
physical_attack_count=1
body_construction_count=1
event_local_body_construction_baseline=<pitched score-event count>
avoided_body_constructions=<baseline - 1>
session_tail_count=1
event_local_tail_baseline=<pitched score-event count>
ordinary_trace_allocation=none
lifetime_end_seconds=<absolute score time>
execution_strategy=single-construction-monotone-no-trace-v1
```

The nested receipt promotes only to:

```text
claim_level    = score-spanning-single-body
evidence_class = validated-numerics+engine-execution+
                 single-construction+monotone-no-trace
```

The existing two-knot linear `pitch_ratio` subset still uses native persistent
retuning and still carries its delay-rank and sideband obligations. A richer
pitch path continues to refuse.

## 4. Performance law

The ordinary session path is a specialization, not an observational render:

- one body construction replaces the event-local construction baseline;
- one body tail replaces the event-local tail baseline;
- trajectory frames are consumed in monotone order with no per-sample search;
- no `WeaveTrace`, course-energy matrix, or observer ledger is allocated;
- output storage reserves exact capacity and grows by `push`, avoiding an
  unnecessary zero-fill pass before the samples are overwritten;
- every refusal occurs before physical-body allocation.

For an identical voice and canonical control-frame list, the no-trace executor
must produce byte-identical audio to `render_traced_with`. This is a permanent
unit law. No wall-time speedup is claimed until the benchmark harness executes.

The measurement entry point is:

```text
mus-rs/tools/benchmark_track_lifetime.py --mus <target/release/mus>
```

It reports event-local and track-lifetime timing distributions separately and
states explicitly that their different attack semantics do not imply audio
parity.

## 5. Next lifetime gate: preallocated rearticulation

Add a validated attack schedule to `StringNetworkVoice`:

```text
AttackFrame {
  at_sample
  active_course_set
  excitation identity and gain
  optional native retune target
}
```

An attack must inject declared work into existing state without zeroing or
reallocating delay lines, filter memories, the modal body, Weave wave variables,
or orbit phase. A pitch change inside fixed capacity needs state transport;
changing course capacity remains a topology edit with an explicit state
disposition.
