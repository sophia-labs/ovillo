// The stage: seven guts (`TractNdAnatomy::fart_v1`, one World each) as flesh
// trombones in a concert hall, drawn every frame from the film's data
// (tools/prep.py: the stems' traces resampled to frames).
//
// What drives what (everything else is a drawing or a direction choice):
// - the canal: the 28 oral sections' areas (the World's network areas) as a
//   tube of rings along a trombone-ish path (the path is a drawing; radii are
//   drawn 2.5x their length-true size so a 1 cm² canal reads on screen), each
//   ring lit from inside by that section's RMS pressure over the frame;
// - the valve: two lips whose gap is the World's valve half-gap over the
//   frame (smallest on even frames, largest on odd: the envelope, x8 as in
//   the release video), amber with the flow through it;
// - the bladder: the reservoir, its size from the reservoir's energy (1.5 J
//   full), flushed by the supply pressure;
// - the flask: the stomach (its fill is the gas it still holds), beans drop
//   in when the part eats (eatenJ), bubbles rise while it digests;
// - the puffs leaving the mouth: the World's radiated power at the moment
//   each left (their drift is a drawing);
// - the clamp at 0.85 of the canal closes with the cheeks' clench (the seal
//   the player commands: tonguing, a shot loading), the one at 0.93 with the
//   rasp;
// - the amber spark: the turbulence the jet injected this frame, at the jet;
// - the camera shakes with the ensemble's radiated power.
import * as THREE from 'three';

const ROMAN = ['I', 'II', 'III', 'IV', 'V', 'VI', 'CANNON'];
const RING_N = 28;
const RADIUS_K = 1.6; // drawn radius over the length-true radius (a drawing choice, stated on screen)
const UNIT = 5.5; // scene units per metre of canal (so a 20 cm canal stands about 1.1 high)
const LUNG_J = 1.5;
const COL = {}; // frame column indices

const ease = (x) => (x <= 0 ? 0 : x >= 1 ? 1 : x * x * (3 - 2 * x));
const lerp = (a, b, t) => a + (b - a) * t;
const clamp01 = (x) => Math.max(0, Math.min(1, x));
const level = (x, ref, rangeDb = 40) => (!(x > 0) || !(ref > 0) ? 0 : clamp01(1 + (10 * Math.log10(x / ref)) / rangeDb));
const hash = (n) => { let x = (n | 0) * 2654435761; x ^= x >>> 15; x = (x * 2246822519) | 0; x ^= x >>> 13; return ((x >>> 0) % 100000) / 100000; };
const V3 = (x = 0, y = 0, z = 0) => new THREE.Vector3(x, y, z);

// Where each gut stands: x from the score's pan (the mix's), depth and size
// by register (a staging choice: highs front, basses back, the cannon on a
// riser at the rear).
function placement(voices, specs, anatomies) {
  const out = {};
  for (const v of voices) {
    const pan = specs[v].pan ?? 0;
    const k = voices.indexOf(v);
    const s = (anatomies[v]?.oralLengthM ?? 0.2) * UNIT; // the gut's size is its canal's length
    if (v === 'cannon') out[v] = { x: 0, z: -5.2, y: 0.9, s, length: anatomies[v]?.oralLengthM ?? 0.2 };
    else out[v] = { x: pan * 6.2, z: [0.6, 0.6, -0.9, -0.9, -2.6, -2.6][k], y: 0, s, length: anatomies[v]?.oralLengthM ?? 0.2 };
  }
  return out;
}

// The canal's path for a gut of scale s: from the valve above the bladder,
// up, then curling forward toward the audience and flaring. A drawing.
function canalPath(s) {
  return new THREE.CatmullRomCurve3([
    V3(0, 0.62 * s, 0), V3(0, 1.05 * s, -0.02 * s), V3(0, 1.42 * s, 0.14 * s), V3(0.02 * s, 1.58 * s, 0.46 * s), V3(0.04 * s, 1.52 * s, 0.82 * s),
  ], false, 'catmullrom', 0.5);
}

