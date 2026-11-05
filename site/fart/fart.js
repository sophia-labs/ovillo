// Ovillo Fart: the tract World's other end (TractNdAnatomy::fart_v1), the
// same mus_ariadne::World and wasm as site/tract/. Provisions pay the
// World's reservoir (tract_refill: its work_input, booked as source work);
// holding RELEASE spends it through the sphincter. The pictures read the
// World's state and never write it, except through the controls a player
// has. The provisions, styles, exercises, scores, achievements and the
// Auditor's lines are the page's inventions, not the World's.

import * as THREE from '../tract/vendor/three/three.module.min.js';

// ---------------------------------------------------------------- layout --
// ovillo-tract-world `abi::layout`.
const L = {
  ORAL_N: 0, FOLD_GAP_MM: 3, GLOTTAL_FLOW: 6, LUNG_PA: 7, LUNG_FRACTION: 8, F0: 9, LUNG_E: 12,
  MUSCLE: 13, TRACT_CTRL: 15, FOLD_CTRL: 16, WALL: 17, JET: 18, FOLD_LOSS: 19, RAD_LIPS: 20,
  FRAMES: 22, MAX_RES: 23, CUM_RES: 25, CUR_LIPS: 28, CUR_PITCH: 30, CUR_BREATH: 32,
  SAFETY: 34, NONFINITE: 35, TURB: 37, CLOSS: 38, MAX_RE: 39, CYCLE: 40, AREAS: 72, PRMS: 136,
  CONS: 248, LEN: 264,
};
const PARAM = { lips: 2, pitch: 4, tense: 5, breath: 6 };
// The cheeks clench inside the cleft (ovillo-tract-world fart::SEAL_AT), so
// the radiating volume is outside the seal.
const SEAL_SLOT = 1, SEAL_AT = 0.85;
const PITCH_MIN = 25, PITCH_MAX = 300; // fart_v0's valve tuning range
const SAG = 0.4; // fart_v0's FlowValveSpec::sag_fraction
const SUPPLY_MAX_PA = 2500; // fart_v0's max_supply_pressure_pa

// fart::FOODS: invented gas (J; the reservoir holds 1.5 J) and time (s).
const FOODS = [
  { id: 'beans', emoji: '\u{1FAD8}', name: 'beans', gasJ: 0.60, seconds: 8, color: 0x8a3b26 },
  { id: 'broccoli', emoji: '\u{1F966}', name: 'broccoli', gasJ: 0.35, seconds: 5, color: 0x4f7a3c },
  { id: 'cabbage', emoji: '\u{1F96C}', name: 'cabbage', gasJ: 0.40, seconds: 6, color: 0x8fb070 },
  { id: 'onion', emoji: '\u{1F9C5}', name: 'onion', gasJ: 0.30, seconds: 4, color: 0xb08aa0 },
  { id: 'egg', emoji: '\u{1F95A}', name: 'egg', gasJ: 0.20, seconds: 3, color: 0xf6efe0 },
  { id: 'soda', emoji: '\u{1F964}', name: 'soda', gasJ: 0.25, seconds: 1, color: 0xb83a2e },
  { id: 'cheese', emoji: '\u{1F9C0}', name: 'cheese', gasJ: 0.15, seconds: 5, color: 0xe0b04a },
  { id: 'salad', emoji: '\u{1F957}', name: 'salad', gasJ: 0.03, seconds: 2, color: 0x6f9f5a },
];
const MAX_FOODS = 14;

// fart::STYLES, as the page plays them (10 ms hops).
const STYLES = [
  { name: 'brap', seconds: 2.5, wobble: 0.6, jitter: 0, at: () => ({ lips: 0.45, pitch: 65, tense: 0.75, breath: 1 }) },
  { name: 'flutter', seconds: 2.5, wobble: 0.6, jitter: 4, at: () => ({ lips: 0.3, pitch: 60, tense: 0.7, breath: 1 }) },
  { name: 'squeaker', seconds: 1.6, wobble: 0.3, jitter: 0.5, at: (t) => ({ lips: 0.1, pitch: 210 + 30 * t, tense: 0.85, breath: t < 1.2 ? 1 : 0 }) },
  { name: 'trombone', seconds: 2.6, wobble: 0.4, depth: 0.4, rate: 4, jitter: 0, at: (t) => ({ lips: 0.35, pitch: 150 * Math.pow(35 / 150, Math.min(1, t / 2.2)), tense: 0.7, breath: 0.85 }) },
  { name: 'pbbt', seconds: 1.2, wobble: 0, jitter: 1, sealed: (t) => t >= 0.9 || (t * 7) % 1 >= 0.5, at: (t) => ({ lips: 0.4, pitch: 90, tense: 0.8, breath: t < 0.9 ? 0.7 : 0 }) },
  { name: 'sbd', seconds: 2.0, wobble: 0, jitter: 0, at: (t) => ({ lips: 0.04, pitch: 60, tense: 0, breath: t < 1.6 ? 0.8 : 0 }) },
];

