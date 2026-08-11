# Ariadne Phase 2 — closed loops on the real ring

Binding source: `ARIADNE-IMPLEMENTATION-CONSENSUS.md` §6 Phase 2 / §7
Phase 2 gates; `IMPLEMENTATION-HANDOFF.md` Commit F;
`EXACT-CLOSED-SO3.md`. Phase 0/1 are landed (`wt/PH0` f2bc7f5,
`wt/PH1` b01f599, merged); this phase makes compiled closed control
words executable inside the production Weave voice and renders the
same-endpoint experiment the paper has been asking for since round 2.

Three lines. A and B are independent; C consumes both.

## A. `mus-gesture` closed-loop compiler (Commit F)

New module `closed_loop.rs` in `mus-rs/crates/mus-gesture`:

- `balanced_commutator_angle(phi) = 2*asin(sqrt(sin(phi/4)))`, domain
  [0, pi], typed error outside.
- `compile_exact_closed_loop_so3(target, graph, duration)`: rotvec of
  target -> (phi, axis); balanced local commutator from R01(t), R12(t);
  conjugator S mapping the commutator axis onto the target axis
  (deterministic antipodal branch, like the reference); macros A = S
  A0 S^T, B = S B0 S^T each compiled by the existing tree-QR; word
  emitted in OUR pinned OperatorProgram convention so it evaluates to
  A B A^-1 B^-1 (write the order out explicitly against the convention
  test — the reference's word order is left-action-specific).
- Identity target -> empty word. Every nonidentity target -> exactly
  12 authored-edge gates.
- Receipt: extends the Phase 1 receipt with target angle/axis, balanced
  generator angle, per-edge closure (assert <= 1e-12, NEVER == 0.0),
  and the thermodynamic timing of the 12-gate word.
- Fixtures: generate a fresh closed-loop fixture set from the Python
  oracle (`compile_exact_closed_loop_so3` in
  `reference/ariadne_gesture/closed_loop.py`) the way Phase 1 did for
  tree-QR — commit generator + JSON. Do NOT parse the packet's
  committed `results.json` for this.
- Gates (handoff Commit F + consensus): 1,000 random targets, endpoint
  < 1e-11 Fro; direct dense commutator and lowered word agree; identity
  empty; near-identity and near-pi targets in the corpus; work_residual
  None or ledger-measured (Phase 1 rule 4 carries over).

## B. The gesture seam in the production voice

The Weave's scattering ring derives its angles from params; a compiled
word needs EXPLICIT plane rotations at declared samples. Add the seam,
zero-limit-lawful like every seam before it:

- `GestureFrame { at_sample: usize, course_a: usize, course_b: usize,
  theta: f64 }` (pub, mus-dsp). At its sample, after the normal
  scattering pass, apply the plane rotation to the two courses' wave
  variables — the same state the ring's own Givens rotations act on.
  Frames sorted by sample; multiple frames at one sample apply in
  order.
- `render_traced_gestured(stride, trajectory, gestures)`; the existing
  entry points delegate with an empty gesture list.
- Laws (tests):
  1. Empty gesture list == today's render, byte-identical (pluck AND
     weave).
  2. theta = 0 frames are byte-inert; [theta, -theta] at the SAME
     sample composes to identity, byte-identical to none.
  3. Energy-neutral at the application site: the two courses' wave
     sum-of-squares is preserved to roundoff by each frame (assert in
     debug or test hook — this is the "factor-stage work is zero" gate
     localized).
  4. Rotations on DISJOINT course pairs at the same sample commute
     byte-identically (they touch disjoint memory — assert exact).
  5. Executor agreement: on a lossless configuration (damp=0, huge sus,
     couple=0, no dispersion), a compiled 12-gate word applied all at
     one sample rotates the course wave-vector by the dense target to
     <= 1e-8; this is the "live Weave state agrees with the compiled
     operator" gate in its honest form (with dynamics between frames
     the state evolves — that is the music, not an error).
  6. Host-block invariance with gestures (render_block parity).
  7. Determinism; out-of-range course indices are a typed error, not a
     clamp.
- Service: `renderWeaveProbe` accepts optional
  `gesture: [{atSeconds, courses: [a, b], theta}]`; refusal
  `unsupported-gesture` for non-weave synth; gestures fold into the
  memo key like trajectories do.

## C. The same-endpoint production experiment + stimuli

Consumes A and B. Deliverable: `.string-research/PHASE2-REPORT.md` +
stimuli WAVs (gitignored dir, generator committed).

- Compile a nonidentity SO(3) target for courses (0,1,2) of a played
  weave chord; schedule the 12 gates across the sustain with the
  thermodynamic timing; render.
- Controls, each rendered identically except the word:
  null (empty), commuting pair (disjoint edges, both orders — assert
  byte-identical), inverse loop (word then its exact reversal-negation
  — assert the trace endpoint returns), orientation (mirrored word),
  and the two orders A,B,-A,-B vs B,A,-B,-A (the round-2 experiment,
  now with CLOSED per-edge controls — this is the render the paper's
  limitation 5 has demanded since round 2).
- Matched loudness: report RMS/peak per stimulus; master limiter
  forbidden (consensus rule 7).
- Report the trace-endpoint state distances, the audible RMS distances,
  and the per-edge closure receipts. No holonomy language beyond what
  the consensus terminology discipline allows: these are closed-control
  monodromy renders; listener validation remains future work.

## Shared rules

House gates (fmt, workspace clippy -D warnings, full release tests,
exit-code honesty, logs never in bare /tmp). Do not touch retune/
telemetry surfaces (Phase 0 owns them); do not modify mus-watsn except
via its public API. Workers commit to their wt/ branch, never push.
