# Result and Artifact Contract

## 0. Purpose

This contract makes a run reproducible, resumable, comparable, and auditable. JSON Schemas in `schemas/` are normative for top-level structure. This document defines semantics that schemas cannot fully express.

---

# 1. Canonical JSON

All identity-bearing JSON uses:

- UTF-8;
- sorted object keys;
- no insignificant whitespace;
- finite JSON numbers only;
- integers where values are semantically integral;
- arrays in declared order;
- no timestamps inside identity payloads.

SHA-256 is computed over these canonical bytes.

---

# 2. Campaign lock

`campaign.lock.json` is produced by the planner and contains:

```json
{
  "schema_version": "extension.campaign.v1",
  "campaign_id": "...",
  "source": {
    "repository": "sophia-labs/instrument",
    "commit": "...",
    "image_digest": "sha256:...",
    "dependency_lock_sha256": "..."
  },
  "scientific_protocol": {
    "spec_revision": "gpu-battery-v1",
    "analysis_commit": "...",
    "primary_endpoints_sha256": "..."
  },
  "budget": {
    "gpu_seconds": 21600,
    "cpu_seconds_soft": 200000
  },
  "process_locks": [],
  "probe_locks": [],
  "experiments": [],
  "run_ids": [],
  "shard_ids": []
}
```

Once any non-smoke run starts, the lock is immutable. A change creates a new campaign ID.

---

# 3. Run manifest

A run manifest contains all variables needed to reproduce one independent training/evaluation unit.

Required semantic groups:

- identity;
- experiment/condition;
- process/task and data split hashes;
- model architecture and state budget;
- initialization and gauge;
- optimizer/scheduler;
- train budget;
- precision/determinism;
- diagnostics schedule;
- primary and secondary metric versions;
- artifact destination;
- parent/preprocessing artifact IDs.

No run may inherit an unrecorded scientific default from environment or source code.

## 3.1 Parent identity

Runs using an observable factorization include:

```text
factor_id
factor_manifest_sha256
factor_artifact_index_sha256
```

Runs using a teacher include `teacher_id` and teacher gauge. Runs resuming a checkpoint include the exact checkpoint hash.

---

# 4. Run lifecycle

Allowed states:

```text
planned
preprocessing
queued
running
preempted_resumable
completed
censored
failed_infrastructure
invalid_numerics
excluded_manifest_mismatch
cancelled_budget
```

State changes are append-only in `status-history.jsonl`. `status.json` is the latest materialized view.

A run counts in scientific analysis only if:

- `completed`;
- `censored` under a declared censoring rule;
- or an allowed exclusion is explicitly handled by the analysis.

---

# 5. Metric stream

`metrics.jsonl.zst` contains records:

```json
{
  "schema_version": "extension.metric.v1",
  "step": 100,
  "phase": "train|validation|diagnostic|test",
  "name": "excess_bps",
  "value": 0.123,
  "count": 100000,
  "wall_seconds": 12.3,
  "gpu_seconds": 12.3,
  "tags": {"mode": "3"}
}
```

Rules:

- `(phase,name,tags,step)` is unique after resume compaction;
- counts accompany aggregate losses;
- test records appear only after confirm lock;
- heavy arrays are stored in NPZ/Parquet and referenced by hash;
- NaN/Inf are represented by an invalid-run status, not JSON strings.

---

# 6. Result object

`result.json` is a compact final summary:

```json
{
  "schema_version": "extension.result.v1",
  "campaign_id": "...",
  "run_id": "...",
  "experiment_id": "E1-PREDCOORD",
  "condition": "T4-FUTURETEST",
  "status": "completed",
  "primary": {
    "metric": "excess_bps",
    "value": 0.042,
    "count": 393216
  },
  "secondary": {},
  "invariants": {},
  "resources": {},
  "artifacts": [],
  "manifest_sha256": "...",
  "result_sha256": "..."
}
```

`result_sha256` is computed with its own field omitted.

---

# 7. Scientific receipt

`receipt.json` records claims about the run's mechanism and validity:

```text
numerical gates
process/data identity
teacher/factor provenance
gauge checks
causal-control status
checkpoint replay status
test-access status
hardware/software identity
known warnings
```

A low loss with a failed receipt is not a valid scientific result.

---

# 8. Aggregate result

The aggregator writes:

```text
aggregate/run-accounting.json
aggregate/primary-effects.parquet
aggregate/secondary-effects.parquet
aggregate/bootstrap.json
aggregate/figures/*
aggregate/decision-<experiment>.json
aggregate/campaign-decision.json
```

Every aggregate table contains run IDs so every number can be traced to source artifacts.

---

# 9. Decision receipt

Per experiment:

```json
{
  "schema_version": "extension.decision.v1",
  "experiment_id": "E3-HEADMECH",
  "outcome": "supported|falsified|inconclusive|underpowered|not_replicated|invalid_numerics",
  "primary_endpoint": "...",
  "estimate": 0.0,
  "interval": [0.0, 0.0],
  "thresholds": {},
  "threshold_results": {},
  "controls": {},
  "exclusions": [],
  "replication": {},
  "promotion": {
    "tier_p_authorized": false,
    "reason": "..."
  },
  "analysis_source_sha256": "...",
  "input_artifact_index_sha256": "..."
}
```

The outcome is computed mechanically from pre-registered thresholds. A human narrative may add interpretation but may not change it without producing an amended analysis version and a visibly different decision object.

---

# 10. Artifact index

`artifact-index.json` lists:

```text
relative path
byte size
SHA-256
media type
semantic role
required/optional
```

`COMPLETE` is uploaded only after all required entries exist and verify.

---

# 11. Checkpoint compatibility

Checkpoint metadata includes:

- schema version;
- source commit and image digest;
- model class/version;
- manifest hash;
- step;
- vectorized ensemble mapping;
- optimizer implementation/version;
- RNG state versions;
- tensor dtype/device origin.

A resume with mismatched manifest or model version fails closed. A migration tool may produce a new checkpoint with a migration receipt; it never overwrites the original.

---

# 12. Privacy and data scope

The core battery uses synthetic process data and contains no personal data. Natural-text residual work is explicitly out of scope. Logs must still avoid environment secrets and presigned object-store URLs.