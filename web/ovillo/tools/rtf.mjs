// node tools/rtf.mjs <ovillo.wasm> [kind ...] [--seconds S] [--json out.json]
// Each World kind's real-time factor at 48 kHz in node (V8), rendered in
// 128-frame quanta as an AudioWorklet runs it, playing the same touches as
// the gates (test/scenarios.mjs): audio seconds over wall seconds, the worst
// quantum against its 2.67 ms budget, and how long building the World took.
// Informational: it says what this machine did while it was this busy.
import { readFileSync, writeFileSync } from "node:fs";
import os from "node:os";
import { OvilloCore } from "../core.js";
import { scenario } from "../test/scenarios.mjs";

const argv = process.argv.slice(2);
const wasm = argv.shift();
let seconds = null;
let jsonOut = null;
const kinds = [];
for (let i = 0; i < argv.length; i++) {
  if (argv[i] === "--seconds") seconds = Number(argv[++i]);
  else if (argv[i] === "--json") jsonOut = argv[++i];
  else kinds.push(argv[i]);
}
const core = await OvilloCore.load(readFileSync(wasm));
const catalog = core.catalog();
const sr = 48000;
const rows = [];
for (const kind of kinds.length ? kinds : catalog.kinds.map((k) => k.id)) {
  const spec = scenario(kind, catalog);
  const options = { ...(spec.options ?? {}) };
  if (spec.score) options.score = spec.score;
  const c0 = performance.now();
  const w = core.create(kind, { anatomy: spec.anatomy, options }, sr);
  const createMs = performance.now() - c0;
  for (const e of spec.events ?? []) w.event(e.name, e.args ?? {}, Math.round(e.at * sr));
  for (const c of spec.controls ?? []) w.set(c.name, c.value, Math.round(c.at * sr), Math.round((c.ramp ?? 0) * sr));
  const total = Math.round((seconds ?? spec.seconds ?? 4) * sr);
  let worst = 0;
  let done = 0;
  const t0 = performance.now();
  while (done < total) {
    const q0 = performance.now();
    done += w.render(Math.min(128, total - done));
    worst = Math.max(worst, performance.now() - q0);
  }
  const wall = (performance.now() - t0) / 1000;
  const r = w.receipt();
  w.dispose();
  const row = {
    kind,
    audioSeconds: total / sr,
    wallSeconds: wall,
    realTimeFactor: total / sr / wall,
    worstQuantumMs: worst,
    quantumBudgetMs: (128 / sr) * 1000,
    createMs,
    cumulativeClosureResidual: r.ledger.cumulativeClosureResidual,
    maximumFrameResidual: r.ledger.maximumFrameResidual,
  };
  rows.push(row);
  console.log(
    `${kind.padEnd(13)} ${row.realTimeFactor.toFixed(2).padStart(6)}x real time  worst quantum ${worst.toFixed(2).padStart(6)} ms of 2.67  build ${createMs.toFixed(0).padStart(5)} ms  residuals ${r.ledger.cumulativeClosureResidual.toExponential(2)} / ${r.ledger.maximumFrameResidual.toExponential(2)} J`,
  );
}
const machine = { node: process.version, cpus: os.cpus().length, cpu: os.cpus()[0]?.model, loadAverage: os.loadavg(), date: new Date().toISOString() };
console.log(`node ${machine.node}, ${machine.cpus} cpus (${machine.cpu}), load average ${machine.loadAverage.map((x) => x.toFixed(1)).join(" ")}`);
if (jsonOut) writeFileSync(jsonOut, JSON.stringify({ schema: "ovillo.portable.rtf.v1", sampleRate: sr, quantum: 128, machine, rows }, null, 2) + "\n");