// Exercises: public-domain tunes, NOTE:beats (fart::parse_tune's syntax).
const TUNES = [
  { id: 'funeral', name: 'Funeral March', by: 'Chopin, 1839', bpm: 60, text: 'Bb2:1 Bb2:0.75 Bb2:0.25 Bb2:1 Db3:0.75 C3:0.25 C3:0.75 Bb2:0.25 Bb2:0.75 A2:0.25 Bb2:2' },
  { id: 'mountain', name: 'Hall of the Mountain King', by: 'Grieg, 1875', bpm: 92, text: 'B2:0.5 C#3:0.5 D3:0.5 E3:0.5 F#3:0.5 D3:0.5 F#3:1 F3:0.5 C#3:0.5 F3:1 E3:0.5 C3:0.5 E3:1' },
  { id: 'joy', name: 'Ode to Joy', by: 'Beethoven, 1824', bpm: 100, text: 'E3 E3 F3 G3 G3 F3 E3 D3 C3 C3 D3 E3 E3:1.5 D3:0.5 D3:2' },
];
function noteHz(name) {
  const m = /^([A-Ga-g])([#b]*)(-?\d+)$/.exec(name.trim());
  if (!m) return null;
  const pc = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 }[m[1].toUpperCase()];
  const acc = [...m[2]].reduce((s, c) => s + (c === '#' ? 1 : -1), 0);
  return 440 * Math.pow(2, (12 * (Number(m[3]) + 1) + pc + acc - 69) / 12);
}
function parseTune(tune) {
  const beat = 60 / tune.bpm;
  let t = 0;
  return tune.text.split(/[\s,]+/).filter(Boolean).map((tok) => {
    const [name, beats = '1'] = tok.split(':');
    const n = { name, hz: noteHz(name), t0: t, t1: t + Number(beats) * beat, hit: 0, total: 0 };
    t = n.t1;
    return n;
  });
}

const query = new URLSearchParams(location.search);
const MUTE = query.has('mute');
const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
const $ = (id) => document.getElementById(id);
const store = {
  get(k, d) { try { const v = localStorage.getItem(`ovillo-fart:${k}`); return v === null ? d : JSON.parse(v); } catch { return d; } },
  set(k, v) { try { localStorage.setItem(`ovillo-fart:${k}`, JSON.stringify(v)); } catch { /* private mode */ } },
};

// ---------------------------------------------------------------- engine --
const wasmBytes = await (await fetch('../tract/tract.wasm')).arrayBuffer();
let state = new Float32Array(L.LEN);
{
  // A main-thread World for the picture before the demonstration (never played).
  const { instance } = await WebAssembly.instantiate(wasmBytes.slice(0), {});
  const x = instance.exports;
  x.fart_init_decimated(48000, 1);
  x.tract_snapshot();
  state = new Float32Array(x.memory.buffer, x.tract_state_ptr(), L.LEN).slice();
}

let audio = null; // { ctx, node, gain }
const sliders = { push: 1, pitch: 0.35, cheeks: 0.55, tone: 0.75, wobble: 0.4 };
const sent = new Map();
function send(msg) { if (audio) audio.node.port.postMessage(msg); }
function set(name, value) {
  const key = `set${PARAM[name]}`;
  if (sent.get(key) === value) return;
  sent.set(key, value);
  send({ type: 'set', param: PARAM[name], value });
}
function refill(watts) {
  if (Math.abs((sent.get('refill') ?? -1) - watts) < 1e-4) return;
  sent.set('refill', watts);
  send({ type: 'refill', watts });
}
function vibrato(rate, depth, wobble) {
  const key = `${rate}|${depth}|${wobble}`;
  if (sent.get('vib') === key) return;
  sent.set('vib', key);
  send({ type: 'vibrato', rate, depth, wobble });
}
function seal(on) {
  if (sent.get('seal') === on) return;
  sent.set('seal', on);
  send({ type: 'constrict', slot: SEAL_SLOT, position: SEAL_AT, degree: on ? 1 : -1 });
}
const pitchHz = (u) => PITCH_MIN * Math.pow(PITCH_MAX / PITCH_MIN, u);
const cheeks = (u) => 0.03 + 0.77 * u;

async function startAudio() {
  const ctx = new AudioContext({ latencyHint: 'interactive' });
  await ctx.audioWorklet.addModule('../tract/worklet.js');
  const node = new AudioWorkletNode(ctx, 'tract', {
    numberOfInputs: 0, numberOfOutputs: 1, outputChannelCount: [2],
    processorOptions: { bytes: wasmBytes.slice(0), decimate: query.get('lite') === '1' ? 2 : 1, anatomy: 'fart', init: [] },
  });
  // The listener is the pressure at the mouth, so steady flow is steady
  // pressure: a DC blocker on what you hear (a listening choice, as
  // fart::dc_block; the World's books are untouched).
  const dc = ctx.createBiquadFilter();
  dc.type = 'highpass';
  dc.frequency.value = 20;
  dc.Q.value = Math.SQRT1_2;
  const gain = ctx.createGain();
  gain.gain.value = MUTE ? 0 : 0.9;
  node.connect(dc).connect(gain).connect(ctx.destination);
  node.port.onmessage = (e) => {
    const m = e.data;
    if (m.type === 'state') state = m.state;
    else if (m.type === 'error') console.error('fart worklet:', m.message);
  };
  audio = { ctx, node, gain };
  sent.clear();
  set('breath', 0);
  applySliders();
  // A browser may hold the context suspended until a real gesture.
  const live = () => { if (ctx.state === 'running') $('cover').hidden = true; };
  ctx.addEventListener('statechange', live);
  ctx.resume().then(live, () => {});
  live();
}
$('start').addEventListener('click', () => { startAudio().catch((err) => { console.error(err); $('status').textContent = `audio failed: ${err}`; }); });

// --------------------------------------------------------------- release --
let holding = false;
let style = null; // { def, t0, rng }
function holdStart() {
  if (!audio) return;
  stopStyle();
  holding = true;
  $('release').classList.add('held');
  set('breath', sliders.push);
}
function holdEnd() {
  if (!holding) return;
  holding = false;
  $('release').classList.remove('held');
  set('breath', 0);
}
const stage = $('stage');
// Up is higher (tension), left is clenched (cheeks).
function aim(e) {
  const r = stage.getBoundingClientRect();
  const fx = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
  const fy = Math.min(1, Math.max(0, (e.clientY - r.top) / r.height));
  setSlider('pitch', 1 - fy);
  setSlider('cheeks', fx);
}
stage.addEventListener('pointerdown', (e) => {
  if (!audio || e.target.closest('.cover, .receipt')) return;
  stage.setPointerCapture(e.pointerId);
  aim(e);
  holdStart();
});
stage.addEventListener('pointermove', (e) => { if (holding && stage.hasPointerCapture(e.pointerId)) aim(e); });
stage.addEventListener('pointerup', holdEnd);
stage.addEventListener('pointercancel', holdEnd);
const rel = $('release');
rel.addEventListener('pointerdown', (e) => { rel.setPointerCapture(e.pointerId); holdStart(); });
rel.addEventListener('pointerup', holdEnd);
rel.addEventListener('pointercancel', holdEnd);
rel.addEventListener('keydown', (e) => { if (e.key === 'Enter' && !e.repeat) holdStart(); });
rel.addEventListener('keyup', (e) => { if (e.key === 'Enter') holdEnd(); });
addEventListener('keydown', (e) => {
  if (e.target.closest('input, button, select')) return;
  if (e.code === 'Space' && !e.repeat) { e.preventDefault(); holdStart(); }
  const step = { ArrowUp: ['pitch', 0.015], ArrowDown: ['pitch', -0.015], ArrowRight: ['cheeks', 0.03], ArrowLeft: ['cheeks', -0.03] }[e.key];
  if (step) { e.preventDefault(); setSlider(step[0], Math.min(1, Math.max(0, sliders[step[0]] + step[1]))); }
});
addEventListener('keyup', (e) => { if (e.code === 'Space') holdEnd(); });
addEventListener('blur', holdEnd);

// --------------------------------------------------------------- sliders --
const FORMAT = {
  push: (v) => `${Math.round(v * 100)}%`,
  pitch: (v) => `${pitchHz(v).toFixed(0)} Hz`,
  cheeks: (v) => (v < 0.05 ? 'clenched' : v > 0.85 ? 'wide' : cheeks(v).toFixed(2)),
  tone: (v) => v.toFixed(2),
  wobble: (v) => v.toFixed(2),
};
function setSlider(name, v) {
  sliders[name] = v;
  $(`s-${name}`).value = String(v);
  $(`o-${name}`).textContent = FORMAT[name](v);
  applySliders();
}
function applySliders() {
  if (style) return;
  set('pitch', pitchHz(sliders.pitch));
  set('lips', cheeks(sliders.cheeks));
  set('tense', sliders.tone);
  vibrato(5.5, 0, sliders.wobble);
  if (holding) set('breath', sliders.push);
}
for (const name of Object.keys(sliders)) {
  const input = $(`s-${name}`);
  input.addEventListener('input', () => setSlider(name, Number(input.value)));
  $(`o-${name}`).textContent = FORMAT[name](sliders[name]);
}

// ---------------------------------------------------------------- styles --
for (const def of STYLES) {
  const b = document.createElement('button');
  b.textContent = def.name;
  b.addEventListener('click', () => { if (audio) { holdEnd(); startStyle(def); } });
  $('styles').append(b);
  def.button = b;
}
function startStyle(def) {
  stopStyle();
  style = { def, t0: performance.now(), rng: 0x2545f491 };
  def.button.setAttribute('aria-pressed', 'true');
  vibrato(def.rate ?? 5.5, def.depth ?? 0, def.wobble);
}
function stopStyle() {
  if (!style) return;
  style.def.button.setAttribute('aria-pressed', 'false');
  style = null;
  seal(false);
  set('breath', 0);
  applySliders();
}
function styleTick() {
  if (!style) return;
  const t = (performance.now() - style.t0) / 1000;
  const def = style.def;
  if (t >= def.seconds) { stopStyle(); return; }
  const c = def.at(t);
  let pitch = c.pitch;
  if (def.jitter > 0) {
    let s = style.rng;
    s ^= s << 13; s ^= s >>> 17; s ^= s << 5; s >>>= 0;
    style.rng = s;
    pitch *= Math.pow(2, ((2 * (s / 4294967296) - 1) * def.jitter) / 12);
  }
  set('lips', c.lips);
  set('pitch', pitch);
  set('tense', c.tense);
  set('breath', c.breath);
  seal(def.sealed ? def.sealed(t) : false);
}
setInterval(styleTick, 10);

// ------------------------------------------------------------------ food --
const eaten = []; // { def, remaining, mesh, rest, v, spin }
let digesting = false;
for (const def of FOODS) {
  const b = document.createElement('button');
  b.innerHTML = `<span class="emoji" aria-hidden="true">${def.emoji}</span><span class="name">${def.name}</span><span class="gas">${def.gasJ.toFixed(2)} J · ${def.seconds} s</span>`;
  b.setAttribute('aria-label', `${def.name}: ${def.gasJ} joules of gas over ${def.seconds} seconds`);
  b.addEventListener('click', () => eat(def));
  $('foods').append(b);
}
$('digest').addEventListener('click', () => {
  digesting = !digesting && eaten.length > 0;
  $('digest').setAttribute('aria-pressed', String(digesting));
});
function eat(def) {
  if (eaten.length >= MAX_FOODS) { toast('The digester is full. Digest first.'); return; }
  const mesh = makeFood(def);
  const a = Math.random() * Math.PI * 2, rr = Math.sqrt(Math.random()) * 0.32;
  const rest = new THREE.Vector3(Math.cos(a) * rr, -0.32 + 0.22 * Math.random(), Math.sin(a) * rr);
  mesh.position.set(FLASK.x + rest.x * 0.3, FLASK.y + 3.6, rest.z);
  mesh.rotation.set(Math.random() * 3, Math.random() * 3, Math.random() * 3);
  scene.add(mesh);
  eaten.push({ def, remaining: def.gasJ, mesh, rest, v: 0, spin: (Math.random() - 0.5) * 5 });
  if (eaten.length >= MAX_FOODS) achieve('full', 'Full Digester', 'fourteen provisions at once');
}
let digestWatts = 0;
let full = false;
function digestTick(dt) {
  full = state[L.LUNG_FRACTION] >= 0.995;
  let want = 0;
  for (const f of eaten) want += f.def.gasJ / f.def.seconds;
  digestWatts = digesting && !full ? Math.min(1, want) : 0;
  const share = want > 0 ? digestWatts / want : 0;
  for (let i = eaten.length - 1; i >= 0; i--) {
    const f = eaten[i];
    f.remaining -= (f.def.gasJ / f.def.seconds) * share * dt;
    if (f.remaining <= 0) { scene.remove(f.mesh); eaten.splice(i, 1); }
  }
  if (digesting && eaten.length === 0) {
    digesting = false;
    $('digest').setAttribute('aria-pressed', 'false');
  }
  refill(digestWatts);
  const left = eaten.reduce((s, f) => s + f.remaining, 0);
  $('belly').textContent = eaten.length === 0 ? 'the digester is empty'
    : `${eaten.length} item${eaten.length > 1 ? 's' : ''}, ${left.toFixed(2)} J to come${digestWatts > 0 ? `, paying ${digestWatts.toFixed(2)} W` : full && digesting ? ', waiting: reservoir full' : ''}`;
}

// --------------------------------------------------------------- picture --
const canvas = $('gl');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
renderer.setPixelRatio(Math.min(devicePixelRatio, 1.5));
renderer.setClearColor(0x000000, 0);
const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(36, 1, 0.1, 100);
scene.add(new THREE.HemisphereLight(0xfff6e5, 0xa89878, 2.2));
const key = new THREE.DirectionalLight(0xfff0d8, 2.0);
key.position.set(-4, 7, 9);
scene.add(key);

const INK = 0x1a1714;
const mat = {
  glass: new THREE.MeshStandardMaterial({ color: 0xdfe8e2, roughness: 0.08, transparent: true, opacity: 0.2, side: THREE.DoubleSide, depthWrite: false }),
  brass: new THREE.MeshStandardMaterial({ color: 0xb8893a, roughness: 0.4, metalness: 0.45 }),
  rubber: new THREE.MeshStandardMaterial({ color: 0x6e1f22, roughness: 0.6 }),
  wood: new THREE.MeshStandardMaterial({ color: 0x7a5230, roughness: 0.85 }),
  ink: new THREE.LineBasicMaterial({ color: INK }),
  inkFaint: new THREE.LineBasicMaterial({ color: INK, transparent: true, opacity: 0.22 }),
};
const FLASK = { x: -4.5, y: -1.15, r: 0.62 };
const JAR = { x: -2.65, y0: -0.2, y1: 1.85, r: 1.15 };
const BLADDER = { x: JAR.x, y: 0.82 };
const CANAL = { x0: -1.0, x1: 3.9, y: -1.0, scale: 3 * (4.9 / 20) }; // radii x3; 20 cm over 4.9 units
const BENCH_Y = -2.3;
const N = state[L.ORAL_N] | 0 || 28;
const SEG = 28;

function circle(r, n = 64, from = 0, to = Math.PI * 2) {
  const pts = [];
  for (let i = 0; i <= n; i++) { const a = from + ((to - from) * i) / n; pts.push(new THREE.Vector3(r * Math.cos(a), r * Math.sin(a), 0)); }
  return new THREE.BufferGeometry().setFromPoints(pts);
}
function line(points, m = mat.ink) { return new THREE.Line(new THREE.BufferGeometry().setFromPoints(points.map(([x, y, z = 0]) => new THREE.Vector3(x, y, z))), m); }
function add(o, x = 0, y = 0, z = 0) { o.position.set(x, y, z); scene.add(o); return o; }

// The bench.
add(new THREE.Mesh(new THREE.BoxGeometry(10.6, 0.16, 1.6), mat.wood), -0.55, BENCH_Y, 0);
add(line([[-5.85, BENCH_Y + 0.08, 0.8], [4.75, BENCH_Y + 0.08, 0.8]]));
// A. The digester: a flask on a brass stand, a funnel in its neck.
add(new THREE.Mesh(new THREE.SphereGeometry(FLASK.r, 32, 20), mat.glass), FLASK.x, FLASK.y, 0);
add(new THREE.Line(circle(FLASK.r), mat.ink), FLASK.x, FLASK.y, 0.01);
add(new THREE.Mesh(new THREE.CylinderGeometry(0.17, 0.17, 0.55, 20, 1, true), mat.glass), FLASK.x, FLASK.y + FLASK.r + 0.2, 0);
add(line([[-0.17, 0.02], [-0.17, 0.5]]), FLASK.x, FLASK.y + FLASK.r - 0.05, 0.01);
add(line([[0.17, 0.02], [0.17, 0.5]]), FLASK.x, FLASK.y + FLASK.r - 0.05, 0.01);
add(new THREE.Mesh(new THREE.ConeGeometry(0.42, 0.45, 24, 1, true), mat.brass), FLASK.x, FLASK.y + FLASK.r + 0.62, 0).rotation.x = Math.PI;
add(new THREE.Mesh(new THREE.CylinderGeometry(0.06, 0.06, FLASK.y - FLASK.r - BENCH_Y, 8), mat.brass), FLASK.x, (FLASK.y - FLASK.r + BENCH_Y) / 2, 0);
add(new THREE.Mesh(new THREE.TorusGeometry(0.34, 0.035, 8, 32), mat.brass), FLASK.x, FLASK.y - FLASK.r + 0.12, 0).rotation.x = Math.PI / 2;
// The gas line, digester to reservoir.
const gasCurve = new THREE.CatmullRomCurve3([
  new THREE.Vector3(FLASK.x + FLASK.r * 0.8, FLASK.y + 0.25, 0),
  new THREE.Vector3(FLASK.x + 1.0, FLASK.y + 0.3, 0),
  new THREE.Vector3(JAR.x - 0.55, JAR.y0 - 0.55, 0),
  new THREE.Vector3(JAR.x - 0.3, JAR.y0 + 0.05, 0),
  new THREE.Vector3(BLADDER.x - 0.1, BLADDER.y - 0.6, 0),
]);
add(new THREE.Mesh(new THREE.TubeGeometry(gasCurve, 48, 0.055, 8, false), mat.brass));
// B. The reservoir: a rubber bladder in a bell jar on a brass plate.
add(new THREE.Mesh(new THREE.CylinderGeometry(JAR.r + 0.18, JAR.r + 0.25, 0.16, 40), mat.brass), JAR.x, JAR.y0 - 0.08, 0);
add(new THREE.Mesh(new THREE.CylinderGeometry(0.14, 0.2, JAR.y0 - BENCH_Y - 0.16, 12), mat.brass), JAR.x, (JAR.y0 + BENCH_Y) / 2 - 0.04, 0);
add(new THREE.Mesh(new THREE.CylinderGeometry(JAR.r, JAR.r, JAR.y1 - JAR.y0, 40, 1, true), mat.glass), JAR.x, (JAR.y0 + JAR.y1) / 2, 0);
add(new THREE.Mesh(new THREE.SphereGeometry(JAR.r, 40, 16, 0, Math.PI * 2, 0, Math.PI / 2), mat.glass), JAR.x, JAR.y1, 0);
add(line([[-JAR.r, JAR.y0], [-JAR.r, JAR.y1]]), JAR.x, 0, 0.01);
add(line([[JAR.r, JAR.y0], [JAR.r, JAR.y1]]), JAR.x, 0, 0.01);
add(new THREE.Line(circle(JAR.r, 48, 0, Math.PI), mat.ink), JAR.x, JAR.y1, 0.01);
add(new THREE.Mesh(new THREE.SphereGeometry(0.1, 12, 8), mat.brass), JAR.x, JAR.y1 + JAR.r + 0.06, 0);
const bladder = add(new THREE.Mesh(new THREE.SphereGeometry(1, 40, 28), mat.rubber), BLADDER.x, BLADDER.y, 0);
// Its outline is drawn over it (no depth test), in its own plane.
const bladderLine = add(new THREE.Line(circle(1), new THREE.LineBasicMaterial({ color: INK, depthTest: false })), BLADDER.x, BLADDER.y, 0);
bladderLine.renderOrder = 5;
// The gut: reservoir to valve.
const gutCurve = new THREE.CatmullRomCurve3([
  new THREE.Vector3(JAR.x + 0.45, JAR.y0 - 0.1, 0),
  new THREE.Vector3(JAR.x + 0.75, CANAL.y + 0.05, 0),
  new THREE.Vector3(CANAL.x0 - 0.75, CANAL.y, 0),
  new THREE.Vector3(CANAL.x0 - 0.22, CANAL.y, 0),
]);
add(new THREE.Mesh(new THREE.TubeGeometry(gutCurve, 40, 0.16, 14, false), mat.brass));
for (const x of [CANAL.x0 - 0.3, CANAL.x0 + 0.02]) add(new THREE.Mesh(new THREE.TorusGeometry(0.42, 0.06, 10, 36), mat.brass), x, CANAL.y, 0).rotation.y = Math.PI / 2;
// C. The valve: two reeds of one mass, apart by the World's half-gap.
const reedGeo = new THREE.BoxGeometry(0.12, 0.42, 0.72);
const reedEdges = new THREE.EdgesGeometry(reedGeo);
const reeds = [0, 1].map(() => {
  const m = add(new THREE.Mesh(reedGeo, mat.brass));
  m.add(new THREE.LineSegments(reedEdges, mat.ink));
  return m;
});
// D. The canal: a glass horn read from the World's own areas, tinted by its
// pressure RMS; its engraved outline is the area function.
const canalGeo = new THREE.BufferGeometry();
const canalPos = new Float32Array(N * SEG * 3);
const canalCol = new Float32Array(N * SEG * 3);
const idx = [];
for (let i = 0; i < N - 1; i++) {
  for (let j = 0; j < SEG; j++) {
    const a = i * SEG + j, b = i * SEG + ((j + 1) % SEG), c = (i + 1) * SEG + j, d = (i + 1) * SEG + ((j + 1) % SEG);
    idx.push(a, c, b, b, c, d);
  }
}
canalGeo.setIndex(idx);
canalGeo.setAttribute('position', new THREE.BufferAttribute(canalPos, 3));
canalGeo.setAttribute('color', new THREE.BufferAttribute(canalCol, 3));
add(new THREE.Mesh(canalGeo, new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.15, transparent: true, opacity: 0.42, side: THREE.DoubleSide, depthWrite: false })));
const outlinePos = new Float32Array(N * 2 * 3);
const outlineGeo = new THREE.BufferGeometry();
outlineGeo.setAttribute('position', new THREE.BufferAttribute(outlinePos, 3));
outlineGeo.setIndex([...Array(N - 1).keys()].flatMap((i) => [i, i + 1, N + i, N + i + 1]));
add(new THREE.LineSegments(outlineGeo, mat.ink));
const ringPos = new Float32Array(N * SEG * 2 * 3);
const ringGeo = new THREE.BufferGeometry();
ringGeo.setAttribute('position', new THREE.BufferAttribute(ringPos, 3));
add(new THREE.LineSegments(ringGeo, mat.inkFaint));
// The stand.
for (const x of [0.2, 3.1]) {
  add(new THREE.Mesh(new THREE.BoxGeometry(0.16, CANAL.y - BENCH_Y - 0.25, 0.5), mat.wood), x, (CANAL.y + BENCH_Y) / 2 - 0.12, 0);
}
const glassTint = new THREE.Color(0xdfe8e2), warm = new THREE.Color(0xd8482b), tmp = new THREE.Color();
const areaSmooth = new Float32Array(N).fill(1);
function radiusAt(i) { return Math.max(0.03, Math.sqrt(areaSmooth[i] / Math.PI) * CANAL.scale); }
function updateCanal(dt) {
  const k = 1 - Math.exp(-dt / 0.03);
  for (let i = 0; i < N; i++) {
    const a = Math.max(0, state[L.AREAS + i]) || 0;
    areaSmooth[i] = Number.isFinite(areaSmooth[i]) ? areaSmooth[i] + (a - areaSmooth[i]) * k : a;
    const r = radiusAt(i);
    const x = CANAL.x0 + (CANAL.x1 - CANAL.x0) * (i / (N - 1));
    tmp.copy(glassTint).lerp(warm, Math.min(1, Math.max(0, Math.log10(1 + state[L.PRMS + i]) / 3.3)));
    for (let j = 0; j < SEG; j++) {
      const th = (j / SEG) * Math.PI * 2, th2 = ((j + 1) / SEG) * Math.PI * 2;
      const o = (i * SEG + j) * 3;
      canalPos[o] = x; canalPos[o + 1] = CANAL.y + r * Math.cos(th); canalPos[o + 2] = r * Math.sin(th);
      canalCol[o] = tmp.r; canalCol[o + 1] = tmp.g; canalCol[o + 2] = tmp.b;
      const q = o * 2;
      ringPos[q] = x; ringPos[q + 1] = CANAL.y + r * Math.cos(th); ringPos[q + 2] = r * Math.sin(th);
      ringPos[q + 3] = x; ringPos[q + 4] = CANAL.y + r * Math.cos(th2); ringPos[q + 5] = r * Math.sin(th2);
    }
    outlinePos.set([x, CANAL.y + r, 0.02], i * 3);
    outlinePos.set([x, CANAL.y - r, 0.02], (N + i) * 3);
  }
  for (const g of [canalGeo, ringGeo, outlineGeo]) g.attributes.position.needsUpdate = true;
  canalGeo.attributes.color.needsUpdate = true;
  canalGeo.computeVertexNormals();
}

