# The bench, in a browser

The Ovillo rig (the simulated guitar, its guitarist and the Ariadne World,
`ovillo_dsp::rig::Rig`) compiled to WebAssembly and run live in an
AudioWorklet. Nothing here synthesizes a note: the guitar's DI touches the
World and the World sounds.

```sh
./build.sh                                  # ovillo-web -> ./ovillo_web.wasm (~650 KB)
python3 -m http.server -d .. 8000           # then open http://localhost:8000/bench/
node rtf.mjs ./ovillo_web.wasm 20 0         # real-time factor at 48 kHz (pattern 0 = WANDER)
```

| file | what |
|---|---|
| `bench.js` | `mountBench(container, { wasmUrl, workletUrl })`: screens, softkeys, strings, MAP |
| `worklet.js` | the `AudioWorkletProcessor`: one `ovillo-web` instance, rendered a quantum at a time |
| `bench.css` | the instrument-CRT look (black field, graticules, square boxes, one monospace face) |
| `ovillo_web.wasm` | built by `build.sh`, not committed |
| `scores/first-light.json` | the score OUTPUT ▸ FIRST LIGHT plays |

The C ABI is in `ovillo/crates/ovillo-web/src/lib.rs` (`ovillo_*` exports;
telemetry slots in `slot`). After Ariadne come two stages that are not part of
the World or any patch: the master (the app's monitor stage, starting at −12 dB)
and a runaway guard that fades out, rests the World and fades back in when the
safety limiter holds most of the time (GUARD on screen).
