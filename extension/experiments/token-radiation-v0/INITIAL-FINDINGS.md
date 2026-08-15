# Token Radiation v0 — initial construction findings

## Status

Exploratory, one deterministic synthetic-process seed. Python execution passed.
Rust source and cross-runtime fixtures are present, but no Rust toolchain was
available in the construction environment.

## Findings

### 1. The loading law closes

Passive rank-four radiation closed the metric storage identity with maximum
validation residual `1.62e-13`. The non-loading control left state unchanged and
booked zero radiation.

### 2. One radiation trajectory supports several vocabulary views

The same body emissions were decoded through byte, BPE, Unigram,
predictive-substring, and oracle-motif vocabularies. Exact likelihood summed all
compatible segmentations rather than selecting one preprocessing path.

### 3. The process-level macro ceiling is substantial

The passive oracle-motif linear view reached `1.4050` BPB at `0.4301` expected
events per byte, versus `2.3378` BPB at exactly one event per byte for the
passive byte-linear view.

This establishes that the body radiation contains information about multi-byte
future structure. It does not establish that the oracle units can be discovered
from data.

### 4. Frequency compression is a strong baseline

BPE-18 was the best learned vocabulary in this short run:

```text
passive BPE-18 linear       1.5754 BPB, 0.4640 events/byte
passive Unigram-18 linear   1.7512 BPB, 0.4810 events/byte
```

### 5. The first predictive heuristic failed to beat compression tokenizers

The predictive-future-divergence vocabulary reached `2.0690` BPB at `0.5946`
events/byte under passive radiation. It was worse than BPE and Unigram.

The likely lesson is not that predictive vocabularies are useless. The present
heuristic scores surface completions by aggregate future divergence and does not
estimate their action over a distribution of body states. The next predictive
vocabulary should cluster **state-transition operators**, not merely substring
endpoints.

### 6. Positive frame geometry is not yet a competitive decoder

The positive detector preserves the contextual tight-frame identity to
floating-point precision, but every positive-frame condition was worse than its
linear counterpart after the short fitting budget.

That separates an exact radiation-partition law from predictive usefulness.
Carrier-assisted or higher-rank/time-resolved positive detectors remain future
hypotheses.

### 7. Passive and non-loading bodies are close on the current task

The body next-byte results were nearly identical:

```text
passive     2.3555 BPB
nonloading  2.3566 BPB
```

Several passive detector views were modestly better, but this is one seed and
the bodies were trained independently. Exact common-initialization and
common-stream continuations are required before attributing the difference to
loading.

## Decision

Proceed to a controlled confirmation and operator-vocabulary tranche. Do not yet
promote:

- passive loading as a useful learning mechanism;
- positive-frame detectors as a competitive output head;
- the current predictive-substring vocabulary;
- natural-language or tokenizer-independence claims.