// Provisions: stand-ins, by kind.
function makeFood(def) {
  const g = new THREE.Group();
  const m = new THREE.MeshStandardMaterial({ color: def.color, roughness: 0.6 });
  const put = (geo, x = 0, y = 0, z = 0, sx = 1, sy = 1, sz = 1, mm = m) => { const mesh = new THREE.Mesh(geo, mm); mesh.position.set(x, y, z); mesh.scale.set(sx, sy, sz); g.add(mesh); return mesh; };
  const ball = new THREE.SphereGeometry(1, 14, 10);
  switch (def.id) {
    case 'beans': for (let i = 0; i < 5; i++) put(new THREE.CapsuleGeometry(0.05, 0.07, 4, 8), (Math.random() - 0.5) * 0.18, (Math.random() - 0.5) * 0.1, (Math.random() - 0.5) * 0.15).rotation.set(Math.random() * 3, 0, Math.random() * 3); break;
    case 'broccoli': put(new THREE.CylinderGeometry(0.035, 0.05, 0.18, 8), 0, -0.07, 0, 1, 1, 1, new THREE.MeshStandardMaterial({ color: 0x9cb872 })); for (const [x, z] of [[-0.06, 0], [0.06, 0], [0, 0.06], [0, -0.05]]) put(ball, x, 0.05, z, 0.07, 0.07, 0.07); break;
    case 'cabbage': put(ball, 0, 0, 0, 0.14, 0.13, 0.14); break;
    case 'onion': put(ball, 0, 0, 0, 0.11, 0.1, 0.11); put(new THREE.ConeGeometry(0.04, 0.09, 8), 0, 0.12, 0); break;
    case 'egg': put(ball, 0, 0, 0, 0.08, 0.11, 0.08); break;
    case 'soda': put(new THREE.CylinderGeometry(0.075, 0.075, 0.2, 14)); break;
    case 'cheese': put(new THREE.CylinderGeometry(0.14, 0.14, 0.08, 3, 1, false, 0, Math.PI / 2.5)); break;
    default: for (let i = 0; i < 3; i++) put(ball, (Math.random() - 0.5) * 0.12, 0, (Math.random() - 0.5) * 0.12, 0.08, 0.02, 0.07).rotation.set(Math.random(), Math.random(), Math.random()); break;
  }
  return g;
}

