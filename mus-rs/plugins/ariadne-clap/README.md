# Ariadne Terminal 0.1 — canonical World host

Ariadne Terminal is the stable VST3/CLAP identity for Ariadne's canonical
`mus_ariadne::World`. The name, CLAP ID, VST3 class ID, and all parameter IDs
remain unchanged so existing host sessions can survive the engine migration.
The compatibility terminal is no longer compiled into this plug-in.

Every wet sample now comes from one persistent 24-coordinate material-and-space
World with a compliant boundary, reservoir-paid active matter, ordered
metric-compatible transport, reciprocal delayed space, radiation, and one
global work ledger. The wrapper owns only host ABI, smoothing, parameter
lowering, dry/wet mixing, and a final full-scale safety bound.

## Install on macOS

Quit the DAW, then copy the bundles:

```text
Ariadne.vst3 -> ~/Library/Audio/Plug-Ins/VST3/
Ariadne.clap -> ~/Library/Audio/Plug-Ins/CLAP/
```

Rescan plug-ins in the host. Local development bundles are ad-hoc signed but
not Apple-notarized.

## Controls

The historical controls remain automatable and now lower into coordinates of
the same World:

- `Character` selects four nearby regions of the string-to-bore configuration
  path without changing or resetting the body.
- `Drive`, `Headroom`, `Bloom`, `Sustain`, and `Sag` control reservoir-paid
  active matter.
- `Interlace` and `Path Memory` control ordered, zero-work internal transport.
- `Bias`, `Tone`, `Curvature`, and `Chirality` move within the declared
  configuration and observation chart.
- `World Return`, `Spatial Coupling`, and `Halo Decay` control reciprocal space.
- `Mix` and `Output` remain host-facing output controls.

`Costa Clara` stays closest to the passive string endpoint. `Edge`, `Grind`,
and `Wall` progressively increase active-material participation, internal
transport, and movement toward the compliant-body region.

## Runtime contract

- One native-rate `World` at 44.1, 48, 96, or 192 kHz; zero reported latency.
- No compatibility terminal and no `mus-dsp` Ariadne implementation in the
  plug-in graph.
- No allocation, locks, I/O, parsing, graph search, or optimization on the
  sample path.
- Exact finite dry path at zero mix and unity output.
- Non-finite input containment, typed World failure fallback, receipt counters,
  and a conservative final ±1.0 safety bound.
- Twenty-second reported tail.

## Build and validate

From `mus-rs/`:

```sh
python3 tools/check_ariadne_architecture.py
python3 tools/check_ariadne_pure_physics.py
cargo test -p mus-ariadne --all-features --release --locked
cargo test -p ariadne-clap --release --locked
cargo clippy -p ariadne-clap --all-targets --locked -- -D warnings
cargo xtask bundle-universal ariadne-clap --release
```

## Offline render

The offline WAV face uses the same `AriadneEngine` as the plug-in:

```sh
cargo run -p ariadne-clap --release --example render_world -- \
  input.wav output.wav edge 4
```

It accepts mono or stereo PCM/float WAV, writes stereo float32 WAV, appends a
bounded tail, and prints a JSON receipt with the World digest, work-ledger
residuals, safety counts, and output levels.

This is a research preview. The canonical World cutover is locally testable;
DAW smoke tests and third-party plug-in validation remain release gates.
