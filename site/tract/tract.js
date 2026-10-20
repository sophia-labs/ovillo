// Ovillo Tract: a vocal tract made of an Ariadne World, after Neil Thapen's
// Pink Trombone (2017). The World runs in WebAssembly (ovillo-tract-wasm); the
// pictures read its state every frame and never write it, except through the
// same controls a player has (articulators, breath, orbs).

import * as THREE from './vendor/three/three.module.min.js';

// ---------------------------------------------------------------- layout --
// Mirrors ovillo-tract-wasm `layout`.
const L = {
  ORAL_N: 0, NASAL_N: 1, VELUM_SECTION: 2, FOLD_GAP_MM: 3, FOLD_GAP_MIN_MM: 4, FOLD_GAP_MAX_MM: 5,
  GLOTTAL_FLOW: 6, LUNG_PA: 7, LUNG_FRACTION: 8, F0: 9, TRACT_E: 10, FOLD_E: 11, LUNG_E: 12,
  MUSCLE: 13, IMPACT: 14, TRACT_CTRL: 15, FOLD_CTRL: 16, WALL: 17, JET: 18, FOLD_LOSS: 19,
  RAD_LIPS: 20, RAD_NOSE: 21, FRAMES: 22, MAX_RES: 23, GLOBAL_RES: 24, CUM_RES: 25,
  CUR_INDEX: 26, CUR_DIAM: 27, CUR_LIPS: 28, CUR_VELUM: 29, CUR_PITCH: 30, CUR_TENSE: 31, CUR_BREATH: 32,
  GESTURE: 33, SAFETY: 34, NONFINITE: 35, IMPACTS: 36, CYCLE: 40, AREAS: 72, PRMS: 136, NASAL: 200, LEN: 248,
};
// Tongue centre range along the tract (TractAnatomy::adult_v0).
const TONGUE_BACK = 0.24, TONGUE_FRONT = 0.80, VELUM_POS = 0.40;
const PARAM = { index: 0, diameter: 1, lips: 2, velum: 3, pitch: 4, tense: 5, breath: 6, asp: 7 };

const query = new URLSearchParams(location.search);
const MUTE = query.has('mute');
const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
const $ = (id) => document.getElementById(id);

// ---------------------------------------------------------------- engine --
const wasmBytes = await (await fetch('./tract.wasm')).arrayBuffer();
const mainModule = await WebAssembly.compile(wasmBytes);

let state = new Float32Array(L.LEN);
let dspLoad = null;
let wasmRtf = null;
let audio = null; // { ctx, node, gain }
const controls = { index: 0.5, diameter: 0.3, lips: 0.6, velum: 0.35, pitch: 120, tense: 0.6, breath: 0.55, asp: 0.15 };

// A silent World on the main thread drives the pictures before START (and
// measures the wasm real-time factor). After START the worklet's World does.
const silent = {
  x: null,
  carry: 0,
  async init() {
    const inst = await WebAssembly.instantiate(mainModule, {});
    this.x = inst.exports;
    this.x.tract_init(48000);
    this.bench();
    this.x.tract_init(48000);
    for (const [k, v] of Object.entries(controls)) this.x.tract_set(PARAM[k], v);
  },
  bench() {
    const x = this.x;
    x.tract_set(PARAM.breath, 0.6);
    x.tract_gesture(0);
    for (let i = 0; i < 40; i++) x.tract_process(128);
    // Best of three thirds of a second of audio (a busy machine preempts).
    const blocks = 125;
    let best = Infinity;
    for (let k = 0; k < 3; k++) {
      const t0 = performance.now();
      for (let i = 0; i < blocks; i++) x.tract_process(128);
      best = Math.min(best, performance.now() - t0);
    }
    wasmRtf = (blocks * 128 / 48) / best;
  },
  run(dtMs) {
    if (!this.x || audio) return;
    this.carry += (48000 * Math.min(dtMs, 60)) / 1000;
    let budget = Math.min(Math.floor(this.carry), 4096);
    this.carry -= budget;
    while (budget > 0) { const n = Math.min(budget, 512); this.x.tract_process(n); budget -= n; }
    this.x.tract_snapshot();
    state = new Float32Array(this.x.memory.buffer, this.x.tract_state_ptr(), L.LEN).slice();
  },
};

function send(msg) {
  if (audio) { audio.node.port.postMessage(msg); return; }
  const x = silent.x;
  if (!x) return;
  if (msg.type === 'set') x.tract_set(msg.param, msg.value);
  else if (msg.type === 'preset') x.tract_preset(msg.id);
  else if (msg.type === 'gesture') x.tract_gesture(msg.id);
  else if (msg.type === 'impact') x.tract_impact(msg.position, msg.energy);
}
function setControl(name, value) { controls[name] = value; send({ type: 'set', param: PARAM[name], value }); }

async function startAudio() {
  const ctx = new AudioContext({ latencyHint: 'interactive' });
  await ctx.audioWorklet.addModule('./worklet.js');
  const node = new AudioWorkletNode(ctx, 'tract', {
    numberOfInputs: 0, numberOfOutputs: 1, outputChannelCount: [2],
    // The World as the player has it now, applied before the first quantum.
    processorOptions: { bytes: wasmBytes.slice(0), init: [
      ...Object.entries(controls).map(([k, v]) => ({ type: 'set', param: PARAM[k], value: v })),
      ...(activeGesture !== null ? [{ type: 'gesture', id: activeGesture }] : []),
    ] },
  });
  const gain = ctx.createGain();
  gain.gain.value = 0;
  node.connect(gain).connect(ctx.destination);
  node.port.onmessage = (e) => {
    const m = e.data;
    if (m.type === 'state') { state = m.state; dspLoad = m.load; }
    else if (m.type === 'error') console.error('tract worklet:', m.message);
  };
  audio = { ctx, node, gain };
  applyMaster();
  await ctx.resume();
}
function applyMaster() {
  if (!audio) return;
  const v = MUTE ? 0 : Number($('s-master').value) ** 2;
  audio.gain.gain.setTargetAtTime(v, audio.ctx.currentTime, 0.05);
}

