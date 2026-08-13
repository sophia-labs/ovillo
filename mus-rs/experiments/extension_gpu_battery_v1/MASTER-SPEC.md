# EXTENSION GPU Battery v1 — Master Specification

## 0. Purpose

This document is the implementation team's plan of record for the first GPU-backed EXTENSION campaign. It translates four theory questions into a single reproducible cloud program while preserving the scientific separations learned in the CPU loops:

1. possessing state;
2. possessing expressive transport;
3. containing a useful predictive realization;
4. reaching that realization through optimization;
5. and, later, compiling it into a local body.

The battery covers items 2–4. It deliberately postpones arbitrary inverse localization and natural text until these bridges are better understood.

The campaign's scientific standard is **registered quantitative prediction**. Scaling up is useful only when it increases seed count, process diversity, phase-diagram coverage, or the fidelity of diagnostics that were too expensive on CPU.

---

# 1. Canonical experiment IDs

The numbering in this package follows the GPU-battery order requested for implementation. It maps to Loop 2's `NEXT-LOOP.md` as follows:

| GPU battery ID | Name | Loop 2 name |
|---|---|---|
| `E1-PREDCOORD` | Canonical predictive-coordinate supervision | Experiment A |
| `E2-CATEGORY` | Untyped transport-category discovery | Experiment C |
| `E3-HEADMECH` | Mechanics of attention-head emergence | Experiment D |
| `E4-PROJOPT` | Quotient-aware positive-realization learning | Experiment B |

Every run ID begins with one of these stable experiment IDs.

---

# 2. Core scientific principles

## 2.1 Predictive alignment, not generic capacity

A model state is useful only when its writable and readable modes align with distinctions in the past that alter the future distribution. Raw dimension, generic participation rank, Lie-algebra size, and declared head count are secondary diagnostics.

## 2.2 Transport category is a hypothesis, not a doctrine

The campaign treats the following as distinct candidate mechanisms:

- positive projective transport for normalized filtering;
- noncommuting Clifford/Lie transport for orientation and path order;
- commuting orthogonal transport as a causal control;
- affine contractive transport for driven forgetting;
- unconstrained linear transport as a capacity control.

No experiment may interpret a projective win as evidence against noncommuting transport on a different task, or vice versa.

## 2.3 Gauge-invariant function before raw factors

Positive operator factorizations and attention heads have non-identifiable coordinates. Primary comparisons must use:

- held-out sequence likelihood;
- future-test probabilities;
- normalized character/trace invariants;
- functional head contributions;
- principal angles after declared alignment;
- and controlled gauge sweeps.

Raw matrix distance is never a primary endpoint unless the gauge has been fixed by contract.

## 2.4 Oracles are for measurement, not training leakage

The controlled processes expose exact hidden states and Bayesian filters. Hidden-state identities may be used only to:

- compute oracle loss;
- validate observable factorization in a separately labeled analysis lane;
- generate exact diagnostic spectra;
- and detect implementation errors.

No training objective, model selection rule, early stopping decision, or hyperparameter choice may use hidden-state labels. The observable teacher in E1 and E4 must be derived from sampled symbols only.

## 2.5 Negative results are first-class

A run that falsifies the prediction is complete when:

- its manifest and environment are valid;
- numerical gates pass;
- the control behaves as expected;
- the primary metric is measured with the pre-registered analysis;
- and the falsification is emitted in the decision receipt.

Do not silently add decoder capacity, change a threshold, or substitute a friendlier process after seeing the result.

---

# 3. Common process suite

The core campaign uses four existing exact process specifications from `extension_theory_loop2.atlas.PROCESS_SPECS`:

| Alias | Existing spec | Role |
|---|---|---|
| `P-EASY4` | `p4_fast_clear` | low-rank, relatively well-observed calibration |
| `P-BASE6` | `p6_baseline` | canonical rank-six baseline |
| `P-NOISY6` | `p6_noisy` | weakly observable filtering |
| `P-WEAK8` | `p8_slow_weak` | ill-conditioned weak predictive modes |

Secondary confirmation may add `p6_cycle`, `p6_reversible`, and `p8_baseline`, but these may not replace a failed core process.

## 3.1 Immutable process lock

At planning time, materialize for every process:

```json
{
  "spec": {"...": "exact ProcessSpec fields"},
  "transition_sha256": "...",
  "emission_sha256": "...",
  "stationary_sha256": "...",
  "operator_sha256": "...",
  "teacher_hankel_sha256": "...",
  "oracle_eval_stream_sha256": "..."
}
```

The exact transition and emission arrays must be serialized in float64 little-endian form before hashing. No worker may regenerate a process from an unpinned package default without checking the resulting digest.

## 3.2 Data partitions

Each process has four independent symbol streams:

- `factor_train`: observable Hankel/factor estimation only;
- `model_train`: gradient training only;
- `validation`: hyperparameter selection and successive halving;
- `test`: one final confirm analysis.

