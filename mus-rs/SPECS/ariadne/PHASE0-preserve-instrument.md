# Ariadne Phase 0 — preserve and instrument

Binding source: `ARIADNE-IMPLEMENTATION-CONSENSUS.md` §6 Phase 0 and §7
Phase 0 gates; `.string-research/NEXT-PHASE-REPORT.md` §7. This spec
localizes those contracts to the production engine. The consensus's
`retune=legacy` byte law is the hard law of this round: **any diff that
changes a legacy render is wrong**, however principled.

## A. `retune=neutral_filters`

The production retune surface is `DelayString::update_loop`
(pluck.rs, runs every 16 samples): it rewrites the two dispersion
allpass coefficients and the fractional-delay allpass coefficient with
filter state silently retained. That is the exact reinterpretation the
next-phase law repairs.

- New param `retune` (Enum `legacy|neutral_filters`, default `legacy`)
  → SYNTH_KEYS, vocab ParamSpec, dump test, static automation class.
- `mus-dsp` gains a dependency on `mus-watsn` (it has zero deps of its
  own). Use `mus_watsn::neutral_allpass_state_transport` — do NOT
  re-derive the law in pluck.rs; mus-watsn owns stage laws per the
  consensus module boundary.
- Under `neutral_filters`, every `FirstOrderAllpass::set_coefficient`
  site that can change a live coefficient applies the neutral state
  transport (`s = x1 − a·y1` preserved as `z = s/√(1−a²)`; retain `y1`,
  reconstruct `x1`). Under `legacy`, byte-identical behavior to today.
- **No-op updates are byte-inert under BOTH modes:** if the clamped new
  coefficient equals the old one, no state arithmetic runs.
- Integer-delay reinterpretation stays LEGACY in both modes — the
  active-line transport is Phase 3's problem. Do not touch it; a
  comment at the site names it as deliberately unsolved.

Laws (tests):
1. `retune` absent == `retune=legacy` == pre-change render:
   byte-identity on a bent note, a tension-decay note, and a weave
   chord (against the current engine — capture-in-test as PW10 §A did).
2. Static note (no bend, tension=0): `neutral_filters` == `legacy`
   byte-identical (coefficients never change after warmup → transport
   never fires; assert this holds exactly).
3. Bent note under `neutral_filters`: all samples finite, bounded ≤ 8.0
   across the bend corner grid (f0 × ratio × stiff), and the rendered
   f0 trajectory still lands within the tuning tolerance of the
   existing bend tests.
4. Per-update allpass work under `neutral_filters` is ≤ 1e-12 in the
   declared storage metric (instrument the transport to report it);
   under `legacy` it is nonzero on at least one bend fixture (the
   honest positive control).

## B. Observer-only telemetry

Extend the traced-render seam (`render_traced_with`) rather than
inventing a second one. A new observer, enabled by a render-entry-point
argument (NOT a patch param — observation is not part of the score):

- Per trace frame (existing stride), record per stage: delay-line
  energy (declared Euclidean research metric, ½Σcell²), allpass storage
  energies (`E_a(s)` per section), body-mode energy, contact state,
  source injections this window (burst, bow, breath, sing drive),
  damping/loop-gain loss estimate, and the output sum.
- **Observer-enabled and observer-disabled renders are byte-identical**
  (observers read; they never write — enforce by taking &self reads
  after the sample step).
- A ledger check test: on an un-bowed, un-breathed, un-sung note
  (passive regime), initial energy + Σsource − Σloss − Σoutput matches
  final energy within a declared residual; record the residual in the
  test. On bowed notes the ledger includes the bow as a source term —
  no passivity claim.

## C. Locate the bend transient

The consensus mandate: find the internal source of the production
3.24 → 5.38 upward-fifth peak rather than attributing it by assumption.
The next-phase reference model does NOT reproduce it, so it lives in a
stage that model omits (body, sympathetics, courses, contact, Weave
scattering, or output interference).

- Reproduce the extreme bend fixture with telemetry on, under both
  retune modes. Per-stage energy traces across the bend.
- Bisect by zero-limit switches (body=0, symp=0, buzz=0, courses,
  detune) — the existing zero-limit laws make each stage removable.
- Deliverable: `.string-research/PHASE0-TRANSIENT-REPORT.md` naming the
  stage(s) carrying the peak, with numbers, and whether
  `neutral_filters` changes it. A negative result (peak from output
  interference, not stored energy) is a valid finding — say so plainly.

## Shared rules

House gates (fmt, workspace clippy -D warnings, full release tests,
exit-code discipline). Consensus §7 Phase 0 gates verbatim: legacy
parity retained; observer on/off byte-identical; static neutral ==
legacy exactly; production allpass update work at numerical tolerance;
telemetry sums to ΔE within declared residual. Do not use the master
limiter in any energy measurement (consensus rule 7).
