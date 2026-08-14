# Learned constitutive media in Ariadne

`law::medium` is the first Ariadne law intended to be emitted by a learned
compiler while remaining independently valid and executable without that
compiler.

It is deliberately a **law module**, not another `World`.

```text
Extension optimizer / developmental program
             |
             v
LocalMediumPlan
             |
             v
LocalConstitutiveMedium
  signed wave state
  non-negative evidence state
  finite reservoirs
  ordered local edges
  sources, loss, observations, receipts
```

## State and storage

At site `i`:

```text
wave[i]       signed local transport coordinate
evidence[i]   non-negative local evidence/occupancy coordinate
reservoir[i]  finite stored active supply
```

The declared computational storage metric is

```text
H = 1/2 sum_i wave_metric[i] * wave[i]^2
  + 1/2 sum_i evidence[i]^2
  + sum_i reservoir[i].
```

The evidence term is a computational storage contract. It is not automatically
a claim of literal mechanical energy.

## Law order

1. source apertures inject signed state, positive evidence, and finite reservoir
   work;
2. thresholded active transfer moves reservoir storage into wave storage;
3. ordered weighted Givens edges transport signed state at zero declared work;
4. ordered symmetric stochastic edges transport positive state, preserving mass
   and non-negativity while booking quadratic contraction as loss;
5. local passive loss is applied and receipted;
6. non-loading affine observers read the state.

Edge order is part of the constitution.

## Damage

Disabling or restoring an edge changes future transport availability while
preserving every live site coordinate and performing zero instantaneous work.
This is suitable for damage and repair experiments, but not a claim that every
physical severing operation is free.

## Reconfiguration

A fixed-capacity site may change constitutive coefficients while retaining its
identity and live coordinates. Changing the wave metric books exact
configuration work. A reconfiguration that would silently discard reservoir
state refuses.

Site creation, deletion, merger, split, and remeshing are outside v1. They
require a future state-transfer artifact with explicit transported, dissipated,
radiated, retired, or refused state.

## World integration boundary

The present canonical `BodySpec` is a fixed 24-mode anatomy. The local-medium
law is not forced into that shape. Future integration should generalize body
construction so a medium can be one ordinary body kernel inside canonical
`WorldPlan`, sharing the existing material clock, reciprocal couplings, shared
fields, trajectories, observations, and global ledger.

The following are forbidden:

- a private medium world or ledger;
- optimizer or task state in `mus-ariadne`;
- an opaque neural residual in the material step;
- topology changes that reinterpret live state silently;
- a powerful observer used to counterfeit body computation.