class Gut {
  constructor(stage, index, voice, place, spec) {
    this.stage = stage; this.index = index; this.voice = voice; this.place = place; this.spec = spec;
    const s = place.s;
    this.group = new THREE.Group();
    this.group.position.set(place.x, place.y, place.z);
    stage.scene.add(this.group);
    this.curve = canalPath(s);
    this.frames = this.curve.computeFrenetFrames(RING_N, false);
    this.pathLen = this.curve.getLength();
    // the canal: (RING_N + 1) rings x M vertices, rebuilt every frame
    const M = 28;
    this.M = M;
    const g = new THREE.BufferGeometry();
    const nv = (RING_N + 1) * M;
    g.setAttribute('position', new THREE.BufferAttribute(new Float32Array(nv * 3), 3));
    g.setAttribute('normal', new THREE.BufferAttribute(new Float32Array(nv * 3), 3));
    g.setAttribute('color', new THREE.BufferAttribute(new Float32Array(nv * 3), 3));
    const idx = [];
    for (let i = 0; i < RING_N; i++) for (let j = 0; j < M; j++) {
      const a = i * M + j, b = i * M + (j + 1) % M, c = (i + 1) * M + j, d = (i + 1) * M + (j + 1) % M;
      idx.push(a, c, b, b, c, d);
    }
    g.setIndex(idx);
    this.canalGeo = g;
    this.flesh = new THREE.MeshStandardMaterial({ color: 0xffffff, vertexColors: true, roughness: 0.36, metalness: 0.03, emissive: 0xff3a5a, emissiveIntensity: 0.0, side: THREE.DoubleSide });
    this.canal = new THREE.Mesh(g, this.flesh);
    this.group.add(this.canal);
    // the mouth: a dark disc just inside the last ring
    this.mouth = new THREE.Mesh(new THREE.CircleGeometry(1, 32), new THREE.MeshBasicMaterial({ color: 0x1a0609, side: THREE.DoubleSide }));
    this.group.add(this.mouth);
    // the lips of the valve
    const lipMat = new THREE.MeshStandardMaterial({ color: 0xc8445c, roughness: 0.35, emissive: 0xff7a00, emissiveIntensity: 0 });
    this.lips = [0, 1].map(() => { const m = new THREE.Mesh(new THREE.SphereGeometry(1, 20, 14), lipMat); this.group.add(m); return m; });
    this.lipMat = lipMat;
    // the bladder (reservoir)
    this.bladderMat = new THREE.MeshStandardMaterial({ color: 0xb8655e, roughness: 0.55, emissive: 0xff2a1a, emissiveIntensity: 0 });
    this.bladder = new THREE.Mesh(new THREE.SphereGeometry(1, 28, 20), this.bladderMat);
    this.group.add(this.bladder);
    // a brass collar where the canal meets the bladder (structure)
    const brass = new THREE.MeshStandardMaterial({ color: 0xc9a24a, roughness: 0.3, metalness: 0.9 });
    const collar = new THREE.Mesh(new THREE.TorusGeometry(0.19 * s, 0.03 * s, 10, 32), brass);
    collar.position.set(0, 0.62 * s, 0); collar.rotation.x = Math.PI / 2;
    this.group.add(collar);
    // the clamps (seal at 0.85, rasp at 0.93): two paddles each
    this.clamps = [0.85, 0.93].map(() => [0, 1].map(() => { const m = new THREE.Mesh(new THREE.BoxGeometry(0.05 * s, 0.26 * s, 0.14 * s), brass); this.group.add(m); return m; }));
    // the flask (digester) beside the gut
    const fx = (this.index % 2 ? 1 : -1) * 0.62 * s, fz = 0.25 * s;
    this.flaskAt = V3(fx, 0, fz);
    const glass = new THREE.MeshStandardMaterial({ color: 0x9fd6d0, roughness: 0.05, metalness: 0.1, transparent: true, opacity: 0.22, depthWrite: false });
    const flask = new THREE.Mesh(new THREE.CylinderGeometry(0.17 * s, 0.2 * s, 0.52 * s, 24, 1, true), glass);
    flask.position.set(fx, 0.26 * s, fz); this.group.add(flask);
    const base = new THREE.Mesh(new THREE.CylinderGeometry(0.2 * s, 0.22 * s, 0.03 * s, 24), brass); base.position.set(fx, 0.015 * s, fz); this.group.add(base);
    this.fill = new THREE.Mesh(new THREE.CylinderGeometry(0.165 * s, 0.195 * s, 1, 24), new THREE.MeshStandardMaterial({ color: 0x6f7a2e, roughness: 0.7, emissive: 0x334a10, emissiveIntensity: 0.3 }));
    this.group.add(this.fill);
    // beans and bubbles (instanced)
    this.beans = new THREE.InstancedMesh(new THREE.SphereGeometry(0.028 * s, 8, 6), new THREE.MeshStandardMaterial({ color: 0x7a4a2a, roughness: 0.6 }), 48);
    this.beans.count = 0; this.group.add(this.beans);
    this.bubbles = new THREE.InstancedMesh(new THREE.SphereGeometry(0.014 * s, 8, 6), new THREE.MeshBasicMaterial({ color: 0xd8ffd0, transparent: true, opacity: 0.55 }), 24);
    this.bubbles.count = 0; this.group.add(this.bubbles);
    // the puffs: radiation leaving the mouth over the last second
    this.puffs = new THREE.InstancedMesh(new THREE.IcosahedronGeometry(1, 1), new THREE.MeshBasicMaterial({ color: 0x8a9a52, transparent: true, opacity: 0.09, depthWrite: false }), 60);
    this.puffs.count = 0; this.group.add(this.puffs);
    // the jet spark
    this.spark = new THREE.Mesh(new THREE.SphereGeometry(0.035 * s, 10, 8), new THREE.MeshBasicMaterial({ color: 0xffb000, transparent: true, opacity: 0.9 }));
    this.group.add(this.spark);
    // the stand and its card (the label is HTML, projected from cardAt)
    const stand = new THREE.Mesh(new THREE.CylinderGeometry(0.01 * s, 0.01 * s, 0.9 * s, 8), brass);
    stand.position.set(-fx * 1.15, 0.45 * s, 0.6 * s); this.group.add(stand);
    const plate = new THREE.Mesh(new THREE.BoxGeometry(0.26 * s, 0.17 * s, 0.012 * s), new THREE.MeshStandardMaterial({ color: 0x1b1612, roughness: 0.8 }));
    plate.position.set(-fx * 1.15, 0.98 * s, 0.6 * s); plate.rotation.x = -0.35; this.group.add(plate);
    this.cardAt = V3(-fx * 1.15, 0.98 * s, 0.62 * s);
    // the spot over this gut
    this.spot = new THREE.SpotLight(0xffe2c0, 0, 14, 0.42, 0.55, 1.2);
    this.spot.position.set(place.x + 0.6, 6.5, place.z + 2.4);
    this.spot.target.position.set(place.x, 1.2 * s, place.z + 0.3);
    stage.scene.add(this.spot); stage.scene.add(this.spot.target);
    this.dummy = new THREE.Object3D();
    this.mouthWorld = V3();
  }

