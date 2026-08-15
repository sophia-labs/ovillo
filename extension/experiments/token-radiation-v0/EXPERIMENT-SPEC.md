# Token Radiation v0 — experiment specification

## Primary question

Given one byte-driven body and one continuous radiation trajectory, which
vocabulary/detector views produce the best exact predictive likelihood at a
given lexical event rate?

## Conditions

### Body radiation

- `passive`: the emitted finite-rank projection is removed from body storage;
- `nonloading`: the same coordinate is observed without changing the body.

### Vocabulary

- bytes;
- BPE at a fixed vocabulary budget;
- Unigram at the same budget;
- predictive-substring heuristic at the same budget;
- oracle process motifs.

### Detector

- linear;
- positive tight frame.

A metric-distance implementation is included for the next registered expansion
but is not part of the committed construction matrix.

## Shared controls

All vocabulary views for one body condition use:

- the same corpus;
- the same trained byte-driven body;
- the same radiation aperture and boundary coordinates;
- the same train/validation partition;
- the same byte-normalized likelihood definition;
- no input macro tokens;
- no detector access to internal body state.

## Primary endpoints

```text
exact validation bits per byte
expected lexical events per byte
```

The Pareto frontier minimizes both.

## Secondary endpoints

```text
body next-byte BPB
direct-state linear-probe BPB
booked radiation per byte
projected intensity per byte
final state norm
maximum radiation closure residual
active vocabulary entries
surface lexicon bytes
positive-frame closure residual
```

## Registered interpretation

- Better vocabulary-view BPB does not mean the body improved.
- Lower events/byte does not mean a better tokenizer unless predictive BPB and
  lexicon complexity are reported.
- Passive beating non-loading at one seed does not establish that loading is
  useful.
- Oracle motifs establish extractable macro structure, not learnability.
- A tight-frame identity establishes detector geometry, not semantic utility.

## Falsifiers

### F0 — radiation law

Falsified if passive storage removal differs from emitted intensity beyond the
frozen numerical tolerance or if non-loading observation alters state.

### F1 — shared pre-token field

Falsified at construction scale if no multi-byte detector view can improve the
BPB/event-rate frontier over bytes while using the same body trajectory.

### F2 — predictive vocabulary heuristic

Supported only if the predictive view beats matched BPE and Unigram across fresh
process seeds. The current construction does not support it.

### F3 — positive detector frame

The geometric law is supported when contextual intensities sum to emitted norm.
A modeling advantage requires held-out BPB competitive with the linear control.

### F4 — consequential loading

Requires replicated differences in body retention, long-horizon prediction, or
adaptation under matched training. One short construction is insufficient.

## Holdout policy for confirmation

A confirmatory tranche should hold out:

- complete process seeds;
- one motif family;
- one vocabulary size;
- one radiation fraction;
- one detector family during model selection.

Random sequences from an already observed process are replication, not
structural extrapolation.
