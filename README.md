# 🥚 Ovillo

*A ball of thread for making sounds with your agent. Ariadne handed Theseus a ball of thread to find his way out of the labyrinth; this one finds its way into a sound.*

## What it is

Nothing in Ovillo synthesizes a note. You write a score in MUS, a plain-text notation, or ask your agent to write one with you. The score does not call an oscillator. It touches a persistent physical world (for the guitar: strings, a wooden body, the air inside it, a room) and that world, which is called Ariadne, does the sounding. A pluck puts energy in. The energy travels from string to bridge to top to air to room, and some of it is lost along the way. Every render prints a receipt with an energy ledger: what the hands put in, what the fretting hand paid, what was lost inside, what was radiated, and the residual left over when you add it up. If the books close, you can say so with a number. If a part of the world is a fit and not a law, the documentation says that too.

Status: **research, pre-alpha.** Interfaces move, and receipts are the source of truth.

## The instruments

Each is an Ariadne World with its own spec, receipts and list of what it does not model.

| instrument | what it is | played by | spec |
|---|---|---|---|
| steel-string guitar | `anatomy=steel-string-v0`: six strings, a top, the air in the box and a room on one World, played from annotated tab by hands (thumb, fingers, fretting hand) | `mus render` | [`SPEC-ARIADNE-STEEL-STRING.md`](SPEC-ARIADNE-STEEL-STRING.md), [`SPEC-TAB.md`](SPEC-TAB.md) |
| piano | `anatomy=piano-v1`: a World shaped like a 190 cm grand (soundboard with ribs and bridges; for each key the score plays, its stiff wires, felt hammer and damper), played from MUS staves and a sustain pedal | `mus render` | [`SPEC-ARIADNE-PIANO.md`](SPEC-ARIADNE-PIANO.md) |
| trombone v1 | one N-dimensional body: a 124-volume tube-network bore, the slide as its coordinate, the lips as the runtime's flow valve, the lungs as its reservoir | a Rust example (a note list, not MUS) | [`TROMBONE-WORLD.md`](TROMBONE-WORLD.md) |
| vocal tract | after Neil Thapen's Pink Trombone (2017), implemented independently as a `mus_ariadne::World`: an oral and a nasal tube network, tongue, lips and velum as coordinates, a flow valve at the glottis, lungs | the browser page `site/tract/` | [`TRACT-WORLD.md`](TRACT-WORLD.md), [`CHECKPOINT-2A.md`](CHECKPOINT-2A.md) |
| the bench | a native app (and its browser build): a simulated electric guitar and guitarist whose DI signal drives an Ariadne World, with screens into the World. The guitar itself is a signal model (waveguide strings, pickups), not a World | `ovillo-app` | [`ovillo/README.md`](ovillo/README.md) |

## Build and render

Rust stable (CI uses 1.99.1). Each instrument from a fresh clone:

```bash
git clone https://github.com/sophia-labs/ovillo.git && cd ovillo

# guitar and piano: the mus CLI
(cd mus-rs && cargo build --release -p mus-cli)
mus-rs/target/release/mus check  examples/tab/steel_string_tour.mus
mus-rs/target/release/mus render examples/tab/steel_string_tour.mus -o tour.wav --json        # the receipt on stdout
mus-rs/target/release/mus render examples/tab/steel_string_tour.mus -o tour.wav --energy trace.json
mus-rs/target/release/mus render examples/piano/prelude-c-major.mus -o prelude.wav --json

# trombone v1: WAV, note list, receipt and energy trace into renders/
(cd ovillo && cargo run --release -p ovillo-dsp --example trombone_nd_solo -- renders solo)

# vocal tract: build its wasm, serve site/, open http://localhost:8765/tract/
bash site/tract/build.sh && python3 -m http.server -d site 8765

# the bench (GPUI; the first build takes a while)
(cd ovillo && cargo run -p ovillo-app --release)
```