The stream seeds are derived with a named hash from `(campaign_id, process_id, split)` rather than by adding an undocumented integer offset.

No sequence window may cross streams. Hidden-state sequences are never persisted in the training artifact store.

---

# 4. Common models and state API

All recurrent models must implement:

```python
class PredictiveStateModel(Protocol):
    state_size: int

    def initial_state(self, batch: int, *, device, dtype): ...
    def step(self, state, event, *, return_receipt: bool = False): ...
    def forward_with_states(self, events, state=None): ...
    def predict_from_state(self, state): ...
    def diagnostics(self) -> dict: ...
```

The battery reuses or adapts:

- `StableCliffordBank`;
- `PositiveProjectiveModel`;
- `LinearStateModel`;
- `SmallGRU`;
- the routed MHA task/model;
- and a bounded small volumetric or compartmental state model for selected E1 confirmation runs.

## 4.1 Device contract

Every tensor constructor must derive `device` and `dtype` from an explicit run context. Existing CPU-only constructors in Loop 2 must be corrected before cloud execution. In particular:

- process tensors `P`, `E`, and `pi` are cached once per process/device;
- sampling is device-aware or pre-generated in pinned host memory;
- `torch.set_num_threads(1)` is not called inside GPU workers;
- exact oracles and fragile decompositions use float64;
- trainable dynamics default to float32;
- bfloat16 is permitted only after a float32 agreement gate;
- no primary scientific run uses TF32 unless explicitly named in its manifest.

## 4.2 Vectorized ensembles

These models are too small to justify one pod or one CUDA stream per seed. The worker must support a leading ensemble dimension over independent model parameters and data streams, using either:

- explicit batched modules;
- `torch.func.functional_call` plus `vmap`;
- or another implementation whose numerical equivalence is tested.

DDP is out of scope for the core battery. One GPU worker should execute 8–64 independent micro-runs concurrently, depending on the experiment's Hessian and diagnostic load.

---

# 5. Campaign tiers

## 5.1 Tier S — local smoke

Purpose: schema, determinism, CLI, artifact, and basic gradient validation.

- one process;
- one seed (`7`);
- 40–80 train steps;
- reduced batch and sequence length;
- no Hessian or expensive factor search;
- must finish on CPU and GPU;
- not used in scientific conclusions.

## 5.2 Tier C — CUDA calibration

Purpose: measure actual throughput and determine ensemble size.

Required calibration matrix:

| Sequence length | Batch | Ensemble count | Precision |
|---:|---:|---:|---|
| 64 | 128 | 8, 16, 32 | fp32 |
| 128 | 256 | 8, 16, 32 | fp32 |
| 256 | 256 | 8, 16 | fp32 |

Record:

- train tokens/s;
- peak allocated/reserved memory;
- host-to-device fraction;
- diagnostic overhead;
- checkpoint upload time;
- CPU versus GPU numerical delta.

The planner uses these measurements to translate the nominal run budget into shard sizes. Do not assume that a faster GPU improves tiny-run throughput without vectorization.

## 5.3 Tier G — core 4–6 GPU-hour campaign

Use core processes and screening/confirmation schedules described in the four experiment documents. The target is a single modern GPU (L4, A10G, A100, or H100); the planner records the device and adjusts parallelism, not scientific step counts.

## 5.4 Tier P — extended 20–40 GPU-hour campaign

Only promoted conditions receive:

- ten seeds;
- all seven process regimes;
- longer sequences;
- broader gauge sweeps;
- more exact second-order diagnostics;
- and independent held-out confirmation configurations.

No Tier P campaign is authorized solely because Tier G produced a visually attractive plot. Promotion follows `ANALYSIS-AND-DECISION-PLAN.md`.

---

# 6. Core seed and split policy

## 6.1 Seeds

- screening: `[7, 11]`;
- core confirmation: `[7, 11, 19, 23, 31]`;
- untouched replication: `[37, 41, 43, 47, 53]`.

The replication seeds are not used for optimizer or threshold selection.

## 6.2 Fixed evaluation

Every process/task condition has a deterministic validation and test probe set stored or regenerable from a digest. Reusing a probe is intentional for paired comparisons. Training streams remain independent across seeds.

## 6.3 Evaluation precision

- likelihood and BPS accumulation: float64;
- singular values and principal angles: float64;
- Hessian-vector products: float32 computation, float64 aggregation where practical;
- classification and routing metrics: integer counts plus Wilson or bootstrap intervals.

---

# 7. Successive-halving policy

The core budget is preserved by staged elimination.

## Stage 0 — implementation smoke

All named conditions run for 5–10% of full steps. Eliminate only conditions that:

- produce NaN/Inf;
- violate exact invariants;
- fail to learn a calibration task;
- or exceed the declared resource envelope.

Scientific underperformance is not a Stage 0 elimination criterion.

## Stage 1 — screen

Run all conditions on two seeds and the screening process/task subset. Rank only by the pre-registered validation metric for that experiment. Continue:

