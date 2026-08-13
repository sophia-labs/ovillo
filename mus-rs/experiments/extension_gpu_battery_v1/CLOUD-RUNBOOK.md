# Cloud Runbook — EXTENSION GPU Battery v1

## 0. Scope

This runbook defines the reference cloud execution model. The scientific package remains in `sophia-labs/instrument`; cloud infrastructure may live in `sophia-labs/cloud` or another internal deployment repository.

The reference target is Kubernetes on AWS/EKS because Sophia's cloud stack already uses Kubernetes-style workloads. The experiment runner itself must remain portable to another Kubernetes cluster or a single Docker host.

The workers are **ephemeral pure-function jobs with checkpointed state**, not long-lived Garden cells. They consume immutable manifests and object-store inputs, then emit content-addressed artifacts.

---

# 1. Execution architecture

```text
campaign YAML
    |
    v
planner CPU Job
    |-- locks source/image/processes/analysis
    |-- enumerates run manifests
    |-- packs vectorized GPU shards
    v
CPU preprocessing Jobs
    |-- streams / Hankel blocks / NMF / NNLS / probes
    v
GPU worker Jobs
    |-- vectorized training micro-runs
    |-- lightweight online diagnostics
    |-- resumable checkpoints
    v
GPU diagnostic replay Jobs (E3 heavy diagnostics)
    v
CPU aggregator Job
    |-- exact metrics / bootstrap / plots / decisions
    v
verifier Job
    |-- hashes / completeness / schema / run accounting
```

The planner and verifier are deterministic. Re-running either against the same locked campaign must produce identical run and decision identities.

---

# 2. Container image

## 2.1 Reference environment

Pin exact versions after CUDA calibration. Initial target:

```text
Python          3.11.x
PyTorch         2.10.x CUDA build or cluster-supported equivalent
CUDA runtime    12.x, pinned by image digest
NumPy           2.x
SciPy           >=1.14
scikit-learn    >=1.5
Matplotlib      >=3.9
pyarrow         pinned
zstandard       pinned
boto3           pinned when S3 adapter enabled
jsonschema      pinned
```

Do not rely on unconstrained `>=` dependencies in the production image. Generate a lock file and record its SHA-256.

## 2.2 Docker requirements

- image is built from an immutable `instrument` commit;
- base image is referenced by digest;
- non-root UID/GID;
- source installed as a wheel or locked editable package for development only;
- no Git credentials inside the image;
- `PYTHONHASHSEED` explicit;
- locale/timezone fixed;
- entrypoint is the battery CLI;
- image includes a `self-check` command that imports CUDA, loads schemas, and runs one exact Clifford and one positive-gauge fixture;
- image digest appears in every campaign and run manifest.

## 2.3 Build outputs

Publish:

```text
<registry>/instrument-extension-gpu:<git-short-sha>
<registry>/instrument-extension-gpu@sha256:<digest>
```

The campaign uses only the digest form.

---

# 3. Kubernetes job classes

## 3.1 Planner

```yaml
resources:
  requests: {cpu: "2", memory: 4Gi}
  limits:   {cpu: "4", memory: 8Gi}
```

- no GPU;
- reads campaign config from ConfigMap or object store;
- writes `campaign.lock.json`, process locks, run manifests, and shard manifests;
- exits before workers launch if schema or budget validation fails.

## 3.2 Preprocessor

```yaml
resources:
  requests: {cpu: "8", memory: 32Gi}
  limits:   {cpu: "16", memory: 64Gi}
```

- no GPU by default;
- separate Jobs by process and stream length;
- caches observed streams, word-count tables, Hankel blocks, NMF candidates, fixed probes;
- writes immutable artifacts before GPU jobs begin;
- may use a high-memory node for depth-3 Hankel blocks.

## 3.3 GPU worker

Recommended baseline:

```yaml
resources:
  requests:
    cpu: "6"
    memory: 24Gi
    ephemeral-storage: 50Gi
    nvidia.com/gpu: "1"
  limits:
    cpu: "12"
    memory: 48Gi
    ephemeral-storage: 100Gi
    nvidia.com/gpu: "1"
```

One GPU per pod. Do not use DDP in the core battery.

Preferred hardware order for cost efficiency:

1. L4 or A10G for ordinary training shards;
2. A100/H100 only if calibration shows materially better ensemble throughput or E3 diagnostics need memory/compute;
3. avoid very small GPU instances if host CPU throttles generation or upload.

