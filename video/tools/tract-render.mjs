// Play the public World tract (ovillo-tract-world, a mus_ariadne::World,
// through ovillo-tract-wasm's C ABI) offline and deterministically: one run
// writes BOTH the audio (the World's output, float32 mono, 48 kHz, no gain)
// and, at every video frame, the World's own state vector (areas, pressures,
// folds, books), so the pictures and the sound are the same performance.
//
//   node tools/tract-render.mjs <tract.wasm> <out-dir> [--fps 30] [--check-observers]
//
// The score is SCRIPT below: the page's controls (tongue index/diameter,
// lips, velum, fold tuning, tenseness, breath, a touch constriction), set
// at the times given, exactly as the page's sliders and presets set them.
// --check-observers renders a second time without any snapshot and asserts
// the samples are bit-identical (the snapshot only reads observers).
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const args = process.argv.slice(2);
const wasmPath = args[0];
const outDir = args[1];
const fps = Number(args.includes('--fps') ? args[args.indexOf('--fps') + 1] : 30);
const checkObservers = args.includes('--check-observers');
if (!wasmPath || !outDir) {
  console.error('usage: node tools/tract-render.mjs <tract.wasm> <out-dir> [--fps 30] [--check-observers]');
  process.exit(2);
}

const SR = 48000;
// ovillo-tract-world abi::layout (identical to ovillo-tract-wasm's).
const L = {
  ORAL_N: 0, NASAL_N: 1, VELUM_SECTION: 2, FOLD_GAP_MM: 3, FOLD_GAP_MIN_MM: 4, FOLD_GAP_MAX_MM: 5,
  GLOTTAL_FLOW: 6, LUNG_PA: 7, LUNG_FRACTION: 8, F0: 9, TRACT_E: 10, FOLD_E: 11, LUNG_E: 12,
  MUSCLE: 13, IMPACT: 14, TRACT_CTRL: 15, FOLD_CTRL: 16, WALL: 17, JET: 18, FOLD_LOSS: 19,
  RAD_LIPS: 20, RAD_NOSE: 21, FRAMES: 22, MAX_RES: 23, GLOBAL_RES: 24, CUM_RES: 25,
  CUR_INDEX: 26, CUR_DIAM: 27, CUR_LIPS: 28, CUR_VELUM: 29, CUR_PITCH: 30, CUR_TENSE: 31, CUR_BREATH: 32,
  GESTURE: 33, SAFETY: 34, NONFINITE: 35, IMPACTS: 36, TURB: 37, CLOSS: 38, MAX_RE: 39,
  CYCLE: 40, AREAS: 72, PRMS: 136, NASAL: 200, CONS: 248, MIN_AP: 258, MIN_AP_POS: 259, DECIMATE: 260, LEN: 264,
};
// ndt_receipt(field): the World's own receipt, f64 (ovillo-tract-world abi.rs).
const RECEIPT = ['sourceWork', 'controlWork', 'internalLoss', 'radiation', 'initialEnergy', 'currentEnergy',
  'cumulativeClosureResidual', 'maximumFrameResidual', 'globalClosureResidual', 'sampleFrames'];
const P = { index: 0, diameter: 1, lips: 2, velum: 3, pitch: 4, tense: 5, breath: 6 };
// The vowels are the page's presets (tract_preset 0..2: world::PUBLIC_ART_*, fitted in
// this World, receipts/public-preset-fit.json); the s takes gestures::ART_S's tongue and
// the sss gesture's alveolar place and degree, on touch slot 1.
const ART_S = { index: 0.85, diameter: 0.55, lips: 0.6, velum: 0.0 };

// The score: [seconds, what, label]. Times are applied at the first 80-sample
// block boundary at or after them (1.67 ms). `ramp: { key: [to, seconds] }` moves
// a control linearly from where it is, block by block, as a hand on the page's
// slider does (a step target makes the articulators' finite speed click).
const SCRIPT = [
  [0.00, { preset: 3, set: { pitch: 110, tense: 0.6, breath: 0 } }, 'rest'],
  [0.00, { vibrato: [5.5, 0.12, 0.3] }, ''],
  [0.12, { preset: 0, set: { breath: 0.6 } }, '/a/'],
  [1.15, { ramp: { lips: [0, 0.06] } }, 'p'],
  [1.44, { preset: 0, set: { pitch: 123.5 } }, 'pa'],
  [2.45, { preset: 1, set: { pitch: 130.8 } }, '/i/'],
  [3.55, { preset: 2, set: { pitch: 146.8 } }, '/u/'],
  [4.52, { ramp: { velum: [1, 0.12] } }, ''],
  [4.62, { ramp: { lips: [0, 0.05] }, set: { index: 0.06, diameter: 0.73 }, pitch: 130.8 }, 'm'],
  [5.05, { preset: 0, set: { pitch: 123.5 } }, 'ma'],
  [6.25, { set: { breath: 0 } }, ''],
  [6.55, { set: { index: ART_S.index, diameter: ART_S.diameter, tense: 0, breath: 0.75 }, constrict: [1, 0.86, 0.93] }, 's'],
  [7.70, { set: { breath: 0 } }, ''],
  [7.84, { constrict: [1, 0, -1] }, ''],
];
const DURATION = 8.6;

const ramps = new Map(); // control -> { from, to, n0, n1 }
function apply(x, step, n) {
  for (const [k, [to, sec]] of Object.entries(step.ramp ?? {})) ramps.set(k, { from: x.tract_get(P[k]), to, n0: n, n1: n + Math.round(sec * SR) });
  if (step.preset !== undefined) x.tract_preset(step.preset);
  for (const [k, v] of Object.entries(step.set ?? {})) x.tract_set(P[k], v);
  if (step.pitch !== undefined) x.tract_set(P.pitch, step.pitch);
  if (step.vibrato) x.tract_vibrato(...step.vibrato);
  if (step.constrict) x.tract_constrict(...step.constrict);
}

