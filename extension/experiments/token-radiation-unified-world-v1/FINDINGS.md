# Unified shared-world construction findings

## Status

Three-seed controlled CPU construction. Python execution and artifact verification passed. Rust integration source and tests are present; Rust execution remains an unpassed gate.

## 1. The delayed shared field carries real sequence-specific information

Shared current-operator accuracy was `0.7145`, compared with `0.3356` for isolated bodies.

The signal collapses when the communication substrate is intervened upon:

```text
cross-world field permutation   0.3367
lexical port disabled              0.2854
predictive port disabled           0.2772
chance                             0.2500
```

The operator observer is downstream of predictive-body state and is not needed to execute the body/field dynamics.

## 2. Direct coupling remains the stronger tiny construction

Instantaneous reciprocal coupling reached `0.8364`, about `0.1219` above the delayed shared field. The field adds propagation, storage, scattering, and loss, all of which make the small optimization problem harder.

This is not a failure of shared space. It establishes a meaningful architecture comparison rather than treating every lawful coupling as equivalent.

## 3. Shared-field latency matters

The field has explicit delayed courses. Exploratory settle-time diagnostics improved shared accuracy when the world was allowed additional material steps between the last surface byte and observation. These diagnostics were not included in the primary digest and should be preregistered in the next tranche.

## 4. Whole-sequence reliability is not adequate

Shared exact eight-event sequence rate was only `0.0382`. Even direct coupling reached only `0.1415`. Event-level information exists, but errors compound quickly.

## 5. One-ledger training diagnostics close at float32 scale

Maximum differentiable-mirror closure residual was `5.72e-05`. This is acceptable for the current training mirror but is many orders looser than Ariadne's Rust f64 law expectations.

## Decision

```json
{
  "shared_field_signal_causally_supported": true,
  "shared_world_performance_gate_passed": false,
  "canonical_rust_world_execution_confirmed": false,
  "gpu_scale_warranted": false
}
```

Proceed with CPU work on world latency, active transduction, articulatory return paths, and canonical Rust validation. Do not restore a privileged operator handoff to improve headline accuracy.
