// Vector drawing for the video: every mark is a stroke (three.js fat lines,
// LineSegments2), drawn additively on black like a phosphor trace, with a
// wider, dimmer copy of the same geometry under it for the bloom. Colour is
// meaning (see PALETTE); brightness is per vertex.
import * as THREE from 'three';
import { LineSegments2 } from 'three/addons/lines/LineSegments2.js';
import { LineSegmentsGeometry } from 'three/addons/lines/LineSegmentsGeometry.js';
import { LineMaterial } from 'three/addons/lines/LineMaterial.js';

// The site's palette (ovillo-site site/site.css), plus the tract page's pink
// and the mark's rose. What each colour means is fixed for the whole video.
export const PALETTE = {
  bg: 0x000000,
  line: 0x2c343c, // structure, graticule
  text: 0xc9d1c9,
  dim: 0x6d776f,
  on: 0x62d26f, // books closed
  warn: 0xffb000, // a touch: source work going in
  world: 0xff9d3b, // energy stored in a body
  blue: 0x5aa0ff, // air, radiation, the room
  pink: 0xff8fa6, // the tract's flesh (after the tract page)
  rose: 0xfda4af, // the Ovillo mark
};

// Widths are CSS pixels: LineSegments2.onBeforeRender sets the material's resolution to the
// renderer's viewport, which three.js keeps in CSS pixels whatever the pixel ratio, so a
// frame captured at deviceScaleFactor 2 is the same picture, supersampled.
function material(width, opacity) {
  const m = new LineMaterial({
    linewidth: width,
    vertexColors: true,
    transparent: true,
    opacity,
    depthTest: false,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
    worldUnits: false,
  });
  return m;
}

// A batch of line segments that is rebuilt every frame: begin(), then
// seg()/poly()/circle() as many times as needed, then end().
export class Strokes {
  constructor(parent, { width = 2, glow = 1, renderOrder = 0 } = {}) {
    this.geometry = new LineSegmentsGeometry();
    this.core = new LineSegments2(this.geometry, material(width, 1));
    this.core.frustumCulled = false;
    this.core.renderOrder = renderOrder + 2;
    parent.add(this.core);
    this.halos = [];
    if (glow > 0) {
      for (const [w, o] of [[width * 3.0, 0.10 * glow], [width * 7.0, 0.035 * glow]]) {
        const h = new LineSegments2(this.geometry, material(w, o));
        h.frustumCulled = false;
        h.renderOrder = renderOrder;
        parent.add(h);
        this.halos.push(h);
      }
    }
    this.pos = [];
    this.col = [];
    this.visible = true;
  }

  begin() { this.pos.length = 0; this.col.length = 0; return this; }

  // one segment a-b, colour c (hex) at brightness k (0..)
  seg(a, b, c, k = 1, k2 = k) {
    if (k <= 0.002 && k2 <= 0.002) return;
    this.pos.push(a.x, a.y, a.z, b.x, b.y, b.z);
    const r = ((c >> 16) & 255) / 255, g = ((c >> 8) & 255) / 255, bl = (c & 255) / 255;
    this.col.push(r * k, g * k, bl * k, r * k2, g * k2, bl * k2);
  }

  // a polyline through points (array of Vector3), brightness per point (number or array)
  poly(points, c, k = 1, closed = false) {
    const n = points.length;
    for (let i = 0; i < n - 1 + (closed ? 1 : 0); i++) {
      const j = (i + 1) % n;
      const ki = typeof k === 'number' ? k : k[i];
      const kj = typeof k === 'number' ? k : k[j];
      this.seg(points[i], points[j], c, ki, kj);
    }
  }

  // a circle of radius r around centre, in the plane spanned by unit vectors u, v
  circle(centre, r, u, v, c, k = 1, n = 48, from = 0, to = Math.PI * 2) {
    let prev = null;
    for (let i = 0; i <= n; i++) {
      const a = from + ((to - from) * i) / n;
      const p = new THREE.Vector3().copy(centre).addScaledVector(u, r * Math.cos(a)).addScaledVector(v, r * Math.sin(a));
      if (prev) this.seg(prev, p, c, k);
      prev = p;
    }
  }

