// Capture the video page frame by frame in muted headless Chromium.
//
//   node tools/capture.mjs --variant 4x5 [--dpr 2] [--frames 0:1260 | --at 10,200,900] [--out frames/4x5] [--jobs 3]
//
// Each frame n is window.renderFrame(n) then a PNG screenshot of the
// viewport. The browser is Playwright's Chromium for Testing (revision pinned
// by playwright-core in package.json), launched headless with --mute-audio,
// software WebGL (SwiftShader) and a throwaway profile that is deleted after.
// The page makes no sound: it has no audio element and no AudioContext; the
// soundtrack is assembled by ffmpeg from the Worlds' WAVs (tools/audio.sh).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';

const here = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const argv = process.argv.slice(2);
const opt = (k, d) => (argv.includes(k) ? argv[argv.indexOf(k) + 1] : d);
const variant = opt('--variant', '4x5');
const dpr = Number(opt('--dpr', '2'));
const jobs = Number(opt('--jobs', '1'));
const out = path.resolve(here, opt('--out', `frames/${variant}`));
const manifest = JSON.parse(fs.readFileSync(path.join(here, 'manifest.json'), 'utf8'));
const timeline = JSON.parse(fs.readFileSync(path.join(here, 'build/timeline.json'), 'utf8'));
const { width, height } = manifest.variants[variant];

let frames;
if (argv.includes('--at')) frames = opt('--at').split(',').map(Number);
else {
  const [a, b] = opt('--frames', `0:${timeline.frames}`).split(':').map(Number);
  frames = Array.from({ length: b - a }, (_, i) => a + i);
}
fs.mkdirSync(out, { recursive: true });

const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.wasm': 'application/wasm' };
function fileFor(url) {
  const p = decodeURIComponent(new URL(url).pathname);
  if (p === '/manifest.json') return path.join(here, 'manifest.json');
  if (p.startsWith('/build/')) return path.join(here, p);
  if (p.startsWith('/vendor/three/')) return path.join(here, 'node_modules/three', p.slice('/vendor/three/'.length));
  return path.join(here, 'page', p);
}

fs.mkdirSync(path.join(here, '.cache'), { recursive: true });
const ARGS = ['--mute-audio', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist',
  '--autoplay-policy=user-gesture-required', '--disable-features=AudioServiceOutOfProcess', '--force-color-profile=srgb',
  '--font-render-hinting=none', '--disable-renderer-backgrounding', '--disable-background-timer-throttling',
  '--disable-backgrounding-occluded-windows'];

// One browser per job (each with its own throwaway profile), so no page is ever a background tab.
async function job(mine, progress) {
  const profile = fs.mkdtempSync(path.join(here, '.cache', 'chromium-profile-'));
  const context = await chromium.launchPersistentContext(profile, { headless: true, args: ARGS, viewport: { width, height }, deviceScaleFactor: dpr });
  try {
    await context.route('http://video.local/**', async (route) => {
      const f = fileFor(route.request().url());
      if (!f.startsWith(here) || !fs.existsSync(f)) return route.fulfill({ status: 404, body: 'not found' });
      return route.fulfill({ status: 200, path: f, headers: { 'content-type': TYPES[path.extname(f)] ?? 'application/octet-stream' } });
    });
    const page = context.pages()[0] ?? await context.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    page.on('console', (m) => { if (m.type() === 'error' || m.type() === 'warning') errors.push(m.text()); });
    await page.goto(`http://video.local/index.html?variant=${variant}&dpr=${dpr}`);
    try {
      await page.waitForFunction(() => window.videoReady === true, null, { timeout: 120000 });
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
    process.stdout.write(`\r${variant}: ${done}/${frames.length} frames, ${(s / done).toFixed(2)} s/frame   `);
  }
};
await Promise.all(chunks.map((c) => job(c, progress)));
process.stdout.write('\n');
