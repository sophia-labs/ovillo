// The public World tract (ovillo-tract-world) as tools/tract-render.mjs played it: every
// video frame is the World's own state vector at that instant (tract_snapshot) and its
// receipt (ndt_receipt), from the same run that wrote the audio.
//
// What drives what:
// - the airway: each of the 44 oral sections' area (the World's network areas), drawn as a
//   ring hanging from the fixed outer wall (the tract page's mid-sagittal layout, cm);
//   its brightness is that section's RMS pressure since the last snapshot;
// - the nasal passage: the 31 nasal sections' areas, above the palate from the velum;
// - the folds: the valve's half-gap, smallest and largest since the last snapshot (x8);
// - amber at the narrowest column: the turbulence the jet injected in this frame;
// - blue arcs: the World's radiation at the lips and at the nostrils, at the moment each
//   leaves (their spacing and speed are invented).
import * as THREE from 'three';
import { Strokes, PALETTE as C, level, V, orbitCamera } from '../lib/draw.js';
import { fmtJ, fmtW, meter } from '../lib/hud.js';

// the tract page's layout (site/tract/tract.js), centimetres
const LEN = 15.7, PH = 6.4, RB = 2.3, ARC = (Math.PI / 2) * RB, T_PALATE = 1.0, VELUM_POS = 0.40;
function wall(s) {
  const l = s * LEN;
  if (l <= PH) return { x: 0, y: l, nx: -1, ny: 0, tx: 0, ty: 1 };
  if (l <= PH + ARC) {
    const th = (l - PH) / RB;
    return { x: RB - RB * Math.cos(th), y: PH + RB * Math.sin(th), nx: -Math.cos(th), ny: Math.sin(th), tx: Math.sin(th), ty: Math.cos(th) };
  }
  return { x: RB + (l - PH - ARC), y: PH + RB, nx: 0, ny: 1, tx: 1, ty: 0 };
}
const diameter = (areaCm2) => 2 * Math.sqrt(Math.max(areaCm2, 0) / Math.PI);
const PRESET = ['/a/', '/i/', '/u/', 'rest'];

function stepText(st) {
  const parts = [];
  if (st.preset !== undefined) parts.push(`preset ${PRESET[st.preset]}`);
  const names = { index: 'tongue', diameter: 'tongue height', lips: 'lips', velum: 'velum', pitch: 'folds', tense: 'tenseness', breath: 'breath' };
  const set = { ...(st.set ?? {}) };
  if (set.index !== undefined && set.diameter !== undefined) { parts.push(`tongue ${set.index}/${set.diameter}`); delete set.index; delete set.diameter; }
  for (const [k, v] of Object.entries(set)) parts.push(`${names[k] ?? k} ${k === 'pitch' ? v + ' Hz' : v}`);
  if (st.pitch !== undefined) parts.push(`folds ${st.pitch} Hz`);
  if (st.constrict) parts.push(st.constrict[2] < 0 ? 'release touch' : `touch ${st.constrict[1]} (${st.constrict[2]})`);
  if (st.vibrato) parts.push(`vibrato ${st.vibrato[1]} st`);
  for (const [k, [to, sec]] of Object.entries(st.ramp ?? {})) parts.unshift(`${names[k] ?? k} → ${to} in ${Math.round(sec * 1000)} ms`);
  return parts.join(' · ');
}

export class TractScene {
  constructor({ seg, data, variant, width, height }) {
    this.seg = seg; this.d = data; this.variant = variant; this.width = width; this.height = height;
    const d = data, L = d.layout;
    this.L = L;
    this.scene = new THREE.Scene();
    this.camera = new THREE.PerspectiveCamera(30, width / height, 0.1, 500);
    const root = new THREE.Group();
    this.scene.add(root);
    this.struct = new Strokes(root, { width: 1.4, glow: 0.5 });
    this.flesh = new Strokes(root, { width: 2.2, glow: 1.0 });
    this.rings = new Strokes(root, { width: 1.4, glow: 0.5 });
    this.marks = new Strokes(root, { width: 2.2, glow: 1.2 });
    this.waves = new Strokes(root, { width: 1.6, glow: 0.8 });
    this.dt = 1 / d.fps;
    const R = d.receiptColumns;
    this.rc = Object.fromEntries(R.map((k, i) => [k, i]));
    const F = d.frames;
    const inWin = (k) => { const t = d.t0 + k * this.dt; return t >= d.window[0] && t <= d.window[1]; };
    this.pmax = Math.max(...F.filter((_, k) => inWin(k)).map((f) => Math.max(...f.slice(L.PRMS, L.PRMS + f[L.ORAL_N]))));
    const diff = (idx) => F.map((f, k) => (k === 0 ? 0 : Math.max(0, f[idx] - F[k - 1][idx]) / this.dt));
    this.radLips = diff(L.RAD_LIPS);
    this.radNose = diff(L.RAD_NOSE);
    this.turb = diff(L.TURB);
    const winMax = (a) => Math.max(...a.filter((_, k) => inWin(k)));
    this.max = {
      rad: Math.max(winMax(this.radLips), winMax(this.radNose)), turb: winMax(this.turb),
      tract: winMax(F.map((f) => f[L.TRACT_E])), flow: winMax(F.map((f) => f[L.GLOTTAL_FLOW])),
      lungs: winMax(F.map((f) => f[L.LUNG_E])),
    };
    this.layout();
  }