  // the frame's row for this gut
  row(f) { return this.stage.row(f, this.index); }

  update(f, t, pmax, radMax, turbMax, stomachMax, follow) {
    const r = this.row(f), s = this.place.s, C = COL;
    const pos = this.canalGeo.attributes.position.array, nor = this.canalGeo.attributes.normal.array, col = this.canalGeo.attributes.color.array;
    const M = this.M;
    const unit = UNIT; // scene units per metre: radii and the canal share one scale
    let hot = 0;
    const centre = V3(), nrm = V3(), bin = V3();
    for (let i = 0; i <= RING_N; i++) {
      const u = i / RING_N;
      const k = Math.min(RING_N - 1, i);
      const area = Math.max(r[C.area0 + k], 1e-7);
      const prms = r[C.prms0 + k];
      const radius = RADIUS_K * unit * Math.sqrt(area / Math.PI) * (i === RING_N ? 1.08 : 1);
      const p = this.curve.getPointAt(u, centre);
      const N = this.frames.normals[Math.min(i, RING_N)], B = this.frames.binormals[Math.min(i, RING_N)];
      const lv = level(prms, pmax, 36);
      hot = Math.max(hot, lv);
      // flesh: pink, warmer and brighter with pressure (the inside glows through)
      const cr = 0.78 + 0.30 * lv, cg = 0.34 + 0.20 * lv, cb = 0.40 + 0.10 * lv;
      for (let j = 0; j < M; j++) {
        const a = (j / M) * Math.PI * 2, ca = Math.cos(a), sa = Math.sin(a);
        // a little organic ribbing: rings bulge between sections (a drawing)
        const rib = 1 + 0.05 * Math.sin(a * 4 + i * 1.7);
        nrm.set(N.x * ca + B.x * sa, N.y * ca + B.y * sa, N.z * ca + B.z * sa);
        const o = (i * M + j) * 3;
        pos[o] = p.x + nrm.x * radius * rib; pos[o + 1] = p.y + nrm.y * radius * rib; pos[o + 2] = p.z + nrm.z * radius * rib;
        nor[o] = nrm.x; nor[o + 1] = nrm.y; nor[o + 2] = nrm.z;
        col[o] = cr; col[o + 1] = cg; col[o + 2] = cb;
      }
      void bin;
    }
    this.canalGeo.attributes.position.needsUpdate = true;
    this.canalGeo.attributes.normal.needsUpdate = true;
    this.canalGeo.attributes.color.needsUpdate = true;
    this.canalGeo.computeBoundingSphere();
    this.flesh.emissiveIntensity = 0.22 * hot * hot;
    // the mouth disc
    const end = this.curve.getPointAt(1), endT = this.curve.getTangentAt(1);
    const endR = RADIUS_K * unit * Math.sqrt(Math.max(r[C.area0 + RING_N - 1], 1e-7) / Math.PI);
    this.mouth.position.copy(end).addScaledVector(endT, -0.02 * s);
    this.mouth.scale.setScalar(endR * 0.98);
    this.mouth.lookAt(end.clone().add(endT));
    this.mouthWorld.copy(end).applyMatrix4(this.group.matrixWorld);
    // the lips: gap envelope x8, alternating smallest/largest (the valve swings faster than frames)
    const gapM = (f % 2 ? r[C.gapMaxM] : r[C.gapMinM]);
    const gap = Math.max(0, gapM) * 8 * unit;
    const lipR = 0.13 * s;
    const start = this.curve.getPointAt(0);
    const flow = level(Math.abs(r[C.flowM3S]), this.stage.max.flow, 30);
    for (const [k, m] of this.lips.entries()) {
      const side = k ? 1 : -1;
      m.position.set(start.x + side * (gap + lipR * 0.55), start.y - 0.02 * s, start.z);
      m.scale.set(lipR * 0.9, lipR * 0.7, lipR);
    }
    this.lipMat.emissiveIntensity = 0.9 * flow;
    // the bladder: volume from the reservoir's energy; pressure flushes it
    const fill = clamp01(r[C.reservoirJ] / LUNG_J);
    const br = 0.27 * s * (0.52 + 0.48 * Math.cbrt(fill));
    this.bladder.position.set(0, 0.27 * s * (0.6 + 0.4 * Math.cbrt(fill)), 0);
    this.bladder.scale.set(br * 1.1, br * 0.85, br);
    this.bladderMat.emissiveIntensity = 0.5 * level(r[C.supplyPa], this.stage.max.supply, 20) * (r[C.sound] > 0.5 || r[C.shot] > 0.5 ? 1 : 0.2);
    // clamps
    for (const [ci, uAt] of [[0, 0.85], [1, 0.93]].map(([a, b]) => [a, b])) {
      const degree = ci === 0 ? r[C.clench] : r[C.rasp];
      const p = this.curve.getPointAt(uAt), k = Math.round(uAt * RING_N);
      const N = this.frames.normals[k], B = this.frames.binormals[k], T = this.frames.tangents[k];
      const radius = RADIUS_K * unit * Math.sqrt(Math.max(r[C.area0 + Math.min(k, RING_N - 1)], 1e-7) / Math.PI);
      for (const [j, m] of this.clamps[ci].entries()) {
        const side = j ? 1 : -1;
        const open = radius * 1.35 - degree * radius * 1.1;
        m.position.copy(p).addScaledVector(N, side * open);
        m.quaternion.setFromRotationMatrix(new THREE.Matrix4().makeBasis(N, T, B));
        m.visible = degree > 0.02 || ci === 0;
        m.scale.setScalar(ci === 0 ? 1 : 0.7);
      }
    }
    // the flask's fill, beans, bubbles
    const sf = clamp01(r[C.stomachJ] / Math.max(stomachMax, 0.6));
    const fh = 0.02 * s + 0.46 * s * sf;
    this.fill.scale.set(1, Math.max(fh, 0.001), 1);
    this.fill.position.set(this.flaskAt.x, fh / 2, this.flaskAt.z);
    this.fill.visible = sf > 0.002;
    let nb = 0;
    const fps = this.stage.film.fps;
    for (let back = 0; back <= 24 && nb < 48; back++) {
      const g = f - back;
      if (g < 0) break;
      const eaten = this.stage.row(g, this.index)[C.eatenJ];
      if (!(eaten > 0)) continue;
      const count = Math.min(10, Math.max(1, Math.round(eaten / 0.6)));
      const age = back / fps;
      for (let b = 0; b < count && nb < 48; b++) {
        const h0 = 1.25 * s + 0.1 * s * hash(g * 31 + b), fall = 4.9 * age * age * 0.6;
        const y = Math.max(fh + 0.03 * s, h0 - fall);
        this.dummy.position.set(this.flaskAt.x + 0.1 * s * (hash(g * 7 + b) - 0.5), y, this.flaskAt.z + 0.1 * s * (hash(g * 13 + b) - 0.5));
        this.dummy.rotation.set(hash(b) * 3, age * 5, 0);
        this.dummy.updateMatrix();
        this.beans.setMatrixAt(nb++, this.dummy.matrix);
      }
    }
    this.beans.count = nb; this.beans.instanceMatrix.needsUpdate = true;
    let nbb = 0;
    if (r[C.digest] > 0.5 && sf > 0.002) {
      for (let b = 0; b < 14; b++) {
        const phase = (t * (0.35 + 0.25 * hash(b * 3 + this.index)) + hash(b * 5 + this.index)) % 1;
        this.dummy.position.set(this.flaskAt.x + 0.11 * s * (hash(b * 11) - 0.5), 0.03 * s + phase * (fh - 0.02 * s), this.flaskAt.z + 0.11 * s * (hash(b * 17) - 0.5));
        this.dummy.rotation.set(0, 0, 0); this.dummy.scale.setScalar(0.6 + 0.8 * hash(b)); this.dummy.updateMatrix();
        this.bubbles.setMatrixAt(nbb++, this.dummy.matrix);
      }
      this.dummy.scale.setScalar(1);
    }
    this.bubbles.count = nbb; this.bubbles.instanceMatrix.needsUpdate = true;
    // puffs: the radiated power at the moment each left, drifting out and up (the drift is a drawing)
    let np = 0;
    const life = 1.8;
    for (let back = 0; back < Math.round(life * fps) && np < 60; back += 1) {
      const g = f - back;
      if (g < 0) break;
      const w = this.stage.row(g, this.index)[C.radW];
      const lv = level(w, radMax, 34);
      if (lv < 0.03) continue;
      const age = back / fps, fade = (1 - age / life) ** 1.4;
      const drift = 0.9 * age + 0.6 * age * age;
      const spread = 0.12 * age;
      this.dummy.position.copy(end).addScaledVector(endT, 0.1 * s + drift * s).add(V3((hash(g * 3) - 0.5) * spread * s, 0.35 * age * age * s + (hash(g * 5) - 0.5) * spread * s, (hash(g * 7) - 0.5) * spread * s));
      this.dummy.rotation.set(hash(g) * 6, hash(g * 2) * 6, 0);
      this.dummy.scale.setScalar(s * (0.025 + 0.10 * lv) * (0.5 + 1.8 * age) * fade + 1e-4);
      this.dummy.updateMatrix();
      this.puffs.setMatrixAt(np++, this.dummy.matrix);
    }
    this.puffs.count = np; this.puffs.instanceMatrix.needsUpdate = true;
    // the jet spark
    const tl = level(r[C.turbW], turbMax, 30);
    this.spark.visible = tl > 0.03;
    if (this.spark.visible) {
      this.spark.position.copy(this.curve.getPointAt(clamp01(r[C.jetPosition])));
      this.spark.scale.setScalar(0.5 + 1.8 * tl);
      this.spark.material.opacity = 0.35 + 0.65 * tl;
    }
    // the spot: a follow spot on the subject, a dim wash on the others
    this.spot.intensity = 70 * s * (0.4 + 0.6 * follow) * this.stage.exposureScale;
    return { hot, rad: level(r[C.radW], radMax, 34) };
  }
}

