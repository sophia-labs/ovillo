# Cloud Adapter

The files in this directory are reference templates. The implementation team should render them through Sophia's cloud tooling rather than hand-editing one Job per run.

## Required adapter inputs

```text
IMAGE_DIGEST
CAMPAIGN_ID
SHARD_ID
SHARD_URI
ARTIFACT_ROOT
GPU_CLASS
SERVICE_ACCOUNT
AWS_REGION
```

Scientific hyperparameters remain inside the immutable shard manifest at `SHARD_URI`.

## Job classes

Render the template with one of:

```text
planner
preprocessor
gpu-worker
diagnostic-replay
aggregator
verifier
```

The provided Kubernetes template is for `gpu-worker`; CPU classes use the same identity, security, scratch, and artifact conventions without GPU resources.

## Cloud repository integration

Recommended split:

- `instrument` owns experiment code, schemas, configs, and scientific manifests;
- `cloud` owns image build/release, workload identity, node labels, Job rendering, quotas, logs, and object-store plumbing;
- object storage owns large streams, factors, checkpoints, and result artifacts;
- no large checkpoint or raw campaign output is committed to Git.

The cloud adapter must treat a completed artifact prefix as immutable and use `campaign_id`/`run_id` as content identities.