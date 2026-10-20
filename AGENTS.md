# AGENTS.md

You are helping a musician write scores and hear what a physical world does with them. Be concrete, and do not make anything up.

## Build and render

```bash
cd mus-rs && cargo build --release -p mus-cli
./target/release/mus check  ../examples/tab/steel_string_tour.mus
./target/release/mus render ../examples/tab/steel_string_tour.mus -o tour.wav --json
./target/release/mus render ../examples/tab/steel_string_tour.mus -o tour.wav --energy trace.json
```

Do not use `cargo ... --workspace`; it can need a newer rustc than the one installed. Test with `cargo test --release -p mus-notation -p mus-engine` and `cargo test --release -p mus-cli --test ariadne_tab --test check_report`. Known stale: `parity_expectations` and some `mus-ariadne` examples.

## Read before explaining

- `SPEC.md` and `SPEC-TAB.md` for the score language and the annotated tab.
- `ARIADNE-PURE-PHYSICS-CONSTITUTION.md` for what the world may and may not do.
- `SPEC-ARIADNE-STEEL-STRING.md` for the steel-string, including "Limits of v0".
- `blender/README.md` for what each thing on screen is driven by, and what is invented.
- `docs/INSTRUMENT.md` for the rest of the map. Comments citing `.string-research/...` refer to notes in a private repository you cannot read.

## Never invent physics or numbers

Every claim about how something sounds or why must cite a spec section or a receipt field. If neither says it, say you don't know, or run a render and read the receipt. Do not state tunings, frequencies, ledger values or limits from memory. A successful render is not by itself a theorem about transport, a biological reconstruction, intelligible speech, or listener perception.

## Reading receipts

With `--json`, `mus render` prints a receipt. Look for:

- `badTokens`: should be empty. Anything there was not understood.
- `ariadneWorlds[]`: one entry per world. It has `planDigest`, `listenerDigest`, `bodies`, counts of touches (plucks, hammer-ons, pull-offs, slides), `strings[]` (hand, pos, pick, palm), and the ledger.
- The ledger, flat fields of the same entry: `sourceWork` (what the hands put in), `controlWork` (what the fretting hand paid), `internalLoss`, `radiation`, and the closure residuals (`cumulativeClosureResidual`, `maximumFrameResidual`). Closed books mean the residuals are near zero (the first full Wayfaring Stranger render reports 9e-16). Report residuals as numbers, never as "looks balanced".

`--energy <file>` writes an energy trace (schema `mus.ariadne.energy-trace.*`, check the `schema` field in the file) with `worlds`: stored energy per body, plus the room, sampled every 10 ms. Use it to say where energy is at a given moment (string, bridge, top, air, room), to compare notes, or to feed `blender/`. Do not infer anything the trace does not contain.

## Working with the musician

Ask what they want to hear, then write MUS. Run `mus check` before rendering, and read `badTokens` in the render receipt too: today `mus check` does not verify tab positions (pitch against `@string:fret`), and `mus render` reports a disagreement there. When something sounds wrong, look at the limits in the steel-string spec first; it is probably listed.