// ------------------------------------------------------- tract geometry --
// Scene units are centimetres. R(s) is the fixed outer wall (posterior
// pharyngeal wall, then palate); the airway hangs inward from it by the
// diameter the World's area gives, so the tongue side moves and the roof stays.
const LEN = 15.7, PH = 6.4, RB = 2.3, ARC = (Math.PI / 2) * RB;
function wall(s) {
  const l = s * LEN;
  if (l <= PH) return { x: 0, y: l, nx: -1, ny: 0, tx: 0, ty: 1 };
  if (l <= PH + ARC) {
    const th = (l - PH) / RB;
    return { x: RB - RB * Math.cos(th), y: PH + RB * Math.sin(th), nx: -Math.cos(th), ny: Math.sin(th), tx: Math.sin(th), ty: Math.cos(th) };
  }
  return { x: RB + (l - PH - ARC), y: PH + RB, nx: 0, ny: 1, tx: 1, ty: 0 };
}
const sample = (arr, base, n, s) => {
  const f = Math.min(Math.max(s * n - 0.5, 0), n - 1);
  const i = Math.floor(f), j = Math.min(i + 1, n - 1), t = f - i;
  return arr[base + i] * (1 - t) + arr[base + j] * t;
};
const diameter = (areaCm2) => 2 * Math.sqrt(Math.max(areaCm2, 0.025) / Math.PI);

const ROWS = 72, COLS = 18, NROWS = 40;
const T_PALATE = 1.0, T_TOP = 1.4, D_LOW = 4.9, SHELL_Z = 3.4;

function gridGeometry(rows, cols) {
  const g = new THREE.BufferGeometry();
  const pos = new Float32Array(rows * cols * 3);
  const uv = new Float32Array(rows * cols * 2);
  const idx = [];
  for (let r = 0; r < rows; r++) for (let c = 0; c < cols; c++) {
    uv[(r * cols + c) * 2] = (r / (rows - 1)) * 6;
    uv[(r * cols + c) * 2 + 1] = c / (cols - 1);
    if (r < rows - 1 && c < cols - 1) {
      const a = r * cols + c, b = a + 1, d = a + cols, e = d + 1;
      idx.push(a, d, b, b, d, e);
    }
  }
  g.setAttribute('position', new THREE.BufferAttribute(pos, 3));
  g.setAttribute('uv', new THREE.BufferAttribute(uv, 2));
  g.setIndex(idx);
  return g;
}

// A wet normal map: smooth value noise, drawn once.
function wetNormalMap() {
  const N = 256, h = new Float32Array(N * N);
  const rnd = (() => { let s = 1234567; return () => ((s = (s * 1103515245 + 12345) >>> 0) / 4294967296); })();
  const blobs = Array.from({ length: 90 }, () => [rnd() * N, rnd() * N, 6 + rnd() * 26, rnd() - 0.5]);
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) {
    let v = 0;
    for (const [bx, by, r, a] of blobs) {
      for (const ox of [-N, 0, N]) for (const oy of [-N, 0, N]) {
        const dx = x - bx - ox, dy = y - by - oy, d2 = dx * dx + dy * dy;
        if (d2 < 9 * r * r) v += a * Math.exp(-d2 / (r * r));
      }
    }
    h[y * N + x] = v;
  }
  const cv = document.createElement('canvas'); cv.width = cv.height = N;
  const cx = cv.getContext('2d'), img = cx.createImageData(N, N);
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) {
    const dx = h[y * N + ((x + 1) % N)] - h[y * N + ((x + N - 1) % N)];
    const dy = h[((y + 1) % N) * N + x] - h[((y + N - 1) % N) * N + x];
    let nx = -dx * 2.2, ny = -dy * 2.2, nz = 1; const m = Math.hypot(nx, ny, nz); nx /= m; ny /= m; nz /= m;
    const o = (y * N + x) * 4;
    img.data[o] = (nx * 0.5 + 0.5) * 255; img.data[o + 1] = (ny * 0.5 + 0.5) * 255; img.data[o + 2] = (nz * 0.5 + 0.5) * 255; img.data[o + 3] = 255;
  }
  cx.putImageData(img, 0, 0);
  const tex = new THREE.CanvasTexture(cv);
  tex.wrapS = tex.wrapT = THREE.RepeatWrapping;
  return tex;
}

// ------------------------------------------------------------- scene ------
const canvas = $('gl');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: false, powerPreference: 'high-performance' });
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.05;
renderer.localClippingEnabled = true;
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x120a0c);
scene.fog = new THREE.Fog(0x120a0c, 40, 90);
const camera = new THREE.PerspectiveCamera(34, 1, 0.1, 200);
const target = new THREE.Vector3(4.2, 5.6, -1.2);
const orbit = { az: 0.42, el: 0.2, r: 31 };
let fitScale = 1;

scene.add(new THREE.HemisphereLight(0xffe6ea, 0x2a0d14, 0.75));
const key = new THREE.DirectionalLight(0xfff1ee, 2.2); key.position.set(10, 18, 16); scene.add(key);
const rim = new THREE.DirectionalLight(0xff7f9a, 1.1); rim.position.set(-14, 6, -10); scene.add(rim);
const sheen = new THREE.PointLight(0xffffff, 60, 40, 1.6); scene.add(sheen);

const normalMap = wetNormalMap();
const clipFront = [new THREE.Plane(new THREE.Vector3(0, 0, -1), 0.02)];
const flesh = (color, rough, extra = {}) => new THREE.MeshPhysicalMaterial({
  color, roughness: rough, metalness: 0, clearcoat: 1, clearcoatRoughness: 0.16, sheen: 0.6, sheenColor: new THREE.Color(0xffc2cf), sheenRoughness: 0.45,
  normalMap, normalScale: new THREE.Vector2(0.32, 0.32), side: THREE.DoubleSide, ...extra,
});
const matShell = flesh(0xe57d90, 0.5);
const matPalate = flesh(0xf09aaa, 0.46);
const matTongue = flesh(0xd4506c, 0.32, { sheenColor: new THREE.Color(0xff9fb4), clearcoatRoughness: 0.1 });
const matMucosa = flesh(0xb83a55, 0.24, { clearcoatRoughness: 0.08 });
const matNasal = flesh(0xc4566d, 0.3);
const matLip = flesh(0xd9637b, 0.3, { clippingPlanes: clipFront });
const matFold = flesh(0xf6d3d8, 0.22, { clearcoatRoughness: 0.05 });
const matVelum = flesh(0xe07087, 0.34);

