// The piano: 88 keys, a felt hammer and a string for every note the notes table plays, the
// grand's case and soundboard. Driven by the notes table (onset s, midi, velocity, release s):
// a key is down from onset to release; its hammer is drawn reaching the string at the onset
// and falling back (the hammer's path is a drawing timed by the table, not simulated).
//
// While the Ariadne piano World is not rendered, the table is a synthetic PLACEHOLDER and
// there is no energy: the strings then show only that they were struck (dim, decaying with
// a fixed drawing envelope), the panel says PLACEHOLDER, and the segment is silent. With the
// World's energy file the strings, soundboard and ledger are its real numbers (see
// energyAdapter below; written against the expected file, to be checked when it lands).
import * as THREE from 'three';
import { Strokes, PALETTE as C, level, V, planeCamera } from '../lib/draw.js';
import { fmtJ, meter } from '../lib/hud.js';

const WHITE = 0.0235, KEY_L = 0.15, BLACK_W = 0.0137, BLACK_L = 0.095;
const isBlack = (m) => [1, 3, 6, 8, 10].includes(m % 12);
const NAMES = ['C', 'C♯', 'D', 'E♭', 'E', 'F', 'F♯', 'G', 'A♭', 'A', 'B♭', 'B'];
const noteName = (m) => `${NAMES[m % 12]}${Math.floor(m / 12) - 1}`;

// x of each key's centre (white keys evenly spaced from A0)
function keyX() {
  const x = {};
  let w = 0;
  for (let m = 21; m <= 108; m++) {
    if (!isBlack(m)) { x[m] = w * WHITE; w++; } else x[m] = (w - 0.5) * WHITE;
  }
  const mid = ((w - 1) * WHITE) / 2;
  for (const m in x) x[m] -= mid;
  return { x, half: (w * WHITE) / 2 };
}
// speaking length (m): a grand's scale, foreshortened in the bass (drawing, not the World)
const stringLength = (m) => Math.min(1.72, 0.052 * 2 ** (((108 - m) / 12) * 0.93));

export class PianoScene {
  constructor({ seg, data, variant, width, height }) {
    this.seg = seg; this.d = data; this.variant = variant; this.width = width; this.height = height;
    this.scene = new THREE.Scene();
    this.camera = new THREE.PerspectiveCamera(30, width / height, 0.01, 50);
    const root = new THREE.Group();
    this.scene.add(root);
    this.case = new Strokes(root, { width: 1.3, glow: 0.4 });
    this.keys = new Strokes(root, { width: 1.3, glow: 0.35 });
    this.strings = new Strokes(root, { width: 1.5, glow: 0.7 });
    this.hammers = new Strokes(root, { width: 1.8, glow: 0.9 });
    const { x, half } = keyX();
    this.kx = x; this.half = half;
    const vmax = Math.max(...data.notes.map((n) => n.velocity));
    this.vscale = vmax > 1.5 ? 1 / 127 : 1;
    this.played = [...new Set(data.notes.map((n) => n.midi))].sort((a, b) => a - b);
    this.energy = energyAdapter(data.energy);
    this.layout();
  }

  layout() {
    const w = this.width, h = this.height;
    if (this.variant === '4x5') planeCamera(this.camera, { target: V(0.0, 0.85, 0), angle: 90, tilt: 50, r: 5.6, fov: 30, w, h, cx: 540, cy: 520 });
    else planeCamera(this.camera, { target: V(0.0, 0.85, 0), angle: 90, tilt: 54, r: 4.4, fov: 30, w, h, cx: 640, cy: 520 });
  }

  keyDepth(n, t) {
    // key travel: down over 15 ms before the onset, back up over 40 ms after the release
    if (t < n.onset - 0.015 || t > n.release + 0.04) return 0;
    if (t < n.onset) return (t - (n.onset - 0.015)) / 0.015;
    if (t > n.release) return 1 - (t - n.release) / 0.04;
    return 1;
  }

