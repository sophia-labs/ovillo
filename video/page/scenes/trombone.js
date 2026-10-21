// The trombone v1 World (one N-D body), drawn from its own trace
// (trombone_nd_solo ... trace: every 10 ms, read through non-loading observers).
//
// What drives what:
// - the bore: the plan's tube network, 124 volumes in five tubes, each ring at a volume's
//   centre with the radius of its declared area; the slide tube is 1.19 m + 2x long (the
//   plan's tube_length term), x the World's slide coordinate at that moment, and its
//   sections stretch uniformly (the model's "rubber slide"); the path it is laid along
//   (legs, crook, bow, bell section beside the slide) is a drawing, not the plan;
// - each ring's brightness: that volume's RMS air pressure over the 10 ms (the World's
//   modes at that volume, `SpaceBody::node_pressure`);
// - the lips: the flow valve's half-gap, its smallest and largest over the 10 ms, drawn
//   x12; their amber, the valve's mean flow (the lungs paying in);
// - blue wavefronts from the bell: the World's radiated power (its books' radiation,
//   differenced) at the moment each leaves; their spacing and speed are invented;
// - the ruler: the anatomy receipt's seven slide positions, the pointer the slide now.
import * as THREE from 'three';
import { Strokes, PALETTE as C, level, sample, sampleRow, recentMax, V, planeCamera } from '../lib/draw.js';
import { fmtJ, fmtW, meter, sci } from '../lib/hud.js';

const NAMES = ['C', 'C♯', 'D', 'E♭', 'E', 'F', 'F♯', 'G', 'A♭', 'A', 'B♭', 'B'];
const noteName = (m) => `${NAMES[m % 12]}${Math.floor(m / 12) - 1}`;
const D_SLIDE = 0.15 * 2 / Math.PI; // the crook: a half circle 0.15 m long (v0's declared crook)
const BELL_PLANE = { y: 0.085, z: -0.11 };
const X_BOW = -0.16;

// A path of straight pieces and arcs; point(s) and tangent(s) by arc length.
class Path {
  constructor() { this.pieces = []; this.length = 0; }
  line(a, b) { const L = a.distanceTo(b); this.pieces.push({ kind: 'line', a, b, L, s0: this.length }); this.length += L; return this; }
  // half circle from a to b, bulging along `out`
  half(a, b, out) {
    const c = a.clone().add(b).multiplyScalar(0.5), r = a.distanceTo(b) / 2;
    const u = a.clone().sub(c).divideScalar(r), v = out.clone().normalize();
    const L = Math.PI * r;
    this.pieces.push({ kind: 'arc', c, r, u, v, L, s0: this.length });
    this.length += L;
    return this;
  }
  at(s) {
    s = Math.max(0, Math.min(this.length - 1e-9, s));
    let p = this.pieces[this.pieces.length - 1];
    for (const q of this.pieces) if (s < q.s0 + q.L) { p = q; break; }
    const f = (s - p.s0) / p.L;
    if (p.kind === 'line') return { pos: p.a.clone().lerp(p.b, f), tan: p.b.clone().sub(p.a).normalize() };
    const a = Math.PI * f;
    const pos = p.c.clone().addScaledVector(p.u, p.r * Math.cos(a)).addScaledVector(p.v, p.r * Math.sin(a));
    const tan = p.u.clone().multiplyScalar(-Math.sin(a)).addScaledVector(p.v, Math.cos(a)).normalize();
    return { pos, tan };
  }
}