  end() {
    const vis = this.visible && this.pos.length > 0;
    this.core.visible = vis;
    for (const h of this.halos) h.visible = vis;
    if (!vis) return;
    this.geometry.setPositions(new Float32Array(this.pos));
    this.geometry.setColors(new Float32Array(this.col));
    this.geometry.computeBoundingSphere();
  }
}

// Energy (or power) to 0..1 on a log scale spanning `rangeDb` below `ref`.
export function level(x, ref, rangeDb = 50) {
  if (!(x > 0) || !(ref > 0)) return 0;
  return Math.max(0, Math.min(1, 1 + (10 * Math.log10(x / ref)) / rangeDb));
}

// Linear interpolation into an evenly sampled series (t0, dt).
export function sample(series, t0, dt, t) {
  const f = (t - t0) / dt;
  if (f <= 0) return series[0];
  const n = series.length;
  if (f >= n - 1) return series[n - 1];
  const i = Math.floor(f), w = f - i;
  return series[i] * (1 - w) + series[i + 1] * w;
}

// Same, for rows of arrays (returns a new array).
export function sampleRow(rows, t0, dt, t) {
  const f = Math.max(0, Math.min(rows.length - 1, (t - t0) / dt));
  const i = Math.min(Math.floor(f), rows.length - 2), w = f - i;
  const a = rows[i], b = rows[i + 1];
  const out = new Array(a.length);
  for (let k = 0; k < a.length; k++) out[k] = a[k] * (1 - w) + b[k] * w;
  return out;
}

// Max of a series over [t-a, t] (peak hold, a CRT-native persistence).
export function recentMax(series, t0, dt, t, a) {
  const i1 = Math.min(series.length - 1, Math.floor((t - t0) / dt));
  const i0 = Math.max(0, Math.floor((t - a - t0) / dt));
  let m = 0;
  for (let i = i0; i <= i1; i++) m = Math.max(m, series[i]);
  return m;
}

export const V = (x = 0, y = 0, z = 0) => new THREE.Vector3(x, y, z);
export const smoothstep = (x) => (x <= 0 ? 0 : x >= 1 ? 1 : x * x * (3 - 2 * x));

// A camera over an object lying in its own x-y plane (z out of it): the object's +y axis
// is drawn at `angle` degrees above screen-horizontal, the camera leans `tilt` degrees
// from the plane's normal toward screen-down, at distance r; the target lands at
// screen pixel (cx, cy) of a (w x h) frame.
export function planeCamera(cam, { target, angle, tilt, r, fov, w, h, cx, cy, roll = 0 }) {
  const a = (angle * Math.PI) / 180, k = (tilt * Math.PI) / 180;
  const up = new THREE.Vector3(-Math.cos(a), Math.sin(a), 0);
  const down = up.clone().multiplyScalar(-1);
  const dir = down.clone().multiplyScalar(Math.sin(k)).add(new THREE.Vector3(0, 0, Math.cos(k))).normalize();
  cam.fov = fov;
  cam.aspect = w / h;
  cam.position.copy(target).addScaledVector(dir, r);
  // screen-up: the in-plane up, tipped toward the normal so it is orthogonal to the view
  const upv = up.clone().multiplyScalar(Math.cos(k)).add(new THREE.Vector3(0, 0, Math.sin(k))).normalize();
  if (roll) upv.applyAxisAngle(dir, (roll * Math.PI) / 180);
  cam.up.copy(upv);
  cam.lookAt(target);
  cam.setViewOffset(w, h, w / 2 - cx, h / 2 - cy, w, h);
  cam.updateProjectionMatrix();
  cam.updateMatrixWorld(true);
}

// A camera orbiting `target`: azimuth about +y (0 = looking along -z), elevation, distance;
// the target lands at screen pixel (cx, cy).
export function orbitCamera(cam, { target, az, el, r, fov, w, h, cx, cy }) {
  const a = (az * Math.PI) / 180, e = (el * Math.PI) / 180;
  cam.fov = fov;
  cam.aspect = w / h;
  cam.position.set(target.x + r * Math.cos(e) * Math.sin(a), target.y + r * Math.sin(e), target.z + r * Math.cos(e) * Math.cos(a));
  cam.up.set(0, 1, 0);
  cam.lookAt(target);
  cam.setViewOffset(w, h, w / 2 - cx, h / 2 - cy, w, h);
  cam.updateProjectionMatrix();
  cam.updateMatrixWorld(true);
}
