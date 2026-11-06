# Using Ovillo from a three.js game

Every sound is a World: a kart engine is an engine, a gate is a gate, a voice
is a vocal tract. The game moves the World's controls from game state and
fires its events; Web Audio puts the World's node where the object is. The
World never knows where the listener is.

## 1. The AudioContext

```js
import * as THREE from "three";
import { Ovillo } from "ovillo";

const ctx = new AudioContext({ sampleRate: 48000, latencyHint: "interactive" });
const ov = await Ovillo.load(ctx);          // once; reuse `ov` for every World
// Browsers start contexts suspended: resume on the first click or key.
addEventListener("pointerdown", () => ctx.resume(), { once: true });
```

48 kHz is required for the tab instruments, the piano and the trombone (their
plans run at 48 kHz); the tract, the fart, the bench and the game sources run at
any rate. With a bundler, make sure `ovillo.wasm` and `ovillo-worklet.js` are
served beside `ovillo.js`, or pass `{ wasmUrl, workletUrl }` (in Vite:
`new URL("ovillo/ovillo.wasm", import.meta.url)`).

## 2. A World per object: `w.node -> PannerNode -> destination`

```js
const listener = ctx.listener;               // move it with the camera each frame (below)
const kart = await ov.create("kart_engine", { options: { seed: playerIndex + 1 } });
const panner = new PannerNode(ctx, {
  panningModel: "HRTF",
  distanceModel: "inverse",
  refDistance: 1,                            // the kart's output is pressure at 1 m
  rolloffFactor: 1,
  maxDistance: 2000,
});
kart.node.connect(panner).connect(ctx.destination);
kart.event("start");
```

Each World's output is its own pressure at its reference distance (catalog
`output.observes`), so `refDistance` is that distance. Mono kinds
(`channels: 1`: trombone, tract, fart, the game sources) are point sources as
they are; for a stereo kind (guitar, piano, the film's anatomies) create it with
`{ channels: 1 }` when it should be a point. For propagation delay and Doppler
put a `DelayNode` before the panner and set its `delayTime` to
`distance / 343` every frame (the request's plan for the jet and the karts):
Doppler then falls out of the changing delay.

Mixing, loudness and limiting are the game's too: a `GainNode` per category
(engines, voices, props) into a master `DynamicsCompressorNode` before
`ctx.destination` is a reasonable start. No World clamps its output except
the tract's and the fart's own listeners (±1).

## 3. Driving controls at 60 Hz from game state

```js
function frame(dt) {
  // the listener is the camera
  const p = camera.getWorldPosition(new THREE.Vector3());
  const f = camera.getWorldDirection(new THREE.Vector3());
  const t = ctx.currentTime;
  listener.positionX.setValueAtTime(p.x, t); listener.positionY.setValueAtTime(p.y, t); listener.positionZ.setValueAtTime(p.z, t);
  listener.forwardX.setValueAtTime(f.x, t); listener.forwardY.setValueAtTime(f.y, t); listener.forwardZ.setValueAtTime(f.z, t);

  for (const k of karts) {
    const q = k.mesh.getWorldPosition(new THREE.Vector3());
    k.panner.positionX.setValueAtTime(q.x, t); k.panner.positionY.setValueAtTime(q.y, t); k.panner.positionZ.setValueAtTime(q.z, t);
    // one message per World per frame: the World ramps between updates
    k.world.set({ rpm: k.rpm, throttle: k.throttle, load: k.load, boost: k.boosting ? 1 : 0, skid: k.drift }, { ramp: dt });
  }
}
```

- Send all of a World's controls for a frame in one `set({...})` call (one
  message). `ramp: dt` makes the World travel to the new values over the frame
  instead of stepping; discrete controls (`surface`, `pedal`, `bowString`) are
  never ramped.
- The catalog gives every control's units, range and a sensible `rateHz`; 60 Hz
  is fine for all of them. Many Worlds move at their own mechanical speed
  whatever you send (`smoothing`).
- Read `w.telemetry` (about 30 times a second) for game logic that wants to
  know what the World is doing: the tract's measured `f0Hz`, the trombone's
  `partial`, a kart's `misfires`, any World's `radiation`.

## 4. One-shots: a gunshot, an impact, a gate slam

A one-shot is an **event on a World that already exists**, not a sound file.
The World is the thing that is struck, and it rings by its own losses:

```js
gate.event("strike", { element: 6, velocity: 1 });          // the panel, fff, now
const at = ctx.currentTime + 0.05;                           // or exactly on a frame
drums.event("strike", { element: 3, velocity: 0.9 }, { at });
kart.event("bump", { impulse: hitStrength });                // the chassis, through its contact
kart.event("hit");                                           // the clunk, then the engine dies
voice.event("impact", { position: 0.5, energy: 2e-4 });      // a knock on the tract wall
```

- Keep a small pool of Worlds per kind of thing (the request's ceilings:
  8 karts, 3 explosions, 16 impacts) and route each event to the nearest free
  or quietest one (`w.telemetry.storedEnergy` says how much is still ringing);
  re-position its panner first. A struck World keeps ringing from the last
  touch: that is the physics, not a voice to steal.
- `at` schedules on the audio clock: fire on the frame a collision happened,
  not when the message arrives.
- Some events are a gesture over time inside the World (the bell's `finish`,
  the fart's `style`, the tract's `gesture`): one event, the World plays it.
- Things that are not in the catalog yet (guns, footsteps, maverick, explosion)
  have no World; nothing here fakes them with samples.

## 5. Disposing

```js
w.dispose();      // the World's handle is freed in the worklet; its node disconnects
panner.disconnect();
```

Dispose Worlds that leave the scene for good; for objects that come and go
(respawning karts), keep the World and `await w.reset()` it (at rest, books at
zero) instead of building a new one on the audio thread.

## 6. Receipts

`await w.receipt()` returns the World's books (`ledger`: source work, control
work, internal loss, radiation, residuals, `closes`), what it was asked to do,
and a digest of everything it rendered. It is the honest answer to "what did
this sound cost and where did the energy go", and it travels with any capture
or replay of a scene.