// Bubbles: in the digester, then along the gas line. Gas: out of the canal.
function softTexture() {
  const c = document.createElement('canvas');
  c.width = c.height = 64;
  const x = c.getContext('2d');
  const grad = x.createRadialGradient(32, 32, 0, 32, 32, 32);
  grad.addColorStop(0, 'rgba(255,255,255,1)');
  grad.addColorStop(0.55, 'rgba(255,255,255,0.45)');
  grad.addColorStop(1, 'rgba(255,255,255,0)');
  x.fillStyle = grad;
  x.fillRect(0, 0, 64, 64);
  return new THREE.CanvasTexture(c);
}
const soft = softTexture();
const bubbleMat = new THREE.SpriteMaterial({ map: soft, color: 0x4f8a72, transparent: true, depthWrite: false, opacity: 0.75 });
const bubbles = Array.from({ length: 60 }, () => { const s = new THREE.Sprite(bubbleMat); s.visible = false; s.userData = { stage: 0, u: 0 }; scene.add(s); return s; });
const puffs = Array.from({ length: 120 }, () => {
  const s = new THREE.Sprite(new THREE.SpriteMaterial({ map: soft, color: 0x4f8a72, transparent: true, depthWrite: false, opacity: 0 }));
  s.visible = false; s.userData = { life: 0, v: new THREE.Vector3() }; scene.add(s); return s;
});
let bubbleCarry = 0, puffCarry = 0;
const spawn = (pool) => pool.find((s) => !s.visible);