- all mandatory baselines and controls;
- the best two experimental conditions;
- any condition within one standard error of the best;
- and one deliberately weak condition as a calibration control.

## Stage 2 — confirm

Run five core seeds and all required core processes/tasks. This stage produces the primary scientific decision.

## Stage 3 — untouched replication

Only if the primary success threshold is met, run the exact locked configuration on the untouched seed set. Replication failure changes the decision to `not_replicated`; it may not be hidden by pooling all ten seeds.

---

# 8. Reproducibility and identity

## 8.1 Campaign ID

`campaign_id` is the first 16 hex characters of SHA-256 over canonical JSON containing:

- source commit;
- Docker image digest;
- locked campaign manifest;
- process locks;
- analysis version;
- schema versions.

## 8.2 Run ID

`run_id` is the first 20 hex characters of SHA-256 over canonical JSON containing:

- campaign ID;
- experiment ID;
- condition;
- process/task;
- seed;
- stage;
- complete hyperparameter map;
- data split hashes;
- precision/determinism flags.

Run IDs are content identities, not timestamps.

## 8.3 Determinism levels

Each run declares one of:

- `exact`: bitwise deterministic on the same software/hardware class;
- `seeded`: deterministic algorithm requested, minor kernel variation permitted;
- `statistical`: nondeterministic kernels permitted, additional replication required.

The core battery targets `seeded`. Exact oracles and artifact hashes remain deterministic.

---

# 9. Common artifact contract

Each run writes, at minimum:

```text
manifest.lock.json
status.json
metrics.jsonl.zst
result.json
receipt.json
checkpoint-last.pt
checkpoint-best.pt          # only when validation-selected
stdout.log.zst
stderr.log.zst
environment.json
COMPLETE                    # zero-byte marker written last
```

The object-store prefix is:

```text
s3://<bucket>/extension/gpu-battery-v1/
  <campaign_id>/<experiment_id>/<shard_id>/<run_id>/
```

Partial prefixes without `COMPLETE` are resumable or failed artifacts, never valid results.

---

# 10. Numerical gates

Every shard must stop and emit `invalid_numerics` if any applicable gate fails:

## Clifford/Lie

- loop closure `<= 2e-6` fp32;
- operator orthogonality `<= 3e-6` fp32;
- commuting-control commutator norm below declared tolerance;
- noncommuting branch commutator norm above its minimum activation gate.

## Positive projective

- all effective factors finite and strictly positive;
- projective normalization residual `<= 1e-6` fp32;
- equivalent-gauge predictions agree within `2e-5` absolute probability;
- terminal and initial normalization finite;
- no factor condition number silently clipped without receipt.

## Attention

- invariant `(B_h,C_h)` reconstruction agrees with factorized forward contributions;
- probe outputs deterministic under internal Q/K and V/O gauge transforms within `2e-5`;
- functional-rank diagnostic uses a fixed probe set and versioned threshold.

## General

- no test split used before confirm completion;
- gradients finite;
- checkpoint reload reproduces validation metrics within tolerance;
- metrics file monotonic in step and free of duplicate records after resume.

---

# 11. Compute model

The planner must estimate and later report:

```text
estimated_gpu_seconds =
  train_steps × measured_step_seconds / ensemble_efficiency
  + diagnostic_seconds
  + checkpoint_seconds
```

The campaign budget is enforced in GPU-seconds, not pod wall time. CPU preprocessing and aggregation are reported separately.

Recommended core allocation after calibration:

| Experiment | Nominal share |
|---|---:|
| E1 predictive coordinates | 25% |
| E2 category discovery | 20% |
| E3 attention emergence | 35% |
| E4 projective optimization | 15% |
| reserve/retries | 5% |

The planner may reassign unused reserve but may not silently reduce required controls.

---

# 12. Cloud execution topology

The cloud implementation has four job types:

1. **Planner (CPU):** validate manifest, lock processes, enumerate runs, pack vectorized shards.
2. **Preprocessor (CPU/high-memory):** generate observable streams, Hankel blocks, NMF/NNLS candidates, and fixed probe sets.
3. **GPU worker:** train vectorized micro-runs, write checkpoints and metric streams, perform online lightweight diagnostics.
4. **Aggregator/verifier (CPU):** run exact analyses, pair conditions, bootstrap, generate decision receipts, and verify artifact hashes.

The GPU worker does not fit NMF in the critical path and does not compute full campaign aggregates.

---

# 13. Required decisions emitted by the campaign

The final `decision.json` contains one outcome per experiment:

```text
supported
falsified
inconclusive
invalid_numerics
underpowered
not_replicated
```

It also records:

- the exact primary endpoint;
- effect size and uncertainty;
- success/falsifier threshold;
- controls and invariant gates;
- exclusions with reasons;
- and whether Tier P promotion is authorized.

A prose report is generated from this receipt, not hand-written independently from the machine-readable result.