  frame(local) {
    const d = this.d;
    const t = d.window[0] + local;
    // a cut, then a short brightening (never from black)
    const fadeIn = Math.min(1, 0.6 + local / 0.5);
    const K = this.keys.begin(), Cs = this.case.begin(), St = this.strings.begin(), H = this.hammers.begin();
    const down = {};
    for (const n of d.notes) down[n.midi] = Math.max(down[n.midi] ?? 0, this.keyDepth(n, t));
    // keys: white then black, the front edge dips 10 mm when down
    for (let m = 21; m <= 108; m++) {
      const b = isBlack(m), wdt = b ? BLACK_W : WHITE * 0.92, len = b ? BLACK_L : KEY_L;
      const x = this.kx[m], z0 = b ? 0.012 : 0, dip = -0.01 * (down[m] ?? 0);
      const p = [V(x - wdt / 2, -len, z0 + dip), V(x + wdt / 2, -len, z0 + dip), V(x + wdt / 2, 0, z0), V(x - wdt / 2, 0, z0)];
      const lit = down[m] ?? 0;
      K.poly(p, lit > 0 ? C.warn : (b ? C.dim : C.text), (lit > 0 ? 0.5 + 0.6 * lit : b ? 0.45 : 0.32) * fadeIn, true);
    }
    // the case: keybed, cheeks, the bentside and tail of a grand (drawing)
    const hw = this.half + 0.03;
    const outline = [V(-hw, 0.02, 0), V(-hw, 1.78, 0), V(-hw + 0.25, 1.98, 0), V(-0.05, 1.92, 0), V(0.32, 1.35, 0), V(hw - 0.05, 0.62, 0), V(hw, 0.02, 0)];
    Cs.poly(outline, C.dim, 0.45 * fadeIn, true);
    Cs.seg(V(-hw, -KEY_L - 0.01, 0), V(hw, -KEY_L - 0.01, 0), C.line, 0.6 * fadeIn);
    Cs.seg(V(-hw, -KEY_L - 0.01, 0), V(-hw, 0.02, 0), C.line, 0.6 * fadeIn);
    Cs.seg(V(hw, -KEY_L - 0.01, 0), V(hw, 0.02, 0), C.line, 0.6 * fadeIn);
    // the soundboard's bridge: a curve under the strings' far ends
    const bridge = this.played.length ? [] : null;
    const sbLv = this.energy ? level(this.energy.board(t), this.energy.boardMax, 40) : 0;
    for (let m = 21; m <= 108; m += 3) bridge?.push(V(this.kx[m], 0.06 + stringLength(m), -0.01));
    if (bridge) Cs.poly(bridge, this.energy ? C.world : C.line, (0.3 + 0.7 * sbLv) * fadeIn);
    // strings and hammers for the notes the table plays
    for (const m of this.played) {
      const x = this.kx[m], L = stringLength(m);
      let lv = 0;
      if (this.energy) lv = level(this.energy.string(m, t), this.energy.stringMax, 45);
      else {
        // PLACEHOLDER only: no energy exists yet, so a struck string fades on a fixed envelope
        for (const n of d.notes) if (n.midi === m && t >= n.onset) {
          const age = t - n.onset, held = t <= n.release;
          lv = Math.max(lv, n.velocity * this.vscale * Math.exp(-age / (held ? 2.5 : 0.25)) * 0.6);
        }
      }
      St.seg(V(x, 0.04, 0.02), V(x, 0.06 + L, 0.02), lv > 0.01 ? C.world : C.line, (0.25 + 0.75 * lv) * fadeIn);
      // hammer: rest 18 mm under the string; it reaches the string at the onset
      let lift = 0;
      for (const n of d.notes) if (n.midi === m) {
        const a = t - n.onset;
        if (a >= -0.03 && a < 0) lift = Math.max(lift, 1 + a / 0.03);
        else if (a >= 0 && a < 0.12) lift = Math.max(lift, 1 - a / 0.12);
      }
      const hz = 0.02 - 0.03 + 0.03 * lift - 0.006, hy = 0.075;
      H.circle(V(x, hy, hz), 0.006, V(0, 1, 0), V(0, 0, 1), lift > 0.05 ? C.warn : C.dim, (0.35 + 0.8 * lift) * fadeIn, 16);
      H.seg(V(x, hy - 0.006, hz - 0.004), V(x, hy - 0.09, hz - 0.012), C.line, 0.5 * fadeIn);
    }
    this.keys.end(); this.case.end(); this.strings.end(); this.hammers.end();

    const active = d.notes.filter((n) => t >= n.onset && t <= n.release).map((n) => noteName(n.midi));
    const placeholder = d.placeholder;
    return {
      flag: placeholder ? 'PLACEHOLDER · SILENT' : null,
      readouts: [{ k: 'KEYS DOWN', v: active.length ? active.slice(-4).join(' ') : '–' }],
      chain: placeholder ? ['HAMMERS', 'STRINGS', 'SOUNDBOARD', 'ROOM'].map((k) => ({ k, c: 'd', lv: 0, pk: 0, v: 'no World yet' })) : this.energy.chain(t),
      caption: this.seg.caption,
      ledger: placeholder
        ? [{ k: 'IN', v: '—', c: 'd' }, { k: 'LOST INSIDE', v: '—', c: 'd' }, { k: 'RADIATED', v: '—', c: 'd' }, { k: 'RESIDUAL', v: '—', c: 'd' }]
        : this.energy.ledger(t),
      foot: placeholder ? 'no ledger: nothing has been rendered for this segment' : this.energy.foot(t),
      mus: {
        hd: placeholder ? 'NOTES · placeholder/piano-demo-notes.PLACEHOLDER.tsv (synthetic)' : 'NOTES · piano-demo-notes.tsv',
        lines: d.notes.filter((n) => n.onset <= t + 1e-6).slice(-3).map((n) => [{ t: `${n.onset.toFixed(2)} s  ${noteName(n.midi).padEnd(3)} vel ${n.velocity}  release ${n.release.toFixed(2)} s`, cls: t <= n.release ? 'tk now' : 'tk past' }]),
      },
    };
  }
}

// The Ariadne piano's energy file, once it exists. Until then: null.
function energyAdapter(e) {
  if (!e) return null;
  throw new Error('piano energy file present: write its adapter against the real schema');
}