const organ = new THREE.Group();
scene.add(organ);
const geoGroove = gridGeometry(ROWS, COLS);
const geoUpperA = gridGeometry(ROWS, 6);
const geoUpperB = gridGeometry(ROWS, 6);
const geoLower = gridGeometry(ROWS, 8);
const geoShell = gridGeometry(ROWS, COLS);
const geoNasal = gridGeometry(NROWS, COLS);
for (const [g, m] of [[geoGroove, matMucosa], [geoUpperA, matPalate], [geoUpperB, matShell], [geoLower, matTongue], [geoShell, matShell], [geoNasal, matNasal]]) organ.add(new THREE.Mesh(g, m));

// End caps (flat fans) close the shell at the larynx and the mouth.
function fan(segments) {
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.BufferAttribute(new Float32Array((segments + 1) * 3), 3));
  const idx = []; for (let i = 0; i < segments - 1; i++) idx.push(segments, i, i + 1);
  g.setIndex(idx); return g;
}
const capLow = fan(COLS), capHigh = fan(COLS);
organ.add(new THREE.Mesh(capLow, matShell), new THREE.Mesh(capHigh, matShell));

// Lips, velum + uvula, folds, velopharyngeal port.
const lipGeo = new THREE.SphereGeometry(1, 28, 18);
const upperLip = new THREE.Mesh(lipGeo, matLip), lowerLip = new THREE.Mesh(lipGeo, matLip);
organ.add(upperLip, lowerLip);
const velum = new THREE.Group();
const velumFlap = new THREE.Mesh(new THREE.CapsuleGeometry(0.42, 1.7, 8, 16), matVelum);
velumFlap.scale.set(1, 1, 1.9); velumFlap.position.y = -1.05; velum.add(velumFlap);
const uvula = new THREE.Mesh(new THREE.SphereGeometry(0.32, 18, 14), matVelum);
uvula.scale.set(1, 1.5, 1); velum.add(uvula);
organ.add(velum);
const foldGeo = new THREE.CapsuleGeometry(0.34, 1.6, 8, 14);
const foldFar = new THREE.Mesh(foldGeo, matFold);
foldFar.rotation.z = Math.PI / 2;
organ.add(foldFar);
const port = new THREE.Mesh(new THREE.CylinderGeometry(1, 1, 1, 18, 1, true), matMucosa);
organ.add(port);

// --------------------------------------------------------------- orbs -----
const orbs = [];
const splashes = [];
const glowTex = (() => {
  const c = document.createElement('canvas'); c.width = c.height = 64;
  const g = c.getContext('2d'), gr = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  gr.addColorStop(0, 'rgba(255,255,255,1)'); gr.addColorStop(0.35, 'rgba(190,240,255,0.55)'); gr.addColorStop(1, 'rgba(190,240,255,0)');
  g.fillStyle = gr; g.fillRect(0, 0, 64, 64); return new THREE.CanvasTexture(c);
})();
const orbColors = [0xbff3ff, 0xd9c8ff, 0xc9ffd8, 0xfff0b8];
let orbHits = 0;
function fireOrb(ray) {
  const r = 0.32 + Math.random() * 0.3;
  const color = orbColors[Math.floor(Math.random() * orbColors.length)];
  const mesh = new THREE.Mesh(new THREE.SphereGeometry(r, 20, 14), new THREE.MeshPhysicalMaterial({ color, emissive: color, emissiveIntensity: 0.3, roughness: 0.1, clearcoat: 1, transmission: 0.2, thickness: 0.5 }));
  const glow = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, color, blending: THREE.AdditiveBlending, depthWrite: false, transparent: true, opacity: 0.45 }));
  glow.scale.setScalar(r * 2.8); mesh.add(glow);
  const origin = camera.position.clone().addScaledVector(ray.direction, 9);
  mesh.position.copy(origin);
  scene.add(mesh);
  const speed = 26 + Math.random() * 8;
  orbs.push({ mesh, r, v: ray.direction.clone().multiplyScalar(speed).add(new THREE.Vector3(0, 2.2, 0)), age: 0, stuck: null, speed });
}
function splash(at, color) {
  for (let i = 0; i < 7; i++) {
    const s = new THREE.Sprite(new THREE.SpriteMaterial({ map: glowTex, color, blending: THREE.AdditiveBlending, depthWrite: false, transparent: true }));
    s.scale.setScalar(0.35 + Math.random() * 0.3); s.position.copy(at);
    scene.add(s);
    splashes.push({ s, v: new THREE.Vector3((Math.random() - 0.5) * 8, 3 + Math.random() * 5, 2 + Math.random() * 5), age: 0 });
  }
}
// Where in the organ (sagittal plane) is a point? Returns { s, q } with q the
// signed distance along the outward normal from the outer wall, or null.
function locate(x, y) {
  let best = null;
  for (let k = 0; k <= 96; k++) {
    const s = k / 96, w = wall(s);
    const vx = x - w.x, vy = y - w.y;
    const along = vx * w.tx + vy * w.ty, q = vx * w.nx + vy * w.ny;
    const limit = k === 0 || k === 96 ? 0.9 : LEN / 96;
    if (Math.abs(along) <= limit && (!best || Math.abs(along) < best.a)) best = { s, q, a: Math.abs(along) };
  }
  if (!best) return null;
  if (best.q > T_PALATE + T_TOP + 2.0 || best.q < -D_LOW - 0.3) return null;
  return best;
}

