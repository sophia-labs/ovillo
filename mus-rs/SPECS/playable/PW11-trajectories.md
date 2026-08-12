# PW11 — trajectory programs through persistent bodies

**Depends on:** PW9 live controls, PW10 sustained sources, Ariadne Phase 0
retuning telemetry.
**Contract:** `SPEC-TRAJECTORY.md`.

## Stage A — runtime-neutral IR

Land `mus-trajectory` with JSON transport, fixed-capacity/topology semantics,
analytic kinematic bounds, deterministic frame lowering, binary canonical
digests, and the committed Python/Rust fixture.

**Exit:** schemas validate; both runtimes reproduce the pinned program and frame
digests; invalid domains, discontinuous fixed-capacity paths, and missing
certificates refuse with stable codes.

## Stage B — string adapter

Map non-pitch coordinates onto the existing `ControlFrame` executor. Lower the
narrow two-knot linear pitch subset onto `StringNetworkVoice`'s native `f0 ->
f1` path. Do not add an oscillator, post-vocoder, or parallel automation loop.

**Exit:** constant-path byte law; deterministic moving path; native pitch
receipt; nonlinear pitch refusal; strict open-obligation refusal.

## Stage C — GARGANTA adapter

Apply articulation tracks through `area_function_cm2` with anatomy held static.
Keep source coordinates and direct area-oracle overrides separate. Emit an
anatomy-static adapter receipt.

**Exit:** real area map changes under control; direct section override remains
inside the positive cone; anatomy serialization is unchanged; unsupported
foreign coordinates refuse.

## Stage D — score integration (open)

Call the registered-arrow split from `mus-engine::source`, preserve unrelated
MUS-A arrows, and include trajectory receipts in render/check receipts.

**Exit:** the committed trajectory etude renders through `mus render`; no
trajectory retains a private per-event identity after lowering.

## Stage E — score-spanning bodies (open)

Move trajectory ownership from an isolated note buffer to a persistent body
clock. Define overlap/order semantics with Ariadne operator words and explicit
retirement at body destruction.
