# Ariadne Lexical Radiation — program

## 1. Thesis

A token is not a primitive constituent of the predictive body. It is a named
event at the boundary between a continuous predictive radiation field and a
discrete communicative codec.

Ordinary token IDs conflate at least six roles:

1. lossless surface span;
2. segmentation boundary;
3. input gesture;
4. output detector;
5. frequency prior;
6. autoregressive feedback operation.

Token Radiation separates them:

```text
surface codec
  bytes and exact reconstruction

source lexicon
  byte or macro gestures into the body

predictive body
  persistent local state and transport

radiation aperture
  loading continuous emission with a declared metric

detector atlas
  byte, subword, morphological, lexical, or multilingual views

event clock
  when a discrete emission is observed

feedback adapter
  how an emitted mark returns through a declared source port
```

Version 0 implements the middle three seams while holding source gestures to
canonical bytes and event timing to vocabulary segmentation.

## 2. Three vocabulary problems

### Surface coding

A vocabulary entry realizes an exact byte string. Compression algorithms such
as BPE and Unigram primarily optimize this surface problem.

### Predictive abstraction

A span is predictively useful when its completion changes a declared family of
future distributions. A predictive vocabulary should cover recurring
future-relevant state transitions, not merely frequent byte pairs.

### Computational macro compilation

A span may deserve a macro even without semantic unity when a shorter operator
program approximates its byte-by-byte action on the body. Macro promotion must
report state, predictive, work, and radiation distortion together with compute
saved.

These problems may share units, but they are not identical objectives.

## 3. Radiation law

For body coordinate `x`, positive metric `G`, and normalized coordinate

```text
q = sqrt(G) x,
```

let `U` have orthonormal rows and let `rho` be a radiation fraction. The emitted
amplitude is

```text
psi = sqrt(rho) U q.
```

A loading step updates

```text
q' = [I - (1 - sqrt(1-rho)) U^T U] q.
```

Therefore

```text
H(x) - H(x') = 1/2 ||psi||^2,
H(x) = 1/2 x^T G x.
```

The non-loading control computes the same `psi` while leaving `x` unchanged and
booking zero radiation. It is deliberately labelled an observation, not a
physical emission.

## 4. Vocabulary as detector atlas

A detector vocabulary is evaluated over one shared radiation trajectory.
Version 0 includes three detector families.

### Linear

```text
logit_v = e_v^T psi + b_v
```

This is the conventional unembedding control.

### Metric distance

```text
logit_v = -||psi - c_v||^2 / (2T) + b_v
```

This makes detector centers and temperature explicit.

### Positive tight frame

For frame vectors `u_v` satisfying

```text
sum_v u_v u_v^T = I,
```

contextual intensity is

```text
I_v = |u_v^T psi|^2,
sum_v I_v = ||psi||^2.
```

A separate additive log prior handles corpus frequency without pretending that
frequency is context-dependent body radiation.

## 5. Exact segmentation marginalization

The body is driven one byte at a time. Thus every segmentation path arriving at
byte boundary `i` shares exactly the same body state and radiation coordinate.

For vocabulary entries whose surfaces match bytes `x[i:j]`, the forward lattice
is

```text
alpha[0] = 1
alpha[j] += alpha[i] p(v | psi_i).
```

EOS is emitted at the final byte boundary. The sequence probability is the sum
over every compatible tokenization.

This is not token-level perplexity comparison. The primary score is exact bits
per byte, and the same forward/backward lattice yields expected lexical events
per byte and posterior token utilization.

## 6. Current synthetic world

The construction process concatenates variable-length motifs under a latent
four-state transition process. Motifs include order-sensitive pairs and
three-byte forms. No separators reveal the latent macro boundaries.

The experiment provides five views:

- canonical bytes;
- BPE;
- Unigram;
- a first predictive-future-divergence heuristic;
- oracle generator motifs.

The oracle is a ceiling, not a learned method.

## 7. Scientific objects kept separate

The result ledger never collapses:

- body next-byte prediction;
- direct-state probe prediction;
- radiation-view prediction;
- segmentation/event rate;
- detector geometry;
- radiated storage;
- state retained after loading;
- vocabulary utilization.

A vocabulary can reduce event rate while worsening prediction. A detector can
improve likelihood without improving the body. A loading law can alter body
state without improving the detector. Each is a distinct result.

## 8. Ownership

Ariadne owns:

- metric radiation algebra;
- loading;
- storage and radiation receipts;
- exact zero and basis laws;
- future body/world integration.

Extension owns:

- corpora and future tests;
- vocabularies;
- detector fitting;
- segmentation lattices;
- predictive-unit discovery;
- macro compilation;
- event clocks;
- feedback and generation;
- architecture comparisons.

No BPE, token ID, language-model loss, or vocabulary semantics may enter the
Ariadne radiation law.