// ------------------------------------------------------- frame update -----
const ss = new Float32Array(ROWS);
let lastShape = null;
function updateOrgan(t) {
  const n = state[L.ORAL_N] || 44, m = state[L.NASAL_N] || 31;
  const breathe = 1 + (reduceMotion ? 0 : 0.012 * Math.sin((2 * Math.PI * t) / 4.6));
  const pScale = 1 / 900; // pressure RMS (Pa) to visible flap
  const P = (a, i, x, y, z) => { a[i] = x; a[i + 1] = y; a[i + 2] = z; };
  const groove = geoGroove.attributes.position.array, upA = geoUpperA.attributes.position.array;
  const upB = geoUpperB.attributes.position.array, low = geoLower.attributes.position.array, shell = geoShell.attributes.position.array;
  const nasalTop = T_PALATE + diameter(1.8) + T_TOP;
  const shape = [];
  for (let r = 0; r < ROWS; r++) {
    const s = r / (ROWS - 1); ss[r] = s;
    const w = wall(s);
    const area = sample(state, L.AREAS, n, s) || 3;
    const prms = sample(state, L.PRMS, n, s) || 0;
    const flap = 1 + Math.min(0.12, prms * pScale) * (0.6 + 0.4 * Math.sin(t * 31 + s * 23));
    const d = diameter(area) * flap * breathe;
    const half = 0.6 + 0.42 * d;
    shape.push(d);
    // Groove: phi 0 at the outer wall .. pi at the tongue, through z < 0.
    for (let c = 0; c < COLS; c++) {
      const ph = (Math.PI * c) / (COLS - 1);
      const cx = w.x - w.nx * d / 2, cy = w.y - w.ny * d / 2;
      P(groove, (r * COLS + c) * 3, cx + w.nx * (d / 2) * Math.cos(ph), cy + w.ny * (d / 2) * Math.cos(ph), -half * Math.sin(ph));
    }
    // Nasal channel above the palate from the velum forward.
    const hasNose = s >= VELUM_POS;
    const u = hasNose ? (s - VELUM_POS) / (1 - VELUM_POS) : 0;
    const dn = hasNose ? diameter(sample(state, L.NASAL, m, u) || 0.5) : 0;
    const floor = T_PALATE, ceil = T_PALATE + dn;
    // Upper ribbons: roof .. nasal floor, nasal ceiling .. top (bulging a little).
    const topQ = nasalTop * breathe;
    for (let c = 0; c < 6; c++) {
      const f = c / 5, bulge = 0.55 * Math.sin(Math.PI * f);
      const qa = hasNose ? f * floor : f * topQ;
      P(upA, (r * 6 + c) * 3, w.x + w.nx * qa, w.y + w.ny * qa, bulge * (hasNose ? 0.5 : 1));
      const qb = hasNose ? ceil + f * (topQ - ceil) : topQ;
      P(upB, (r * 6 + c) * 3, w.x + w.nx * qb, w.y + w.ny * qb, bulge * 0.5);
    }
    // Tongue / front tissue: tongue surface .. jaw line.
    const jaw = D_LOW * breathe;
    for (let c = 0; c < 8; c++) {
      const taper = Math.sin(Math.PI * Math.min(1, Math.max(0, (s - 0.06) / 0.9)));
      const f = c / 7, q = -(d + f * (jaw - d)), bulge = (0.35 + 1.1 * taper) * Math.pow(Math.sin(Math.PI * f), 0.7);
      P(low, (r * 8 + c) * 3, w.x + w.nx * q, w.y + w.ny * q, bulge);
    }
    // Shell: from the top round the back to the jaw line.
    const mid = (topQ - jaw) / 2, a = (topQ + jaw) / 2;
    for (let c = 0; c < COLS; c++) {
      const ph = (Math.PI * c) / (COLS - 1), q = mid + a * Math.cos(ph);
      P(shell, (r * COLS + c) * 3, w.x + w.nx * q, w.y + w.ny * q, -SHELL_Z * breathe * Math.sin(ph));
    }
    if (r === 0 || r === ROWS - 1) {
      const cap = (r === 0 ? capLow : capHigh).attributes.position.array;
      for (let c = 0; c < COLS; c++) { cap[c * 3] = shell[(r * COLS + c) * 3]; cap[c * 3 + 1] = shell[(r * COLS + c) * 3 + 1]; cap[c * 3 + 2] = shell[(r * COLS + c) * 3 + 2]; }
      cap[COLS * 3] = w.x + w.nx * mid; cap[COLS * 3 + 1] = w.y + w.ny * mid; cap[COLS * 3 + 2] = 0;
    }
  }
  // Nasal groove.
  const nas = geoNasal.attributes.position.array;
  for (let r = 0; r < NROWS; r++) {
    const u = r / (NROWS - 1), s = VELUM_POS + u * (1 - VELUM_POS), w = wall(s);
    const dn = diameter(sample(state, L.NASAL, m, u) || 0.5);
    const half = 0.4 + 0.4 * dn;
    for (let c = 0; c < COLS; c++) {
      const ph = (Math.PI * c) / (COLS - 1), q = T_PALATE + dn / 2 - (dn / 2) * Math.cos(ph);
      P(nas, (r * COLS + c) * 3, w.x + w.nx * q, w.y + w.ny * q, -half * Math.sin(ph));
    }
  }
  for (const g of [geoGroove, geoUpperA, geoUpperB, geoLower, geoShell, geoNasal, capLow, capHigh]) {
    g.attributes.position.needsUpdate = true; g.computeVertexNormals(); g.computeBoundingSphere();
  }
  lastShape = shape;

  // Lips: the opening is the last section's diameter; rounding pushes them out.
  const wl = wall(1), dl = shape[ROWS - 1];
  const lipsCtl = state[L.CUR_LIPS];
  const pout = 0.55 + 0.9 * (1 - Math.min(1, lipsCtl / 0.6));
  const lipFlap = Math.min(0.15, (sample(state, L.PRMS, n, 0.98) || 0) / 700) * Math.sin(t * 47);
  upperLip.position.set(wl.x + pout, wl.y + 0.55 + lipFlap, -0.2);
  upperLip.scale.set(0.95, 0.62, 2.2);
  lowerLip.position.set(wl.x + pout - 0.1, wl.y - dl - 0.62 - lipFlap, -0.2);
  lowerLip.scale.set(1.0, 0.7, 2.25);

  // Velum hinged on the roof just behind the port; open swings it down/forward.
  const vel = state[L.CUR_VELUM];
  const wv = wall(VELUM_POS + 0.04);
  velum.position.set(wv.x - wv.nx * 0.05, wv.y - wv.ny * 0.05, -0.9);
  const base = Math.atan2(-wv.nx, wv.ny);
  velum.rotation.z = base + 0.25 - 0.9 * vel + uvulaSwing.a * 0.4;
  uvula.position.set(0, -2.05 - 0.15 * Math.sin(t * 2.1), 0);
  uvula.rotation.z = uvulaSwing.a;

  // Port: a short dark tube through the palate, as wide as the velum lets it be.
  const wp = wall(VELUM_POS), portD = diameter(state[L.NASAL] || 0.025);
  port.position.set(wp.x + wp.nx * T_PALATE * 0.5, wp.y + wp.ny * T_PALATE * 0.5, -0.4);
  port.rotation.z = Math.atan2(-wp.nx, wp.ny) + Math.PI / 2;
  port.scale.set(portD / 2, T_PALATE * 1.1, portD / 2);

  // Folds: the far fold, its edge at the measured half-gap (stroboscope).
  const w0 = wall(0.012), d0 = shape[0];
  const gapMm = strobeGap(t);
  foldFar.position.set(w0.x - w0.nx * d0 / 2, w0.y + 0.15, -0.36 - Math.max(-0.2, gapMm) * 1.6);
  foldFar.scale.set(1, Math.max(0.6, d0 / 2.0), 1);
}