export class TromboneScene {
  constructor({ seg, data, variant, width, height }) {
    this.seg = seg; this.d = data; this.variant = variant; this.width = width; this.height = height;
    const d = data;
    this.c = Object.fromEntries(d.columns.map((k, i) => [k, i]));
    this.scene = new THREE.Scene();
    this.camera = new THREE.PerspectiveCamera(30, width / height, 0.01, 50);
    const root = new THREE.Group();
    this.scene.add(root);
    this.struct = new Strokes(root, { width: 1.3, glow: 0.4 });
    this.rings = new Strokes(root, { width: 1.4, glow: 0.45 });
    this.lips = new Strokes(root, { width: 2.2, glow: 1.2 });
    this.waves = new Strokes(root, { width: 1.6, glow: 0.8 });
    this.ruler = new Strokes(root, { width: 1.4, glow: 0.4 });
    this.radii = d.areasM2.map((a) => Math.sqrt(a / Math.PI));
    this.t0 = d.rows[0][this.c.t];
    this.hop = d.hop;
    const inWin = (r) => r[this.c.t] >= d.window[0] && r[this.c.t] <= d.window[1];
    const rowsW = d.rows.filter(inWin);
    this.pmax = Math.max(...d.pressureRmsPa.filter((_, i) => inWin(d.rows[i])).map((p) => Math.max(...p)));
    const col = (k) => d.rows.map((r) => r[this.c[k]]);
    this.series = {
      lungs: col('lungsJ'), flow: col('flowMeanM3s'), bore: col('boreEnergyJ'), lip: col('lipEnergyJ'),
      radiation: col('radiation'),
    };
    this.series.radW = this.series.radiation.map((v, i, a) => (i === 0 ? 0 : (v - a[i - 1]) / this.hop));
    const maxW = (k) => Math.max(...this.series[k].filter((_, i) => inWin(d.rows[i])));
    this.max = { lungs: maxW('lungs'), flow: maxW('flow'), bore: maxW('bore'), radW: maxW('radW') };
    this.layout();
  }

  layout() {
    const w = this.width, h = this.height;
    if (this.variant === '4x5') {
      planeCamera(this.camera, { target: V(0.72, -0.01, -0.05), angle: 66, tilt: -22, r: 4.7, fov: 30, w, h, cx: 590, cy: 540, roll: 0 });
    } else {
      planeCamera(this.camera, { target: V(0.72, -0.01, -0.05), angle: 76, tilt: -22, r: 3.3, fov: 30, w, h, cx: 640, cy: 540 });
    }
  }

  // the bore's centre line for slide extension x (m per leg)
  path(x) {
    const d = D_SLIDE, legEnd = 0.328 + 0.52 + x;
    const p = new Path()
      .line(V(0, 0, 0), V(0.088, 0, 0)) // mouthpiece
      .line(V(0.088, 0, 0), V(0.328, 0, 0)) // leadpipe
      .line(V(0.328, 0, 0), V(legEnd, 0, 0)) // slide, first leg
      .half(V(legEnd, 0, 0), V(legEnd, -d, 0), V(1, 0, 0)) // the crook
      .line(V(legEnd, -d, 0), V(0.328, -d, 0)) // second leg
      .line(V(0.328, -d, 0), V(X_BOW, -d, 0)); // bell tail: back past the player
    const bowTo = V(X_BOW, BELL_PLANE.y, BELL_PLANE.z);
    p.half(V(X_BOW, -d, 0), bowTo, V(-1, 0, 0));
    return p;
  }