The tract and the browser bench need the `wasm32-unknown-unknown` target. Scores to start from: [`examples/tab/steel_string_tour.mus`](examples/tab/steel_string_tour.mus) (nine bars, the steel-string's acceptance score: open strings, hammer-on, pull-off, slide, palm-muted thumb), [`examples/tab/wayfaring-stranger-doc.mus`](examples/tab/wayfaring-stranger-doc.mus) (a full annotated tab), [`examples/piano/prelude-c-major.mus`](examples/piano/prelude-c-major.mus) (the piano's demo, Bach's BWV 846). An energy trace can drive [`blender/`](blender/README.md) to watch the guitar play.

## Watch / listen

[ovillo.sophia-labs.com](https://ovillo.sophia-labs.com) is a Sophia app: playlists of pieces played by these Worlds, each with its score, its receipt and where the energy went, the bench and the vocal tract running in the browser, and a score page that renders on the steel-string World in the browser ([`mus-rs/crates/mus-wasm`](mus-rs/crates/mus-wasm)). Agents can read the same data from its read-only graph (`/llms.txt` says how). The site's sources in this repository are under [`site/`](site/README.md); the release video's tools and provenance are under [`video/`](video/README.md).

## Receipts

- Every `mus render --json` prints `mus.audio.render-receipt.v1`: `renderDigest` (the WAV's sha256), `badTokens`, and per World in `ariadneWorlds[]` its `planDigest` and ledger (`sourceWork`, `controlWork`, `internalLoss`, `radiation`, `cumulativeClosureResidual`, `maximumFrameResidual`). [`AGENTS.md`](AGENTS.md) says how to read one.
- Each anatomy's own receipt is in [`profiles/`](profiles/) (`*.receipt.json`, beside the plan it describes).
- Gates with their numbers: [`CHECKPOINT-2A.md`](CHECKPOINT-2A.md) (N-dimensional configuration; the tour's and Wayfaring Stranger's digests held), [`POC-3.md`](POC-3.md), the piano's [§9](SPEC-ARIADNE-PIANO.md#9-gates), the trombone's [gates](TROMBONE-WORLD.md#gates-renders-ovillorenderstrombone-nd-wav), the tract's [gates](TRACT-WORLD.md#gates), and the tract crates' `receipts/` directories.

## What it does not model

Read these before saying what an instrument can do:

- steel-string: [Limits of v0](SPEC-ARIADNE-STEEL-STRING.md#limits-of-v0-said-plainly) (no damping at note-off yet, one pluck point per string, 24 partials per string, a linear model, rigid back and neck, harmonics played as plucks; a guitar-*shaped* body, not a model of any particular guitar);
- piano: [§7](SPEC-ARIADNE-PIANO.md#7-what-it-does-not-model-said-plainly) (one polarization, linear strings, only the score's keys strung, the action a throw, a flat board and rigid frame, no room; not a model of any particular piano);
- trombone: [Limits](TROMBONE-WORLD.md#limits) (no brightening with loudness, a rubber slide, fitted lips, no room);
- vocal tract: [Gaps](TRACT-WORLD.md#gaps) (no glottal aspiration noise, one jet).

The project keeps an explicit claim boundary: a successful render is not by itself a theorem about transport, a biological reconstruction, intelligible speech, or listener perception. Open obligations travel in receipts.

## Plateaus

The repository is a set of plateaus. Each one stands on its own, and you can enter anywhere.

| plateau | what is there |
|---|---|
| [`SPEC.md`](SPEC.md) | MUS, the score language |
| [`SPEC-AUDIO.md`](SPEC-AUDIO.md) | how a score becomes audio |
| [`SPEC-TAB.md`](SPEC-TAB.md) | annotated tab: `@string:fret`, hands, hammer-ons, slides |
| [`ARIADNE-PURE-PHYSICS-CONSTITUTION.md`](ARIADNE-PURE-PHYSICS-CONSTITUTION.md) | the rule of the world: no synthesis, only physics whose energy balances |
| [`SPEC-ARIADNE-STEEL-STRING.md`](SPEC-ARIADNE-STEEL-STRING.md), [`SPEC-ARIADNE-PIANO.md`](SPEC-ARIADNE-PIANO.md) | the instruments `mus render` plays |
| [`TROMBONE-WORLD.md`](TROMBONE-WORLD.md), [`TRACT-WORLD.md`](TRACT-WORLD.md) | the trombone and the vocal tract as Worlds |
| [`blender/README.md`](blender/README.md) | seeing the world play |
| [`aigua/README.md`](aigua/README.md) | the Aigua studies |
| [`ovillo/README.md`](ovillo/README.md) | the bench: a native app for playing a guitar through the world and watching inside it |
| [`site/README.md`](site/README.md), [`video/README.md`](video/README.md) | the sources of ovillo.sophia-labs.com and of the release video |
| [`docs/INSTRUMENT.md`](docs/INSTRUMENT.md) | the long map of the whole instrument and its repository |

Agents: start at [`AGENTS.md`](AGENTS.md). Code comments that cite `.string-research/...` point at research notes in a private repository; they are not needed to build or run anything here.

## License

MIT. See [`LICENSE`](LICENSE).