export class Stage {
  constructor({ film, data, variant, width, height, cut }) {
    this.film = film; this.data = data; this.variant = variant; this.width = width; this.height = height; this.cut = cut;
    const L = film.layout;
    this.V = L.voices; this.Cn = L.columns;
    film.frameColumns.forEach((c, i) => { COL[c] = i; });
    COL.area0 = COL['areaM2[0]']; COL.prms0 = COL['pressureRmsPa[0]'];
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(0x030203);
    this.scene.fog = new THREE.FogExp2(0x070406, 0.038);
    this.camera = new THREE.PerspectiveCamera(42, width / height, 0.05, 120);
    this.exposureScale = 1;
    this.max = {
      radAll: Math.max(...Object.values(film.maxima).map((m) => m.radW)),
      flow: Math.max(...Object.values(film.maxima).map((m) => m.flowM3S)),
      supply: Math.max(...Object.values(film.maxima).map((m) => m.supplyPa)),
    };
    this.buildHall();
    const place = placement(film.voices, film.voiceSpecs, film.anatomies ?? {});
    this.guts = film.voices.map((v, i) => new Gut(this, i, v, place[v], film.voiceSpecs[v]));
    this.scene.updateMatrixWorld(true);
  }

  row(f, vi) {
    const ff = Math.max(0, Math.min(this.film.frames - 1, f));
    const o = (ff * this.V + vi) * this.Cn;
    return this.data.subarray(o, o + this.Cn);
  }

