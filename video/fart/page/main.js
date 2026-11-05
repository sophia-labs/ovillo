// The film as a pure function of the frame number.
//
//   /page/index.html?variant=16x9|9x16[&dpr=1]
//
// window.videoReady resolves when the film's data is loaded and the stage is
// built; window.renderFrame(n) draws frame n and resolves when the canvas and
// the panel show it. Nothing depends on wall time or on the previous frame.
import * as THREE from 'three';
import { Stage } from './film.js';

const q = new URLSearchParams(location.search);
const variant = q.get('variant') === '9x16' ? '9x16' : '16x9';
document.body.classList.add('v' + variant);
const $ = (id) => document.getElementById(id);

const manifest = await (await fetch('/manifest.json')).json();
const film = await (await fetch('/build/film.json')).json();
const buf = await (await fetch('/build/frames.f32')).arrayBuffer();
const data = new Float32Array(buf);
const { width, height } = manifest.variants[variant];
const cut = manifest.cuts?.[variant] ?? null;
const dpr = Number(q.get('dpr') || 1);

const canvas = $('gl');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: false, preserveDrawingBuffer: true, powerPreference: 'high-performance' });
renderer.setPixelRatio(dpr);
renderer.setSize(width, height, false);
renderer.setClearColor(0x000000, 1);
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.0;

const stage = new Stage({ film, data, variant, width, height, cut });
await document.fonts.ready;

const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const ROMAN = ['I', 'II', 'III', 'IV', 'V', 'VI', 'CANNON'];
const NAMES = ['C', 'C♯', 'D', 'E♭', 'E', 'F', 'F♯', 'G', 'A♭', 'A', 'B♭', 'B'];
function noteName(hz) {
  const midi = 69 + 12 * Math.log2(hz / 440);
  const n = Math.round(midi);
  return `${NAMES[((n % 12) + 12) % 12]}${Math.floor(n / 12) - 1}`;
}

// The frame's time on the film's timeline (seconds from the film's start, where
// the music starts at film.leaderS); a cut maps its own frames onto the film's.
function filmTime(n) {
  const fps = film.fps;
  if (!cut) return n / fps;
  const local = n / fps;
  if (local < cut.leaderS) return { card: 'title', local, t: film.leaderS + cut.t0 };
  const musicLen = cut.t1 - cut.t0;
  if (local >= cut.leaderS + musicLen) return { card: 'end', local: local - cut.leaderS - musicLen, t: film.leaderS + cut.t1 };
  return film.leaderS + cut.t0 + (local - cut.leaderS);
}

function card(kind, local) {
  const host = $('card');
  if (host.dataset.kind !== kind) {
    host.dataset.kind = kind;
    const books = Object.values(film.books).flatMap((s) => Object.values(s));
    const worst = Math.max(...books.map((b) => b.maximumFrameResidualJ ?? 0));
    const tune = books.reduce((a, b) => { const [x, y] = b.inTune50Cents.split('/').map(Number); return [a[0] + x, a[1] + y]; }, [0, 0]);
    if (kind === 'title') {
      host.innerHTML = `<div class="over">A CONCERT BROADCAST</div><div class="comp">P. I. TCHAIKOVSKY</div><div class="name">THE YEAR 1812</div>
        <div class="op">Festival Overture in E♭ major, Op. 49</div><div class="for">for seven guts</div>
        <div class="by">played by twenty-three Haiku composers · reviewed by Sonnet · directed by Fable<br>every sound is a World's own breath</div>`;
    } else {
      host.innerHTML = `<div class="end-h">THE BOOKS CLOSED</div>
        <div class="end-l">7 guts · ${Object.keys(film.notes).length} sections · ${books.length} parts · ${tune[1].toLocaleString()} notes, ${(100 * tune[0] / tune[1]).toFixed(1)}% within 50 cents<br>
        <span class="d">worst frame residual of any World</span> <span class="on">${worst.toExponential(2)} J</span><br>
        <span class="d">no samples, no synths, no effects: each gut is one physical World, fed beans by its composer</span><br>
        <span class="d">score: OpenScore edition (CC0) · ovillo · scores/fart/1812 · diaries included</span></div>`;
    }
  }
  const kids = [...host.children];
  const at = kind === 'title' ? [0.1, 0.5, 0.8, 1.3, 1.7, 2.1] : [0.1, 0.5];
  kids.forEach((el, i) => { el.style.opacity = String(Math.max(0, Math.min(1, (local - (at[i] ?? 0)) / 0.5))); });
  host.hidden = false;
}

function hud(model) {
  $('h-section').textContent = model.section;
  $('h-parts').textContent = model.parts;
  $('h-shot').innerHTML = `<span class="rec"></span>${esc(model.shot)}`;
  $('h-guts').innerHTML = model.guts.map((g) => `
    <div class="g ${g.hot ? 'hot' : ''}"><div class="n">${g.name}</div><div class="w">${esc(g.part)}</div>
      <div class="f ${g.f0 > 0 ? '' : 'still'}">${g.f0 > 0 ? `${g.f0.toFixed(0)} Hz` : g.shot ? 'FIRING' : 'still'}</div>
      <div class="c ${g.cents === null ? 'none' : Math.abs(g.cents) > 50 ? 'off' : ''}">${g.cents === null ? (g.f0 > 0 ? noteName(g.f0) : '—') : `${g.target} ${g.cents >= 0 ? '+' : '−'}${Math.abs(g.cents).toFixed(0)} c`}</div>
      <div class="bar"><div class="lv" style="width:${(100 * g.gas).toFixed(1)}%"></div><div class="st" style="width:${(100 * Math.min(1, g.stomach)).toFixed(1)}%"></div></div>
      <div class="k">GAS ${(100 * g.gas).toFixed(0)}% · ${g.supply.toFixed(0)} Pa${g.digest ? ' · DIGESTING' : ''}</div></div>`).join('');
  $('h-ledger').innerHTML = model.ledger;
  $('h-caption').innerHTML = model.caption ?? '';
  $('labels').innerHTML = model.labels.map((l) => `<div class="lab" style="left:${l.x.toFixed(1)}px;top:${l.y.toFixed(1)}px;font-size:${l.size.toFixed(1)}px;opacity:${l.alpha.toFixed(3)}"><div class="n">${esc(l.name)}</div><div class="p">${esc(l.part)}</div></div>`).join('');
}

window.renderFrame = async (n) => {
  const ft = filmTime(n);
  let t, cardKind = null, cardLocal = 0;
  if (typeof ft === 'object') { t = ft.t; cardKind = ft.card; cardLocal = ft.local; } else t = ft;
  if (!cut) {
    if (t < film.leaderS) { cardKind = 'title'; cardLocal = t; }
    else if (t >= film.leaderS + film.musicS) { cardKind = 'end'; cardLocal = t - film.leaderS - film.musicS; }
  }
  const model = stage.frame(t, { card: cardKind, cardLocal, cutLocal: cut ? n / film.fps : null });
  renderer.toneMappingExposure = model.exposure;
  renderer.render(stage.scene, stage.camera);
  if (cardKind) {
    $('hud').hidden = true;
    $('labels').innerHTML = '';
    card(cardKind, cardLocal);
  } else {
    $('card').hidden = true;
    $('hud').hidden = false;
    hud(model);
  }
  await new Promise((r) => requestAnimationFrame(() => r()));
  return { t, card: cardKind };
};
window.videoInfo = { variant, width, height, dpr, frames: film.frames, fps: film.fps };
window.videoReady = true;
