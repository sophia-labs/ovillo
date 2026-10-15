# 🥚 Ovillo

*A ball of thread for making sounds with your agent. Ariadne handed Theseus a ball of thread to find his way out of the labyrinth; this one finds its way into a sound.*

## What it is

Nothing in Ovillo synthesizes a note. You write a score in MUS, a plain-text notation, or ask your agent to write one with you. The score does not call an oscillator. It touches a persistent physical world (strings, a wooden body, the air inside it, a room) and that world, which is called Ariadne, does the sounding. A pluck puts energy in. The energy travels from string to bridge to top to air to room, and some of it is lost along the way. Every render prints a receipt with an energy ledger: what the hands put in, what the fretting hand paid, what was lost inside, what was radiated, and the residual left over when you add it up. If the books close, you can say so with a number. If a part of the world is a fit and not a law, the documentation says that too.

## Three steps (with your agent)

1. Build it (Rust, stable toolchain):
   ```bash
   git clone https://github.com/sophia-labs/ovillo.git && cd ovillo/mus-rs && cargo build --release -p mus-cli
   ```
2. Write a score, or ask your agent for one. Two to start from: [`examples/tab/steel_string_tour.mus`](examples/tab/steel_string_tour.mus) (a short tour of the steel-string's techniques) and [`examples/tab/wayfaring-stranger-doc.mus`](examples/tab/wayfaring-stranger-doc.mus) (a full annotated tab).
3. Render it, still in `mus-rs`:
   ```bash
   ./target/release/mus render ../examples/tab/steel_string_tour.mus -o tour.wav
   ```
   Add `--energy trace.json` to keep the world's energies every 10 ms, then follow [`blender/README.md`](blender/README.md) to watch the guitar play.

## Watch / listen

[First plateau: *Wayfaring Stranger*, after Doc Watson](https://github.com/sophia-labs/ovillo/releases/tag/v0.1.0), rendered by the steel-string, with the world's own energies on screen.

## Plateaus

The repository is a set of plateaus. Each one stands on its own, and you can enter anywhere.

| plateau | what is there |
|---|---|
| [`SPEC.md`](SPEC.md) | MUS, the score language |
| [`SPEC-AUDIO.md`](SPEC-AUDIO.md) | how a score becomes audio |
| [`SPEC-TAB.md`](SPEC-TAB.md) | annotated tab: `@string:fret`, hands, hammer-ons, slides |
| [`ARIADNE-PURE-PHYSICS-CONSTITUTION.md`](ARIADNE-PURE-PHYSICS-CONSTITUTION.md) | the rule of the world: no synthesis, only physics whose energy balances |
| [`SPEC-ARIADNE-STEEL-STRING.md`](SPEC-ARIADNE-STEEL-STRING.md) | the steel-string assemblage: strings, hands, top, air, room, receipts |
| [`blender/README.md`](blender/README.md) | seeing the world play |
| [`aigua/README.md`](aigua/README.md) | the Aigua studies |
| [`docs/INSTRUMENT.md`](docs/INSTRUMENT.md) | the long map of the whole instrument and its repository |

Agents: start at [`AGENTS.md`](AGENTS.md).

## Status

Research, pre-alpha. Interfaces move, and receipts are the source of truth. The steel-string is deliberately young. Said plainly (see [`SPEC-ARIADNE-STEEL-STRING.md`](SPEC-ARIADNE-STEEL-STRING.md)):

- No damping at note-off yet, so `[stac]` and lifted fingers can't be heard.
- Each string has one pluck point, its hand's.
- 24 partials per string; the model is linear (no tension modulation, no fret buzz).
- The back, ribs and neck are rigid, and radiation has no directivity.
- High frequencies cross the bridge weakly; the band-rising bridge rate is a fit, not a law. Today the guitar is warm.
- Harmonics are checked against pitch but played as ordinary plucks.
- It is a guitar-*shaped* body, not a model of any particular guitar.

The project keeps an explicit claim boundary: a successful render is not by itself a theorem about transport, a biological reconstruction, intelligible speech, or listener perception. Open obligations travel in receipts.

## License

MIT. See [`LICENSE`](LICENSE).
