# ovillo: every World, one wasm, one API

Nothing in Ovillo synthesizes a note. An event or a control touches a
**World**, the World does the sounding, and every render closes an energy
ledger with a receipt. This package puts every World the repository has
behind one WebAssembly module (`ovillo.wasm`, built from
[`ovillo/crates/ovillo-portable`](../../ovillo/crates/ovillo-portable)) and one
JavaScript API that runs in a browser AudioWorklet, in Node (offline renders),
and inside a three.js game.

| file | what |
|---|---|
| `ovillo.js` | the main-thread API: `Ovillo.load(ctx)`, `ov.create(kind)`, `World` (its `node` is a real AudioNode), `Ovillo.render(...)` offline |
| `ovillo-worklet.js` | the AudioWorkletProcessor (`"ovillo-world"`): one wasm instance per AudioContext, one World handle per node |
| `core.js` | the wasm wrapper the other two use; no Web Audio, no fetch, no TextDecoder, so it runs anywhere |
| `ovillo.d.ts` | types |
| `ovillo-catalog.json` | every World kind, generated from the code (`build.sh`); a game dev's wardrobe catalog |
| `audition/` | one page with every World, live |
| [`THREE.md`](THREE.md) | using Ovillo from a three.js game |
| `receipts/` | the wasm path's receipt for one offline render of each kind (`npm test`) |

Build: `bash build.sh` (needs the `wasm32-unknown-unknown` target and node).
Test: `npm test` (node), and in `ovillo/`: `cargo test --release -p ovillo-portable`.
Audition: serve the repository root (`python3 -m http.server 8766`) and open
`http://localhost:8766/web/ovillo/audition/`.

## The API

```js
import { Ovillo } from "ovillo";                       // or "./web/ovillo/ovillo.js"

const ctx = new AudioContext({ sampleRate: 48000 });   // the tab instruments, piano and trombone run at 48 kHz only
const ov = await Ovillo.load(ctx);                     // compile ovillo.wasm, register the worklet (once per context)
ov.kinds();                                            // ["steel-string", "gate", ..., "kart_engine", "race_props", "f16"]

const w = await ov.create("trombone");                 // { anatomy, options, score, channels: 1 | 2 }
w.node.connect(ctx.destination);                       // a real AudioNode: the mix is yours
w.set({ mouth: 0.4, lip: 240 });                       // continuous controls (catalog: name, units, range, rate)
w.set("slide", 0.18, { at: ctx.currentTime + 0.5, ramp: 0.2 });
w.event("note", { pitch: 58, velocity: 0.6, articulation: "slur" }, { at: ctx.currentTime + 1 });
w.telemetry;                                           // ~30 Hz: frame, the ledger, peak, rms, the kind's own slots
await w.play(scoreText);                               // a score, in the kind's format (catalog `score`)
const receipt = await w.receipt();                     // ovillo.portable.receipt.v1
w.dispose();

// offline, in Node or on a page: the same Worlds, the same receipts
const { left, right, receipt } = await Ovillo.render({
  kind: "steel-string", score: musText, seconds: 10,          // or options / anatomy / events / controls
  events: [{ at: 0.5, name: "pluck", args: { string: 6, fret: 0, velocity: 0.7 } }],
});
```

- **Events** are scheduled to the sample: a render is split at every due
  event, so an event lands on its frame (`at` is AudioContext time; omitted,
  the next render quantum). A kind that schedules inside its World hands the
  event over early (`lookaheadSeconds`): a tab fret lands 8 ms before its
  pluck when there is time; a piano damper rises 15 ms before its hammer
  arrives (a key pressed *now* sounds 15 ms later, as the action does).
- **Continuous controls** are set on their frame; `ramp` moves them linearly,
  re-set every 32 frames. Most Worlds also move at their own mechanical speed
  whatever the update rate (the tract's articulators, the bow's landing, the
  bench's parameter smoothing); the catalog's `smoothing` says which. Drive
  them at the catalog's `rateHz` (60 Hz from a game loop is fine).
- **Many Worlds per module**: every World node of one AudioContext shares one
  wasm instance and owns one handle in it. Building a World runs on the audio
  thread (a few ms for most kinds; see `cost`), so build them before they are
  needed.
- **Telemetry** (`w.telemetry`): the common slots (`catalog.commonTelemetry`:
  frame, source work, control work, internal loss, radiation, stored energy,
  residuals, peak, rms, errors, clamped, events applied and refused), then the
  kind's own (`catalog.kinds[].telemetry`).
- **Receipts** (`await w.receipt()`, `Ovillo.render(...).receipt`):
  `ovillo.portable.receipt.v1` with the World's own books (`ledger`, closing
  within `closureTolerance`), its renderer's own receipt (`world`), what was
  asked of it (`events`, `scores`), an `outputDigest` (sha256 of every frame
  rendered), and what it does not model.

### Why a plain C ABI, not wasm-bindgen