The campaign records actual GPU name, memory, driver, CUDA, power state when available, and measured throughput.

## 3.4 Diagnostic replay

E3 Hessian/noise replay may request:

```yaml
resources:
  requests:
    cpu: "12"
    memory: 48Gi
    nvidia.com/gpu: "1"
  limits:
    cpu: "24"
    memory: 96Gi
    nvidia.com/gpu: "1"
```

It loads selected checkpoints and computes expensive diagnostics without blocking ordinary training shards.

## 3.5 Aggregator/verifier

CPU-only; 16–64 GiB depending on result volume. Exact SVD/bootstrap may be multithreaded but must fix thread counts in the manifest.

---

# 4. GPU node scheduling

The template uses placeholders for the cloud team's actual labels:

```yaml
nodeSelector:
  accelerator.sophia-labs.com/class: ${GPU_CLASS}

tolerations:
  - key: nvidia.com/gpu
    operator: Exists
    effect: NoSchedule
```

The cloud adapter may translate `GPU_CLASS=l4-24g` to provider-specific labels. Scientific manifests never contain EC2 instance type as a required semantic variable; hardware identity is an execution receipt.

Use topology spread only when multiple simultaneous workers are launched. One shard never spans nodes.

---

# 5. Object-store contract

## 5.1 Authentication

Use workload identity/IRSA. No static AWS access keys in ConfigMaps, environment manifests, checkpoints, or logs.

Required permissions are prefix-scoped:

- planner: read configs, write campaign plan;
- preprocessor: read plan, read/write preprocessing prefix;
- worker: read assigned shard and inputs, read/write assigned output prefix;
- aggregator: read all campaign outputs, write aggregate prefix;
- verifier: read all, write final verification receipt.

Workers must not list or overwrite other campaign prefixes unless required for resume checks.

## 5.2 Prefix layout

```text
s3://<bucket>/extension/gpu-battery-v1/<campaign_id>/
  campaign.lock.json
  environment.lock.json
  processes/<process_id>/...
  probes/<probe_id>/...
  factors/<factor_id>/...
  shards/<shard_id>.json
  runs/<experiment_id>/<shard_id>/<run_id>/...
  aggregate/...
  verification/...
```

## 5.3 Upload discipline

- write to `/scratch/<run_id>`;
- fsync critical JSON and checkpoint files;
- compute SHA-256 locally;
- upload content files;
- upload `artifact-index.json` with hashes;
- upload `COMPLETE` last;
- never mutate a completed run prefix;
- on retry, compare manifest identity and resume into the same run prefix only if no `COMPLETE` marker exists.

---

# 6. Checkpoint and preemption semantics

## 6.1 Checkpoint cadence

Checkpoint at the earliest of:

- every 100 training steps;
- every five minutes;
- immediately before a planned heavy diagnostic;
- immediately after first emergence detection in E3;
- or on SIGTERM/preStop when enough grace remains.

A checkpoint includes:

- model parameters;
- optimizer and scheduler state;
- RNG states for Python, NumPy, CPU torch, and CUDA;
- data-stream cursor or deterministic batch counter;
- metric-stream offset;
- manifest digest;
- source and image digest;
- vectorized ensemble member identities.

## 6.2 Spot/preemptible workers

Core training shards may use spot nodes when:

- checkpoint interval is at most five minutes;
- `backoffLimit >= 3`;
- active shard state is entirely recoverable from object store;
- and aggregation distinguishes infrastructure retry from scientific failure.

Use on-demand nodes for planner, aggregator, and short diagnostic replay when restart overhead would dominate.

## 6.3 Signal handling

On SIGTERM:

1. stop taking a new optimizer step;
2. finish or discard the current step atomically;
3. write checkpoint to local scratch;
4. upload checkpoint and status `preempted_resumable`;
5. exit nonzero so Kubernetes retries.

Use a termination grace period of at least 180 seconds.

---

# 7. Shard packing

## 7.1 Compatibility key

Runs may share a vectorized shard only when they match:

- model class and tensor shapes;
- sequence length;
- precision;
- optimizer implementation;
- diagnostic schedule class;
- process alphabet and state dimensions where batching requires it.

Different seeds and scalar hyperparameters may be batched.

## 7.2 Adaptive ensemble count

Calibration produces a lookup table:

```text
(model_class, seq_len, batch, diagnostic_class, gpu_class)
  -> safe ensemble count and expected step time.
```

