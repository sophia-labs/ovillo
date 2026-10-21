// The steel-string World playing a tab (mus render --energy), drawn from its trace.
//
// What drives what (cf. blender/README.md, whose rules this follows):
// - the top's lines: the sum of its real coupled mode shapes (visual anatomy), each at its
//   real energy (displacement ∝ √(2E)/ω, high modes lifted by (f/f0)^flatten); the swing's
//   tempo is slowed and invented (0.45·√(f/f0) Hz), the size compressed (√ of the sum);
// - each string: its body's stored energy over its current speaking length (the stop the
//   tab names); the shimmer's tempo is invented (7.5 Hz);
// - amber marks: every touch the score made, at the hand's pluck point (or the fret, for
//   hammer-ons, pull-offs and slides);
// - ticks at the saddle: energy per second crossing each string's bridge edges into the box;
// - blue rings in the soundhole: the air-dominated share of the box's modal energy;
// - blue shells: onsets of radiated power, sized by age and faded by strength (their shape is
//   invented: radiation here is monopole-weighted, not a measured field);
// - the floor graticule: the room field's stored energy.
// Everything is on a log scale against its own loudest moment in the window.
import * as THREE from 'three';
import { Strokes, PALETTE as C, level, sample, recentMax, V, planeCamera } from '../lib/draw.js';
import { fmtJ, fmtW, meter } from '../lib/hud.js';

export class GuitarScene {
  constructor({ seg, data, variant, width, height }) {
    this.seg = seg;
    this.d = data;
    this.variant = variant;
    this.width = width;
    this.height = height;
    const d = data;
    this.scene = new THREE.Scene();
    this.camera = new THREE.PerspectiveCamera(variant === '4x5' ? 30 : 26, width / height, 0.01, 50);
    const root = new THREE.Group();
    this.root = root;
    this.scene.add(root);
    this.room = new Strokes(root, { width: 1.2, glow: 0.4 });
    this.struct = new Strokes(root, { width: 1.4, glow: 0.5 });
    this.top = new Strokes(root, { width: 1.5, glow: 0.6 });
    this.strings = new Strokes(root, { width: 1.3, glow: 0.45 });
    this.air = new Strokes(root, { width: 1.8, glow: 1.0 });
    this.shells = new Strokes(root, { width: 1.5, glow: 0.8 });
    this.marks = new Strokes(root, { width: 2.4, glow: 1.2 });

    const g = d.geometry;
    this.outline = g.outline.map(([x, y]) => V(x, y, 0));
    this.back = g.outline.map(([x, y]) => V(x, y, -g.depth));
    this.fretsY = [g.nutY, ...g.neck.frets_y];
    this.hop = d.hop;
    this.t0 = d.t0;
    const inWin = (i) => { const t = d.t0 + i * d.hop; return t >= d.window[0] && t <= d.window[1]; };
    const maxIn = (arr) => arr.reduce((m, v, i) => (inWin(i) ? Math.max(m, v) : m), 0);
    this.maxString = Math.max(...d.strings.map((s) => maxIn(s.energy)));
    this.maxInto = Math.max(...d.strings.map((s) => maxIn(s.intoBox)));
    this.sumStrings = d.air.map((_, i) => d.strings.reduce((a, s) => a + s.energy[i], 0));
    this.sumInto = d.air.map((_, i) => d.strings.reduce((a, s) => a + Math.max(0, s.intoBox[i]), 0));
    this.max = {
      strings: maxIn(this.sumStrings), into: maxIn(this.sumInto), top: maxIn(d.top), air: maxIn(d.air),
      room: maxIn(d.room), rad: maxIn(d.radiatedW),
    };
    // the top's displayed size: the summed mode amplitudes, compressed by a square root
    this.ampSum = d.modes.amp.map((row) => row.reduce((a, v) => a + v, 0));
    this.maxAmpSum = maxIn(this.ampSum);
    this.fvis = d.modes.frequencyHz.map((f) => 0.45 * Math.sqrt(f / d.modes.frequencyHz[0]));
    // radiation onsets (as blender/ariadne_scene.py): local maxima of the rise in radiated power
    const rad = d.radiatedW;
    const sm = rad.map((_, i) => { let a = 0, n = 0; for (let k = -3; k <= 3; k++) { const j = i + k; if (j >= 0 && j < rad.length) { a += rad[j]; n++; } } return a / n; });
    const onset = sm.map((v, i) => (i === 0 ? 0 : Math.max(0, v - sm[i - 1])));
    const picks = [];
    for (let i = 1; i < onset.length - 1; i++) {
      const strength = level(sm[i], this.max.rad, 30);
      if (onset[i] > 0 && onset[i] >= onset[i - 1] && onset[i] >= onset[i + 1] && strength > 0.45) {
        if (!picks.length || (i - picks[picks.length - 1].i) * d.hop > 0.12) picks.push({ i, t: d.t0 + i * d.hop, strength });
      }
    }
    this.picks = picks;
    this.touches = d.touches;
    this.layout();
  }