  layout() {
    const w = this.width, h = this.height;
    const target = V(4.4, 4.6, 0);
    if (this.variant === '4x5') orbitCamera(this.camera, { target, az: 24, el: 8, r: 56, fov: 30, w, h, cx: 520, cy: 566 });
    else orbitCamera(this.camera, { target, az: 24, el: 8, r: 47, fov: 30, w, h, cx: 600, cy: 545 });
  }

  frameAt(t) {
    const k = Math.max(0, Math.min(this.d.frames.length - 1, Math.round((t - this.d.t0) / this.dt)));
    return k;
  }

  frame(local) {
    const d = this.d, L = this.L;
    const t = d.window[0] + local;
    const k = this.frameAt(t);
    const f = d.frames[k];
    const rec = d.receipts[k];
    const n = f[L.ORAL_N], m = f[L.NASAL_N];
    // a cut, then a short brightening (never from black)
    const fadeIn = Math.min(1, 0.6 + local / 0.5);
    const S = this.struct.begin(), Fl = this.flesh.begin(), R = this.rings.begin(), M = this.marks.begin(), W = this.waves.begin();
    const Z = V(0, 0, 1);

    // outer wall (posterior pharynx, palate) and the airway's inner boundary (tongue side)
    const outer = [], inner = [];
    for (let i = 0; i < n; i++) {
      const s = (i + 0.5) / n, w = wall(s);
      const dd = diameter(f[L.AREAS + i]);
      outer.push(V(w.x, w.y, 0));
      inner.push(V(w.x - w.nx * dd, w.y - w.ny * dd, 0));
      const c = V(w.x - w.nx * dd / 2, w.y - w.ny * dd / 2, 0);
      const lv = level(f[L.PRMS + i], this.pmax, 30);
      R.circle(c, Math.max(dd / 2, 0.02), V(-w.nx, -w.ny, 0), Z, C.world, (0.05 + 0.9 * lv * lv) * fadeIn, 40);
    }
    S.poly(outer, C.pink, 0.45 * fadeIn);
    Fl.poly(inner, C.pink, 0.95 * fadeIn);
    // a lateral hint of the walls: the same two lines at the tube's half width either side
    for (const zz of [-1, 1]) {
      const side = [];
      for (let i = 0; i < n; i++) {
        const s = (i + 0.5) / n, w = wall(s), dd = diameter(f[L.AREAS + i]);
        side.push(V(w.x - w.nx * dd / 2, w.y - w.ny * dd / 2, zz * Math.max(dd / 2, 0.02)));
      }
      S.poly(side, C.pink, 0.22 * fadeIn);
    }
    // nasal passage above the palate, from the velum to the nostrils
    const nasal = [];
    for (let j = 0; j < m; j++) {
      const u = (j + 0.5) / m, s = VELUM_POS + u * (1 - VELUM_POS), w = wall(s);
      const dn = diameter(f[L.NASAL + j]);
      const c = V(w.x + w.nx * (T_PALATE + dn / 2), w.y + w.ny * (T_PALATE + dn / 2), 0);
      R.circle(c, Math.max(dn / 2, 0.02), V(w.nx, w.ny, 0), Z, C.pink, 0.30 * fadeIn, 32);
      nasal.push(V(w.x + w.nx * (T_PALATE + dn), w.y + w.ny * (T_PALATE + dn), 0));
    }
    S.poly(nasal, C.pink, 0.4 * fadeIn);
    // the palate's upper face (the nasal floor)
    const floor = [];
    for (let j = 0; j <= 24; j++) { const s = VELUM_POS + (j / 24) * (1 - VELUM_POS), w = wall(s); floor.push(V(w.x + w.nx * T_PALATE, w.y + w.ny * T_PALATE, 0)); }
    S.poly(floor, C.pink, 0.35 * fadeIn);
    // the velopharyngeal port: the first nasal section's opening
    const wv = wall(VELUM_POS), port = diameter(f[L.NASAL]);
    if (port > 0.03) Fl.seg(V(wv.x, wv.y, 0), V(wv.x + wv.nx * T_PALATE, wv.y + wv.ny * T_PALATE, 0), C.pink, 0.25 + 0.6 * Math.min(1, port / 1.2));

    // the lips: the last section's opening, a profile above and below it
    const wl = wall(1), dl = diameter(f[L.AREAS + n - 1]);
    Fl.circle(V(wl.x, wl.y + 0.42, 0), 0.42, V(0, -1, 0), V(1, 0, 0), C.pink, 0.85 * fadeIn, 20, 0, Math.PI);
    Fl.circle(V(wl.x, wl.y - dl - 0.42, 0), 0.42, V(0, 1, 0), V(1, 0, 0), C.pink, 0.85 * fadeIn, 20, Math.PI, 2 * Math.PI);
    // the folds at the glottis: half-gap envelope, x8 (mm -> cm x0.8)
    const w0 = wall(0.012), d0 = diameter(f[L.AREAS]);
    const g1 = Math.max(0, f[L.FOLD_GAP_MAX_MM]) * 0.8, g0 = Math.max(0, f[L.FOLD_GAP_MIN_MM]) * 0.8;
    const flowLv = level(f[L.GLOTTAL_FLOW], this.max.flow, 30);
    const gc = V(w0.x - w0.nx * d0 / 2, 0.15, 0);
    for (const [g, kk] of [[g1, 1], [g0, 0.45]]) {
      M.seg(V(gc.x - d0 / 2 - 0.3, gc.y, -g - 0.02), V(gc.x - g * 0.0, gc.y, -g - 0.02), C.warn, (0.3 + 0.8 * flowLv) * kk * fadeIn);
      M.seg(V(gc.x - d0 / 2 - 0.3, gc.y, g + 0.02), V(gc.x - g * 0.0, gc.y, g + 0.02), C.warn, (0.3 + 0.8 * flowLv) * kk * fadeIn);
    }
    // trachea stub toward the lungs (a drawing aid: structure only)
    S.seg(V(gc.x - 0.9, 0, 0), V(gc.x - 0.9, -1.1, 0), C.line, 0.6 * fadeIn);
    S.seg(V(gc.x + 0.9, 0, 0), V(gc.x + 0.9, -1.1, 0), C.line, 0.6 * fadeIn);
    // the jet: the narrowest column, amber with the turbulence injected this frame
    const tl = level(this.turb[k], this.max.turb, 30);
    if (tl > 0.02) {
      const s = f[L.MIN_AP_POS], w = wall(Math.min(0.999, Math.max(0, s))), dd = diameter(f[L.AREAS + Math.min(n - 1, Math.floor(s * n))]);
      const p = V(w.x - w.nx * dd / 2, w.y - w.ny * dd / 2, 0);
      const r = 0.25 + 0.35 * tl;
      M.seg(p.clone().add(V(-r, -r, 0)), p.clone().add(V(r, r, 0)), C.warn, 0.4 + 0.8 * tl);
      M.seg(p.clone().add(V(-r, r, 0)), p.clone().add(V(r, -r, 0)), C.warn, 0.4 + 0.8 * tl);
    }
    // touch constrictions (slots 1..4): where the page's touch would press
    for (let slot = 1; slot < 5; slot++) {
      const deg = f[L.CONS + 2 * slot + 1];
      if (!(deg > 0.001)) continue;
      const w = wall(f[L.CONS + 2 * slot]);
      M.seg(V(w.x, w.y, 0), V(w.x - w.nx * 1.4 * deg, w.y - w.ny * 1.4 * deg, 0), C.text, 0.7);
    }

    // radiation leaving the lips (forward) and the nostrils (forward and up)
    const every = 2 / 30, life = 0.7, speed = 4.5;
    const nose = nasal[nasal.length - 1];
    for (let j = Math.floor((t - life) / every); j <= Math.floor(t / every); j++) {
      const te = j * every, age = t - te;
      if (age < 0 || age > life || te < d.t0) continue;
      const ke = this.frameAt(te);
      const fade = (1 - age / life) ** 1.5;
      const pl = level(this.radLips[ke], this.max.rad, 36), pn = level(this.radNose[ke], this.max.rad, 36);
      const rr = 0.6 + speed * age;
      if (pl > 0.01) W.circle(V(wl.x + 0.6, wl.y - dl / 2, 0), rr, V(0, 1, 0), V(1, 0, 0), C.blue, 0.9 * pl * fade, 40, Math.PI * 0.2, Math.PI * 0.8);
      if (pn > 0.01) W.circle(V(nose.x + 0.3, nose.y, 0), rr * 0.8, V(0, 1, 0), V(1, 0, 0), C.blue, 0.9 * pn * fade, 32, Math.PI * 0.1, Math.PI * 0.6);
    }
    this.struct.end(); this.flesh.end(); this.rings.end(); this.marks.end(); this.waves.end();

    // labels
    const proj = (v) => { const q = v.clone().project(this.camera); return { x: (q.x + 1) / 2 * this.width, y: (1 - q.y) / 2 * this.height }; };
    const lab = (p, text, c, align) => { const q = proj(p); return { x: q.x, y: q.y, text, c, align, alpha: fadeIn }; };
    const wt = wall(0.62);
    const labels = [
      lab(V(gc.x + 1.5, 0.1, 0), 'FOLDS', 'a', 'l'),
      lab(V(5.4, 4.6, 0), 'TONGUE', 'p'),
      lab(V(wv.x - 1.6, wv.y + 1.9, 0), 'VELUM', 'p', 'r'),
      lab(V(nose.x + 1.0, nose.y + 0.9, 0), 'NOSE', 'b', 'l'),
      lab(V(wl.x + 1.0, wl.y - dl - 1.6, 0), 'LIPS', 'b', 'l'),
    ];

    // panel
    const script = d.script;
    let cur = 0;
    script.forEach((st, i) => { if (st.t <= t + 1e-6) cur = i; });
    const named = script.map((st, i) => ({ ...st, i })).filter((st) => st.label);
    let now = named[0];
    for (const st of named) if (st.t <= t + 1e-6) now = st;
    const near = script.map((st, i) => ({ st, i })).filter(({ st }) => st.label !== '' || st.set?.breath === 0);
    const ci = near.findIndex(({ i }) => i >= cur && script[i].t <= t + 1e-6 && (near.find((x) => x.i > i)?.st.t ?? 1e9) > t) ;
    const pick = near.filter(({ st }) => st.t <= t + 1e-6).slice(-2).concat(near.filter(({ st }) => st.t > t + 1e-6).slice(0, 1));
    const lines = pick.map(({ st, i }) => {
      const isNow = st.t <= t + 1e-6 && !near.some(({ st: o }) => o.t > st.t && o.t <= t + 1e-6);
      return [{ t: `${st.t.toFixed(2)} s  ${stepText(st)}${st.label ? `   "${st.label}"` : ''}`, cls: isNow ? 'tk now' : st.t <= t ? 'tk past' : 'tk' }];
    });
    void ci;
    const f0 = f[L.F0];
    const rr = (key) => rec[this.rc[key]];
    return {
      labels,
      readouts: [
        { k: 'NOW', v: now.label.split(' ')[0], c: 'p' },
        { k: 'F0 MEASURED', v: f0 > 0 ? `${f0.toFixed(0)} Hz` : 'still' },
        { k: 'NARROWEST', v: f[L.MIN_AP] < 1e-4 ? 'closed' : `${f[L.MIN_AP].toFixed(2)} cm²` },
      ],
      chain: [
        meter('LUNGS', 'a', f[L.LUNG_E], this.max.lungs, f[L.LUNG_E], fmtJ, 6),
        meter('FOLDS', 'a', f[L.GLOTTAL_FLOW], this.max.flow, f[L.GLOTTAL_FLOW], (v) => `${v.toFixed(0)} cm³/s`, 30),
        meter('TRACT', 'w', f[L.TRACT_E], this.max.tract, f[L.TRACT_E], fmtJ, 40),
        meter('LIPS+NOSE', 'b', this.radLips[k] + this.radNose[k], this.max.rad, this.radLips[k] + this.radNose[k], fmtW, 40),
      ],
      caption: this.seg.caption,
      ledger: [
        { k: 'IN (LUNGS)', v: fmtJ(rr('sourceWork')), c: 'a' },
        { k: 'LOST INSIDE', v: fmtJ(rr('internalLoss')), c: 'w' },
        { k: 'RADIATED', v: fmtJ(rr('radiation')), c: 'b' },
        { k: 'RESIDUAL', v: fmtJ(rr('cumulativeClosureResidual')), c: 'g' },
      ],
      foot: `moves ${fmtJ(rr("controlWork"), 2)} · worst frame ${fmtJ(rr('maximumFrameResidual'), 2)} · ${this.seg.gaps}`,
      mus: { hd: 'SCRIPT · tools/tract-render.mjs (the tract page\'s own controls; no MUS)', lines },
    };
  }
}
