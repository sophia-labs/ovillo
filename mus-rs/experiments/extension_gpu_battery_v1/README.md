# EXTENSION GPU Battery v1

**Status:** complete implementation specification; no GPU results are claimed here  
**Target repository:** `sophia-labs/instrument`  
**Scientific base:** `mus-rs/experiments/extension_theory_loop_2`  
**Target implementation package:** `mus-rs/experiments/extension_gpu_battery_v1`  
**Nominal core budget:** 4–6 single-GPU hours after calibration

This directory specifies the first cloud-scale battery for the EXTENSION predictive-transport program. It converts the four highest-value open questions from Theory → Experiment Loop 2 into pre-registered, resumable, cloud-executable campaigns:

1. **Canonical predictive-coordinate supervision** — whether observable Hankel-derived predictive coordinates teach weak future modes better than generic future losses or predictive CCA.
2. **Untyped transport-category discovery** — whether a sparse bank selects positive-projective, noncommuting Clifford/Lie, commuting orthogonal, or affine-contracting transport when all branches receive the same raw event stream.
3. **Mechanics of attention-head emergence** — whether departure from the head-collision stratum admits a low-dimensional quantitative clock across optimizers, scales, and task spectra.
4. **Quotient-aware positive-realization learning** — whether canonical gauge control and projective preconditioning stabilize observable positive factorization and make performance more monotone in data volume.

The campaign is scientific rather than benchmark-driven. Every experiment has:

- a named theoretical prediction;
- a matched control;
- a primary metric and falsifier fixed before launch;
- an immutable manifest and exact process/data hashes;
- a staged smoke → screen → confirm budget;
- gauge-invariant or gauge-audited measurements;
- explicit negative-result handling;
- and a machine-readable result contract.

## Documents

- [`MASTER-SPEC.md`](MASTER-SPEC.md) — campaign-wide architecture, process suite, compute tiers, common invariants, and execution protocol.
- [`EXPERIMENT-01-PREDICTIVE-COORDINATES.md`](EXPERIMENT-01-PREDICTIVE-COORDINATES.md) — observable predictive-state and operator supervision.
- [`EXPERIMENT-02-UNTYPED-TRANSPORT.md`](EXPERIMENT-02-UNTYPED-TRANSPORT.md) — common-input transport-category discovery.
- [`EXPERIMENT-03-ATTENTION-EMERGENCE.md`](EXPERIMENT-03-ATTENTION-EMERGENCE.md) — secant-stratum emergence mechanics.
- [`EXPERIMENT-04-PROJECTIVE-OPTIMIZATION.md`](EXPERIMENT-04-PROJECTIVE-OPTIMIZATION.md) — quotient-aware positive realization.
- [`ANALYSIS-AND-DECISION-PLAN.md`](ANALYSIS-AND-DECISION-PLAN.md) — pre-registered statistics, stopping rules, and interpretation boundaries.
- [`CLOUD-RUNBOOK.md`](CLOUD-RUNBOOK.md) — image, Kubernetes, object-store, retry, checkpoint, and observability contracts.
- [`RESULT-CONTRACT.md`](RESULT-CONTRACT.md) — canonical run identity and artifact schemas.
- [`IMPLEMENTATION-HANDOFF.md`](IMPLEMENTATION-HANDOFF.md) — modules, tickets, tests, and definition of done.
- `configs/` — smoke and core campaign manifests.
- `schemas/` — JSON Schemas for campaign, run, and result objects.
- `cloud/` — reference Kubernetes Job template.

## Relationship to Loop 2

Loop 2 established that:

- future-mode decodability ranked models better than raw state size;
- an observable NMF + NNLS bridge could enter a near-oracle positive-projective basin;
- generic CCA could raise a balanced-rank statistic while worsening prediction;
- functional attention rank followed independent routing demand rather than declared head count;
- typed filtering and loop tasks selected projective and Clifford branches respectively;
- and a favorable positive operator could be compiled into local mass-conserving compartment steps.

This battery does **not** rerun those claims at greater scale. It attacks the newly isolated bottlenecks: canonical predictive coordinates, untyped category selection, the timing law of head symmetry breaking, and optimization on a gauge quotient.

## Non-goals

This battery is not:

- a natural-language benchmark;
- a claim that Clifford transport is universal;
- a claim that three-dimensional neurons outperform Transformers;
- a production training platform;
- a proof that NMF recovery is statistically consistent;
- or a substitute for later inverse-localization and residual-text experiments.

## Required top-level commands

The implementation must expose one stable CLI:

```bash
python -m extension_gpu_battery plan \
  --campaign configs/campaign-core.yaml \
  --output /work/plan

python -m extension_gpu_battery run-shard \
  --shard /work/plan/shards/00017.json \
  --artifact-root s3://<bucket>/extension/gpu-battery-v1

python -m extension_gpu_battery aggregate \
  --campaign /work/plan/campaign.lock.json \
  --artifact-root s3://<bucket>/extension/gpu-battery-v1

python -m extension_gpu_battery verify \
  --campaign /work/plan/campaign.lock.json \
  --artifact-root s3://<bucket>/extension/gpu-battery-v1
```

A campaign is not complete because pods exited successfully. It is complete only when `verify` confirms manifests, expected/censored runs, artifact hashes, primary analyses, and decision receipts.