// The folds move ~120 times a second; the picture shows the last measured
// cycle slowed to 1.4 cycles a second, as a laryngeal stroboscope does.
function strobeGap(t) {
  const ph = (t * 1.4) % 1, k = ph * 32, i = Math.floor(k) % 32, j = (i + 1) % 32, f = k - Math.floor(k);
  const a = state[L.CYCLE + i], b = state[L.CYCLE + j];
  if (!(state[L.F0] > 0)) return state[L.FOLD_GAP_MM];
  return a * (1 - f) + b * f;
}
const uvulaSwing = { a: 0, v: 0, lastVel: 0 };
function swingUvula(dt) {
  const vel = state[L.CUR_VELUM];
  uvulaSwing.v += (-(vel - uvulaSwing.lastVel) * 40) - uvulaSwing.a * 60 * dt - uvulaSwing.v * 3 * dt;
  uvulaSwing.lastVel = vel;
  uvulaSwing.a += uvulaSwing.v * dt;
  uvulaSwing.a = Math.max(-0.8, Math.min(0.8, uvulaSwing.a));
}

function updateOrbs(dt) {
  for (let i = orbs.length - 1; i >= 0; i--) {
    const o = orbs[i];
    o.age += dt;
    if (o.stuck) {
      o.stuck.t += dt;
      const k = o.stuck.t / 0.9;
      o.mesh.scale.set(1 + 0.35 * k, 1 - 0.5 * k, 0.55);
      o.mesh.position.y -= dt * 1.2;
      o.mesh.material.opacity = 1 - k; o.mesh.material.transparent = true;
      if (k >= 1) { scene.remove(o.mesh); o.mesh.geometry.dispose(); orbs.splice(i, 1); }
      continue;
    }
    const steps = 4;
    for (let sIdx = 0; sIdx < steps; sIdx++) {
      const h = dt / steps;
      o.v.y -= 9.8 * 0.6 * h;
      o.mesh.position.addScaledVector(o.v, h);
      const p = o.mesh.position;
      if (p.z - o.r <= 0.05 && p.z > -SHELL_Z) {
        const hit = locate(p.x, p.y);
        if (hit) {
          const energy = 1.2e-4 * Math.pow(o.r / 0.45, 3) * Math.pow(o.speed / 30, 2);
          send({ type: 'impact', position: Math.min(1, Math.max(0, hit.s)), energy });
          orbHits++;
          o.stuck = { t: 0 };
          p.z = Math.max(p.z, 0.15);
          splash(p.clone(), o.mesh.material.color.getHex());
          break;
        }
      }
    }
    if (!o.stuck && (o.age > 4 || o.mesh.position.z < -12 || o.mesh.position.y < -15)) {
      scene.remove(o.mesh); o.mesh.geometry.dispose(); orbs.splice(i, 1);
    }
  }
  for (let i = splashes.length - 1; i >= 0; i--) {
    const sp = splashes[i];
    sp.age += dt; sp.v.y -= 14 * dt; sp.s.position.addScaledVector(sp.v, dt);
    sp.s.material.opacity = Math.max(0, 1 - sp.age / 0.7);
    if (sp.age > 0.7) { scene.remove(sp.s); splashes.splice(i, 1); }
  }
}