Pack to 70–90% measured GPU memory and prefer at least 50% sustained utilization. If utilization is lower because the model is tiny, increase ensemble count before requesting a larger GPU.

## 7.3 Straggler handling

Do not mix runs with radically different early stopping or heavy diagnostic schedules in the same fixed vectorized tensor unless inactive members can be masked without substantial waste.

For E3, ordinary training and heavy diagnostics are separate shards.

---

# 8. Environment variables

Reference variables:

```text
EXT_CAMPAIGN_ID
EXT_SHARD_URI
EXT_ARTIFACT_ROOT
EXT_SCRATCH_DIR=/scratch
EXT_DEVICE=cuda
EXT_TRAIN_DTYPE=float32
EXT_ORACLE_DTYPE=float64
EXT_DETERMINISM=seeded
EXT_CHECKPOINT_SECONDS=300
EXT_LOG_LEVEL=INFO
EXT_UPLOAD_CONCURRENCY=4
AWS_REGION
```

No scientific hyperparameter is supplied solely through an environment variable. The locked shard manifest is authoritative.

---

# 9. Observability

## 9.1 Structured logs

Every line is JSON with:

```text
timestamp, campaign_id, shard_id, run_id, event, step,
metric_name, metric_value, gpu_seconds, retry_count
```

Human-readable summaries may be emitted to stderr but are not the source of truth.

## 9.2 Metrics

Expose or scrape:

- pod/shard state;
- GPU utilization and memory;
- step time;
- data generation time;
- diagnostic time;
- upload time;
- checkpoint age;
- completed/failed/censored runs;
- invariant-gate failures;
- remaining estimated GPU seconds.

Do not export raw model states or training sequences to a shared metrics backend.

## 9.3 Alert conditions

- no metric heartbeat for 10 minutes;
- checkpoint older than 15 minutes;
- GPU utilization below 20% for 10 minutes after warmup;
- artifact upload failures;
- NaN or invariant failure;
- campaign budget at 90%;
- unexpected test-split access before confirm lock.

---

# 10. Budget enforcement

Planner computes a hard budget in GPU-seconds. Workers update consumed GPU-seconds from wall time × requested GPU count.

At 90% budget:

- no new exploratory Stage 1 shards launch;
- mandatory controls and already-promoted confirmation runs retain priority;
- reserve is used only for retries or missing primary cells.

At 100%:

- workers checkpoint and stop at the next safe boundary;
- campaign becomes `budget_exhausted` unless all required cells are complete;
- the aggregator may still produce an `underpowered` decision.

Do not silently exceed budget because a grid was larger than expected.

---

# 11. Security and supply chain

- pin image digest and Python lock;
- generate SBOM if the cloud build system supports it;
- no arbitrary code fetched at runtime;
- no pickle loaded from untrusted prefixes;
- checkpoints are campaign-scoped and hash-verified before load;
- object-store bucket uses encryption at rest;
- network policy allows only object store, logging/metrics, and required control-plane endpoints;
- no public ingress to worker pods;
- run container as non-root with read-only root filesystem where possible;
- writable storage limited to `/scratch`.

---

# 12. Launch checklist

## Before calibration

- [ ] PR/commit frozen.
- [ ] Image digest published.
- [ ] Schemas pass.
- [ ] CPU reference tests pass.
- [ ] GPU self-check passes.
- [ ] Process and probe digests emitted.
- [ ] Object-store IAM verified with least privilege.
- [ ] Resume from forced SIGTERM tested.

## Before core campaign

- [ ] Calibration lookup table generated.
- [ ] Smoke campaign verified end to end.
- [ ] Exact-vs-GPU tolerance report accepted.
- [ ] All primary endpoints and thresholds embedded in `campaign.lock.json`.
- [ ] Test split sealed.
- [ ] Budget estimate within quota.
- [ ] Mandatory control cells present in plan.

## Before declaring completion

- [ ] Every required run is complete, censored, or has an allowed exclusion.
- [ ] No incomplete prefix is counted.
- [ ] Test access log is valid.
- [ ] Aggregate code/version matches campaign lock.
- [ ] Decision receipts generated.
- [ ] Artifact hashes verified.
- [ ] Negative results included.

---

# 13. Reference Kubernetes template

See [`cloud/kubernetes-job-template.yaml`](cloud/kubernetes-job-template.yaml). The cloud team should wrap it in the existing Sophia deployment tooling rather than editing scientific fields into Helm values. The authoritative input remains the shard manifest URI.