async function render(snapshot) {
  ramps.clear();
  const bytes = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const x = instance.exports;
  const sections = x.tract_init_decimated(SR, 1); // the full schedule
  const total = Math.round(DURATION * SR);
  const block = 80;
  const perFrame = SR / fps;
  const audio = new Float32Array(total);
  const frames = [];
  const books = [];
  let next = 0;
  for (let n = 0; n < total; n += block) {
    while (next < SCRIPT.length && Math.round(SCRIPT[next][0] * SR) <= n) apply(x, SCRIPT[next++][1], n);
    for (const [k, r] of ramps) {
      const f = Math.min(1, (n - r.n0) / Math.max(1, r.n1 - r.n0));
      x.tract_set(P[k], r.from + (r.to - r.from) * f);
      if (f >= 1) ramps.delete(k);
    }
    const m = Math.min(block, total - n);
    x.tract_process(m);
    audio.set(new Float32Array(x.memory.buffer, x.tract_out_ptr(), m), n);
    if (snapshot && (n + m) % perFrame === 0) {
      x.tract_snapshot();
      frames.push(Array.from(new Float32Array(x.memory.buffer, x.tract_state_ptr(), L.LEN)));
      books.push(RECEIPT.map((_, i) => x.ndt_receipt(i)));
    }
  }
  return { audio, frames, books, sections };
}

function wavFloat32(samples) {
  const data = Buffer.from(samples.buffer, samples.byteOffset, samples.byteLength);
  const h = Buffer.alloc(44);
  h.write('RIFF', 0); h.writeUInt32LE(36 + data.length, 4); h.write('WAVE', 8);
  h.write('fmt ', 12); h.writeUInt32LE(16, 16); h.writeUInt16LE(3, 20); h.writeUInt16LE(1, 22);
  h.writeUInt32LE(SR, 24); h.writeUInt32LE(SR * 4, 28); h.writeUInt16LE(4, 32); h.writeUInt16LE(32, 34);
  h.write('data', 36); h.writeUInt32LE(data.length, 40);
  return Buffer.concat([h, data]);
}

const t0 = performance.now();
const run = await render(true);
const wall = (performance.now() - t0) / 1000;
const last = run.frames[run.frames.length - 1];
const sha = (buf) => crypto.createHash('sha256').update(buf).digest('hex');
const wav = wavFloat32(run.audio);
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(path.join(outDir, 'tract.wav'), wav);
let observersCheck = null;
if (checkObservers) {
  const bare = await render(false);
  const same = Buffer.compare(Buffer.from(bare.audio.buffer), Buffer.from(run.audio.buffer)) === 0;
  observersCheck = { renderedWithoutSnapshots: true, bitIdentical: same };
  if (!same) { console.error('snapshots changed the audio'); process.exit(1); }
}
let peak = 0; for (const v of run.audio) peak = Math.max(peak, Math.abs(v));
const receipt = {
  schema: 'ovillo.tract-world-video-render.v1',
  engine: 'ovillo-tract-world (TractNdAnatomy::public_v1, full schedule), via the ovillo-tract-wasm C ABI',
  wasm: path.basename(wasmPath), wasmSha256: sha(fs.readFileSync(wasmPath)),
  sampleRate: SR, seconds: DURATION, fps, oralSections: run.sections, wavSha256: sha(wav), peak,
  renderWallSeconds: wall,
  script: SCRIPT.map(([t, step, label]) => ({ t, ...step, label })),
  receipt: Object.fromEntries(RECEIPT.map((k, i) => [k, run.books[run.books.length - 1][i]])),
  books: {
    ledgerFrames: last[L.FRAMES], breathingMuscles: last[L.MUSCLE], orbs: last[L.IMPACT],
    tractControlWork: last[L.TRACT_CTRL], foldControlWork: last[L.FOLD_CTRL],
    wallLoss: last[L.WALL], glottalJetLoss: last[L.JET], foldLoss: last[L.FOLD_LOSS], constrictionJetLoss: last[L.CLOSS],
    turbulenceInjected: last[L.TURB], radiationLips: last[L.RAD_LIPS], radiationNose: last[L.RAD_NOSE],
    storedTract: last[L.TRACT_E], storedFolds: last[L.FOLD_E], lungs: last[L.LUNG_E],
    maximumFrameResidual: last[L.MAX_RES], globalClosureResidual: last[L.GLOBAL_RES], cumulativeClosureResidual: last[L.CUM_RES],
    nonFinite: last[L.NONFINITE], safetyLimited: last[L.SAFETY],
  },
  observersCheck,
  note: 'receipt: World::receipt at the last video frame (f64, ndt_receipt); books: the ABI snapshot categories (f32)',
};
fs.writeFileSync(path.join(outDir, 'tract-receipt.json'), JSON.stringify(receipt, null, 1));
fs.writeFileSync(path.join(outDir, 'tract-frames.json'), JSON.stringify({ schema: 'ovillo.tract-world-frames.v1', fps, layout: L, receiptColumns: RECEIPT, receipts: run.books, frames: run.frames.map((f) => f.map((v) => Number(v.toPrecision(6)))) }));
console.log(`tract: ${DURATION} s in ${wall.toFixed(2)} s wall, ${run.frames.length} frames, peak ${(20 * Math.log10(peak)).toFixed(1)} dBFS, max frame residual ${last[L.MAX_RES].toExponential(2)} J`);
const rc = run.books[run.books.length - 1];
console.log('receipt', Object.fromEntries(RECEIPT.map((k, i) => [k, rc[i]])));
