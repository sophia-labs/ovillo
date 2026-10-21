// The release video as a pure function of the frame number.
//
//   /page/index.html?variant=4x5|16x9[&dpr=2]
//
// window.videoReady resolves when every segment's data is loaded and its
// scene built; window.renderFrame(n) draws frame n (t = n / fps) and resolves
// when the canvas and the panel show it. Nothing depends on wall time or on
// the previous frame, so any frame can be rendered alone, in any order.
import * as THREE from 'three';
import { renderCard } from './scenes/card.js';
import { GuitarScene } from './scenes/guitar.js';
import { TromboneScene } from './scenes/trombone.js';
import { TractScene } from './scenes/tract.js';
import { PianoScene } from './scenes/piano.js';

const q = new URLSearchParams(location.search);
const variant = q.get('variant') === '16x9' ? '16x9' : '4x5';
document.body.classList.add('v' + variant);
const $ = (id) => document.getElementById(id);

const manifest = await (await fetch('/manifest.json')).json();
const timeline = await (await fetch('/build/timeline.json')).json();
const { width, height } = manifest.variants[variant];
const dpr = Number(q.get('dpr') || window.devicePixelRatio || 1);

const canvas = $('gl');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: false, stencil: true, preserveDrawingBuffer: true, powerPreference: 'high-performance' });
renderer.setPixelRatio(dpr);
renderer.setSize(width, height, false);
renderer.setClearColor(0x000000, 1);
renderer.outputColorSpace = THREE.SRGBColorSpace;

const SCENES = { guitar: GuitarScene, trombone: TromboneScene, tract: TractScene, piano: PianoScene };
const segments = [];
for (const seg of manifest.segments) {
  const tl = timeline.segments.find((s) => s.id === seg.id);
  const entry = { seg, tl, scene: null };
  if (SCENES[seg.kind]) {
    const data = await (await fetch(`/build/${seg.id}.json`)).json();
    entry.scene = new SCENES[seg.kind]({ seg, data, variant, width, height, renderer });
  }
  segments.push(entry);
}
await document.fonts.ready;

function segmentAt(n) {
  for (const e of segments) if (n >= e.tl.firstFrame && n < e.tl.firstFrame + e.tl.frames) return e;
  return segments[segments.length - 1];
}

const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

function drawHud(e, model) {
  const { seg } = e;
  $('h-idx').textContent = seg.index ?? '';
  $('h-name').textContent = seg.title ?? '';
  $('h-sub').textContent = seg.subtitle ?? '';
  $('h-world').textContent = seg.world ?? '';
  const flag = $('h-flag');
  flag.hidden = !model.flag;
  if (model.flag) flag.textContent = model.flag;
  $('h-readouts').innerHTML = (model.readouts ?? []).map((r) =>
    `<div class="r"><div class="k">${esc(r.k)}</div><div class="v ${r.c ?? ''}">${r.html ?? esc(r.v)}</div></div>`).join('');
  $('h-chain').innerHTML = (model.chain ?? []).map((s, i, all) => `
    <div class="st ${s.c}"><div class="k">${esc(s.k)}${i < all.length - 1 && variant === '4x5' ? '<i>→</i>' : ''}</div>
      <div class="bar"><div class="lv" style="width:${(100 * s.lv).toFixed(2)}%"></div><div class="pk" style="left:calc(${(100 * s.pk).toFixed(2)}% - 1px)"></div></div>
      <div class="v">${esc(s.v)}</div></div>`).join('');
  $('h-caption').innerHTML = (model.caption ?? '') + (model.note ? `<span class="note" style="font-size:0.62em">${model.note}</span>` : '');
  $('h-ledger').innerHTML = model.ledger
    ? `<div class="row">${model.ledger.map((c) => `<div class="c"><div class="k">${esc(c.k)}</div><div class="v ${c.c ?? ''}">${esc(c.v)}</div></div>`).join('')}</div>`
      + (model.foot ? `<div class="foot">${model.foot}</div>` : '')
    : '';
  $('h-mus').classList.toggle('dense', !!model.mus && model.mus.lines.length > 3);
  $('h-mus').innerHTML = model.mus
    ? `<div class="hd">${esc(model.mus.hd)}</div>` + model.mus.lines.map((ln) =>
      `<div class="ln">${ln.map((p) => `<span class="${p.cls ?? ''}">${esc(p.t)}</span>`).join('')}</div>`).join('')
    : '';
  const labels = $('labels');
  labels.innerHTML = (model.labels ?? []).map((l) =>
    `<div class="lab ${l.align ?? ''} ${l.c ?? ''}" style="left:${l.x.toFixed(1)}px;top:${l.y.toFixed(1)}px;${l.size ? `font-size:${l.size}px;` : ''}${l.alpha !== undefined ? `opacity:${l.alpha.toFixed(3)};` : ''}">${esc(l.text)}</div>`).join('');
}

let current = null;
window.renderFrame = async (n) => {
  const e = segmentAt(n);
  const local = (n - e.tl.firstFrame) / timeline.fps;
  if (e.scene) {
    $('card').hidden = true;
    $('hud').hidden = false;
    const model = e.scene.frame(local);
    renderer.render(e.scene.scene, e.scene.camera);
    drawHud(e, model);
  } else {
    $('hud').hidden = true;
    $('labels').innerHTML = '';
    renderer.clear();
    renderCard($('card'), e.seg, local, e.tl.frames / timeline.fps);
    $('card').hidden = false;
  }
  current = e.seg.id;
  await new Promise((r) => requestAnimationFrame(() => r()));
  return { segment: current, local };
};
window.videoInfo = { variant, width, height, dpr, frames: timeline.frames, fps: timeline.fps, segments: timeline.segments };
window.videoReady = true;
