// Time the N-D tract World as wasm32 under node (V8), wall and CPU.
// node examples/wasm_bench.mjs <path to ovillo_tract_world.wasm> [seconds]
import { readFileSync } from "node:fs";
const [, , path, secondsArg] = process.argv;
const seconds = Number(secondsArg ?? 10);
const { instance } = await WebAssembly.instantiate(readFileSync(path), {});
const x = instance.exports;
const configs = [
  [44, 31, 32, 0, 0], [44, 31, 32, 1, 0], [44, 31, 64, 1, 0], [44, 31, 128, 1, 0],
  [44, 31, 32, 1, 0.001], [32, 16, 32, 1, 0], [32, 16, 64, 1, 0],
];
for (const [oral, nasal, interval, moving, minMove] of configs) {
  x.ndt_init(oral, nasal, interval, moving, minMove);
  for (let i = 0; i < 48; i++) x.ndt_render(1000); // warm-up 1 s
  const blocks = Math.round((seconds * 48000) / 128);
  const cpu0 = process.cpuUsage();
  const t0 = performance.now();
  let acc = 0;
  for (let i = 0; i < blocks; i++) acc += x.ndt_render(128);
  const wall = (performance.now() - t0) / 1000;
  const cpu = process.cpuUsage(cpu0);
  const cpuSeconds = (cpu.user + cpu.system) / 1e6;
  console.log(
    `wasm32 oral ${oral} nasal ${nasal} interval ${interval} min_move ${minMove} ${moving ? "moving /aiua/" : "held /a/"}: ` +
    `audio ${seconds} s, wall ${wall.toFixed(2)} s (RTF ${(seconds / wall).toFixed(2)}x), cpu ${cpuSeconds.toFixed(2)} s (RTF ${(seconds / cpuSeconds).toFixed(2)}x), ` +
    `moves ${x.ndt_moves()}, max frame residual ${x.ndt_max_frame_residual().toExponential(2)} J, cumulative ${x.ndt_cumulative_residual().toExponential(2)} J, energy ${acc.toExponential(3)}`
  );
}