// ----------------------------------------------------- pointer: 3D view --
let mode = 'drag';
const stage = $('stage');
const raycaster = new THREE.Raycaster();
const ndc = new THREE.Vector2();
const midPlane = new THREE.Plane(new THREE.Vector3(0, 0, 1), 0);
const pointers = new Map();
let gesture3d = null; // 'tongue' | 'orbit' | 'pinch'
let pinchStart = 0, pinchR = 0;
function rayAt(ev) {
  const rect = canvas.getBoundingClientRect();
  ndc.set(((ev.clientX - rect.left) / rect.width) * 2 - 1, -((ev.clientY - rect.top) / rect.height) * 2 + 1);
  raycaster.setFromCamera(ndc, camera);
  return raycaster.ray;
}
function tongueFromPoint(ev) {
  const hit = new THREE.Vector3();
  if (!rayAt(ev).intersectPlane(midPlane, hit)) return false;
  const loc = locate(hit.x, hit.y);
  if (!loc) return false;
  const inward = -loc.q;
  if (inward < 0.15 || inward > D_LOW) return false;
  const index = (loc.s - TONGUE_BACK) / (TONGUE_FRONT - TONGUE_BACK);
  const diam = 1 - (inward - 0.25) / 3.3;
  setTongue(Math.min(1, Math.max(0, index)), Math.min(1, Math.max(0, diam)));
  return true;
}
canvas.addEventListener('pointerdown', (ev) => {
  canvas.setPointerCapture(ev.pointerId);
  pointers.set(ev.pointerId, { x: ev.clientX, y: ev.clientY });
  if (pointers.size === 2) {
    const [a, b] = [...pointers.values()];
    gesture3d = 'pinch'; pinchStart = Math.hypot(a.x - b.x, a.y - b.y); pinchR = orbit.r; return;
  }
  if (mode === 'shoot') { fireOrb(rayAt(ev)); gesture3d = 'orbit-shot'; return; }
  gesture3d = tongueFromPoint(ev) ? 'tongue' : 'orbit';
});
canvas.addEventListener('pointermove', (ev) => {
  const prev = pointers.get(ev.pointerId);
  if (!prev) return;
  const dx = ev.clientX - prev.x, dy = ev.clientY - prev.y;
  pointers.set(ev.pointerId, { x: ev.clientX, y: ev.clientY });
  if (gesture3d === 'pinch' && pointers.size === 2) {
    const [a, b] = [...pointers.values()];
    orbit.r = Math.min(48, Math.max(16, pinchR * pinchStart / Math.max(20, Math.hypot(a.x - b.x, a.y - b.y))));
  } else if (gesture3d === 'tongue') tongueFromPoint(ev);
  else if (gesture3d === 'orbit') {
    orbit.az = Math.min(1.35, Math.max(-1.35, orbit.az - dx * 0.006));
    orbit.el = Math.min(1.0, Math.max(-0.35, orbit.el + dy * 0.005));
  }
});
const endPointer = (ev) => { pointers.delete(ev.pointerId); if (pointers.size === 0) gesture3d = null; };
canvas.addEventListener('pointerup', endPointer);
canvas.addEventListener('pointercancel', endPointer);
canvas.addEventListener('wheel', (ev) => { ev.preventDefault(); orbit.r = Math.min(48, Math.max(16, orbit.r * (1 + ev.deltaY * 0.001))); }, { passive: false });
function setMode(m) {
  mode = m;
  $('m-drag').setAttribute('aria-pressed', String(m === 'drag'));
  $('m-shoot').setAttribute('aria-pressed', String(m === 'shoot'));
  $('hint').textContent = m === 'shoot' ? 'tap to throw an orb · where it lands, the wall is knocked and dented' : 'drag the tongue · drag elsewhere to turn';
}
$('m-drag').onclick = () => setMode('drag');
$('m-shoot').onclick = () => setMode('shoot');

// ------------------------------------------------- the 2D cross-section --
const sec = $('section2d'), sctx = sec.getContext('2d');
let secDrag = false;
function setTongue(index, diam) {
  stopGestureUi();
  setControl('index', index); setControl('diameter', diam);
}
function secToTongue(ev) {
  const rect = sec.getBoundingClientRect();
  const fx = (ev.clientX - rect.left) / rect.width, fy = (ev.clientY - rect.top) / rect.height;
  const index = (fx - TONGUE_BACK) / (TONGUE_FRONT - TONGUE_BACK);
  const diam = 1 - (fy - 0.30) / 0.6;
  setTongue(Math.min(1, Math.max(0, index)), Math.min(1, Math.max(0, diam)));
}
sec.addEventListener('pointerdown', (ev) => { sec.setPointerCapture(ev.pointerId); secDrag = true; secToTongue(ev); });
sec.addEventListener('pointermove', (ev) => { if (secDrag) secToTongue(ev); });
sec.addEventListener('pointerup', () => { secDrag = false; });
sec.addEventListener('pointercancel', () => { secDrag = false; });
function drawSection() {
  const dpr = Math.min(devicePixelRatio, 2);
  const W = sec.clientWidth, H = sec.clientHeight;
  if (sec.width !== Math.round(W * dpr)) { sec.width = Math.round(W * dpr); sec.height = Math.round(H * dpr); }
  const g = sctx; g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, W, H);
  const n = state[L.ORAL_N] || 44, m = state[L.NASAL_N] || 31;
  const roof = H * 0.30, scale = (H * 0.62) / 3.6;
  g.fillStyle = '#5e2633'; g.fillRect(0, 0, W, H);
  // Nasal band above the roof.
  g.fillStyle = '#2b0f16';
  g.beginPath();
  for (let k = 0; k <= 40; k++) {
    const u = k / 40, x = (VELUM_POS + u * (1 - VELUM_POS)) * W, dn = diameter(sample(state, L.NASAL, m, u) || 0.5) * scale * 0.6;
    k === 0 ? g.moveTo(x, roof - 6) : g.lineTo(x, roof - 6);
    if (k === 40) { for (let j = 40; j >= 0; j--) { const uu = j / 40; g.lineTo((VELUM_POS + uu * (1 - VELUM_POS)) * W, roof - 6 - diameter(sample(state, L.NASAL, m, uu) || 0.5) * scale * 0.6); } }
  }
  g.closePath(); g.fill();
  // Airway (dark) hanging from the roof; tongue below.
  g.fillStyle = '#1a080d';
  g.beginPath(); g.moveTo(0, roof);
  for (let k = 0; k <= 88; k++) { const s = k / 88; g.lineTo(s * W, roof); }
  for (let k = 88; k >= 0; k--) { const s = k / 88, d = diameter(sample(state, L.AREAS, n, s) || 3) * scale; g.lineTo(s * W, roof + d); }
  g.closePath(); g.fill();
  g.strokeStyle = '#ff8fa6'; g.lineWidth = 1.5; g.beginPath();
  for (let k = 0; k <= 88; k++) { const s = k / 88, d = diameter(sample(state, L.AREAS, n, s) || 3) * scale; k === 0 ? g.moveTo(0, roof + d) : g.lineTo(s * W, roof + d); }
  g.stroke();
  g.strokeStyle = '#f3dfe2'; g.globalAlpha = 0.5; g.beginPath(); g.moveTo(0, roof); g.lineTo(W, roof); g.stroke(); g.globalAlpha = 1;
  // Tongue handle.
  const ti = state[L.CUR_INDEX], td = state[L.CUR_DIAM];
  const hx = (TONGUE_BACK + ti * (TONGUE_FRONT - TONGUE_BACK)) * W, hy = (0.30 + (1 - td) * 0.6) * H;
  g.fillStyle = 'rgba(255,143,166,0.25)'; g.beginPath(); g.arc(hx, hy, 16, 0, Math.PI * 2); g.fill();
  g.strokeStyle = '#ffd0d8'; g.lineWidth = 2; g.beginPath(); g.arc(hx, hy, 9, 0, Math.PI * 2); g.stroke();
  g.fillStyle = '#b08f95'; g.font = '10px ui-monospace, Menlo, monospace';
  g.fillText('glottis', 4, H - 6); g.fillText('lips', W - 28, H - 6); g.fillText('nose', VELUM_POS * W + 4, 12);
  g.fillText('velum ' + (state[L.CUR_VELUM] * 100).toFixed(0) + '%', VELUM_POS * W - 30, roof + 12 + diameter(sample(state, L.AREAS, n, VELUM_POS) || 3) * scale);
}