// Engraved labels, placed by projecting their anchors.
const LABELS = [
  { k: 'A', text: 'Digester', at: () => new THREE.Vector3(FLASK.x, FLASK.y + FLASK.r + 1.12, 0) },
  { k: 'B', text: 'Reservoir', at: () => new THREE.Vector3(JAR.x, JAR.y1 + JAR.r + 0.38, 0) },
  { k: 'C', text: 'Valve', at: () => new THREE.Vector3(CANAL.x0 - 0.15, CANAL.y + 0.95, 0) },
  { k: 'D', text: 'Canal, 28 sections', at: () => new THREE.Vector3((CANAL.x0 + CANAL.x1) / 2 - 0.4, CANAL.y + 1.25, 0) },
  { k: 'E', text: 'Cheeks', at: () => new THREE.Vector3(CANAL.x1 + 0.1, CANAL.y + radiusAt(N - 1) + 0.35, 0) },
];
for (const l of LABELS) { l.el = document.createElement('span'); l.el.innerHTML = `<b>${l.k}.</b>${l.text}`; $('labels').append(l.el); }
const v3 = new THREE.Vector3();
function placeLabels() {
  const w = stage.clientWidth, h = stage.clientHeight;
  for (const l of LABELS) {
    v3.copy(l.at()).project(camera);
    l.el.style.left = `${((v3.x + 1) / 2) * w}px`;
    l.el.style.top = `${((1 - v3.y) / 2) * h}px`;
  }
}

// Camera: the whole apparatus below the exercise lane (the top LANE_FRAC).
const LANE_FRAC = 0.26;
const camBase = new THREE.Vector3();
const camLook = new THREE.Vector3();
function resize() {
  const w = stage.clientWidth, h = stage.clientHeight;
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  const tan = Math.tan(THREE.MathUtils.degToRad(camera.fov / 2));
  const span = 11.4; // the bench and the gas leaving the canal
  const tall = 6.4 / (1 - LANE_FRAC); // bench to dome, plus the lane
  const dist = Math.max(span / (2 * tan * camera.aspect), tall / (2 * tan), 9);
  const visH = 2 * dist * tan;
  const top = JAR.y1 + JAR.r + 0.7 + LANE_FRAC * visH;
  camLook.set(-0.3, top - visH / 2, 0);
  camBase.set(-0.3, camLook.y + 0.4, dist);
  camera.position.copy(camBase);
  camera.lookAt(camLook);
  camera.updateProjectionMatrix();
  resizeLane();
}

// ------------------------------------------------------------------ lane --
const laneCanvas = $('lane');
const lane = { ctx: laneCanvas.getContext('2d'), w: 1, h: 1, dpr: 1, font: getComputedStyle(document.body).fontFamily };
function resizeLane() {
  lane.dpr = Math.min(devicePixelRatio, 2);
  lane.w = stage.clientWidth; lane.h = stage.clientHeight;
  laneCanvas.width = lane.w * lane.dpr; laneCanvas.height = lane.h * lane.dpr;
}
const trail = []; // { t, f0, cmd } seconds on the performance clock
let exercise = null; // { tune, notes, start, lo, hi, done }
const NOTE_NAMES = ['C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B'];
const midiOf = (hz) => 69 + 12 * Math.log2(hz / 440);
function drawLane(nowS) {
  const { ctx, w, dpr } = lane;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, lane.h);
  if (!audio) return;
  const top = 10, bottom = lane.h * LANE_FRAC - 6, hgt = bottom - top;
  const [lo, hi] = exercise ? [exercise.lo, exercise.hi] : [38, 300];
  const y = (hz) => top + (1 - (Math.log2(hz) - Math.log2(lo)) / (Math.log2(hi) - Math.log2(lo))) * hgt;
  const nowX = w * 0.3, pps = Math.max(90, w * 0.15);
  // Staff: one rule per semitone, named on the naturals of the tune (or C, E, G).
  ctx.font = `11px ${lane.font}`;
  for (let m = Math.ceil(midiOf(lo)); m <= Math.floor(midiOf(hi)); m++) {
    const yy = y(440 * Math.pow(2, (m - 69) / 12));
    const name = NOTE_NAMES[m % 12];
    const named = exercise ? exercise.notes.some((n) => Math.round(midiOf(n.hz)) === m) : ['C', 'E', 'G'].includes(name);
    ctx.strokeStyle = named ? 'rgba(26,23,20,0.28)' : 'rgba(26,23,20,0.07)';
    ctx.lineWidth = 1;
    ctx.beginPath(); ctx.moveTo(34, yy); ctx.lineTo(w - 8, yy); ctx.stroke();
    if (named) { ctx.fillStyle = '#4a4238'; ctx.fillText(`${name}${Math.floor(m / 12) - 1}`, 6, yy + 4); }
  }
  const xAt = (t) => nowX + (t - nowS) * pps;
  if (exercise) {
    const t = nowS - exercise.start;
    for (const n of exercise.notes) {
      const x0 = xAt(exercise.start + n.t0), x1 = xAt(exercise.start + n.t1) - 3;
      if (x1 < 0 || x0 > w) continue;
      const active = t >= n.t0 && t < n.t1;
      const score = n.total > 0 ? n.hit / n.total : 0;
      ctx.fillStyle = active ? (exercise.hitting ? '#4f8a72' : '#b8893a') : n.total > 0 ? `rgba(79,138,114,${0.25 + 0.6 * score})` : 'rgba(184,137,58,0.55)';
      ctx.strokeStyle = '#1a1714';
      ctx.lineWidth = 1.2;
      ctx.beginPath(); ctx.roundRect(x0, y(n.hz) - 6, Math.max(4, x1 - x0), 12, 3); ctx.fill(); ctx.stroke();
    }
    if (t < 0) {
      ctx.fillStyle = '#6e1f22';
      ctx.font = `italic 40px ${lane.font}`;
      ctx.textAlign = 'center';
      ctx.fillText(String(Math.ceil(-t / (60 / exercise.tune.bpm))), nowX, top + hgt / 2 + 14);
      ctx.textAlign = 'left';
    }
  }
  // What was asked (dashed) and what the World measured (solid).
  for (const [key, style, width, dash] of [['cmd', 'rgba(26,23,20,0.35)', 1.2, [4, 4]], ['f0', '#d8482b', 2.4, []]]) {
    ctx.strokeStyle = style; ctx.lineWidth = width; ctx.setLineDash(dash);
    ctx.beginPath();
    let pen = false;
    for (const p of trail) {
      const v = p[key];
      if (!(v > 0) || v < lo * 0.7 || v > hi * 1.4) { pen = false; continue; }
      const xx = xAt(p.t), yy = y(v);
      if (pen) ctx.lineTo(xx, yy); else ctx.moveTo(xx, yy);
      pen = true;
    }
    ctx.stroke();
  }
  ctx.setLineDash([]);
  ctx.strokeStyle = '#1a1714'; ctx.lineWidth = 1;
  ctx.beginPath(); ctx.moveTo(nowX, top - 4); ctx.lineTo(nowX, bottom + 4); ctx.stroke();
  ctx.strokeStyle = 'rgba(26,23,20,0.5)';
  ctx.beginPath(); ctx.moveTo(34, bottom + 3); ctx.lineTo(w - 8, bottom + 3); ctx.stroke();
}
function startExercise(tune) {
  stopStyle();
  const notes = parseTune(tune);
  const hz = notes.map((n) => n.hz);
  const lead = 4 * (60 / tune.bpm);
  exercise = { tune, notes, start: performance.now() / 1000 + lead, lo: Math.min(...hz) / Math.pow(2, 5 / 12), hi: Math.max(...hz) * Math.pow(2, 5 / 12), hitting: false };
  for (const b of $('tunes').children) b.setAttribute('aria-pressed', String(b.dataset.id === tune.id));
  hideReceipt();
  // A tune needs more gas than the reservoir holds: the house provides, and
  // digestion runs while you play (the tune voice's push, as fart::TUNE_PUSH).
  const served = eaten.length === 0;
  if (served) { for (const id of ['beans', 'beans', 'beans', 'beans', 'soda', 'soda']) eat(FOODS.find((f) => f.id === id)); }
  digesting = eaten.length > 0;
  $('digest').setAttribute('aria-pressed', String(digesting));
  setSlider('push', 0.7);
  toast(served ? `<b>${tune.name}</b>: the house provides four beans and two sodas. Hold, and follow the bars.` : `<b>${tune.name}</b>: hold, and follow the bars. Up is higher.`);
}
function exerciseTick(nowS) {
  if (!exercise) return;
  const t = nowS - exercise.start;
  const f0 = state[L.F0];
  exercise.hitting = false;
  for (const n of exercise.notes) {
    if (t >= n.t0 + 0.04 && t < n.t1 - 0.02) {
      n.total++;
      if (f0 > 0 && Math.abs(1200 * Math.log2(f0 / n.hz)) <= 50) { n.hit++; exercise.hitting = true; }
    }
  }
  if (t > exercise.notes[exercise.notes.length - 1].t1 + 0.6) finishExercise();
}
function finishExercise() {
  const ex = exercise;
  exercise = null;
  for (const b of $('tunes').children) b.setAttribute('aria-pressed', 'false');
  const scores = ex.notes.map((n) => (n.total > 0 ? n.hit / n.total : 0));
  const score = Math.round((100 * scores.reduce((a, b) => a + b, 0)) / scores.length);
  const grade = score >= 90 ? 'A' : score >= 75 ? 'B' : score >= 60 ? 'C' : score >= 40 ? 'D' : 'F';
  const bests = store.get('bests', {});
  const isBest = score > (bests[ex.tune.id] ?? -1);
  if (isBest) { bests[ex.tune.id] = score; store.set('bests', bests); }
  showReceipt(`${ex.tune.name}`, [
    `<div class="grade">${grade}</div>`,
    row('score (invented)', `${score}%`),
    row('notes within 50 cents', `${scores.filter((s) => s >= 0.5).length}/${scores.length}`),
    isBest ? row('', 'new best') : row('best', `${bests[ex.tune.id]}%`),
    '<div class="sep"></div>',
    row('scored on', 'measured F0'),
  ]);
  if (score >= 90) achieve(`tune-${ex.tune.id}`, `${ex.tune.name}, A`, `${ex.tune.by}, on the other end`);
  renderBests();
}
for (const tune of TUNES) {
  const b = document.createElement('button');
  b.dataset.id = tune.id;
  b.innerHTML = `<span>${tune.name}</span><small>${tune.by}</small>`;
  b.addEventListener('click', () => { if (audio) startExercise(tune); });
  $('tunes').append(b);
}
function renderBests() {
  const bests = store.get('bests', {});
  const eff = store.get('bestEfficiency', null), sus = store.get('bestSustain', null);
  const parts = TUNES.filter((t) => bests[t.id] !== undefined).map((t) => `${t.name} ${bests[t.id]}%`);
  if (eff !== null) parts.push(`efficiency ${eff.toFixed(2)}%`);
  if (sus !== null) parts.push(`longest ${sus.toFixed(2)} s`);
  $('best').textContent = parts.length ? `Bests: ${parts.join(' · ')}` : 'No bests yet.';
}
renderBests();