An `AudioWorkletGlobalScope` has no `fetch`, no `TextDecoder`, and must build
its World synchronously (an OfflineAudioContext renders before an async
compile could finish). The C ABI module has **no imports**: the same bytes
instantiate synchronously in the worklet, in Node, in a Worker and on a page,
with no generated glue to keep in step. Numbers cross as arguments; text (the
catalog, options, scores, receipts) crosses as UTF-8 in one shared buffer each
way. Every other Ovillo wasm (`ovillo-web`, `ovillo-tract-world`, `mus-wasm`)
already does it this way. The module also carries those crates' older
single-instance exports (`ovillo_*`, `tract_*`, `ndt_*`, `fart_*`); this
package uses only `ov_*` (`ovillo/crates/ovillo-portable/src/abi.rs`).

## The boundary: what is the World's and what is yours

A World's output is **its own observation times one fixed listening gain**
(catalog `output`): the steel-string's stereo listener ×300 (what `mus render`
mixes into its bus), the piano's far-field pressure in Pa ×0.1, the
trombone's bell pressure ×1/300, the tract's lip-and-nostril pressure ×1/2500,
a game source's pressure at its reference distance over its `OUTPUT_SCALE_PA`.
There is **no mix bus, no master, no limiter, no high-pass, no room, no
spatialization** after it. Mono kinds (`channels: 1`) are heard at one point;
the others at two listening points (create with `channels: 1` for a point
source). Output is not clamped, except where a kind's own listener clamps
(the tract and the fart at ±1, the bench's plug-in stage at ±1 and its runaway
guard) and says so (`clamped`).

Mixing, panning, distance, HRTF, propagation delay, Doppler, reverberation,
loudness and limiting are **outside the World, and they are the consumer's**:
connect `w.node` to whatever Web Audio graph you like. The audition page's pan,
gain and limiter are labelled as the listener's mix for this reason. A World
does not know where the listener is (the turbofan's `aspect` is its one
declared, non-loading directivity readout).

## The catalog

`ovillo-catalog.json` (`ovillo.portable.catalog.v1`) is generated from the
kind modules' own tables (the indices the ABI takes). For each kind: `events`
and `controls` (name, units, range, default, update rate, smoothing),
`anatomies`, `options`, `telemetry`, `score` format, `output` (what it
observes, the gain), `cost`, and **`doesNotModel`** (quoted from its spec, with
`doesNotModelRef`). The kinds today:

| kind | family | what |
|---|---|---|
| `steel-string` | tab | the steel-string guitar (`SPEC-ARIADNE-STEEL-STRING.md`): pluck, hammer-on, pull-off, slide; the bow (land/lift; `bowString`, `bowSpeed`, `bowForce`, `bowPosition`); MUS-T scores |
| `gate`, `waiting`, `glass-bells`, `fleas`, `slam-drums` | tab | the film's anatomies (`examples/ensemble/before-the-law`) |
| `sunwire`, `tumbler`, `prism`, `pocket` | tab | Sunskip's anatomies (`examples/ensemble/sunskip`) |
| `piano` | piano | `piano-v1` (`SPEC-ARIADNE-PIANO.md`): key, release, note; the pedal; MUS scores; `keys` chooses what is strung |
| `trombone` | trombone | v1 (`TROMBONE-WORLD.md`): direct `mouth`, `lip`, `embouchure`, `slide`; the calibrated player's `note`/`release`; the film driver's note lists |
| `tract` | tract | the vocal tract (`TRACT-WORLD.md`): 21 anatomies (public-v1, the checkpoint's two, six choir voice types, the choir's twelve singers with their pitch tables: `sing`) |
| `fart` | fart | Vera's October work (`fart.rs`): styles, foods, the cannon shot; push, cheeks, tone, tension, clench, rasp, digest |
| `bench` | bench | the simulated guitarist and guitar driving the canonical World (`ovillo/README.md`); the guitar is a signal model, not a World |
| `kart_engine`, `race_props`, `f16` | game-sound | the game's sources (`ovillo/crates/ovillo-game-sound`, partial): standalone Worlds on that crate's own runtime, booking a `mus_watsn` ledger; not `mus_ariadne` plans. `f16` is probed only, not auditioned |

## Adding a World

A new kind is a module in `ovillo/crates/ovillo-portable/src/kinds/` that
returns `KindDef`s (its catalog entry and a `build` function returning a
`Source`), and one line in `kinds::all`. A `GameWorld` from
`ovillo-game-sound` is one more entry in `kinds/game.rs` (its control and
event names are the trait's own strings). Then `bash build.sh`; the JS package
needs no change, because it reads names, units and indices from the catalog.
`cargo test -p ovillo-portable` refuses a kind without a gate.

## Real-time factor

`node tools/rtf.mjs ovillo.wasm` renders each kind in 128-frame quanta, as a
worklet does, and prints audio seconds over wall seconds, the worst quantum
against its 2.67 ms budget, and how long building the World took. The numbers
in `rtf.json` beside it are one run on a shared, heavily loaded laptop:
informational only.