const strobe = $('strobe'), stx = strobe.getContext('2d');
function drawStrobe(t) {
  const dpr = Math.min(devicePixelRatio, 2), W = strobe.clientWidth, H = strobe.clientHeight;
  if (strobe.width !== Math.round(W * dpr)) { strobe.width = Math.round(W * dpr); strobe.height = Math.round(H * dpr); }
  const g = stx; g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, W, H);
  const lo = -0.4, hi = 1.2, Y = (v) => H - 6 - ((v - lo) / (hi - lo)) * (H - 12);
  g.strokeStyle = '#3a2328'; g.beginPath(); g.moveTo(0, Y(0)); g.lineTo(W, Y(0)); g.stroke();
  g.fillStyle = '#6f555a'; g.font = '10px ui-monospace, Menlo, monospace'; g.fillText('closed', 4, Y(0) - 3);
  g.fillText('half-gap, mm', W - 78, 11);
  if (state[L.F0] > 0) {
    g.strokeStyle = '#ffd0d8'; g.lineWidth = 1.5; g.beginPath();
    for (let k = 0; k <= 64; k++) { const i = k % 32, v = state[L.CYCLE + i]; const x = (k / 64) * W; k === 0 ? g.moveTo(x, Y(v)) : g.lineTo(x, Y(v)); }
    g.stroke();
    const ph = (t * 1.4) % 1;
    g.fillStyle = '#ff8fa6'; g.beginPath(); g.arc(ph * W / 2, Y(strobeGap(t)), 3, 0, Math.PI * 2); g.fill();
  } else {
    g.fillStyle = '#b08f95'; g.fillText('folds still (no breath, or below threshold)', 8, H / 2);
  }
}

// ------------------------------------------------------- panel controls --
const sliders = {
  breath: ['s-breath', 'o-breath', (v) => v.toFixed(2)],
  pitch: ['s-pitch', 'o-pitch', (v) => v.toFixed(0) + ' Hz'],
  tense: ['s-tense', 'o-tense', (v) => v.toFixed(2)],
  lips: ['s-lips', 'o-lips', (v) => v.toFixed(2)],
  velum: ['s-velum', 'o-velum', (v) => v.toFixed(2)],
  asp: ['s-asp', 'o-asp', (v) => v.toFixed(2)],
};
const sliderBusy = new Set();
for (const [name, [sid, oid, fmt]] of Object.entries(sliders)) {
  const el = $(sid), out = $(oid);
  out.textContent = fmt(Number(el.value));
  el.addEventListener('input', () => {
    if (name === 'lips' || name === 'velum') stopGestureUi();
    setControl(name, Number(el.value)); out.textContent = fmt(Number(el.value));
  });
  el.addEventListener('pointerdown', () => sliderBusy.add(name));
  el.addEventListener('pointerup', () => sliderBusy.delete(name));
}
$('o-master').textContent = MUTE ? 'muted' : Number($('s-master').value).toFixed(2);
$('s-master').addEventListener('input', () => { $('o-master').textContent = MUTE ? 'muted' : Number($('s-master').value).toFixed(2); applyMaster(); });
if (MUTE) $('h-muted').hidden = false;

let activeGesture = null;
function stopGestureUi() {
  if (activeGesture !== null) { activeGesture = null; send({ type: 'gesture', id: -1 }); }
  for (const b of document.querySelectorAll('.presets button')) b.setAttribute('aria-pressed', 'false');
}
for (const b of document.querySelectorAll('.presets button')) {
  b.addEventListener('click', () => {
    for (const o of document.querySelectorAll('.presets button')) o.setAttribute('aria-pressed', 'false');
    if (b.dataset.preset !== undefined) {
      activeGesture = null; send({ type: 'gesture', id: -1 });
      send({ type: 'preset', id: Number(b.dataset.preset) });
      if (controls.breath < 0.3 && b.dataset.preset !== '3') { setControl('breath', 0.55); $('s-breath').value = 0.55; $('o-breath').textContent = '0.55'; }
    } else {
      activeGesture = Number(b.dataset.gesture);
      send({ type: 'gesture', id: activeGesture });
    }
    b.setAttribute('aria-pressed', 'true');
  });
}
$('start').addEventListener('click', async () => {
  $('start').disabled = true;
  try { await startAudio(); $('cover').hidden = true; }
  catch (err) { console.error(err); $('start').disabled = false; $('cover').querySelector('.sub').textContent = 'audio could not start: ' + err.message; }
});