// --------------------------------------------- receipts, toasts, auditor --
const row = (k, v) => `<div class="row"><span>${k}</span><span>${v}</span></div>`;
let receiptNo = store.get('receiptNo', 0);
let receiptTimer = 0;
function showReceipt(title, rows, stamp = true) {
  receiptNo++;
  store.set('receiptNo', receiptNo);
  const res = state[L.MAX_RES];
  const closed = Math.abs(res) < 1e-12 && Math.abs(state[L.CUM_RES]) < 1e-9;
  $('receipt').innerHTML = `<h3>${title}</h3>${rows.join('')}${row('receipt no.', String(receiptNo).padStart(5, '0'))}`
    + (stamp ? `<div class="stamp ${closed ? '' : 'bad'}">${closed ? 'BOOKS CLOSED' : 'AUDIT FLAGGED'} ${res.toExponential(1)} J</div>` : '');
  $('receipt').classList.add('on');
  clearTimeout(receiptTimer);
  receiptTimer = setTimeout(hideReceipt, 9000);
  if (closed) achieve('first-law', 'First Law', 'the books closed on a fart');
}
function hideReceipt() { $('receipt').classList.remove('on'); }
$('receipt').addEventListener('click', hideReceipt);
let toastTimer = 0;
function toast(html) {
  $('toast').innerHTML = html;
  $('toast').classList.add('on');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => $('toast').classList.remove('on'), 2800);
}
const achieved = new Set(store.get('achieved', []));
function achieve(id, name, why) {
  if (achieved.has(id)) return;
  achieved.add(id);
  store.set('achieved', [...achieved]);
  toast(`<b>${name}</b>: ${why}`);
}

// One fart, from the first push to the silence after it: a receipt.
let ep = null;
let quietFor = 0;
let donut = 0;
function episodeTick(dt) {
  const breathing = state[L.CUR_BREATH] > 0.02;
  const gas = state[L.LUNG_FRACTION], f0 = state[L.F0];
  if (!ep && breathing) {
    ep = { t: 0, f0s: [], maxRe: 0, sag: false, saggedOut: false, emptied: false, style: style?.def.name ?? null,
      at: [L.RAD_LIPS, L.JET, L.FOLD_LOSS, L.WALL, L.CLOSS, L.TURB].map((i) => state[i]) };
  }
  if (f0 > 0 && Math.abs(f0 - 100) <= 1) { donut += dt; if (donut >= 1) achieve('donut', 'Hertz Donut', '100 Hz, held for a second'); } else donut = 0;
  if (state[L.MAX_RE] > 6000) achieve('reynolds', 'Reynolds 6000', 'fully turbulent, as booked');
  if (state[L.CONS + 3] > 0.97) achieve('seal', 'Exact Seal', 'the cheeks closed to zero aperture');
  if (!ep) return;
  ep.t += dt;
  if (exercise) ep.exercise = true;
  if (f0 > 0) ep.f0s.push(f0);
  ep.maxRe = Math.max(ep.maxRe, state[L.MAX_RE]);
  if (breathing && gas < SAG) ep.sag = true;
  if (breathing && gas < SAG && f0 === 0 && ep.f0s.length > 10) ep.saggedOut = true;
  if (breathing && gas < 0.02) ep.emptied = true;
  quietFor = breathing ? 0 : quietFor + dt;
  if (quietFor > 0.35) { finishEpisode(ep); ep = null; quietFor = 0; }
}
function finishEpisode(e) {
  const d = [L.RAD_LIPS, L.JET, L.FOLD_LOSS, L.WALL, L.CLOSS, L.TURB].map((i, k) => state[i] - e.at[k]);
  const [rad, jet, fold, wall, closs, turb] = d;
  const spent = rad + jet + fold + wall + closs;
  const dur = e.t - 0.35;
  if (dur < 0.25 || exercise || e.exercise) return;
  const f0s = e.f0s.slice().sort((a, b) => a - b);
  const median = f0s.length ? f0s[f0s.length >> 1] : 0;
  const eff = spent > 0 ? (100 * rad) / spent : 0;
  const ended = e.emptied ? 'an empty reservoir' : e.saggedOut ? 'the sag law' : e.style ? `the end of ${e.style}` : 'you let go';
  if (e.saggedOut || (e.sag && e.emptied)) achieve('below-sag', 'Below Sag', 'ended by the valve’s own law');
  if (dur >= 1 && f0s.length > 20 && e.maxRe < 3000) achieve('laminar', 'Laminar', 'a whole one without turbulence');
  const bestEff = store.get('bestEfficiency', null), bestSus = store.get('bestSustain', null);
  const newEff = f0s.length > 10 && (bestEff === null || eff > bestEff);
  const newSus = f0s.length > 10 && (bestSus === null || dur > bestSus);
  if (newEff) store.set('bestEfficiency', eff);
  if (newSus) store.set('bestSustain', dur);
  renderBests();
  showReceipt(e.style ? `Receipt: ${e.style}` : 'Receipt', [
    row('duration', `${dur.toFixed(2)} s${newSus ? ' ★' : ''}`),
    row('median pitch', median ? `${median.toFixed(0)} Hz` : 'no swing'),
    row('gas spent', fmtJ(spent)),
    row('radiated as sound', fmtJ(rad)),
    row('efficiency', `${eff.toFixed(2)}%${newEff ? ' ★' : ''}`),
    row('lost at the jet', fmtJ(jet)),
    row('turbulence', fmtJ(turb)),
    row('peak Reynolds', e.maxRe.toFixed(0)),
    '<div class="sep"></div>',
    row('ended by', ended),
  ]);
}

