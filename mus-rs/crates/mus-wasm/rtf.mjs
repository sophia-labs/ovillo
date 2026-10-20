// node mus-rs/crates/mus-wasm/rtf.mjs <mus_wasm.wasm> <score.mus> [receipt.json from `mus render --json`]
//
// Checks and renders a score in the wasm engine the way a Worker does (one
// second of World per step), prints the real-time factor at 48 kHz, and, given
// the native receipt, compares renderDigest and the ledger with `mus render`'s.
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";

const [wasmPath, scorePath, receiptPath] = process.argv.slice(2);
if (!wasmPath || !scorePath) {
  console.error("usage: node rtf.mjs <mus_wasm.wasm> <score.mus> [native-receipt.json]");
  process.exit(2);
}
const bytes = readFileSync(wasmPath);
const mod = await WebAssembly.compile(bytes);
console.log(`wasm ${bytes.length} bytes, imports ${JSON.stringify(WebAssembly.Module.imports(mod))}`);
const { exports: x } = await WebAssembly.instantiate(mod, {});
const text = () => new TextDecoder().decode(new Uint8Array(x.memory.buffer, x.mus_text_ptr(), x.mus_text_len()));
const put = (s) => {
  const b = new TextEncoder().encode(s);
  new Uint8Array(x.memory.buffer, x.mus_input_ptr(b.length), b.length).set(b);
  return b.length;
};

const score = readFileSync(scorePath, "utf8");
x.mus_info();
console.log("info", text());
let t = performance.now();
x.mus_check(put(score));
const report = JSON.parse(text());
console.log(`check ${(performance.now() - t).toFixed(1)} ms: clean=${report.clean}, ${report.diagnostics.length} diagnostics${report.diagnostics.map((d) => `\n  ${d.severity} ${d.code}: ${d.message}`).join("")}`);

t = performance.now();
const cpu0 = process.cpuUsage();
const rc = x.mus_render_begin(put(score));
if (rc !== 0) {
  console.log(`refused (${rc}):`, text().slice(0, 400));
  process.exit(rc === 1 ? 0 : 1);
}
const total = x.mus_render_total();
let state = 0;
let lastPrint = 0;
while (state === 0) {
  state = x.mus_render_step(48000);
  const now = performance.now();
  if (now - lastPrint > 10000) {
    lastPrint = now;
    console.log(`  ${(100 * x.mus_render_done() / total).toFixed(0)}% after ${((now - t) / 1000).toFixed(0)} s`);
  }
}
if (state !== 1) throw new Error(text());
const worldSeconds = (performance.now() - t) / 1000;
const t2 = performance.now();
if (x.mus_render_finish(worldSeconds) !== 0) throw new Error(text());
const receipt = JSON.parse(text());
const wall = (performance.now() - t) / 1000;
const frames = x.mus_render_frames();
const wav = new Uint8Array(x.memory.buffer, x.mus_render_wav_ptr(), x.mus_render_wav_len());
const digest = "sha256:" + createHash("sha256").update(wav).digest("hex");
const audioSeconds = frames / 48000;
const cpu = process.cpuUsage(cpu0);
const cpuSeconds = (cpu.user + cpu.system) / 1e6;
console.log(`cpu: ${cpuSeconds.toFixed(1)} s -> ${(audioSeconds / cpuSeconds).toFixed(3)}x real time per CPU second (one thread)`);
console.log(`render: ${audioSeconds.toFixed(2)} s of audio (${(total / 48000).toFixed(2)} s of World) in ${wall.toFixed(1)} s -> ${(audioSeconds / wall).toFixed(3)}x real time (World ${worldSeconds.toFixed(1)} s, bus ${((performance.now() - t2) / 1000).toFixed(1)} s); memory ${(x.memory.buffer.byteLength / 2 ** 20).toFixed(0)} MiB`);
console.log(`renderDigest ${receipt.renderDigest} (wav bytes hash to ${digest === receipt.renderDigest ? "the same" : digest})`);
const world = receipt.ariadneWorlds?.[0];
if (world) {
  console.log(`ledger ${world.instrument}: source ${world.sourceWork} control ${world.controlWork} loss ${world.internalLoss} radiation ${world.radiation} closure ${world.cumulativeClosureResidual} max frame ${world.maximumFrameResidual}`);
}
x.mus_render_energy();
const energy = JSON.parse(text());
console.log(`energy: ${energy.schema}, ${energy.worlds[0]?.frames.length} frames of ${energy.worlds[0]?.channels.join(",")}`);

if (receiptPath) {
  const native = JSON.parse(readFileSync(receiptPath, "utf8"));
  const same = (k) => JSON.stringify(native[k]) === JSON.stringify(receipt[k]);
  console.log(`vs mus render: renderDigest ${same("renderDigest") ? "EQUAL" : "DIFFERS"}; ariadneWorlds ${same("ariadneWorlds") ? "EQUAL" : "DIFFER"}; peak ${same("peakDbfs") ? "equal" : `${native.peakDbfs} vs ${receipt.peakDbfs}`}`);
  if (!same("ariadneWorlds") && world) {
    const nw = native.ariadneWorlds[0];
    for (const k of ["planDigest", "listenerDigest", "sourceWork", "controlWork", "internalLoss", "radiation", "cumulativeClosureResidual", "maximumFrameResidual"]) {
      if (nw[k] !== world[k]) {
        const rel = typeof nw[k] === "number" ? ` (rel ${(Math.abs(nw[k] - world[k]) / Math.max(Math.abs(nw[k]), 1e-300)).toExponential(2)})` : "";
        console.log(`  ${k}: native ${nw[k]} wasm ${world[k]}${rel}`);
      }
    }
  }
  console.log(`native wallSeconds ${native.wallSeconds.toFixed(1)} -> native ${(native.durationSeconds / native.wallSeconds).toFixed(3)}x real time`);
}