// Keep sliders honest while a gesture moves the World.
function syncSliders() {
  const map = { lips: L.CUR_LIPS, velum: L.CUR_VELUM, pitch: L.CUR_PITCH, breath: L.CUR_BREATH, tense: L.CUR_TENSE };
  if (state[L.GESTURE] < 0.5) return;
  for (const [name, idx] of Object.entries(map)) {
    if (sliderBusy.has(name)) continue;
    const [sid, oid, fmt] = sliders[name];
    $(sid).value = state[idx]; $(oid).textContent = fmt(state[idx]);
  }
}

// ------------------------------------------------------------ readouts ---
const fmtJ = (v) => (Math.abs(v) < 1e-3 && v !== 0 ? v.toExponential(2) : v.toFixed(4)) + ' J';
function drawReadout() {
  const s = state;
  const source = s[L.MUSCLE] + s[L.IMPACT];
  const rel = Math.abs(s[L.GLOBAL_RES]) / Math.max(1e-30, source);
  const rows = [
    ['measured F0 / fold tuning', `${s[L.F0] > 0 ? s[L.F0].toFixed(1) : '–'} / ${s[L.CUR_PITCH].toFixed(0)} Hz`],
    ['lung pressure · reservoir', `${s[L.LUNG_PA].toFixed(0)} Pa · ${(s[L.LUNG_FRACTION] * 100).toFixed(0)}%`],
    ['glottal flow', `${s[L.GLOTTAL_FLOW].toFixed(0)} cm³/s`],
    ['breathing muscles in', fmtJ(s[L.MUSCLE])],
    [`orbs in (${s[L.IMPACTS].toFixed(0)} hits)`, fmtJ(s[L.IMPACT])],
    ['walls + folds moved', fmtJ(s[L.TRACT_CTRL] + s[L.FOLD_CTRL])],
    ['lost: walls · jet · folds', `${s[L.WALL].toFixed(3)} · ${s[L.JET].toFixed(3)} · ${s[L.FOLD_LOSS].toFixed(3)}`],
    ['radiated: lips · nose', `${(s[L.RAD_LIPS] * 1e3).toFixed(2)} · ${(s[L.RAD_NOSE] * 1e3).toFixed(2)} mJ`],
    ['in the air now', `${(s[L.TRACT_E] * 1e6).toFixed(1)} µJ`],
    ['worst block residual', `<span class="ok">${s[L.MAX_RES].toExponential(1)} J</span>`],
    ['books, relative', `<span class="${rel < 1e-8 ? 'ok' : 'warn'}">${rel.toExponential(1)}</span>`],
    ['DSP load (worklet)', dspLoad === null ? 'not started' : `${(dspLoad * 100).toFixed(1)}%`],
    ['wasm real-time factor', wasmRtf ? `${wasmRtf.toFixed(1)}× (main thread)` : '–'],
    ['sections oral · nasal', `${s[L.ORAL_N]} · ${s[L.NASAL_N]}`],
  ];
  $('readout').innerHTML = rows.map(([k, v]) => `<span>${k}</span><span>${v}</span>`).join('');
  $('h-f0').textContent = s[L.F0] > 0 ? s[L.F0].toFixed(0) + ' Hz' : 'still';
  $('h-lung').textContent = (s[L.LUNG_FRACTION] * 100).toFixed(0) + '%';
  $('h-dsp').textContent = (dspLoad === null ? '' : (dspLoad * 100).toFixed(1) + '% · ') + (wasmRtf ? wasmRtf.toFixed(1) + '× rt' : '–');
  $('h-books').textContent = s[L.MAX_RES].toExponential(0) + ' J';
}

// ---------------------------------------------------------------- loop ---
function resize() {
  const w = stage.clientWidth, h = stage.clientHeight;
  renderer.setSize(w, h, false);
  camera.aspect = w / Math.max(1, h);
  camera.fov = camera.aspect < 0.9 ? 44 : 34;
  camera.updateProjectionMatrix();
  // Fit the organ's width on narrow (portrait) stages.
  const hfov = 2 * Math.atan(Math.tan((camera.fov * Math.PI) / 360) * camera.aspect);
  fitScale = camera.aspect >= 1.25 ? 1 : Math.min(2.2, 0.414 / Math.tan(hfov / 2)) * (camera.aspect < 0.9 ? 0.74 : 1);
}
new ResizeObserver(resize).observe(stage);
resize();

await silent.init();
let last = performance.now(), readoutAt = 0;
function frame(now) {
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  const t = now / 1000;
  silent.run(dt * 1000);
  swingUvula(dt);
  updateOrgan(t);
  updateOrbs(dt);
  const sway = reduceMotion ? 0 : 0.03 * Math.sin(t * 0.21);
  const az = orbit.az + sway, el = orbit.el + (reduceMotion ? 0 : 0.015 * Math.sin(t * 0.17));
  const rr = orbit.r * fitScale;
  camera.position.set(target.x + rr * Math.cos(el) * Math.sin(az), target.y + rr * Math.sin(el), target.z + rr * Math.cos(el) * Math.cos(az));
  camera.lookAt(target);
  sheen.position.set(4 + 9 * Math.cos(t * 0.23), 7 + 4 * Math.sin(t * 0.31), 9 + 3 * Math.sin(t * 0.19));
  renderer.render(scene, camera);
  drawSection();
  drawStrobe(t);
  if (now - readoutAt > 120) { readoutAt = now; drawReadout(); syncSliders(); }
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
window.__tract = { get state() { return state; }, get wasmRtf() { return wasmRtf; }, get dspLoad() { return dspLoad; }, send, fireOrb: () => fireOrb(rayAt({ clientX: canvas.getBoundingClientRect().left + canvas.clientWidth * 0.55, clientY: canvas.getBoundingClientRect().top + canvas.clientHeight * 0.45 })), get hits() { return orbHits; } };
