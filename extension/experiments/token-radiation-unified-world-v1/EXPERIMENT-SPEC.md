# Unified shared-world experiment specification

## Primary question

Can a lexical body communicate a context-dependent latent operation to a surface-neutral predictive body through Unified Ariadne's delayed shared field, without transmitting token IDs, operator IDs, logits, or hidden tensors?

## Controlled process

The latent state is a dihedral-like noncommutative state plus persistent lexical mode. Operators are:

```text
R  quarter-turn
L  inverse quarter-turn
F  reflection
T  lexical-mode toggle
```

Several variable-length byte surfaces realize every operation. `ba` realizes `R` in one mode and `F` in the other.

## Conditions

### Oracle handoff

The predictive body receives the true operator through a declared source gesture after each surface event. This is a positive ceiling and a deliberately privileged channel.

### Isolated

The two bodies share a clock but all direct and field port weights are zero.

### Direct

A storage-free Givens exchange couples one lexical and one predictive boundary direction. No operator label is transmitted.

### Shared

There is no direct edge. Both bodies exchange only with the delayed shared field.

## Shared-field controls

- cross-world permutation of the live field head before predictive-body exchange;
- lexical world port set to zero;
- predictive world port set to zero.

These are interventions on the communication substrate, not on the analysis head.

## Endpoints

Primary:

```text
current latent-operator accuracy
homograph operator accuracy
exact event-sequence rate
```

Secondary:

```text
next-operator accuracy
persistent-mode accuracy
source work
internal loss
final stored energy
maximum closure residual
```

## Gates

### Causal shared-field signal

Pass when:

- shared improves current-operator accuracy over isolated by at least 0.20;
- shared exceeds every field/port control by at least 0.20;
- the result holds in the aggregate over registered seeds.

### Performance

Pass only when:

- shared current accuracy is at least 0.90;
- exact sequence rate is at least 0.80;
- neither port ablation retains the capability.

### Rust canonical-runtime gate

Requires successful Rust formatting, Clippy, full crate tests, constitutive-world integration tests, and proof example.

### GPU gate

Requires the performance and Rust gates plus evidence that CPU architecture search has saturated.

## Registered non-interpretations

- A field beating isolation does not mean it beats direct coupling.
- Operator-probe accuracy does not mean a vocabulary is learned.
- The differentiable mirror is not the canonical Rust executor.
- Float32 closure is a training diagnostic, not an Ariadne law tolerance.
- Port transduction is a physical constitutive operation; it is not a non-loading observer.