  layout() {
    const w = this.width, h = this.height;
    if (this.variant === '4x5') {
      planeCamera(this.camera, { target: V(0.0, 0.47, -0.03), angle: 33, tilt: 38, r: 2.8, fov: 30, w, h, cx: 560, cy: 516 });
    } else {
      planeCamera(this.camera, { target: V(0.0, 0.47, -0.03), angle: 22, tilt: 40, r: 2.15, fov: 30, w, h, cx: 600, cy: 540 });
    }
  }

  stopAt(stringNo, t) {
    let fret = this.d.stopAtStart[String(stringNo)] ?? 0;
    let from = fret, slideAt = null;
    for (const tc of this.touches) {
      if (tc.string !== stringNo) continue;
      const lead = tc.kind === 'pluck' ? 0.008 : 0; // the fretting finger lands 8 ms early
      if (tc.seconds - lead > t) break;
      from = fret;
      fret = tc.fret;
      slideAt = tc.kind === 'slide' ? tc.seconds : null;
    }
    if (slideAt !== null && t - slideAt < 0.06) return from + (fret - from) * ((t - slideAt) / 0.06); // 60 ms slide
    return fret;
  }

  yOfFret(f) {
    // fractional frets interpolate between fret lines (a slide in progress)
    const i = Math.floor(f), w = f - i;
    const a = this.fretsY[Math.min(i, 20)], b = this.fretsY[Math.min(i + 1, 20)];
    return a + (b - a) * w;
  }

  stringPoint(s, y, z0 = 0.0125) {
    const g = this.d.geometry;
    const u = (y - g.saddleY) / (g.nutY - g.saddleY);
    return V(s.saddleX + u * (s.nutX - s.saddleX), y, z0 - 0.001 * u);
  }