  buildHall() {
    const S = this.scene;
    // the floor: planks, a canvas texture (a drawing)
    const c = document.createElement('canvas'); c.width = 1024; c.height = 1024;
    const g = c.getContext('2d');
    g.fillStyle = '#2a1a10'; g.fillRect(0, 0, 1024, 1024);
    for (let i = 0; i < 32; i++) {
      const y = i * 32;
      g.fillStyle = `hsl(${22 + 6 * hash(i)}, ${40 + 10 * hash(i + 9)}%, ${13 + 7 * hash(i + 3)}%)`;
      g.fillRect(0, y, 1024, 30);
      g.fillStyle = 'rgba(0,0,0,0.35)'; g.fillRect(0, y + 30, 1024, 2);
      for (let k = 0; k < 6; k++) { g.fillRect(hash(i * 7 + k) * 1024, y, 2, 30); }
    }
    const tex = new THREE.CanvasTexture(c); tex.wrapS = tex.wrapT = THREE.RepeatWrapping; tex.repeat.set(3, 3); tex.colorSpace = THREE.SRGBColorSpace;
    const floor = new THREE.Mesh(new THREE.PlaneGeometry(40, 40), new THREE.MeshStandardMaterial({ map: tex, roughness: 0.55, metalness: 0.05 }));
    floor.rotation.x = -Math.PI / 2; S.add(floor);
    // the cannon's riser
    const riser = new THREE.Mesh(new THREE.BoxGeometry(5.2, 0.9, 3.2), new THREE.MeshStandardMaterial({ color: 0x1c1410, roughness: 0.8 }));
    riser.position.set(0, 0.45, -5.2); S.add(riser);
    const trim = new THREE.Mesh(new THREE.BoxGeometry(5.3, 0.04, 3.3), new THREE.MeshStandardMaterial({ color: 0xc9a24a, roughness: 0.3, metalness: 0.9 }));
    trim.position.set(0, 0.9, -5.2); S.add(trim);
    // the curtain: velvet folds (a canvas)
    const cc = document.createElement('canvas'); cc.width = 2048; cc.height = 256;
    const gg = cc.getContext('2d');
    for (let x = 0; x < 2048; x++) {
      const fold = 0.5 + 0.5 * Math.sin(x / 2048 * Math.PI * 2 * 26) * (0.7 + 0.3 * Math.sin(x / 2048 * Math.PI * 2 * 3));
      const l = 10 + 22 * fold;
      gg.fillStyle = `hsl(352, 62%, ${l}%)`; gg.fillRect(x, 0, 1, 256);
    }
    const ctex = new THREE.CanvasTexture(cc); ctex.colorSpace = THREE.SRGBColorSpace;
    const curtain = new THREE.Mesh(new THREE.PlaneGeometry(44, 14), new THREE.MeshStandardMaterial({ map: ctex, roughness: 0.95 }));
    curtain.position.set(0, 7, -9.5); S.add(curtain);
    for (const side of [-1, 1]) {
      const wing = new THREE.Mesh(new THREE.PlaneGeometry(16, 14), new THREE.MeshStandardMaterial({ map: ctex, roughness: 0.95 }));
      wing.position.set(side * 12.5, 7, 0); wing.rotation.y = -side * Math.PI / 2; S.add(wing);
    }
    // the proscenium: gilt
    const gilt = new THREE.MeshStandardMaterial({ color: 0xb8923a, roughness: 0.35, metalness: 0.85 });
    const arch = new THREE.Mesh(new THREE.BoxGeometry(27, 0.9, 0.6), gilt); arch.position.set(0, 9.6, 5.5); S.add(arch);
    for (const side of [-1, 1]) { const col = new THREE.Mesh(new THREE.CylinderGeometry(0.42, 0.5, 10, 18), gilt); col.position.set(side * 13, 5, 5.5); S.add(col); }
    // the footlights: a row of small warm bulbs along the stage lip (static)
    const bulb = new THREE.MeshBasicMaterial({ color: 0xffd9a0 });
    for (let i = -8; i <= 8; i++) { const b = new THREE.Mesh(new THREE.SphereGeometry(0.05, 8, 6), bulb); b.position.set(i * 1.5, 0.06, 4.4); S.add(b); }
    // lights
    S.add(new THREE.AmbientLight(0x4a2a33, 0.55));
    this.hemi = new THREE.HemisphereLight(0x5a3a40, 0x100806, 0.5); S.add(this.hemi);
    this.foot = new THREE.PointLight(0xffb070, 26, 22, 1.6); this.foot.position.set(0, 0.4, 4.3); S.add(this.foot);
    this.key = new THREE.DirectionalLight(0xffe6d0, 1.1); this.key.position.set(5, 9, 8); S.add(this.key);
    this.backlight = new THREE.PointLight(0xff3a5a, 14, 30, 1.4); this.backlight.position.set(0, 6, -8.5); S.add(this.backlight);
  }

