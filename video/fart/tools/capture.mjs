// Capture the film's page frame by frame in muted headless Chromium (after
// ../../tools/capture.mjs: the release video's capture, the same browser).
//
//   node tools/capture.mjs --variant 16x9|9x16 [--dpr 1] [--frames 0:26270 | --at 10,200] [--out frames/16x9] [--jobs 3]
//                          [--build build] [--manifest manifest.json] [--gpu]
//
// Each frame n is window.renderFrame(n) then a PNG screenshot. The page makes
// no sound (no audio element, no AudioContext); the soundtrack is the mix the
// guts' Worlds wrote, muxed by ffmpeg (tools/assemble.sh). --gpu drops the
// software-GL flags and lets Chromium pick a GPU (faster; the picture may
// differ by a rounding, never in what is drawn).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';

const here = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const modules = path.resolve(here, '..', 'node_modules');
const argv = process.argv.slice(2);
const opt = (k, d) => (argv.includes(k) ? argv[argv.indexOf(k) + 1] : d);
const variant = opt('--variant', '16x9');
const dpr = Number(opt('--dpr', '1'));
const jobs = Number(opt('--jobs', '1'));
const gpu = argv.includes('--gpu');
const out = path.resolve(here, opt('--out', `frames/${variant}`));
const manifestPath = path.resolve(here, opt('--manifest', 'manifest.json'));
const buildDir = path.resolve(here, opt('--build', 'build'));
const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
const film = JSON.parse(fs.readFileSync(path.join(buildDir, 'film.json'), 'utf8'));
const { width, height } = manifest.variants[variant];
const cut = manifest.cuts?.[variant];
const total = cut ? Math.round((cut.leaderS + (cut.t1 - cut.t0) + cut.trailerS) * film.fps) : film.frames;

let frames;
if (argv.includes('--at')) frames = opt('--at').split(',').map(Number);
else {
  const [a, b] = opt('--frames', `0:${total}`).split(':').map(Number);
  frames = Array.from({ length: b - a }, (_, i) => a + i);
}
fs.mkdirSync(out, { recursive: true });

const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.f32': 'application/octet-stream' };
function fileFor(url) {
  const p = decodeURIComponent(new URL(url).pathname);
  if (p === '/manifest.json') return manifestPath;
  if (p.startsWith('/build/')) return path.join(buildDir, p.slice('/build/'.length));
  if (p.startsWith('/vendor/three/')) return path.join(modules, 'three', p.slice('/vendor/three/'.length));
  return path.join(here, 'page', p);
}

fs.mkdirSync(path.join(here, '.cache'), { recursive: true });
const SOFT = ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'];
const ARGS = ['--mute-audio', ...(gpu ? ['--ignore-gpu-blocklist', '--enable-gpu-rasterization', '--use-angle=metal'] : SOFT),
  '--autoplay-policy=user-gesture-required', '--disable-features=AudioServiceOutOfProcess', '--force-color-profile=srgb',
  '--font-render-hinting=none', '--disable-renderer-backgrounding', '--disable-background-timer-throttling',
  '--disable-backgrounding-occluded-windows'];

async function job(mine, progress) {
  const profile = fs.mkdtempSync(path.join(here, '.cache', 'chromium-profile-'));
  const context = await chromium.launchPersistentContext(profile, { headless: true, args: ARGS, viewport: { width, height }, deviceScaleFactor: dpr });
  try {
    await context.route('http://film.local/**', async (route) => {
      const f = fileFor(route.request().url());
      if (!(f.startsWith(here) || f.startsWith(buildDir) || f.startsWith(modules) || f === manifestPath) || !fs.existsSync(f)) return route.fulfill({ status: 404, body: 'not found' });
      return route.fulfill({ status: 200, path: f, headers: { 'content-type': TYPES[path.extname(f)] ?? 'application/octet-stream' } });
    });
    const page = context.pages()[0] ?? await context.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    page.on('console', (m) => { if (m.type() === 'error' || m.type() === 'warning') errors.push(m.text()); });
    await page.goto(`http://film.local/index.html?variant=${variant}&dpr=${dpr}`);
    try {
      await page.waitForFunction(() => window.videoReady === true, null, { timeout: 180000 });
    } catch (e) {
      throw new Error(`the page never became ready: ${errors.join(' | ') || e.message}`);
    }
    for (const n of mine) {
      await page.evaluate((k) => window.renderFrame(k), n);
      const file = path.join(out, `${String(n).padStart(5, '0')}.png`);
      for (let attempt = 1; ; attempt++) {
        try {
          await page.screenshot({ path: file, type: 'png', animations: 'disabled', caret: 'hide', timeout: 120000 });
          break;
        } catch (e) {
          if (attempt >= 3) throw e;
          console.error(`\nframe ${n}: screenshot retry ${attempt} (${e.message.split('\n')[0]})`);
        }
      }
      progress();
    }
    const bad = errors.filter((e) => !/GPU stall due to ReadPixels/.test(e));
    if (bad.length) { console.error('\npage errors:\n' + bad.join('\n')); process.exitCode = 1; }
  } finally {
    await context.close();
    fs.rmSync(profile, { recursive: true, force: true });
  }
}

const chunks = Array.from({ length: jobs }, (_, j) => frames.filter((_, i) => i % jobs === j)).filter((c) => c.length);
const t0 = Date.now();
let done = 0;
const progress = () => {
  done++;
  if (done % 30 === 0 || done === frames.length) {
    const s = (Date.now() - t0) / 1000;
    process.stdout.write(`\r${variant}: ${done}/${frames.length} frames, ${(s / done).toFixed(2)} s/frame, ${((frames.length - done) * s / done / 60).toFixed(1)} min left   `);
  }
};
await Promise.all(chunks.map((c) => job(c, progress)));
process.stdout.write('\n');