  frame(local) {
    const d = this.d, g = d.geometry;
    const t = d.window[0] + local;
    const at = (series) => sample(series, this.t0, this.hop, t);
    // a cut, then a short brightening (never from black)
    const fadeIn = Math.min(1, 0.6 + local / 0.5);

    // ---- the room: a floor graticule under the guitar, lit by the room field's energy
    const roomLv = level(at(d.room), this.max.room, 30);
    this.room.begin();
    const zf = -g.depth - 0.03;
    const kR = (0.03 + 0.09 * roomLv) * fadeIn;
    // the graticule stays out of the panel: each line is cut where it leaves the scene's box
    const box = this.variant === '4x5' ? { x0: 0, x1: this.width, y0: 236, y1: 850 } : { x0: 0, x1: 1240, y0: 200, y1: 880 };
    const grid = (a, b) => { const c = clipToBox(a, b, this.camera, this.width, this.height, box); if (c) this.room.seg(c[0], c[1], C.blue, kR); };
    for (let k = 0; k <= 12; k++) {
      const x = -0.39 + k * 0.065;
      grid(V(x, -0.13, zf), V(x, 1.17, zf));
    }
    for (let k = 0; k <= 20; k++) {
      const y = -0.13 + k * 0.065;
      grid(V(-0.39, y, zf), V(0.39, y, zf));
    }
    this.room.end();

    // ---- the body, neck, bridge: structure
    const S = this.struct.begin();
    const kS = 0.55 * fadeIn;
    S.poly(this.outline, C.dim, kS, true);
    S.poly(this.back, C.line, kS * 0.9, true);
    for (let i = 0; i < this.outline.length; i += 19) S.seg(this.outline[i], this.back[i], C.line, kS * 0.8);
    const hole = g.soundhole;
    const hc = V(hole.center[0], hole.center[1], 0);
    S.circle(hc, hole.radius, V(1, 0, 0), V(0, 1, 0), C.dim, kS, 64);
    S.circle(hc, hole.radius + 0.008, V(1, 0, 0), V(0, 1, 0), C.line, kS, 64);
    S.circle(hc, hole.radius + 0.014, V(1, 0, 0), V(0, 1, 0), C.line, kS, 64);
    const br = g.bridge;
    const by0 = br.y - br.depth / 2 - 0.004, by1 = br.y + br.depth / 2 - 0.004;
    S.poly([V(-br.halfwidth, by0, 0.002), V(br.halfwidth, by0, 0.002), V(br.halfwidth, by1, 0.002), V(-br.halfwidth, by1, 0.002)], C.dim, kS, true);
    S.seg(V(-0.0375, g.saddleY, 0.011), V(0.0375, g.saddleY, 0.011), C.text, kS);
    const nk = g.neck, wj = nk.width_at_joint / 2, wn = nk.width_at_nut / 2;
    const boardEnd = nk.frets_y[19] - 0.006;
    const halfAt = (y) => wn + (wj - wn) * (g.nutY - y) / (g.nutY - nk.joint_y);
    S.poly([V(-halfAt(boardEnd), boardEnd, 0.009), V(-wn, g.nutY, 0.009), V(wn, g.nutY, 0.009), V(halfAt(boardEnd), boardEnd, 0.009)], C.dim, kS, false);
    S.seg(V(-halfAt(boardEnd), boardEnd, 0.009), V(halfAt(boardEnd), boardEnd, 0.009), C.dim, kS);
    for (const y of nk.frets_y) S.seg(V(-halfAt(y), y, 0.0102), V(halfAt(y), y, 0.0102), C.line, kS * 1.2);
    S.seg(V(-wn, g.nutY + 0.002, 0.011), V(wn, g.nutY + 0.002, 0.011), C.text, kS);
    // headstock
    const hy0 = g.nutY + 0.005, hy1 = g.nutY + 0.185;
    S.poly([V(-wn, hy0, 0.004), V(-0.043, hy0 + 0.05, 0.004), V(-0.041, hy1, 0.004), V(0.041, hy1, 0.004), V(0.043, hy0 + 0.05, 0.004), V(wn, hy0, 0.004)], C.line, kS, false);
    for (const s of d.strings) {
      const side = s.number >= 4 ? -1 : 1, k = s.number >= 4 ? s.number - 4 : s.number - 1;
      S.circle(V(side * 0.03, hy0 + 0.045 + 0.045 * k, 0.005), 0.005, V(1, 0, 0), V(0, 1, 0), C.line, kS, 16);
    }
    S.end();

    // ---- the top: real mode shapes at their real energies
    const ampRow = sample_row(d.modes.amp, this.t0, this.hop, t);
    const sumNow = ampRow.reduce((a, v) => a + v, 0);
    const zmax = 0.045; // the loudest moment of the window swings 45 mm (exaggerated)
    // drawn size grows as the 0.4 power of the summed amplitudes (quiet moments stay visible)
    const gain = sumNow > 0 ? (zmax / this.maxAmpSum) * (this.maxAmpSum / sumNow) ** 0.6 : 0;
    const coef = ampRow.map((a, m) => gain * a * Math.cos(2 * Math.PI * this.fvis[m] * t + 0.7 * d.modes.chosen[m]));
    const topLv = level(at(d.top), this.max.top, 45);
    const T = this.top.begin();
    for (const line of d.grid) {
      const pts = [], ks = [];
      for (let i = 0; i < line.xy.length; i++) {
        const sh = line.shape[i];
        let z = 0;
        for (let m = 0; m < coef.length; m++) z += coef[m] * sh[m];
        pts.push(V(line.xy[i][0], line.xy[i][1], z));
        ks.push(Math.min(0.95, 0.08 + 0.36 * topLv + 30 * Math.abs(z)) * fadeIn);
      }
      T.poly(pts, C.world, ks);
    }
    T.end();

    // ---- strings
    const St = this.strings.begin();
    const M = this.marks.begin();
    let into = 0;
    for (const s of d.strings) {
      const e = at(s.energy);
      const lv = level(e, this.maxString, 45);
      const fret = this.stopAt(s.number, t);
      const yStop = fret > 0 ? this.yOfFret(fret) : g.nutY;
      const amp = 0.008 * lv;
      const phase = 2 * Math.PI * 7.5 * t + s.number * 1.3;
      const N = 40;
      const cur = [], up = [], dn = [];
      for (let i = 0; i <= N; i++) {
        const u = i / N, y = g.saddleY + u * (yStop - g.saddleY);
        const base = this.stringPoint(s, y);
        const env = amp * Math.sin(Math.PI * u);
        cur.push(base.clone().add(V(0, 0, env * Math.cos(phase))));
        up.push(base.clone().add(V(0, 0, env)));
        dn.push(base.clone().add(V(0, 0, -env)));
      }
      St.poly([this.stringPoint(s, g.saddleY), this.stringPoint(s, g.nutY)], C.dim, 0.16 * fadeIn);
      if (lv > 0.01) {
        St.poly(cur, C.world, 0.25 + 0.6 * lv);
        St.poly(up, C.world, 0.16 * lv);
        St.poly(dn, C.world, 0.16 * lv);
      }
      // the stop the tab names
      if (fret > 0.01) {
        const yf = this.yOfFret(fret) + 0.35 * (this.yOfFret(Math.max(0, fret - 1)) - this.yOfFret(fret));
        const p = this.stringPoint(s, yf, 0.016);
        M.circle(p, 0.0042, V(1, 0, 0), V(0, 1, 0), C.text, 0.85, 20);
      }
      // energy per second crossing this string's bridge edges into the box
      const flow = Math.max(0, at(s.intoBox));
      into += flow;
      const fl = level(flow, this.maxInto, 40);
      if (fl > 0.02) {
        const p = this.stringPoint(s, g.saddleY, 0.011);
        M.seg(p, p.clone().add(V(0, 0, 0.006 + 0.05 * fl)), C.world, 0.5 + 0.9 * fl);
      }
    }
    // touches: amber, at the pluck point (or the fret)
    for (const tc of this.touches) {
      const age = t - tc.seconds;
      if (age < 0 || age > 0.45) continue;
      const s = d.strings.find((x) => x.number === tc.string);
      const fret = tc.fret;
      const yStop = fret > 0 ? this.yOfFret(fret) : g.nutY;
      const atFret = tc.kind !== 'pluck';
      const y = atFret ? this.yOfFret(fret) + 0.35 * (this.yOfFret(Math.max(0, fret - 1)) - this.yOfFret(fret))
        : g.saddleY + tc.pos * (yStop - g.saddleY);
      const p = this.stringPoint(s, y, 0.014);
      const k = (1 - age / 0.45) ** 2 * (0.55 + 0.45 * Math.max(0, Math.min(1, (tc.db + 30) / 26)));
      const r = 0.004 + 0.022 * Math.sqrt(age / 0.45);
      if (tc.hand.endsWith('.p')) {
        M.poly([p.clone().add(V(r, 0, 0)), p.clone().add(V(0, r, 0)), p.clone().add(V(-r, 0, 0)), p.clone().add(V(0, -r, 0))], C.warn, 1.4 * k, true);
      } else {
        M.circle(p, r, V(1, 0, 0), V(0, 1, 0), C.warn, 1.4 * k, 28);
      }
    }
    this.strings.end();
    this.marks.end();

    // ---- the air in the box and the radiation leaving it
    const airLv = level(at(d.air), this.max.air, 40);
    const A = this.air.begin();
    if (airLv > 0.01) {
      for (const [r, z, k] of [[0.62, -0.012, 1.0], [0.42, -0.035, 0.75], [0.25, -0.06, 0.5]]) {
        A.circle(V(hc.x, hc.y, z), hole.radius * r * (0.75 + 0.35 * airLv), V(1, 0, 0), V(0, 1, 0), C.blue, (0.2 + 0.9 * airLv) * k, 48);
      }
    }
    A.end();
    const Sh = this.shells.begin();
    const life = 1.0;
    for (const pk of this.picks) {
      const age = t - pk.t;
      if (age < 0 || age > life) continue;
      const r = 0.03 + 0.30 * (age / life);
      const k = 0.42 * pk.strength * (1 - age / life) ** 2;
      Sh.circle(hc, r, V(1, 0, 0), V(0, 1, 0), C.blue, k, 72);
      Sh.circle(V(hc.x, hc.y, r * 0.5), r * 0.866, V(1, 0, 0), V(0, 1, 0), C.blue, 0.55 * k, 64);
      for (const a of [0, Math.PI / 3, (2 * Math.PI) / 3]) {
        const u = V(Math.cos(a), Math.sin(a), 0);
        Sh.circle(hc, r, u, V(0, 0, 1), C.blue, 0.55 * k, 36, 0, Math.PI);
      }
    }
    Sh.end();

    // ---- the panel
    const li = Math.max(0, Math.min(d.ledger.length - 1, Math.round((t - this.t0) / this.hop)));
    const L = d.ledger[li];
    const col = (name) => L[d.ledgerColumns.indexOf(name)];
    const counts = { pluck: 0, 'hammer-on': 0, 'pull-off': 0, slide: 0 };
    for (const tc of this.touches) if (tc.seconds <= t && tc.seconds >= d.window[0]) counts[tc.kind]++;
    const r = d.receipt;
    return {
      readouts: [],
      chain: [
        meter('STRINGS', 'w', at(this.sumStrings), this.max.strings, recentMax(this.sumStrings, this.t0, this.hop, t, 0.4), fmtJ),
        meter('BRIDGE', 'w', at(this.sumInto), this.max.into, recentMax(this.sumInto, this.t0, this.hop, t, 0.4), fmtW),
        meter('TOP', 'w', at(d.top), this.max.top, recentMax(d.top, this.t0, this.hop, t, 0.4), fmtJ),
        meter('AIR', 'b', at(d.air), this.max.air, recentMax(d.air, this.t0, this.hop, t, 0.4), fmtJ),
        meter('ROOM', 'b', at(d.room), this.max.room, recentMax(d.room, this.t0, this.hop, t, 0.4), fmtJ, 30),
      ],
      caption: captionHtml(this.seg.caption),
      ledger: [
        { k: 'IN (HANDS)', v: fmtJ(col('sourceWork')), c: 'a' },
        { k: 'LOST INSIDE', v: fmtJ(col('internalLoss')), c: 'w' },
        { k: 'RADIATED', v: fmtJ(col('radiation')), c: 'b' },
        { k: 'RESIDUAL', v: fmtJ(col('cumulativeClosureResidual')), c: 'g' },
      ],
      foot: `fretting hand ${fmtJ(col('controlWork'), 2)} · worst frame ${fmtJ(col('maximumFrameResidual'), 2)} · ${counts.pluck + counts['hammer-on'] + counts['pull-off'] + counts.slide} touches so far`,
      mus: musModel(d.bars, t, this.variant),
    };
  }
}