  sectionAt(t) {
    const S = this.film.sections;
    let cur = null;
    for (const s of S) if (t >= s.startS) cur = s;
    return cur;
  }

  shotAt(t) {
    // the short has its own shot list; the film's is prep.py's
    const list = this.cut ? shortShots(this.cut, this.film, this) : this.film.shots;
    let cur = list[0];
    for (const s of list) if (t >= s.t0) cur = s;
    return cur;
  }

  // Camera for shot `sh` at film time t. Every shot is a pure function of t.
  placeCamera(sh, t, f, shake) {
    const cam = this.camera, w = this.width, h = this.height;
    const u = clamp01((t - sh.t0) / Math.max(0.01, sh.t1 - sh.t0));
    const G = this.guts, sub = G[Math.max(0, sh.subject)] ?? G[2], sec = G[Math.max(0, sh.second)] ?? G[3];
    const subAt = V3(sub.place.x, sub.place.y, sub.place.z), ss = sub.place.s;
    const mouth = sub.mouthWorld.clone();
    const centre = V3(0, 1.1, -1.6);
    let pos, look, fov = 42;
    const portrait = this.variant === '9x16';
    switch (sh.kind) {
      case 'card': case 'end':
        pos = V3(0.6 * Math.sin(t * 0.05), 2.4, 10.5); look = centre; fov = 44; break;
      case 'push':
        pos = V3(0, lerp(3.2, 1.9, ease(u)), lerp(13.5, 7.4, ease(u))); look = V3(0, lerp(1.6, 1.2, u), -1.6); fov = 40; break;
      case 'wide':
        pos = V3(lerp(-2.2, 2.2, u) * (hash(Math.round(sh.t0 * 10)) > 0.5 ? 1 : -1), 2.6 + 0.3 * u, 11.0 - 0.8 * u); look = centre; fov = portrait ? 56 : 44; break;
      case 'dolly':
        pos = V3(lerp(-6.0, 6.0, u) * (hash(Math.round(sh.t0 * 7)) > 0.5 ? 1 : -1), 1.3, 4.2); look = V3(pos.x * 0.35, 1.0, -1.2); fov = 48; break;
      case 'close': {
        const az = (hash(Math.round(sh.t0 * 3)) - 0.5) * 1.6 + 0.15 * (u - 0.5);
        const r = 1.15 * ss;
        pos = V3(mouth.x + r * Math.sin(az), mouth.y + 0.12 * ss + 0.08 * ss * u, mouth.z + r * Math.cos(az)); look = mouth.clone().add(V3(0, -0.12 * ss, 0)); fov = 38; break;
      }
      case 'orbit': {
        const dir = hash(Math.round(sh.t0 * 5)) > 0.5 ? 1 : -1;
        const az = dir * lerp(-0.9, 0.9, u);
        const r = 2.2 * ss;
        pos = V3(subAt.x + r * Math.sin(az), subAt.y + 1.05 * ss + 0.3 * ss * u, subAt.z + 0.4 * ss + r * Math.cos(az)); look = V3(subAt.x, subAt.y + 0.95 * ss, subAt.z + 0.3 * ss); fov = 40; break;
      }
      case 'two': {
        const mid = V3((sub.place.x + sec.place.x) / 2, 1.0, (sub.place.z + sec.place.z) / 2);
        const d = Math.max(2.4, Math.abs(sub.place.x - sec.place.x) * 0.95 + 1.6);
        pos = V3(mid.x + 0.6 * (u - 0.5), 1.1 + 0.2 * u, mid.z + d); look = V3(mid.x, 0.95, mid.z); fov = 40; break;
      }
      case 'low':
        pos = V3(subAt.x + (hash(Math.round(sh.t0 * 11)) - 0.5) * 2.0 * ss, 0.3, subAt.z + 2.1 * ss - 0.3 * ss * u); look = V3(subAt.x, subAt.y + 0.95 * ss, subAt.z + 0.35 * ss); fov = 50; break;
      case 'cannon': {
        const can = G[6], cs = can.place.s, cm = can.mouthWorld, cb = V3(can.place.x, can.place.y + 0.5 * cs, can.place.z);
        const loading = this.row(f, 6)[COL.clench] > 0.5;
        const vary = sh.variant ?? Math.floor(hash(Math.round(sh.t0 * 17)) * 4);
        if (vary === 0) { // from below and in front: the whole cannon, bladder to mouth
          pos = V3(cm.x + 0.6 * cs * Math.sin(0.5 - 0.3 * u), can.place.y + 0.25 * cs, cm.z + 2.6 * cs); look = V3(can.place.x, can.place.y + 1.0 * cs, can.place.z + 0.3 * cs); fov = 44;
        } else if (vary === 1) { // the mouth, close, the puff coming at the lens
          pos = V3(cm.x + 0.9 * cs * Math.sin(0.5 - 0.3 * u), cm.y - 0.45 * cs + 0.15 * cs * u, cm.z + 1.5 * cs); look = cm.clone().add(V3(0, -0.15 * cs, 0)); fov = loading ? 34 : 40;
        } else if (vary === 2) { // profile from the wing, the puff crossing the frame
          const side = hash(Math.round(sh.t0 * 29)) > 0.5 ? 1 : -1;
          pos = V3(can.place.x + side * 3.2 * cs, can.place.y + 1.2 * cs, cm.z - 0.2 * cs + 0.3 * cs * u); look = V3(can.place.x, can.place.y + 1.1 * cs, can.place.z + 0.5 * cs); fov = 42;
        } else { // from behind the cannon, over its shoulder, across the ensemble
          pos = V3(can.place.x + 1.1 * cs, can.place.y + 1.7 * cs, can.place.z - 1.3 * cs); look = V3(-0.5 + 1.0 * u, 0.9, 0.2); fov = 54;
        }
        void cb;
        break;
      }
      case 'crane':
        pos = V3(lerp(-3, 3, u) * (hash(Math.round(sh.t0 * 13)) > 0.5 ? 1 : -1), lerp(1.0, 6.5, ease(u)), lerp(9.5, 12.5, u)); look = V3(0, lerp(1.4, 0.8, u), -2.0); fov = 50; break;
      default:
        pos = V3(0, 2.4, 10.5); look = centre;
    }
    if (portrait && sh.kind !== 'close' && sh.kind !== 'cannon') { pos.multiplyScalar(1.0); fov += 14; }
    // handheld: the ensemble's radiated power shakes the camera (a drawing of the data)
    const n1 = Math.sin(t * 23.1) + 0.6 * Math.sin(t * 41.7 + 1.1), n2 = Math.sin(t * 19.3 + 2.2) + 0.6 * Math.sin(t * 37.3 + 0.7);
    pos.add(V3(0.035 * shake * n1, 0.03 * shake * n2, 0));
    cam.position.copy(pos);
    cam.up.set(0, 1, 0);
    cam.lookAt(look);
    cam.rotation.z += 0.012 * shake * n1;
    cam.fov = fov; cam.aspect = w / h;
    cam.updateProjectionMatrix();
    cam.updateMatrixWorld(true);
  }