let auditorAt = 0, auditorIdle = 0;
function auditor(nowMs) {
  if (nowMs - auditorAt < 1600) return;
  auditorAt = nowMs;
  const breathing = state[L.CUR_BREATH] > 0.02, gas = state[L.LUNG_FRACTION], f0 = state[L.F0], re = state[L.MAX_RE];
  let line;
  if (!audio) line = 'Awaiting the demonstration.';
  else if (state[L.NONFINITE] > 0) line = `The World was rebuilt ${state[L.NONFINITE] | 0} times. We do not discuss it.`;
  else if (Math.abs(state[L.MAX_RES]) > 1e-12) line = 'AUDIT FLAGGED.';
  else if (breathing && gas < 0.02) line = 'Nothing left to push. The books note it.';
  else if (breathing && gas < SAG) line = 'Sag law in effect. Nothing personal.';
  else if (breathing && f0 > 0) line = re < 3000 ? 'Laminar. Disappointing.' : re < 6000 ? 'Transitional. Keep going.' : 'Fully turbulent. Noted.';
  else if (breathing) line = 'Flow without a swing. Hiss is still work.';
  else if (digestWatts > 0) line = `Digestion in progress: ${digestWatts.toFixed(2)} W, booked as source work.`;
  else if (full && digesting) line = 'Reservoir at capacity. Digestion suspended.';
  else {
    const idle = [
      `The books are closed at ${state[L.MAX_RES].toExponential(1)} J.`,
      'Smell: not modeled. Do not ask.',
      `Radiated so far: ${fmtJ(state[L.RAD_LIPS])}. The rest was heat.`,
      gas < 0.05 ? 'The reservoir is empty. Provisions are in section I.' : 'The reservoir is ready.',
    ];
    line = idle[auditorIdle++ % idle.length];
  }
  if ($('auditor').textContent !== line) $('auditor').textContent = line;
}

// ---------------------------------------------------------------- gauges --
function polar(a, r) { const t = (a * Math.PI) / 180; return [50 + r * Math.cos(t), 52 - r * Math.sin(t)]; }
function arc(a0, a1, r) { const [x0, y0] = polar(a0, r), [x1, y1] = polar(a1, r); return `M${x0.toFixed(2)} ${y0.toFixed(2)} A${r} ${r} 0 ${Math.abs(a1 - a0) > 180 ? 1 : 0} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`; }
const A0 = 205, A1 = -25; // sweep, degrees
function makeGauge(el, { label, ticks = 5, zone = null, needle = true, plate = '' }) {
  const ang = (f) => A0 + (A1 - A0) * f;
  let svg = `<svg viewBox="0 0 100 66" role="img" aria-label="${label}"><circle cx="50" cy="52" r="46" fill="#f6efdf" stroke="#1a1714" stroke-width="1.2"/>`;
  svg += `<path d="${arc(A0, A1, 38)}" fill="none" stroke="#1a1714" stroke-width="0.8"/>`;
  if (zone) svg += `<path d="${arc(ang(zone[0]), ang(zone[1]), 36)}" fill="none" stroke="#d8482b" stroke-width="4"/>`;
  for (let i = 0; i <= ticks * 2; i++) { const [x0, y0] = polar(ang(i / (ticks * 2)), 38), [x1, y1] = polar(ang(i / (ticks * 2)), i % 2 ? 34 : 31); svg += `<line x1="${x0}" y1="${y0}" x2="${x1}" y2="${y1}" stroke="#1a1714" stroke-width="${i % 2 ? 0.6 : 1}"/>`; }
  if (plate) svg += `<text x="50" y="40" text-anchor="middle" font-size="7.5" font-family="serif" font-style="italic" fill="#6e1f22">${plate}</text>`;
  if (needle) svg += `<line class="needle" x1="50" y1="52" x2="50" y2="20" stroke="#6e1f22" stroke-width="1.8" stroke-linecap="round" style="transform-origin:50px 52px"/>`;
  svg += `<circle cx="50" cy="52" r="3" fill="#b8893a" stroke="#1a1714" stroke-width="0.6"/></svg>`;
  el.innerHTML = `${svg}<div>${label}</div><div class="val">–</div>`;
  const n = el.querySelector('.needle'), val = el.querySelector('.val');
  let shown = 0;
  return {
    set(f, text) {
      shown += (Math.min(1, Math.max(0, f)) - shown) * (reduceMotion ? 1 : 0.25);
      if (n) n.style.transform = `rotate(${90 - ang(shown)}deg)`;
      val.textContent = text;
    },
  };
}
const gRes = makeGauge($('g-res'), { label: 'reservoir', zone: [0, SAG], plate: 'sag' });
const gPa = makeGauge($('g-pa'), { label: 'push, Pa' });
makeGauge($('g-nose'), { label: 'olfactory', needle: false, plate: 'not modeled' });
$('g-nose').querySelector('.val').textContent = 'no such field';
$('g-nose').title = 'The World books joules. It has no nose.';

// ----------------------------------------------------------- books, cycle --
let booksAt = 0;
const fmtJ = (j) => (Math.abs(j) >= 0.1 ? `${j.toFixed(3)} J` : Math.abs(j) >= 1e-4 ? `${(j * 1e3).toFixed(2)} mJ` : j === 0 ? '0 J' : `${(j * 1e6).toFixed(2)} µJ`);
function books(nowMs) {
  if (nowMs - booksAt < 150) return;
  booksAt = nowMs;
  const rad = state[L.RAD_LIPS];
  const spent = state[L.JET] + state[L.FOLD_LOSS] + state[L.WALL] + state[L.CLOSS] + rad;
  const rows = [
    ['provisions paid (source work)', fmtJ(state[L.MUSCLE])],
    ['gas held now', fmtJ(state[L.LUNG_E])],
    ['radiated as sound', fmtJ(rad)],
    ['sound, of everything spent', spent > 0 ? `${((100 * rad) / spent).toFixed(2)}%` : '–'],
    ['lost at the jet', fmtJ(state[L.JET])],
    ['lost at the narrowest point', fmtJ(state[L.CLOSS])],
    ['valve tissue', fmtJ(state[L.FOLD_LOSS])],
    ['walls', fmtJ(state[L.WALL])],
    ['turbulence injected', fmtJ(state[L.TURB])],
    ['tension retunes', fmtJ(state[L.FOLD_CTRL])],
    ['cheek moves', fmtJ(state[L.TRACT_CTRL])],
    ['largest frame residual', `<span class="${Math.abs(state[L.MAX_RES]) < 1e-12 ? 'ok' : 'warn'}">${state[L.MAX_RES].toExponential(1)} J</span>`],
    ['cumulative residual', `<span class="${Math.abs(state[L.CUM_RES]) < 1e-9 ? 'ok' : 'warn'}">${state[L.CUM_RES].toExponential(1)} J</span>`],
    ['samples', String(state[L.FRAMES] | 0)],
  ];
  if (state[L.SAFETY] > 0) rows.push(['clipped samples', `<span class="warn">${state[L.SAFETY] | 0}</span>`]);
  $('books').innerHTML = rows.map(([k, v]) => `<span>${k}</span><span>${v}</span>`).join('');
  const cmd = state[L.CUR_PITCH], f0 = state[L.F0];
  $('cyc-note').textContent = f0 > 0 && cmd > 0
    ? `asked ${cmd.toFixed(0)} Hz, swinging at ${f0.toFixed(1)} Hz (${(1200 * Math.log2(f0 / cmd)).toFixed(0)} cents)`
    : 'the valve is still';
}
const cycleCanvas = $('cycle');
function drawCycle() {
  const c = cycleCanvas, dpr = Math.min(devicePixelRatio, 2);
  const w = c.clientWidth * dpr, h = c.clientHeight * dpr;
  if (c.width !== w || c.height !== h) { c.width = w; c.height = h; }
  const x = c.getContext('2d');
  x.clearRect(0, 0, w, h);
  let hi = 0.05;
  for (let k = 0; k < 32; k++) hi = Math.max(hi, state[L.CYCLE + k]);
  x.strokeStyle = state[L.F0] > 0 ? '#6e1f22' : '#8a7d69';
  x.lineWidth = 1.8 * dpr;
  x.beginPath();
  for (let k = 0; k < 32; k++) {
    const px = (k / 31) * (w - 8 * dpr) + 4 * dpr;
    const py = h - 6 * dpr - (Math.max(0, state[L.CYCLE + k]) / hi) * (h - 16 * dpr);
    if (k === 0) x.moveTo(px, py); else x.lineTo(px, py);
  }
  x.stroke();
  x.fillStyle = '#4a4238';
  x.font = `italic ${11 * dpr}px Georgia, serif`;
  x.fillText(`half-gap, peak ${hi.toFixed(2)} mm`, 6 * dpr, 13 * dpr);
}

