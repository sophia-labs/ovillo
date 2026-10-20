// node site/bench/rtf.mjs <ovillo_web.wasm> [seconds] [pattern-index]
// The wasm rig's real-time factor at 48 kHz, 128-frame quanta, as a worklet runs it.
import { readFileSync } from "node:fs";
const [wasmPath, secs = "20", pat = "0"] = process.argv.slice(2);
const mod = await WebAssembly.compile(readFileSync(wasmPath));
console.log("imports:", JSON.stringify(WebAssembly.Module.imports(mod)));
const { exports: x } = await WebAssembly.instantiate(mod, {});
const sr = 48000;
if (x.ovillo_init(sr, 12345) !== 0) throw new Error("init failed");
const len = x.ovillo_info(); // may grow memory: take views after
const info = JSON.parse(new TextDecoder().decode(new Uint8Array(x.memory.buffer, x.ovillo_text_ptr(), len)));
x.ovillo_set_pattern(Number(pat));
console.log("pattern:", info.patterns[Number(pat)].id, "plan:", info.planDigest.slice(0, 16));
const n = Math.round(Number(secs) * sr / 128);
let peak = 0, worst = 0;
const t0 = performance.now();
for (let i = 0; i < n; i++) {
  const q0 = performance.now();
  x.ovillo_render(128);
  const dt = performance.now() - q0;
  if (i > 50) worst = Math.max(worst, dt);
  const l = new Float32Array(x.memory.buffer, x.ovillo_left_ptr(), 128);
  for (let k = 0; k < 128; k++) peak = Math.max(peak, Math.abs(l[k]));
}
const wall = (performance.now() - t0) / 1000;
const t = new Float64Array(x.memory.buffer, x.ovillo_telemetry_ptr(), info.slots.count);
console.log(`audio ${secs}s in ${wall.toFixed(2)}s -> ${(Number(secs) / wall).toFixed(2)}x real time; worst quantum ${worst.toFixed(2)} ms of ${(128 / sr * 1000).toFixed(2)} ms`);
console.log(`wet peak ${(20 * Math.log10(peak)).toFixed(1)} dBFS (master -12 dB); residual ${t[info.slots.closureResidual].toExponential(2)}; limited ${t[info.slots.safetyLimitedFrames]}; guard ${t[info.slots.guard]}`);