  frame(t, { card, cardLocal }) {
    const film = this.film, fps = film.fps;
    const f = Math.max(0, Math.min(film.frames - 1, Math.floor(t * fps + 1e-6)));
    const sh = this.shotAt(t);
    const sec = this.sectionAt(t);
    const subject = sh.kind === 'cannon' ? 6 : sh.subject;
    // levels
    let radSum = 0, shotNow = false;
    for (let i = 0; i < 7; i++) { const r = this.row(f, i); radSum += r[COL.radW]; shotNow ||= r[COL.shot] > 0.5 && r[COL.clench] < 0.5 && r[COL.sound] > 0.5; }
    const shake = (0.35 + 0.65 * level(radSum, this.max.radAll * 2, 30)) * (sh.kind === 'cannon' ? 2.2 : 1) * (shotNow ? 2.5 : 1) * (card ? 0.2 : 1);
    this.exposureScale = card ? 0.55 : 1;
    const guts = [];
    for (const [i, g] of this.guts.entries()) {
      const mx = film.maxima[g.voice];
      const follow = subject === i ? 1 : (sh.second === i ? 0.5 : 0);
      guts.push(g.update(f, t, mx.prms, mx.radW, mx.turbW, mx.stomachJ, follow));
    }
    this.scene.updateMatrixWorld(true);
    this.placeCamera(sh, t, f, shake);
    // the panel
    const parts = sec ? partNames(sec.about) : [];
    const labels = [];
    for (const [i, g] of this.guts.entries()) {
      const q = g.cardAt.clone().applyMatrix4(g.group.matrixWorld).project(this.camera);
      if (q.z > 1 || Math.abs(q.x) > 1.1 || Math.abs(q.y) > 1.1) continue;
      const dist = this.camera.position.distanceTo(g.cardAt.clone().applyMatrix4(g.group.matrixWorld));
      labels.push({ x: (q.x + 1) / 2 * this.width, y: (1 - q.y) / 2 * this.height, size: Math.max(8, Math.min(15, 80 / dist)), alpha: clamp01((14 - dist) / 6) * (dist < 2.0 ? 0 : 1), name: i < 6 ? `GUT ${ROMAN[i]}` : 'THE CANNON', part: i < 6 ? (parts[i] ?? '') : 'the Canon' });
    }
    const gutsModel = this.guts.map((g, i) => {
      const r = this.row(f, i);
      const note = sec ? currentNote(film.notes[sec.id]?.[g.voice], t - sec.startS) : null;
      return {
        roman: ROMAN[i], name: i < 6 ? `GUT ${ROMAN[i]}` : 'THE CANNON', part: i < 6 ? (parts[i] ?? '—') : 'the Canon', f0: r[COL.f0Hz], shot: r[COL.shot] > 0.5,
        target: note?.targetHz ? noteOf(note.targetHz) : null, cents: note && note.cents !== null && note.cents !== undefined && r[COL.f0Hz] > 0 ? note.cents : null,
        gas: clamp01(r[COL.gasFraction]), stomach: r[COL.stomachJ] / Math.max(film.maxima[g.voice].stomachJ, 0.6), supply: r[COL.supplyPa], digest: r[COL.digest] > 0.5,
        hot: subject === i,
      };
    });
    const books = sec ? Object.values(film.books[sec.id] ?? {}) : [];
    const worst = books.length ? Math.max(...books.map((b) => b.maximumFrameResidualJ ?? 0)) : 0;
    const bars = sec ? sec.about.split(':')[0].trim() : '';
    const shotIndex = (this.cut ? shortShots(this.cut, film, this) : film.shots).indexOf(sh);
    const caption = sec && t - sec.startS < 4.0 && !card ? captionFor(sec, parts, film) : '';
    return {
      exposure: (card ? 0.55 : 1.0) * (sh.kind === 'cannon' && shotNow ? 1.35 : 1.0),
      section: sec ? `${bars} · ♩ = ${sec.bpm}${sec.bar ? ` · ${sec.bar % 1 === 0 ? sec.bar : sec.bar}/4` : ''} · ${fmtClock(t - film.leaderS)}` : '',
      parts: sec ? `the guts are playing: ${parts.join(' · ')}` : '',
      shot: `SHOT ${shotIndex} · ${sh.kind.toUpperCase()}${sh.subject >= 0 && sh.kind !== 'wide' && sh.kind !== 'crane' && sh.kind !== 'dolly' ? ` · ${subject < 6 ? `GUT ${ROMAN[subject]}` : 'THE CANNON'}` : ''}`,
      guts: gutsModel,
      ledger: `${books.length} Worlds · books closed · worst frame residual <span class="on">${worst.toExponential(1)} J</span><br>canals to scale, radii ${RADIUS_K}× · valve gap ×8 · the puffs' drift is a drawing`,
      caption, labels,
    };
  }
}

