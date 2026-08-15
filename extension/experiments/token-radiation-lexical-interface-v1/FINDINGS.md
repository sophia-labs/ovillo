# Lexical Interface v1 — findings

## 1. The simplification removed the computational role of vocabulary

A byte-driven body distinguishes surface forms even when they implement the
same latent operation. Surface macros cut state error by more than half relative
to shared operator macros and reach 0.973 next-operator agreement. Shared
operator macros reach only 0.856.

The body was never trained to make several spellings equivalent at its source
boundary.

## 2. A short radiation trace is a better lexical interface than one instant

The lexical body's instantaneous operator head reaches only 0.589 on the
counterfactual test set. A four-step radiation-trace detector reaches 0.993 and
perfectly resolves the context-dependent homograph.

Temporal shuffling reduces boundary AUC from 0.975 to 0.499. Ordered radiation,
not only its marginal distribution, carries the event signal.

## 3. Surface and radiation are complementary

Surface-only boundary AUC is 0.982. Radiation-only AUC is 0.975. Together they
reach 0.999.

Surface-only operator classification reaches 0.908 overall and 0.814 on the
homograph. Radiation reaches 0.993 overall and 1.0 on the homograph. Persistent
state supplies context the surface fragment lacks.

## 4. Surface-neutral predictive feedback works exactly

The predictive body reaches perfect next-operator accuracy and perfect latent
closed-loop sequences. Iterative feedback and batched teacher forcing produce
identical state and emission tensors under the deterministic CPU configuration.

## 5. The surface loop works, but is not reliable enough to scale

```text
emitted operator step accuracy        0.9396
surface semantic round trip           0.9507
exact semantic sequence rate          0.6750
exact boundary sequence rate          0.6250
```

Small event-clock and operator errors compound through feedback.

## 6. Claim boundary

The event clock is supervised, the operator ontology is provided, and the
articulator is declared. This is a construction proof of a bidirectional lexical
interface, not evidence that natural lexical units or articulation laws have
been discovered.

Proceed on CPU to a learned articulatory body and a likelihood-trained latent
event process. Do not request GPU scale yet.