  frame(local) {
    const d = this.d, c = this.c;
    const t = d.window[0] + local;
    const row = sampleRow(d.rows, this.t0, this.hop, t);
    const pres = sampleRow(d.pressureRmsPa, this.t0, this.hop, t);
    const x = row[c.slideM];
    // a cut, then a short brightening (never from black)
    const fadeIn = Math.min(1, 0.6 + local / 0.5);

    // ---- volume centres along the drawn path (tube by tube, sections uniform in each)
    const P = this.path(x);
    const lengths = d.tubes.map((tb, k) => (k === d.slideTerm.tube ? d.slideTerm.length_m.constant + 2 * x : tb.lengthM));
    // bell tail and bell continue forward from the bow
    const s3 = P.length; // where the bow ends
    const tailForward = lengths[3] - (s3 - (lengths[0] + lengths[1] + lengths[2]));
    const bowEnd = V(X_BOW, BELL_PLANE.y, BELL_PLANE.z);
    P.line(bowEnd, bowEnd.clone().add(V(tailForward, 0, 0)));
    const bellStart = bowEnd.clone().add(V(tailForward, 0, 0));
    P.line(bellStart, bellStart.clone().add(V(lengths[4], 0, 0)));
    const centres = [];
    let s0 = 0;
    d.tubes.forEach((tb, k) => {
      for (let j = 0; j < tb.sections; j++) centres.push(P.at(s0 + ((j + 0.5) * lengths[k]) / tb.sections));
      s0 += lengths[k];
    });
    // parallel-transported frames, so the long lines do not twist
    let n1 = V(0, 1, 0);
    const frames = centres.map(({ tan }) => {
      n1 = n1.clone().addScaledVector(tan, -n1.dot(tan)).normalize();
      return [n1, tan.clone().cross(n1).normalize()];
    });

    // ---- rings at every volume (the bell's coloured by pressure), and the pressure itself as a
    // trace riding outside the bore: its offset is each volume's RMS pressure on a log scale
    const R = this.rings.begin();
    const S = this.struct.begin();
    const ringPts = [];
    const nTail = d.tubes[0].sections + d.tubes[1].sections + d.tubes[2].sections;
    const backLen = 0.328 - X_BOW, bowLen = Math.PI * V(0, BELL_PLANE.y + D_SLIDE, BELL_PLANE.z).length() / 2;
    const trace = [], traceK = [];
    let sTail = 0;
    const pNow = Math.max(...pres), traceLv = level(pNow, this.pmax, 30);
    centres.forEach(({ pos, tan }, i) => {
      const [a, b] = frames[i];
      const r = this.radii[i];
      const lv = level(pres[i], this.pmax, 30);
      const bellish = r > 0.012;
      R.circle(pos, r, a, b, bellish ? C.world : C.dim, bellish ? (0.08 + 0.9 * lv * lv) * fadeIn : 0.28 * fadeIn, r > 0.03 ? 64 : 20);
      ringPts.push({ pos, a, b, r });
      // outward: the U of the slide from the transported frame; past the bow, up
      let out = a;
      if (i >= nTail) {
        const j = i - nTail;
        sTail = ((j + 0.5) * lengths[3]) / d.tubes[3].sections;
        if (i >= nTail + d.tubes[3].sections || sTail > backLen + bowLen) out = V(0, 1, 0).addScaledVector(tan, -tan.y).normalize();
        else if (sTail > backLen) out = pos.clone().sub(V(X_BOW, (BELL_PLANE.y - D_SLIDE) / 2, BELL_PLANE.z / 2)).normalize();
        else out = V(0, -1, 0);
      }
      // the shape on a linear scale against this moment's largest pressure (nodes dip to the
      // wall), its brightness the level against the window's loudest
      trace.push(pos.clone().addScaledVector(out, r + 0.004 + 0.07 * (pNow > 0 ? pres[i] / pNow : 0)));
      traceK.push((0.2 + 0.8 * traceLv) * fadeIn);
    });
    R.poly(trace, C.world, traceK);
    for (let j = 0; j < 4; j++) {
      const th = (j * Math.PI) / 2;
      const pts = ringPts.map(({ pos, a, b, r }) => pos.clone().addScaledVector(a, r * Math.cos(th)).addScaledVector(b, r * Math.sin(th)));
      S.poly(pts, C.dim, 0.38 * fadeIn);
    }
    // slide brace near the crook, and the hand stocking (drawing aids, structure colour)
    const legEnd = 0.328 + 0.52 + x;
    S.seg(V(legEnd - 0.06, 0.012, 0), V(legEnd - 0.06, -D_SLIDE - 0.012, 0), C.dim, 0.5 * fadeIn);
    S.seg(V(0.30, 0.012, 0), V(0.30, -D_SLIDE - 0.012, 0), C.dim, 0.35 * fadeIn);
    S.seg(V(0.30, 0.012, 0), V(0.20, BELL_PLANE.y - 0.012, BELL_PLANE.z), C.line, 0.4 * fadeIn);
    this.rings.end();
    this.struct.end();

    // ---- the lips: the valve's half-gap envelope over the hop (x12), amber with the flow
    const L = this.lips.begin();
    const flow = row[c.flowMeanM3s];
    const flv = level(flow, this.max.flow, 30);
    const gmax = row[c.halfGapMaxM] * 12, gmin = row[c.halfGapMinM] * 12;
    const lipW = 0.022, lx = -0.006;
    for (const [g, k] of [[gmax, 1.0], [gmin, 0.45]]) {
      const gy = Math.max(-0.002, g);
      L.seg(V(lx, gy, -lipW), V(lx, gy, lipW), C.warn, (0.25 + 0.9 * flv) * k * fadeIn);
      L.seg(V(lx, -gy, -lipW), V(lx, -gy, lipW), C.warn, (0.25 + 0.9 * flv) * k * fadeIn);
    }
    L.seg(V(lx - 0.03, gmax + 0.012, 0), V(lx, Math.max(gmax, 0) + 0.002, 0), C.dim, 0.5 * fadeIn);
    L.seg(V(lx - 0.03, -gmax - 0.012, 0), V(lx, -Math.max(gmax, 0) - 0.002, 0), C.dim, 0.5 * fadeIn);
    if (flv > 0.02) {
      for (const z of [-0.008, 0, 0.008]) L.seg(V(lx, 0, z), V(0.03, 0, z * 0.6), C.warn, 0.55 * flv);
    }
    this.lips.end();

    // ---- wavefronts leaving the bell
    const W = this.waves.begin();
    const bellEnd = centres[centres.length - 1].pos;
    const rb = this.radii[this.radii.length - 1];
    const every = 0.08, life = 0.6, speed = 0.32;
    for (let k = Math.floor((t - life) / every); k <= Math.floor(t / every); k++) {
      const te = k * every, age = t - te;
      if (age < 0 || age > life || te < this.t0) continue;
      const p = level(Math.max(0, sample(this.series.radW, this.t0, this.hop, te)), this.max.radW, 36);
      if (p <= 0.01) continue;
      const kk = 0.6 * p * (1 - age / life) ** 1.5;
      W.circle(bellEnd.clone().add(V(speed * age, 0, 0)), rb + 0.12 * age, V(0, 1, 0), V(0, 0, 1), C.blue, kk, 64);
    }
    this.waves.end();

    // ---- the slide ruler: the anatomy's seven positions, the slide now
    const U = this.ruler.begin();
    const ry = -D_SLIDE - 0.075, x0 = 0.328 + 0.52 + D_SLIDE / 2;
    U.seg(V(x0, ry, 0), V(x0 + 0.72, ry, 0), C.dim, 0.6 * fadeIn);
    d.positionsM.forEach((p, k) => U.seg(V(x0 + p, ry, 0), V(x0 + p, ry - 0.02, 0), C.dim, 0.7 * fadeIn));
    const px = x0 + x;
    U.poly([V(px, ry + 0.004, 0), V(px - 0.012, ry + 0.03, 0), V(px + 0.012, ry + 0.03, 0)], C.text, 0.95 * fadeIn, true);
    U.seg(V(px, ry + 0.03, 0), V(px, -D_SLIDE - 0.01, 0), C.text, 0.25 * fadeIn);
    this.ruler.end();

    // labels
    const labels = [];
    const proj = (v) => { const q = v.clone().project(this.camera); return { x: (q.x + 1) / 2 * this.width, y: (1 - q.y) / 2 * this.height }; };
    const nearest = d.positionsM.reduce((best, p, k) => (Math.abs(p - x) < Math.abs(d.positionsM[best] - x) ? k : best), 0);
    d.positionsM.forEach((p, k) => {
      const q = proj(V(x0 + p, ry - 0.045, 0));
      labels.push({ x: q.x, y: q.y, text: String(k + 1), c: k === nearest ? 't' : 'd', size: 18 });
    });
    const lq = proj(V(lx - 0.035, 0.05, 0));
    labels.push({ x: lq.x, y: lq.y, text: 'LIPS', c: 'a', align: 'r', alpha: fadeIn });
    const bq = proj(bellEnd.clone().add(V(0.02, rb + 0.06, 0)));
    labels.push({ x: bq.x, y: bq.y, text: 'BELL', c: 'b', alpha: fadeIn });
    const sq = proj(V(x0 - 0.06, ry - 0.045, 0));
    labels.push({ x: sq.x, y: sq.y, text: 'SLIDE POSITION', c: 'd', align: 'r', size: 16 });

    // ---- the panel
    const note = [...d.notes].reverse().find((n) => n.start <= t + 1e-6) ?? d.notes[0];
    const sounding = note && t < note.end + 0.05;
    const fx = [];
    if (note.fx & 1) fx.push('scoop'); if (note.fx & 2) fx.push('fall'); if (note.fx & 4) fx.push('vib');
    if (note.fx & 16) fx.push('rip'); if (note.fx & 32) fx.push('swell'); if (note.fx & 64) fx.push('dim');
    const at = (k) => sample(this.series[k], this.t0, this.hop, t);
    const recent = (k) => recentMax(this.series[k], this.t0, this.hop, t, 0.4);
    const idx = d.notes.indexOf(note);
    const rowsTxt = [d.notes[idx - 1], note, d.notes[idx + 1]].filter(Boolean).map((n) => {
      const nfx = [];
      if (n.fx & 1) nfx.push('scoop'); if (n.fx & 2) nfx.push('fall'); if (n.fx & 4) nfx.push('vib');
      if (n.fx & 16) nfx.push('rip'); if (n.fx & 32) nfx.push('swell'); if (n.fx & 64) nfx.push('dim');
      const txt = `${n.start.toFixed(2)} s  ${noteName(n.midi).padEnd(3)} partial ${String(n.partial).padStart(2)}  pos ${n.position}  ${(n.slide * 100).toFixed(1).padStart(4)} cm  ${n.art.toLowerCase()}${nfx.length ? ' ' + nfx.join(' ') : ''}`;
      return [{ t: txt, cls: n === note ? (sounding ? 'tk now' : 'tk past') : 'tk' }];
    });
    const rc = d.receipt;
    return {
      labels,
      readouts: [
        { k: 'NOTE', v: sounding ? noteName(note.midi) : '–' },
        { k: 'PARTIAL', v: sounding ? String(note.partial) : '–' },
        { k: 'SLIDE', v: `${(x * 100).toFixed(1)} cm` },
        { k: 'MEASURED', v: sounding ? `${note.cents >= 0 ? '+' : '−'}${Math.abs(note.cents).toFixed(1)} ¢` : '–', c: 'd' },
      ],
      chain: [
        meter('LUNGS', 'a', at('lungs'), this.max.lungs, recent('lungs'), fmtJ, 6),
        { ...meter('LIPS', 'a', at('flow'), this.max.flow, recent('flow'), (v) => `${(v * 1e6).toFixed(0)} cm³/s`, 30) },
        meter('BORE', 'w', at('bore'), this.max.bore, recent('bore'), fmtJ, 40),
        meter('BELL', 'b', Math.max(0, at('radW')), this.max.radW, recent('radW'), fmtW, 40),
      ],
      caption: this.seg.caption,
      ledger: [
        { k: 'IN (LUNGS)', v: fmtJ(row[c.sourceWork]), c: 'a' },
        { k: 'LOST INSIDE', v: fmtJ(row[c.internalLoss]), c: 'w' },
        { k: 'RADIATED', v: fmtJ(row[c.radiation]), c: 'b' },
        { k: 'RESIDUAL', v: fmtJ(row[c.cumulativeClosureResidual]), c: 'g' },
      ],
      foot: `slide + lips ${fmtJ(row[c.controlWork], 2)} · worst frame ${fmtJ(row[c.maximumFrameResidual], 2)} · solo: ${d.analysis.sounded_intended_partial}/${d.analysis.notes} on the intended partial`,
      mus: { hd: 'NOTES · trombone-nd-solo-notes.tsv · as the player placed them', lines: rowsTxt },
    };
  }
}
