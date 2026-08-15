# Token Radiation Lexical Interface v1

**A bidirectional lexical compiler around two persistent radiating bodies.**

This tranche restores the Extension-side seam that Token Radiation v0 held
fixed. A vocabulary entry is no longer only a detector row over a byte-driven
trajectory. The controlled architecture separates:

```text
surface bytes
    -> lexical radiation body
    -> event clock + latent operator detector
    -> declared operator source gesture
    -> surface-neutral predictive body
    -> predictive radiation
    -> declared surface articulator
    -> surface bytes
    -> lexical body again
```

The predictive body never receives byte IDs or surface-token embeddings. Its
source alphabet consists only of four latent operators and sixteen initial
predictive states. Teacher forcing and generated feedback use the exact same
operator source table.

## Controlled world

The synthetic world has four operators:

```text
R  quarter-turn
L  inverse quarter-turn
F  reflection
T  lexical-mode toggle
```

`R` and `F` do not commute. Surface realizations are variable length and
heavily overlap. The same bytes `ba` mean `R` in one persistent mode and `F` in
the other. Several surface forms realize each operator, while the next operator
is determined by the latent dihedral state and lexical mode.

## Run

```bash
cd extension/experiments/token-radiation-lexical-interface-v1
python -m pip install -e '.[dev]'
pytest -q

token-radiation-lexical-interface-v1 run \
  configs/construction.json \
  runs/construction

token-radiation-lexical-interface-v1 verify runs/construction
```

## Construction evidence

The committed deterministic CPU result uses 560 latent training sequences, 120
calibration sequences, 120 structurally held-out latent sequences, three
surface rerenderings per interface-training sequence, twelve events per
sequence, sixteen sites per body, and passive rank-sixteen radiation with
fraction `0.04`.

### Why the one-body simplification was inadequate

| Macro | Mean state-relative error | Next-operator agreement |
|---|---:|---:|
| One macro per latent operator | 0.3618 | 0.8558 |
| One macro per surface form | **0.1672** | **0.9730** |

The byte body entangles surface realization with predictive operation. It does
not already contain a clean surface-invariant operator source alphabet.

### Radiation event interface

| Detector | Boundary AUC | Operator accuracy | Homograph accuracy |
|---|---:|---:|---:|
| Surface only | 0.9824 | 0.9083 | 0.8142 |
| Radiation only | 0.9747 | **0.9931** | **1.0000** |
| Combined | **0.9987** | **1.0000** | **1.0000** |
| Temporally shuffled radiation | 0.4992 | — | — |

The instantaneous lexical head is substantially weaker than the trace
detector. The relevant emitted object is a short radiation history, not one
static vector.

### Surface-neutral predictive body

```text
teacher-forced next-operator accuracy       1.0000
teacher-forced mode accuracy                1.0000
teacher-forced group accuracy               0.9949
latent closed-loop step accuracy            1.0000
latent closed-loop exact-sequence rate      1.0000
teacher/feedback state disagreement         0
teacher/feedback emission disagreement      0
```

### Surface round trip

```text
emitted operator step accuracy              0.9396
surface round-trip operator accuracy        0.9507
exact semantic sequence rate                0.6750
exact event-boundary sequence rate          0.6250
```

This clears a construction gate, but not a scale gate. The articulator is
declared rather than learned, and the event clock is supervised. Whole-sequence
reliability is not yet high enough for GPU expansion.

Scientific digest:

```text
4072fe49daa7dd353c85b0aac74eb31e6f2812739717ea94daf40e99339662a2
```

## Decision

Supported:

- a surface-neutral predictive body with an explicit operator source alphabet;
- radiation-only lexical interpretation of a context-dependent homograph;
- a learned supervised event interface over radiation through time;
- exact identity between teacher-forced and generated feedback paths;
- a closed surface-to-operator-to-surface loop with no direct hidden-state
  assignment;
- the need for two-body operator/surface separation in this world.

Not supported:

- an unsupervised event clock;
- a learned articulatory body;
- a reliable whole-sequence free-generation loop;
- replacement of BPE as an exact surface codec;
- natural-language utility;
- GPU scale.

## Documents

- [`PROGRAM.md`](PROGRAM.md) — restored architecture and ownership boundary.
- [`EXPERIMENT-SPEC.md`](EXPERIMENT-SPEC.md) — conditions, controls, and gates.
- [`FINDINGS.md`](FINDINGS.md) — interpretation of the committed construction.
- [`NEXT-TRANCHE.md`](NEXT-TRANCHE.md) — learned articulation and latent event
  inference.
