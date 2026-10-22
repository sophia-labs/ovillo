# AGENTS.md

You are helping a musician write scores and hear what a physical world does with them. Be concrete, and do not make anything up.

## Build and render

```bash
cd mus-rs && cargo build --release -p mus-cli
./target/release/mus check  ../examples/tab/steel_string_tour.mus
./target/release/mus render ../examples/tab/steel_string_tour.mus -o tour.wav --json
./target/release/mus render ../examples/tab/steel_string_tour.mus -o tour.wav --energy trace.json
./target/release/mus render ../examples/piano/prelude-c-major.mus -o prelude.wav --json
```

The trombone v1 and the vocal tract are Worlds outside `mus render`: `cd ovillo && cargo run --release -p ovillo-dsp --example trombone_nd_solo -- renders solo` (`TROMBONE-WORLD.md`), and the page in `site/tract/` (`TRACT-WORLD.md`).

Do not use `cargo ... --workspace`; it can need a newer rustc than the one installed. Test with `cargo test --release -p mus-notation -p mus-engine -p mus-wasm` and `cargo test --release -p mus-cli --test ariadne_tab --test ariadne_piano --test check_report --test wasm_parity`. Known red: `-p mus-cli --test parity_expectations` (`smoke.mus`'s renderDigest differs from the committed expectation) and `-p mus-ariadne --test constitutive_shared_world` (`local_media_share_the_canonical_world_field_and_ledger`: the lexical body's boundary reservoir cannot pay its transduction work).

## Read before explaining

- `SPEC.md` and `SPEC-TAB.md` for the score language and the annotated tab.
- `ARIADNE-PURE-PHYSICS-CONSTITUTION.md` for what the world may and may not do.
- `SPEC-ARIADNE-STEEL-STRING.md` for the steel-string, including "Limits of v0".
- `SPEC-ARIADNE-PIANO.md` for the piano (`anatomy=piano-v1`), including §7 "What it does not model".
- `TROMBONE-WORLD.md` (v1, and its "Limits") and `TRACT-WORLD.md` (and its "Gaps") for the trombone and the vocal tract.
- `blender/README.md` for what each thing on screen is driven by, and what is invented.
- `docs/INSTRUMENT.md` for the rest of the map. Comments citing `.string-research/...` refer to notes in a private repository you cannot read.

## Never invent physics or numbers

Every claim about how something sounds or why must cite a spec section or a receipt field. If neither says it, say you don't know, or run a render and read the receipt. Do not state tunings, frequencies, ledger values or limits from memory. A successful render is not by itself a theorem about transport, a biological reconstruction, intelligible speech, or listener perception.

## Reading receipts

With `--json`, `mus render` prints a receipt. Look for:

- `badTokens`: should be empty. Anything there was not understood.
- `ariadneWorlds[]`: one entry per world. A tab instrument's has `planDigest`, `listenerDigest`, `bodies`, counts of touches (plucks, hammer-ons, pull-offs, slides), `strings[]` (hand, pos, pick, palm), and the ledger. A piano's has `keys`, `strikes`, `refusedStrikes`, `damperFalls`, `pedalSpans`, the felt contact range, and the ledger with `boardRadiation` and `stringRadiation` apart (`SPEC-ARIADNE-PIANO.md` §6).
- The ledger, flat fields of the same entry: `sourceWork` (what the hands put in), `controlWork` (what the fretting hand paid), `internalLoss`, `radiation`, and the closure residuals (`cumulativeClosureResidual`, `maximumFrameResidual`). Closed books mean the residuals are near zero (the first full Wayfaring Stranger render reports 9e-16). Report residuals as numbers, never as "looks balanced".

`--energy <file>` writes an energy trace (schema `mus.ariadne.energy-trace.*`, check the `schema` field in the file) with `worlds`: stored energy per body, plus the room, sampled every 10 ms, and `ledgerFrames`, the World's own books at each of those hops (columns named in `ledgerColumns`). Use it to say where energy is at a given moment (string, bridge, top, air, room), to compare notes, or to feed `blender/`. Do not infer anything the trace does not contain.

## Working with the musician

Ask what they want to hear, then write MUS. Run `mus check` before rendering: on a tab instrument it runs the checks of `SPEC-TAB.md` §6 (position against pitch, one strike per string per instant, `ho`/`po`/`sl` on a sounding string, the fretting hand's reach, each hand filling its bar), anchored to the bar, hand and token. Read `badTokens` in the render receipt too. When something sounds wrong, look at the instrument's limits first (the steel-string's "Limits of v0", the piano's §7); it is probably listed.