// ---------------------------------------------------------------- status --
function status() {
  const gas = state[L.LUNG_FRACTION], breathing = state[L.CUR_BREATH] > 0.02;
  let s;
  if (!audio) s = 'Begin the demonstration.';
  else if (exercise) s = performance.now() / 1000 < exercise.start ? `${exercise.tune.name}, ${exercise.tune.by}: get ready to hold` : exercise.hitting ? 'in tune' : 'follow the bar: up is higher';
  else if (breathing && gas < 0.02) s = 'empty: nothing left to push';
  else if (breathing && state[L.F0] > 0) s = `the valve swings at ${state[L.F0].toFixed(0)} Hz`;
  else if (breathing) s = 'flow without a swing: hiss';
  else if (eaten.length && !digesting) s = 'provisions in the digester: press digest';
  else s = gas < 0.05 ? 'the reservoir is empty: provisions, then digest' : 'hold anywhere on the plate to release';
  if ($('status').textContent !== s) $('status').textContent = s;
}

// ------------------------------------------------------------------ loop --
new ResizeObserver(resize).observe(stage);
resize();
let last = performance.now();
let lastFrames = state[L.FRAMES];
let lastRad = state[L.RAD_LIPS];
let power = 0;
let gapShown = 0.1;
function frame(now) {
  // The first frame's timestamp can precede `last`.
  const dt = Math.min(0.05, Math.max(0, (now - last) / 1000));
  last = now;
  const nowS = now / 1000;
  if (audio) { digestTick(dt); exerciseTick(nowS); episodeTick(dt); }
  // Radiated power from the World's own books (J per World second).
  const frames = state[L.FRAMES];
  if (frames > lastFrames) {
    const p = (state[L.RAD_LIPS] - lastRad) / ((frames - lastFrames) / 48000);
    power += (Math.max(0, p) - power) * 0.3;
    lastFrames = frames; lastRad = state[L.RAD_LIPS];
  }
  if (audio) {
    trail.push({ t: nowS, f0: state[L.F0], cmd: state[L.CUR_BREATH] > 0.02 ? state[L.CUR_PITCH] : 0 });
    while (trail.length && trail[0].t < nowS - 6) trail.shift();
  }
  // Shake with the radiated power (none under reduced motion).
  const kick = reduceMotion ? 0 : Math.min(0.12, 0.04 * Math.sqrt(power / 0.003));
  camera.position.set(camBase.x + kick * (Math.random() - 0.5), camBase.y + kick * (Math.random() - 0.5), camBase.z);
  camera.lookAt(camLook);
  const gas = state[L.LUNG_FRACTION];
  // The bladder swells with the gas the World holds (radius ~ cube root).
  const r = 0.32 + 0.68 * Math.cbrt(Math.max(0, gas));
  const wob = reduceMotion ? 0 : (digestWatts > 0 ? 0.025 : 0) * Math.sin(now / 90);
  bladder.scale.set(r * (1 + wob), r * (1 - wob), r);
  bladderLine.scale.set(r * (1 + wob), r * (1 - wob), 1);
  for (const f of eaten) {
    const target = new THREE.Vector3(FLASK.x + f.rest.x * FLASK.r, FLASK.y + f.rest.y * FLASK.r * 1.4, f.rest.z * FLASK.r);
    if (f.mesh.position.y > target.y + 0.01) {
      f.v -= 9 * dt;
      f.mesh.position.y = Math.max(target.y, f.mesh.position.y + f.v * dt);
      f.mesh.position.x += (target.x - f.mesh.position.x) * Math.min(1, dt * 6);
      f.mesh.rotation.x += f.spin * dt;
    } else {
      f.mesh.position.lerp(target, 1 - Math.exp(-dt * 6));
      f.v = 0;
    }
    f.mesh.scale.setScalar(0.35 + 0.65 * Math.max(0, f.remaining / f.def.gasJ));
  }
  // Bubbles: rise in the digester, then travel the gas line to the bladder.
  bubbleCarry += digestWatts * 30 * dt;
  while (bubbleCarry >= 1) {
    bubbleCarry -= 1;
    const b = spawn(bubbles);
    if (!b) break;
    b.visible = true;
    b.userData.stage = 0;
    b.userData.u = 0;
    b.position.set(FLASK.x + (Math.random() - 0.5) * FLASK.r, FLASK.y - FLASK.r * 0.5, (Math.random() - 0.5) * 0.4);
    b.scale.setScalar(0.05 + Math.random() * 0.05);
  }
  for (const b of bubbles) {
    if (!b.visible) continue;
    if (b.userData.stage === 0) {
      b.position.y += dt * 0.8;
      if (b.position.y > FLASK.y + 0.2) b.userData.stage = 1;
    } else {
      b.userData.u += dt * 0.55;
      if (b.userData.u >= 1) { b.visible = false; continue; }
      b.position.copy(gasCurve.getPoint(b.userData.u));
      b.position.z += 0.08;
    }
  }
  // The valve: its half-gap as the World reports it (mm), drawn x0.5.
  const gap = Math.max(0, state[L.FOLD_GAP_MM]);
  gapShown = reduceMotion ? gapShown + (gap - gapShown) * 0.1 : gap;
  const open = 0.01 + gapShown * 0.5;
  reeds[0].position.set(CANAL.x0 - 0.14, CANAL.y + 0.21 + open, 0);
  reeds[1].position.set(CANAL.x0 - 0.14, CANAL.y - 0.21 - open, 0);
  updateCanal(dt);
  // Gas out of the canal: as much as the flow.
  const flow = Math.max(0, state[L.GLOTTAL_FLOW]);
  puffCarry += (flow / 140) * dt * 60 * (reduceMotion ? 0.3 : 1);
  const mouthR = radiusAt(N - 1);
  while (puffCarry >= 1) {
    puffCarry -= 1;
    const p = spawn(puffs);
    if (!p) break;
    p.visible = true;
    p.userData.life = 1;
    p.position.set(CANAL.x1 + 0.05, CANAL.y + (Math.random() - 0.5) * mouthR, (Math.random() - 0.5) * mouthR);
    const speed = 0.8 + Math.min(2.2, flow / 300);
    p.userData.v.set(speed * (0.7 + Math.random() * 0.6), (Math.random() - 0.2) * 0.5, (Math.random() - 0.5) * 0.6);
    p.scale.setScalar(0.12);
  }
  for (const p of puffs) {
    if (!p.visible) continue;
    p.userData.life -= dt * 0.6;
    p.position.addScaledVector(p.userData.v, dt);
    p.userData.v.multiplyScalar(1 - dt * 0.9);
    p.userData.v.y += dt * 0.22;
    p.scale.setScalar(0.12 + (1 - p.userData.life) * 0.9);
    p.material.opacity = Math.max(0, p.userData.life) * 0.32;
    if (p.userData.life <= 0) p.visible = false;
  }
  renderer.render(scene, camera);
  placeLabels();
  drawLane(nowS);
  gRes.set(gas, `${Math.round(gas * 100)}%`);
  gPa.set(state[L.LUNG_PA] / SUPPLY_MAX_PA, `${state[L.LUNG_PA].toFixed(0)}`);
  books(now);
  drawCycle();
  auditor(now);
  status();
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