// the part of segment a-b whose projection lies inside a screen box (pixels), or null
function clipToBox(a, b, camera, w, h, box) {
  const inside = (p) => { const q = p.clone().project(camera); const x = (q.x + 1) / 2 * w, y = (1 - q.y) / 2 * h; return x >= box.x0 && x <= box.x1 && y >= box.y0 && y <= box.y1; };
  const N = 48;
  let first = -1, last = -1;
  for (let i = 0; i <= N; i++) if (inside(a.clone().lerp(b, i / N))) { if (first < 0) first = i; last = i; }
  if (first < 0 || last <= first) return null;
  return [a.clone().lerp(b, first / N), a.clone().lerp(b, last / N)];
}

function sample_row(rows, t0, dt, t) {
  const f = Math.max(0, Math.min(rows.length - 1, (t - t0) / dt));
  const i = Math.min(Math.floor(f), rows.length - 2), w = f - i;
  return rows[i].map((a, k) => a * (1 - w) + rows[i + 1][k] * w);
}

// "string → bridge → top → air → room": each stage word in its colour
function captionHtml(text) {
  const colour = { string: 'w', bridge: 'w', top: 'w', air: 'b', room: 'b' };
  return text.replace(/\b(string|bridge|top|air|room)\b/g, (m) => `<span class="${colour[m]}">${m}</span>`)
    .replace(/→/g, '<span class="d">→</span>');
}

function musModel(bars, t, variant) {
  if (!bars.length) return null;
  let bar = bars[0];
  for (const b of bars) if (b.start <= t + 1e-6) bar = b;
  const lines = [];
  for (const v of bar.voices) {
    if (v.tokens.length === 1 && v.tokens[0] === '-') continue;
    const parts = [{ t: v.hand + '=', cls: 'vo' }];
    // the token sounding now: the last one whose onset has passed
    let now = -1;
    v.onsets.forEach((o, i) => { if (o !== null && o <= t + 1e-6) now = i; });
    v.tokens.forEach((tk, i) => {
      const cls = i === now ? 'tk now' : (v.onsets[i] !== null && v.onsets[i] < t) ? 'tk past' : 'tk';
      parts.push({ t: (i ? ' ' : '') + tk, cls: tk.startsWith('{') ? 'd' : cls });
    });
    lines.push(parts);
  }
  return { hd: `MUS · wayfaring-stranger-doc.mus · bar ${bar.bar} ${bar.attrs}`, lines };
}
