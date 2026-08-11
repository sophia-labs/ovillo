# MUS protection contract

**Status:** companion contract to `SPEC-GESTURE.md`.

## 1. Why this contract exists

The word `protected` is not a synonym for `geometric`, `robust in one sweep`,
or `non-Abelian`.  A compiler must state which object is protected, by which
invariant or code, against which perturbations, and under which gap or threshold
hypotheses.

## 2. Claim levels

```text
geometric
  path-dependent transport with a connection and gauge-invariant observables

gapped_family
  a family carrying a named topological invariant that cannot change while the
  declared gap and hypotheses remain valid

quantized_pump
  a transport observable or discrete pump action fixed by a topological
  invariant under the declared cycle class

fault_tolerant_gate
  a logical operation with an error model, code/encoding, and threshold theorem
```

Claims are monotone only when all additional hypotheses are supplied.  A
gapped family does not automatically make every Wilson loop quantized.  A
quantized pump is not automatically a fault-tolerant gate.

## 3. Required protected-family receipt

```text
identity
  family digest
  compiler/oracle version

state
  bundle rank
  ambient dimension
  metric digest
  degeneracy tolerance

topology
  invariant kind
  sign/orientation convention
  requested value
  measured/computed value
  independent cross-check
  theorem or algorithm reference

gap
  spectral object
  minimum value
  sampling/certification method
  parameter domain

perturbations
  allowed family
  homotopy construction
  minimum gap over homotopy
  invariant residual

path
  base point
  control loop
  endpoint closure
  connection/transport rule

adiabaticity
  duration
  speed convention
  leakage estimate
  geometric endpoint error

lowering
  realification
  authored graph
  sparse factor word
  reconstruction and energy residuals

claim
  exact claim level
  explicitly withheld stronger levels
```

## 4. Syntax

```mus
@family yang_loom : ProtectedFamily {
  state {
    ambient: C^4
    occupied_rank: 2
    metric: euclidean
  }

  topology {
    invariant: second_chern(convention=@degree_t4_s4)
    value: 1
  }

  require {
    spectral_gap >= 1.5
    perturbations within @yang_loom.robustness_v1
  }
}

@gesture braid_ab = achieve protected.gapped_family {
  family: @yang_loom
  path: @loop.A >> @loop.B
  duration: 128u

  require {
    adiabatic.leakage <= 0.025
    endpoint.error <= 1e-3
    work.abs <= 1e-9J
  }
}
```

A discrete pump may state:

```mus
@gesture z3_route = achieve protected.quantized_pump {
  family: @acoustic_z3
  sequence: [C3, C3, C3prime, C3]
  target: shift(cell=-1, internal=minus)
}
```

## 5. Compiler refusals

Compilation fails if:

- `protected` appears without a claim level;
- an invariant lacks a convention;
- the family rank changes without a typed transition;
- the path crosses a gap closing;
- the perturbation family is unspecified;
- an adiabatic gate lacks a leakage/error budget;
- a geometric Wilson loop is labeled quantized without an invariant fixing it;
- a fault-tolerant claim lacks a code, error model, and threshold;
- compiler-predicted protection is presented as measured production behavior.

## 6. Ariadne reference status

The protected non-Abelian reference branch establishes:

```text
gapped_family:
  second Chern = 1, occupied rank 2

geometric:
  two noncommuting Kato/Wilson loops

quantized_pump fixture:
  exact acoustic Z3 endpoint algebra
```

It does not establish `fault_tolerant_gate`.
