// Time any tract engine through the shared C ABI (ovillo-tract-wasm's), so
// the tube tract and the World tract run the same workloads.
// node examples/abi_bench.mjs <engine.wasm> [seconds]
import { readFileSync } from "node:fs";
const [, , path, secondsArg] = process.argv;
const seconds = Number(secondsArg ?? 8);
const { instance } = await WebAssembly.instantiate(readFileSync(path), {});
const x = instance.exports;
const workloads = [
  ["held /a/", () => { x.tract_set(6, 0.6); x.tract_preset(0); }],
  ["mama loop (the page's probe)", () => { x.tract_set(6, 0.6); x.tract_gesture(0); }],
  ["papapa loop", () => { x.tract_gesture(10); }],
  ["sss", () => { x.tract_gesture(2); }],
];
for (const decimate of [1, 2]) {
  for (const [name, start] of workloads) {
    const t0i = performance.now();
    x.tract_init_decimated(48000, decimate);
    const initMs = performance.now() - t0i;
    start();
    for (let i = 0; i < 375; i++) x.tract_process(128);
    const blocks = Math.round((seconds * 48000) / 128);
    const cpu0 = process.cpuUsage();
    const t0 = performance.now();
    for (let i = 0; i < blocks; i++) {
      x.tract_process(128);
      if (i % 4 === 0) x.tract_snapshot();
      if (name === "sss" && i % 600 === 0) x.tract_gesture(2);
    }
    const wall = (performance.now() - t0) / 1000;
    const c = process.cpuUsage(cpu0);
    const cpu = (c.user + c.system) / 1e6;
    x.tract_snapshot();
    const s = new Float32Array(x.memory.buffer, x.tract_state_ptr(), x.tract_state_len());
    console.log(`${path.split("/").pop()} decimate ${decimate} ${name.padEnd(30)} init ${initMs.toFixed(0)} ms, wall RTF ${(seconds / wall).toFixed(2)}x (load ${(100 * wall / seconds).toFixed(1)}% of one core), cpu RTF ${(seconds / cpu).toFixed(2)}x, max frame residual ${s[23].toExponential(1)} J, F0 ${s[9].toFixed(0)}`);
  }
}
