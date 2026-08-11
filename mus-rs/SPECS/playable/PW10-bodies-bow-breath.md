# PW10 — bodies, bow, breath: the 40%

The listening verdict on the ensemble (2026-08-10): one excitation
family plus one scaled body = one synth in costumes. This round closes
the engine half of that gap. The mathematics half (nonlinear source-
port work contracts; passive-fit projection) is queued for round 3 —
these implementations are **empirically bounded, declared-domain** per
house law, upgraded to ledgered later.

## A. Body profiles (`bodyp`)

`BODY_MODE_SPECS` becomes one member of a profile registry.

- `mus-dsp/src/body_profiles.rs`: `pub struct BodyProfile { name,
  modes: &'static [(f64, f64, f64, f64)] }` (freq, t60, coupling,
  radiation — same shape as today). Profiles: `guitar` (the current
  ten modes VERBATIM), `violin`, `cello`, `bass`, `mandolin` —
  literature-informed modal tables (main air resonance, main wood
  modes, plausible T60s/weights; cite reasoning in comments; these are
  design profiles, not measurements — say so in the doc strings).
- New param `bodyp` (Enum guitar|violin|cello|bass|mandolin, default
  guitar) → `make_body` selects profile; `body_size` still scales.
- **Compat law (hard):** `bodyp` absent or `guitar` ⇒ byte-identical
  to today across pluck AND weave (test against a captured render).
- Invariants: every profile bounded under the extreme patch; profiles
  mutually distinct (pairwise spectral distance on a reference note
  above a floor); vocab + SYNTH_KEYS + dump test.

## B. The bow (`bow`, `bow_speed`)

A sustained friction port at the excitation point — the first
excitation that RESPONDS to the string.

- Params: `bow` (Ratio 0..1, default 0 — pressure; 0 = no bow, hard
  zero-limit: byte-identical) and `bow_speed` (Ratio 0..1, default
  0.5 — maps to bow velocity ~0.05..0.4 in wave units). Both join the
  block-automatable set (crescendo/diminuendo bowing via trajectory)
  — extend `ControlTarget`/`automation_class` accordingly.
- Mechanism v1 (explicit, one-step): per sample, per ACTIVE course:
  `dv = v_bow − wave_value` (last loop value as the string-velocity
  proxy); friction force `f = bow_gain * bow * mu(dv)` with the
  hyperbolic curve `mu(dv) = sign(dv) * (mu_d + (mu_s − mu_d) /
  (1 + |dv|/v0))`, constants declared (`mu_s=0.8, mu_d=0.3, v0=0.1`
  starting points; tune against the invariants, record final). Inject
  `f` at the same point the pluck burst enters. The pluck burst scales
  by `(1 − bow)` like `sing` does.
- **Declared domain + honesty:** the bow is an energy SOURCE; the
  rotation proof does not cover it. Do not claim passivity. Invariants:
  bow=0 byte-identity; sustained tone (bowed note holds energy while
  the same note un-bowed decays); release (bow force stops at slot
  end and the tail decays to silence); boundedness across the (bow,
  bow_speed, sus, damp) corner grid at ≤ 8.0 peak; determinism.
  One-step explicit coupling (not iterated) is v1 — note the
  iterate-to-convergence upgrade in a comment.

## C. Breath (`breath`)

Sustained noise excitation — the aeolian/flute/shakuhachi corner.

- Param `breath` (Ratio 0..1, default 0, block-automatable). While in
  the slot, inject content-keyed filtered noise: np-rand stream from
  `content_seed` (tag the hash so breath and pick roughness differ),
  one-pole lowpassed with cutoff ≈ `2.5 × f0` of each course (per-
  course filter state), scaled `breath * 0.25 * coupling_scale`,
  bridge-common like `sing`. Pluck burst scales by `(1 − breath)`.
- Zero-limit byte-identity at 0; determinism (content-keyed, no global
  RNG); silence after slot + decay; boundedness. Combined with weave
  coupling this should give breathy halos — render one demo probe in
  the report.

## Shared rules

Every param: SYNTH_KEYS, typed vocabulary entry, dump-test additions,
zero-limit law tested directly. `bow`/`breath`/`sing` compose by
summing excitations (document the interaction; no hidden priority).
Gates: fmt, workspace clippy -D warnings, full workspace tests, the
honest exit-code discipline. Round-3 note additions (do not implement):
nonlinear source-port ledger contract; passive-fit body projection.

---

## Status — landed 2026-08-10

All three lines built in parallel worktrees off `bc7cf2e`, independently
gated, adversarially reviewed (land-ready ×3), merged A → C → B with the
synthesist's conflict analysis, and re-gated on the merged head (fmt 0,
clippy `-D warnings` 0, `test --release --workspace` 0: 38 suites, 263
passed, `pluck_invariants` 33). Post-merge, three composition laws were
added that no per-line suite could see: combined explicit-zeros identity,
bow+breath boundedness, and non-guitar bodies under bow+breath.

**Honest notes for the round record:**

1. **§A compat law satisfied by substitution, declared here.** The spec
   said "test against a captured render"; no captured-render golden exists
   anywhere in the repo for the string-network engine (`synth=pluck`/
   `weave` — checked: `golden_*.rs` covers the subtractive synth only).
   The landed law is the two-part argument: absent == `guitar` == any
   unrecognised name (byte-identity), plus the guitar table asserted
   verbatim against an independently transcribed historical literal.
   Creating a real captured-render golden for this engine is an open
   item for a future round.
2. **Course-set asymmetry (deliberate):** `breath` is bridge-common like
   `sing` and reaches every course — sympathetic and Weave-virtual
   included. The bow touches only ACTIVE courses. Documented at the
   composition site in `next_sample`.
3. **§C's "render one demo probe in the report"** was scoped out of the
   worker's delegation; closed at landing instead: `aliento y arco`
   (8 bars — cello/violin bodies bowed, mandolin-bodied weave halo
   breathed, all three sources composed in bar 8) rendered clean with
   the merged engine (`renderDigest sha256:f58a5b66…`, peak −2.9 dBFS,
   zero bad tokens) → `~/Desktop/mus-listening/pw10_aliento_y_arco.wav`.
4. **§B landed two declared deviations with measurements:** `BOW_GAIN`
   0.15 tuned against a sweep table (Schelleng-like minimum bow force
   emerges); the injected friction force is DC-blocked per course
   (~5 Hz pole) — without it the v1 relay friction pins the course at
   loop DC and no tone speaks (tonality 1.16 raw vs 2.0e6 blocked).
5. **Declared-unusable regions are asserted as refusals:** below minimum
   bow force (no sustain claimed), and `bow_speed=0` with heavy bow
   (broadband chatter, not offered as a timbre).
6. Sing/bow/breath compose by summing at one excitation node; the pluck
   burst scales by `(1-sing)(1-bow)(1-breath)` — no hidden priority.