// "m380-395 : Piccolo, Corno in F I, ..." -> the six part names, v1..v6
function partNames(about) {
  const i = about.indexOf(':');
  if (i < 0) return [];
  return about.slice(i + 1).split(',').map((s) => s.trim()).filter(Boolean);
}
const NOTE_NAMES = ['C', 'C♯', 'D', 'E♭', 'E', 'F', 'F♯', 'G', 'A♭', 'A', 'B♭', 'B'];
function noteOf(hz) { const m = Math.round(69 + 12 * Math.log2(hz / 440)); return `${NOTE_NAMES[((m % 12) + 12) % 12]}${Math.floor(m / 12) - 1}`; }
function currentNote(notes, local) {
  if (!notes) return null;
  let cur = null;
  for (const n of notes) { if (n.atS <= local + 1e-6) cur = n; else break; }
  return cur;
}
function fmtClock(s) { s = Math.max(0, s); const m = Math.floor(s / 60); return `${m}:${String(Math.floor(s - 60 * m)).padStart(2, '0')}`; }

// A line at a section's start: who takes the tune (the first listed part), deadpan.
function captionFor(sec, parts, film) {
  const first = parts[0] ?? '';
  const n = sec.index;
  const fixed = {
    0: 'The chorale <i>God, preserve Thy people</i>. Six cellos and violas, as written, played by six guts fed on beans.',
    2: 'The strings enter. Gut VI is a contrabass now.',
    7: 'Horns. Four of them. Guts I to IV, one horn each.',
    9: 'Allegro giusto. The valves cannot tune this fast; the composers thinned what they could not tune.',
    20: 'The horns again, a hundred and thirty-eight to the minute.',
    43: 'Largo. The hymn returns. Every gut has been refed.',
    45: 'Allegro vivace. The cannon takes its place. Five shots in this section, where Tchaikovsky wrote them.',
    46: 'The bells would go here. We have no bells. We have guts.',
    47: 'Coda. Everything that still has gas.',
  };
  if (fixed[n]) return fixed[n];
  if (n >= 25 && n <= 42) return n === 25 ? 'A ritardando in eighteen tempo changes: a section each, a gut each time.' : '';
  return first ? `Gut I takes the <i>${first}</i>.` : '';
}

// The short's shots: blocked around the cannon's shots in the window (their
// times are the trace's `shot` flag rising; the rest is a direction choice).
let SHORT = null;
function shortShots(cut, film, stage) {
  if (SHORT) return SHORT;
  const L = film.leaderS, a = L + cut.t0, b = L + cut.t1, fps = film.fps;
  const fires = [];
  let was = false;
  for (let f = Math.floor(a * fps); f < Math.ceil(b * fps); f++) {
    const r = stage.row(f, 6);
    const on = r[COL.shot] > 0.5 && r[COL.clench] > 0.5; // the shot is loading
    if (on && !was) fires.push(f / fps);
    was = on;
  }
  const features = [
    { kind: 'close', subject: 0, second: 1 }, { kind: 'low', subject: 5, second: 4 }, { kind: 'orbit', subject: 2, second: 3 },
    { kind: 'two', subject: 0, second: 1 }, { kind: 'close', subject: 4, second: 5 }, { kind: 'wide', subject: 0, second: 1 },
  ];
  const list = [{ t0: 0, t1: a, kind: 'card', subject: -1, second: -1 }];
  let t = a, k = 0;
  const first = fires[0] ?? b;
  list.push({ t0: t, t1: Math.max(t + 1.5, first - 0.7), kind: 'crane', subject: 0, second: 1 });
  t = list[list.length - 1].t1;
  const chosen = fires.filter((_, i) => i % 2 === 0).slice(0, 4);
  let c = 0;
  for (const fire of chosen) {
    if (fire - 0.7 > t + 1.2) {
      const ft = features[k++ % features.length];
      list.push({ t0: t, t1: fire - 0.7, ...ft });
      t = fire - 0.7;
    }
    const end = Math.min(b, fire + 1.9);
    if (end > t) { list.push({ t0: t, t1: end, kind: 'cannon', subject: 6, second: 0, variant: [0, 1, 2, 3][c++ % 4] }); t = end; }
  }
  if (b - t > 0.3) list.push({ t0: t, t1: b, kind: 'crane', subject: 0, second: 1 });
  list.push({ t0: b, t1: b + cut.trailerS, kind: 'end', subject: -1, second: -1 });
  SHORT = list;
  return SHORT;
}
