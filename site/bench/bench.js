// Ovillo bench for the browser: a guitar played through the Ariadne World,
// live, in an AudioWorklet (worklet.js + ovillo_web.wasm), drawn after the
// instrument-CRT doctrine: a black field, fixed graticules, 1–2 px traces,
// colour as meaning, softkeys on the right bezel, boxed annunciators.
//
//   import { mountBench } from "/bench/bench.js";
//   const bench = mountBench(document.querySelector("#bench"));
//
// mountBench(container, { wasmUrl, workletUrl }) returns
//   { start(), suspend(), destroy(), pluck(string, fret, velocity),
//     setPattern(id), loadScore(json) -> Promise<summary>, setParam(id, value),
//     setMaster(dB, mute), element }
// Strings count from the low E: 0 = E2 … 5 = E4. Audio starts on the first
// gesture (START AUDIO, any softkey, a string); the master starts at −12 dB.

const N = 4096;
const COLORS = {
  dry: "#4f9dff", // the guitar, before the World
  wet: "#ff9a3c", // the World's output
  hold: "#8a5a2e", // max hold of the World's output
  material: "#62d26f",
  reservoir: "#b48cff",
  space: "#3fd2d2",
  mode: "#62d26f",
  grat: "#20262c",
  gratMajor: "#2c343c",
  text: "#c9d1c9",
  dim: "#6d776f",
  amber: "#ffb000",
  red: "#ff4040",
  marker: "#e8e8e8",
};
const FONT = 'ui-monospace, "SF Mono", Menlo, Consolas, "DejaVu Sans Mono", "Liberation Mono", monospace';
const STRING_NAMES = ["E2", "A2", "D3", "G3", "B3", "E4"];
const STRING_KEYS = ["z", "x", "c", "v", "b", "n"];
const FRETS = 12;
// Knob menu: Ariadne PARAMS indices (ovillo-dsp ariadne.rs).
const KNOBS = [6, 7, 9, 13, 14, 4, 16];
const DB = (x) => 20 * Math.log10(Math.max(x, 1e-12));

export function mountBench(container, opts = {}) {
  const wasmUrl = opts.wasmUrl || new URL("./ovillo_web.wasm", import.meta.url).href;
  const workletUrl = opts.workletUrl || new URL("./worklet.js", import.meta.url).href;
  const scoreUrl = opts.scoreUrl || new URL("./scores/first-light.json", import.meta.url).href;

  // ---------------------------------------------------------------- state
  const st = {
    ctx: null,
    node: null,
    info: null,
    starting: null,
    playing: false,
    pattern: 0, // index into info.patterns
    menu: "root",
    lit: null, // { kind: "param", index } | { kind: "master" } | { kind: "tempo" } | { kind: "pattern" }
    masterDb: -12,
    muted: false,
    upper: "spectrum",
    mapY: 0,
    pickup: 2,
    bpm: 92,
    params: [],
    load: 0,
    slots: null,
    dry: new Float32Array(N),
    wet: new Float32Array(N),
    fresh: false,
    hold: new Float32Array(N / 2).fill(-200),
    world: [], // [material, reservoir, space] dB per frame, 10 s
    trail: [],
    lastLimited: 0,
    limitedAt: -1e9,
    clipAt: -1e9,
    message: "START AUDIO: PRESS 1, OR TAP A STRING",
    messageAt: performance.now(),
    focused: false,
    visible: true,
    scoreWait: new Map(),
    scoreId: 0,
    destroyed: false,
  };

  // ---------------------------------------------------------------- DOM
  const root = el("div", "ovb");
  root.tabIndex = 0;
  root.setAttribute("role", "application");
  root.setAttribute("aria-label", "Ovillo bench: a simulated guitar played through the Ariadne World. Keys Z X C V B N pluck the strings, 1 to 8 press the softkeys.");
  const head = el("div", "ovb-head");
  const title = el("span", "ovb-title", "OVILLO BENCH · ARIADNE WORLD");
  const rate = el("span", "ovb-rate", "AUDIO OFF");
  head.append(title, rate);
  const body = el("div", "ovb-body");
  const screens = el("div", "ovb-screens");
  const upper = canvasBox("ovb-upper", "UPPER SCREEN");
  const mid = el("div", "ovb-mid");
  const world = canvasBox("ovb-world", "WORLD ENERGIES");
  const square = canvasBox("ovb-square", "MAP");
  mid.append(world.box, square.box);
  const strings = canvasBox("ovb-strings", "STRINGS");
  screens.append(upper.box, mid, strings.box);
  const keys = el("div", "ovb-keys");
  const softkeys = [];
  for (let i = 0; i < 8; i++) {
    const b = el("button", "ovb-key");
    b.type = "button";
    const n = el("span", "ovb-key-n", String(i + 1));
    const l = el("span", "ovb-key-l", "");
    const v = el("span", "ovb-key-v", "");
    b.append(n, l, v);
    b.addEventListener("click", (e) => { e.preventDefault(); press(i, e.shiftKey); });
    knobDrag(b, i);
    keys.append(b);
    softkeys.push({ b, l, v });
  }
  body.append(screens, keys);
  const ann = el("div", "ovb-ann");
  const A = {};
  for (const [k, t] of [
    ["audio", "AUDIO OFF"], ["load", "DSP LOAD --- %"], ["mute", "MUTE"], ["limit", "LIMIT"],
    ["clip", "CLIP"], ["guard", "GUARD OK"], ["resid", "RESID ---"], ["master", "MASTER -12.0 DB"],
    ["keys", "KEYS"],
  ]) {
    A[k] = el("span", "ovb-a", t);
    ann.append(A[k]);
  }
  const msg = el("div", "ovb-msg", st.message);
  root.append(head, body, ann, msg);
  container.append(root);

  // ---------------------------------------------------------------- audio
  async function start() {
    if (st.node) {
      if (st.ctx.state !== "running") await st.ctx.resume();
      return;
    }
    if (st.starting) return st.starting;
    st.starting = (async () => {
      say("STARTING AUDIO …");
      const ctx = new AudioContext({ latencyHint: "balanced" });
      const [bytes] = await Promise.all([
        fetch(wasmUrl).then((r) => {
          if (!r.ok) throw new Error(`wasm ${r.status}`);
          return r.arrayBuffer();
        }),
        ctx.audioWorklet.addModule(workletUrl),
      ]);
      const node = new AudioWorkletNode(ctx, "ovillo", { numberOfInputs: 0, numberOfOutputs: 1, outputChannelCount: [2] });
      const ready = new Promise((resolve, reject) => {
        node.port.onmessage = (e) => {
          const m = e.data;
          if (m.op === "ready") resolve(m.info);
          else if (m.op === "error") reject(new Error(m.message));
        };
      });
      node.port.postMessage({ op: "wasm", bytes, master: { gain: dbToGain(st.masterDb), mute: st.muted } }, [bytes]);
      const info = await ready;
      node.port.onmessage = (e) => onMessage(e.data);
      node.connect(ctx.destination);
      st.ctx = ctx;
      st.node = node;
      st.info = info;
      st.params = info.params.map((p) => p.value);
      st.pickup = info.pickup ?? 2;
      send({ op: "master", gain: dbToGain(st.masterDb), mute: st.muted });
      if (ctx.state !== "running") await ctx.resume();
      rate.textContent = `${ctx.sampleRate} HZ · ${info.planDigest.slice(7, 19).toUpperCase()}`;
      say("AUDIO ON. 1 STARTS THE GUITARIST; Z X C V B N PLUCK");
      render();
    })().catch((err) => {
      st.starting = null;
      say(`NO AUDIO: ${String(err.message || err).toUpperCase()}`, true);
      throw err;
    });
    return st.starting;
  }

  function send(m, transfer) {
    if (st.node) st.node.port.postMessage(m, transfer || []);
  }

  function onMessage(m) {
    if (m.op === "frame") {
      const f = new Float32Array(m.buf, 0, 2 * N);
      st.dry.set(f.subarray(0, N));
      st.wet.set(f.subarray(N, 2 * N));
      const slotsView = new Float64Array(m.buf, 2 * N * 4, st.info.slots.count);
      st.slots = st.slots && st.slots.length === slotsView.length ? st.slots : new Float64Array(slotsView.length);
      st.slots.set(slotsView);
      st.load = m.load;
      st.fresh = true;
      send({ op: "return", buf: m.buf }, [m.buf]);
      ingest();
    } else if (m.op === "score") {
      const w = st.scoreWait.get(m.id);
      st.scoreWait.delete(m.id);
      if (w) (m.ok ? w.resolve : w.reject)(m.ok ? m.text : new Error(m.text));
      if (m.ok) {
        let t = m.text;
        try {
          const j = JSON.parse(m.text);
          t = `${j.name ?? "SCORE"} · ${j.tempo} BPM · ${Number(j.seconds).toFixed(1)} S · ${j.events_written} EVENTS${j.loop ? " · LOOP" : ""}`;
        } catch {}
        say(`SCORE: ${t.toUpperCase()}`);
      } else say(`SCORE REFUSED: ${m.text.toUpperCase()}`, true);
    } else if (m.op === "info") {
      st.info = m.info;
      st.params = m.info.params.map((p) => p.value);
      paintKeys();
    }
  }

  // Telemetry that has history: the World strip chart, LIMIT, CLIP.
  function ingest() {
    const s = st.slots;
    const S = st.info.slots;
    const t = performance.now();
    st.world.push([10 * Math.log10(Math.max(s[S.material], 1e-30)), 10 * Math.log10(Math.max(s[S.reservoir], 1e-30)), 10 * Math.log10(Math.max(s[S.space], 1e-30))]);
    if (st.world.length > 300) st.world.shift();
    if (s[S.safetyLimitedFrames] > st.lastLimited) st.limitedAt = t;
    st.lastLimited = s[S.safetyLimitedFrames];
    if (s[S.wetPeak] * s[S.master] >= 0.99) st.clipAt = t;
    const pi = s[S.pattern] | 0;
    if (pi !== st.pattern) {
      st.pattern = pi;
      paintKeys();
    }
  }

  // ---------------------------------------------------------------- softkeys
  function patternLabel() {
    if (!st.info) return "WANDER";
    return st.info.patterns[st.pattern]?.label ?? "-";
  }

  function paramInfo(i) {
    return st.info ? st.info.params[i] : null;
  }

  function fmtParam(i) {
    const p = paramInfo(i);
    if (!p) return "";
    const v = st.params[i];
    return p.max > 2 ? v.toFixed(2) : v.toFixed(3);
  }

  function menuKeys() {
    const lit = (k) => st.lit && Object.keys(k).every((x) => st.lit[x] === k[x]);
    if (st.menu === "knobs") {
      return [
        ...KNOBS.map((pi) => ({ label: paramInfo(pi)?.label ?? "—", value: fmtParam(pi), lit: lit({ kind: "param", index: pi }), act: () => (st.lit = { kind: "param", index: pi }) })),
        { label: "BACK", value: "", act: () => { st.menu = "root"; st.lit = null; } },
      ];
    }
    if (st.menu === "output") {
      return [
        { label: "MASTER", value: `${st.masterDb.toFixed(1)} DB`, lit: lit({ kind: "master" }), act: () => (st.lit = { kind: "master" }) },
        { label: "MUTE", value: st.muted ? "ON" : "OFF", lit: st.muted, act: () => setMaster(st.masterDb, !st.muted) },
        { label: "TEMPO", value: `${st.bpm.toFixed(0)} BPM`, lit: lit({ kind: "tempo" }), act: () => (st.lit = { kind: "tempo" }) },
        { label: "CLEAR HOLD", value: "", act: () => st.hold.fill(-200) },
        { label: "MUTE STRINGS", value: "", act: () => send({ op: "muteStrings" }) },
        { label: "DEFAULT PATCH", value: "", act: () => { send({ op: "defaultPatch" }); say("DEFAULT PATCH"); } },
        { label: "FIRST LIGHT", value: "SCORE", act: () => playFirstLight() },
        { label: "BACK", value: "", act: () => { st.menu = "root"; st.lit = null; } },
      ];
    }
    const yi = st.info ? st.info.mapY[st.mapY] : 4;
    return [
      { label: st.playing ? "STOP" : st.node ? "START" : "START AUDIO", value: st.playing ? "PLAYING" : "", lit: st.playing, act: () => transport(!st.playing) },
      { label: "PATTERN", value: patternLabel(), lit: lit({ kind: "pattern" }), act: (shift) => { st.lit = { kind: "pattern" }; stepPattern(shift ? -1 : 1); } },
      { label: "KNOBS ▸", value: "", act: () => { st.menu = "knobs"; st.lit = { kind: "param", index: KNOBS[0] }; } },
      { label: "MAP Y", value: paramInfo(yi)?.label ?? "DRIVE", act: () => { st.mapY = (st.mapY + 1) % (st.info?.mapY.length ?? 6); } },
      { label: "PICKUP", value: st.info?.pickups[st.pickup] ?? "BRIDGE SC", act: (shift) => { const n = st.info?.pickups.length ?? 4; st.pickup = (st.pickup + (shift ? n - 1 : 1)) % n; send({ op: "pickup", index: st.pickup }); } },
      { label: "SCREEN", value: st.upper === "spectrum" ? "SPECTRUM" : "SCOPE", act: () => { st.upper = st.upper === "spectrum" ? "scope" : "spectrum"; } },
      { label: "RESET WORLD", value: "", act: () => { send({ op: "reset" }); say("WORLD AT REST"); } },
      { label: "OUTPUT ▸", value: "", act: () => { st.menu = "output"; st.lit = { kind: "master" }; } },
    ];
  }

  function paintKeys() {
    const ks = menuKeys();
    ks.forEach((k, i) => {
      softkeys[i].l.textContent = k.label;
      softkeys[i].v.textContent = k.value;
      softkeys[i].b.classList.toggle("lit", !!k.lit);
      softkeys[i].b.setAttribute("aria-label", `${i + 1}: ${k.label} ${k.value}`.trim());
    });
  }

  async function press(i, shift = false) {
    const first = !st.node;
    try {
      await start();
    } catch {
      return;
    }
    if (first && i !== 0) {
      paintKeys();
    }
    const k = menuKeys()[i];
    if (k) k.act(shift);
    paintKeys();
  }

  function transport(on) {
    st.playing = on;
    send({ op: "transport", on });
    say(on ? `THE GUITARIST PLAYS ${patternLabel()}` : "STOPPED: WHAT RINGS, RINGS OUT");
  }

  function stepPattern(d) {
    if (!st.info) return;
    // Everything but SCORE and OFF, in display order.
    const cyc = st.info.patterns.map((p, i) => [p, i]).filter(([p]) => p.id !== "score" && p.id !== "off").map(([, i]) => i);
    const at = Math.max(0, cyc.indexOf(st.pattern));
    st.pattern = cyc[(at + d + cyc.length) % cyc.length];
    if (st.playing) send({ op: "pattern", index: st.pattern });
    else { send({ op: "pattern", index: st.pattern }); st.playing = true; }
    say(st.info.patterns[st.pattern].about.toUpperCase());
  }

  // The lit softkey's quantity, turned by a knob (wheel, arrows, drag).
  function turn(steps, fine = false) {
    const L = st.lit;
    if (!L) return;
    if (L.kind === "param") {
      const p = paramInfo(L.index);
      if (!p) return;
      const step = ((p.max - p.min) / 100) * (fine ? 0.1 : 1);
      setParam(L.index, st.params[L.index] + steps * step);
    } else if (L.kind === "master") {
      setMaster(st.masterDb + steps * (fine ? 0.1 : 1), st.muted);
    } else if (L.kind === "tempo") {
      st.bpm = clamp(st.bpm + steps * (fine ? 0.2 : 2), 40, 220);
      send({ op: "tempo", bpm: st.bpm });
    } else if (L.kind === "pattern") {
      stepPattern(steps > 0 ? 1 : -1);
    }
    paintKeys();
  }

  function setParam(index, value) {
    const p = paramInfo(index);
    if (!p) return;
    const v = clamp(value, p.min, p.max);
    st.params[index] = v;
    send({ op: "param", index, value: v });
  }

  function setMaster(db, mute) {
    st.masterDb = clamp(Math.round(db * 10) / 10, -60, 0);
    st.muted = !!mute;
    send({ op: "master", gain: dbToGain(st.masterDb), mute: st.muted });
    paintKeys();
  }

  async function playFirstLight() {
    try {
      const json = await fetch(scoreUrl).then((r) => r.text());
      await loadScore(json);
    } catch (e) {
      say(`SCORE: ${String(e.message || e).toUpperCase()}`, true);
    }
  }

  function loadScore(json) {
    return start().then(() => new Promise((resolve, reject) => {
      const id = ++st.scoreId;
      st.scoreWait.set(id, { resolve, reject });
      send({ op: "score", json: typeof json === "string" ? json : JSON.stringify(json), id });
      st.playing = true;
    }));
  }

  // Drag a softkey up or down to turn it (touch has no wheel).
  function knobDrag(b, i) {
    let y0 = null;
    let acc = 0;
    let moved = false;
    b.addEventListener("pointerdown", (e) => {
      if (e.pointerType === "mouse") return;
      y0 = e.clientY;
      acc = 0;
      moved = false;
    });
    b.addEventListener("pointermove", (e) => {
      if (y0 === null) return;
      const k = menuKeys()[i];
      const knobby = ["MASTER", "TEMPO", "PATTERN"].includes(k?.label) || st.menu === "knobs";
      if (!knobby) return;
      const dy = y0 - e.clientY;
      if (Math.abs(dy) > 6 && !moved) {
        moved = true;
        try { b.setPointerCapture(e.pointerId); } catch {}
        if (!k.lit) k.act(false);
      }
      if (!moved) return;
      acc += dy;
      y0 = e.clientY;
      const steps = Math.trunc(acc / 8);
      if (steps) {
        acc -= steps * 8;
        turn(steps);
      }
      e.preventDefault();
    });
    const end = (e) => {
      if (moved) {
        e.preventDefault();
        b.dataset.dragged = "1";
        setTimeout(() => delete b.dataset.dragged, 0);
      }
      y0 = null;
    };
    b.addEventListener("pointerup", end);
    b.addEventListener("pointercancel", end);
    b.addEventListener("click", (e) => {
      if (b.dataset.dragged) {
        e.stopImmediatePropagation();
        e.preventDefault();
      }
    }, true);
  }

  // ---------------------------------------------------------------- input
  root.addEventListener("focus", () => { st.focused = true; });
  root.addEventListener("blur", () => { st.focused = false; });
  root.addEventListener("keydown", (e) => {
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    const k = e.key.toLowerCase();
    const si = STRING_KEYS.indexOf(k);
    if (si >= 0) {
      e.preventDefault();
      if (!e.repeat) pluck(si, 0, e.shiftKey ? 0.95 : 0.7);
      return;
    }
    if (/^[1-8]$/.test(e.key)) { e.preventDefault(); press(Number(e.key) - 1, e.shiftKey); return; }
    const fk = /^F([1-8])$/.exec(e.key);
    if (fk) { e.preventDefault(); press(Number(fk[1]) - 1, e.shiftKey); return; }
    switch (e.key) {
      case " ": e.preventDefault(); start().then(() => { transport(!st.playing); paintKeys(); }).catch(() => {}); break;
      case "ArrowUp": e.preventDefault(); turn(1, e.shiftKey); break;
      case "ArrowDown": e.preventDefault(); turn(-1, e.shiftKey); break;
      case "Escape": st.menu = "root"; st.lit = null; paintKeys(); break;
      case "0": send({ op: "reset" }); say("WORLD AT REST"); break;
      case "-": case "_": setMaster(st.masterDb - (e.shiftKey ? 0.1 : 1), st.muted); break;
      case "=": case "+": setMaster(st.masterDb + (e.shiftKey ? 0.1 : 1), st.muted); break;
      case "\\": case "`": setMaster(st.masterDb, !st.muted); break;
      case "m": send({ op: "muteStrings" }); break;
      case "h": st.hold.fill(-200); break;
      default: return;
    }
  });
  // The wheel is the knob, but only over the bezel, and only with a lit softkey.
  let wheelAcc = 0;
  keys.addEventListener("wheel", (e) => {
    if (!st.lit) return;
    e.preventDefault();
    wheelAcc += e.deltaY;
    const steps = Math.trunc(wheelAcc / 40);
    if (steps) {
      wheelAcc -= steps * 40;
      turn(-steps, e.shiftKey);
    }
  }, { passive: false });

  function pluck(string, fret = 0, velocity = 0.7) {
    start().then(() => send({ op: "pluck", string, fret, velocity })).catch(() => {});
  }

  // Strings: tap where the fret is. High e on top, as tab is written.
  strings.canvas.addEventListener("pointerdown", (e) => {
    e.preventDefault();
    root.focus({ preventScroll: true });
    const r = strings.canvas.getBoundingClientRect();
    const g = st.stringsGeom || stringsGeom(r.width, r.height);
    const yi = clamp(Math.round((e.clientY - r.top - g.top) / g.gap), 0, 5);
    const string = 5 - yi;
    const x = e.clientX - r.left;
    let fret = 0;
    for (let f = 1; f <= FRETS; f++) if (x > g.fretX(f - 1)) fret = f;
    if (x < g.fretX(0)) fret = 0;
    pluck(string, fret, 0.75);
  });

  // The MAP square: drag through configuration space.
  let dragging = false;
  const mapAt = (e) => {
    const r = square.canvas.getBoundingClientRect();
    const g = st.squareGeom || squareGeom(r.width, r.height);
    const x = clamp((e.clientX - r.left - g.x) / g.s, 0, 1);
    const y = clamp(1 - (e.clientY - r.top - g.y) / g.s, 0, 1);
    setParam(0, x);
    const yi = st.info ? st.info.mapY[st.mapY] : 4;
    const p = paramInfo(yi);
    if (p) setParam(yi, p.min + y * (p.max - p.min));
    st.trail.push([x, y, performance.now()]);
    if (st.trail.length > 400) st.trail.shift();
    paintKeys();
  };
  square.canvas.addEventListener("pointerdown", (e) => {
    e.preventDefault();
    root.focus({ preventScroll: true });
    start().then(() => {
      dragging = true;
      try { square.canvas.setPointerCapture(e.pointerId); } catch {}
      mapAt(e);
    }).catch(() => {});
  });
  square.canvas.addEventListener("pointermove", (e) => { if (dragging) { e.preventDefault(); mapAt(e); } });
  square.canvas.addEventListener("pointerup", () => { dragging = false; });
  square.canvas.addEventListener("pointercancel", () => { dragging = false; });

  // ---------------------------------------------------------------- drawing
  const fft = makeFft(N);
  const specDry = new Float32Array(N / 2);
  const specWet = new Float32Array(N / 2);
  let raf = 0;
  const io = typeof IntersectionObserver === "function"
    ? new IntersectionObserver((es) => { st.visible = es.some((x) => x.isIntersecting); if (st.visible) render(); })
    : null;
  io?.observe(root);
  const ro = typeof ResizeObserver === "function" ? new ResizeObserver(() => { sizeAll(); render(); }) : null;
  for (const c of [upper, world, square, strings]) ro?.observe(c.box);

  function sizeAll() {
    for (const c of [upper, world, square, strings]) size(c.canvas);
  }

  function render() {
    if (raf || st.destroyed) return;
    raf = requestAnimationFrame(() => {
      raf = 0;
      drawAll();
      if (st.visible && st.node) render();
    });
  }

  function drawAll() {
    if (st.fresh) {
      fft.mag(st.dry, specDry);
      fft.mag(st.wet, specWet);
      for (let i = 0; i < specWet.length; i++) if (specWet[i] > st.hold[i]) st.hold[i] = specWet[i];
      st.fresh = false;
    }
    if (st.upper === "spectrum") drawSpectrum(upper.canvas);
    else drawScope(upper.canvas);
    drawWorld(world.canvas);
    drawSquare(square.canvas);
    drawStrings(strings.canvas);
    annunciate();
  }

  function annunciate() {
    const t = performance.now();
    const s = st.slots;
    const S = st.info?.slots;
    set(A.audio, st.node ? (st.ctx.state === "running" ? "AUDIO ON" : "AUDIO SUSPENDED") : "AUDIO OFF", st.node && st.ctx.state === "running" ? "on" : "");
    const load = st.load * 100;
    set(A.load, `DSP LOAD ${st.node ? load.toFixed(0).padStart(3) : "---"} %`, load > 100 ? "bad" : load > 70 ? "warn" : st.node ? "on" : "");
    set(A.mute, "MUTE", st.muted ? "warn" : "");
    set(A.limit, "LIMIT", t - st.limitedAt < 500 ? "warn" : "");
    set(A.clip, "CLIP", t - st.clipAt < 800 ? "bad" : "");
    if (s && S) {
      const g = s[S.guard] | 0;
      const trips = s[S.guardTrips] | 0;
      const gt = ["GUARD OK", "GUARD FADE", "GUARD REST", "GUARD RETURN"][g] + (trips ? ` ×${trips}` : "");
      set(A.guard, gt, g ? "warn" : "");
      set(A.resid, `RESID ${expo(s[S.closureResidual])}`, "on");
    }
    set(A.master, `MASTER ${st.masterDb.toFixed(1)} DB`, st.muted ? "" : "on");
    set(A.keys, "KEYS", st.focused ? "on" : "");
    msg.textContent = st.message;
    msg.classList.toggle("bad", !!st.messageBad);
  }

  function drawSpectrum(c) {
    const { g, w, h } = begin(c);
    const T = header(g, w, [["SPECTRUM", COLORS.text], ["DRY", COLORS.dry], ["WORLD", COLORS.wet], ["HOLD", COLORS.hold], ["MODES", COLORS.mode]], "DBFS · HZ · HANN 4096");
    const L = 54, R = 8, B = 22;
    const pw = w - L - R, ph = h - T - B;
    const f0 = 20, f1 = Math.min(20000, (st.ctx?.sampleRate ?? 48000) / 2);
    const fx = (f) => L + (Math.log10(f / f0) / Math.log10(f1 / f0)) * pw;
    const dbTop = 0, dbBot = -120;
    const dy = (d) => T + ((dbTop - d) / (dbTop - dbBot)) * ph;
    // Graticule: decades and their 2–9, 20 dB rows.
    g.lineWidth = 1;
    for (let dec = 10; dec <= 10000; dec *= 10) {
      for (let k = 1; k < 10; k++) {
        const f = dec * k;
        if (f < f0 || f > f1) continue;
        line(g, fx(f), T, fx(f), T + ph, k === 1 ? COLORS.gratMajor : COLORS.grat);
      }
    }
    for (let d = dbTop; d >= dbBot; d -= 20) line(g, L, dy(d), L + pw, dy(d), d === dbTop ? COLORS.gratMajor : COLORS.grat);
    rect(g, L, T, pw, ph, COLORS.gratMajor);
    for (const [f, t] of [[100, "100"], [1000, "1K"], [10000, "10K"]]) label(g, t, fx(f), h - 6, COLORS.dim, "center");
    for (let d = 0; d >= -120; d -= 40) label(g, String(d), L - 6, dy(d) + 4, COLORS.dim, "right");
    if (!st.node) {
      label(g, "AUDIO OFF", L + pw / 2, T + ph / 2, COLORS.dim, "center");
      return;
    }
    const sr = st.ctx.sampleRate;
    // The World's modes as stems, relative heights over 60 dB.
    const s = st.slots, S = st.info.slots;
    if (s) {
      let emax = 1e-30;
      for (let i = 0; i < 24; i++) emax = Math.max(emax, s[S.modeEnergy + i]);
      g.strokeStyle = COLORS.mode;
      g.lineWidth = 1;
      g.beginPath();
      for (let i = 0; i < 24; i++) {
        const hz = s[S.modeHz + i];
        if (!(hz > f0 && hz < f1)) continue;
        const rel = clamp(10 * Math.log10(Math.max(s[S.modeEnergy + i], 1e-30) / emax) / 60 + 1, 0, 1);
        const x = fx(hz);
        g.moveTo(x, T + ph);
        g.lineTo(x, T + ph - rel * ph * 0.35);
      }
      g.stroke();
      // The six strings' sounding pitches, ticks on the top edge.
      g.strokeStyle = COLORS.dry;
      g.beginPath();
      for (let i = 0; i < 6; i++) {
        const hz = s[S.stringHz + i];
        if (!(hz > f0)) continue;
        const x = fx(hz);
        g.moveTo(x, T);
        g.lineTo(x, T + 6);
      }
      g.stroke();
    }
    // One point per pixel column: the largest bin that falls in it (a
    // peak-detecting display, as a swept analyser draws), so the dense top
    // octaves read as an envelope, not a thicket.
    const trace = (spec, color, dash) => {
      g.strokeStyle = color;
      g.lineWidth = 1.25;
      g.setLineDash(dash || []);
      g.beginPath();
      let started = false, col = -1, best = -Infinity;
      const flush = () => {
        if (col < 0) return;
        const y = dy(clamp(best, dbBot, dbTop));
        if (!started) { g.moveTo(col, y); started = true; } else g.lineTo(col, y);
      };
      for (let i = 1; i < spec.length; i++) {
        const f = (i * sr) / N;
        if (f < f0 || f > f1) continue;
        const x = Math.round(fx(f));
        if (x !== col) { flush(); col = x; best = spec[i]; } else if (spec[i] > best) best = spec[i];
      }
      flush();
      g.stroke();
      g.setLineDash([]);
    };
    trace(st.hold, COLORS.hold, [2, 3]);
    trace(specDry, COLORS.dry);
    trace(specWet, COLORS.wet);
  }

  function drawScope(c) {
    const { g, w, h } = begin(c);
    const divs = 10, msPerDiv = 2, ampPerDiv = 0.1;
    const T = header(g, w, [["SCOPE", COLORS.text], ["DRY", COLORS.dry], ["WORLD", COLORS.wet]], `${msPerDiv} MS/DIV · ${ampPerDiv} /DIV · TRIG DRY ↑`);
    const L = 54, R = 8, B = 22;
    const pw = w - L - R, ph = h - T - B;
    const sr = st.ctx?.sampleRate ?? 48000;
    for (let i = 0; i <= divs; i++) line(g, L + (i * pw) / divs, T, L + (i * pw) / divs, T + ph, i === divs / 2 ? COLORS.gratMajor : COLORS.grat);
    for (let i = 0; i <= 8; i++) line(g, L, T + (i * ph) / 8, L + pw, T + (i * ph) / 8, i === 4 ? COLORS.gratMajor : COLORS.grat);
    rect(g, L, T, pw, ph, COLORS.gratMajor);
    if (!st.node) { label(g, "AUDIO OFF", L + pw / 2, T + ph / 2, COLORS.dim, "center"); return; }
    const span = Math.round((divs * msPerDiv * sr) / 1000);
    let trig = N - span - 1;
    for (let i = N - span - 2; i > 1; i--) {
      if (st.dry[i - 1] < 0 && st.dry[i] >= 0) { trig = i; break; }
    }
    const yAt = (v) => T + ph / 2 - (v / (ampPerDiv * 4)) * (ph / 2);
    for (const [buf, color] of [[st.dry, COLORS.dry], [st.wet, COLORS.wet]]) {
      g.strokeStyle = color;
      g.lineWidth = 1.25;
      g.beginPath();
      for (let k = 0; k < span; k++) {
        const x = L + (k / span) * pw;
        const y = clamp(yAt(buf[trig + k] || 0), T, T + ph);
        if (k === 0) g.moveTo(x, y); else g.lineTo(x, y);
      }
      g.stroke();
    }
  }

  function drawWorld(c) {
    const { g, w, h } = begin(c);
    const T = header(g, w, [["WORLD", COLORS.text], ["MATERIAL", COLORS.material], ["RESERVOIR", COLORS.reservoir], ["SPACE", COLORS.space]], "10·LOG10 E · 10 S");
    const L = 40, R = 8, B = 18;
    const pw = w - L - R, ph = h - T - B;
    const top = 0, bot = -160;
    const dy = (d) => T + ((top - d) / (top - bot)) * ph;
    for (let d = top; d >= bot; d -= 20) line(g, L, dy(d), L + pw, dy(d), COLORS.grat);
    for (let s = 0; s <= 10; s++) line(g, L + (s * pw) / 10, T, L + (s * pw) / 10, T + ph, COLORS.grat);
    rect(g, L, T, pw, ph, COLORS.gratMajor);
    for (let d = 0; d >= bot; d -= 40) label(g, String(d), L - 4, dy(d) + 4, COLORS.dim, "right");
    const n = st.world.length;
    if (!n) return;
    const series = [[0, COLORS.material, []], [1, COLORS.reservoir, [4, 3]], [2, COLORS.space, []]];
    for (const [k, color, dash] of series) {
      g.strokeStyle = color;
      g.lineWidth = 1.25;
      g.setLineDash(dash);
      g.beginPath();
      for (let i = 0; i < n; i++) {
        const x = L + pw - ((n - 1 - i) / 299) * pw;
        const y = dy(clamp(st.world[i][k], bot, top));
        if (i === 0) g.moveTo(x, y); else g.lineTo(x, y);
      }
      g.stroke();
    }
    g.setLineDash([]);
  }

  function squareGeom(w, h, top = 20) {
    const s = Math.max(40, Math.min(w - 28, h - top - 20));
    return { s, x: (w - s) / 2, y: top };
  }

  function drawSquare(c) {
    const { g, w, h } = begin(c);
    const yi = st.info ? st.info.mapY[st.mapY] : 4;
    const p = paramInfo(yi);
    const x = st.params.length ? st.params[0] : 0.06;
    const yv = p && st.params.length ? (st.params[yi] - p.min) / (p.max - p.min) : 0.28;
    const top = header(g, w, [["MAP", COLORS.text], ["X ORGANIZ", COLORS.dim], [`Y ${p?.label ?? "DRIVE"}`, COLORS.dim]], `X ${x.toFixed(3)}  Y ${p && st.params.length ? st.params[yi].toFixed(3) : "-"}`);
    const q = squareGeom(w, h, top);
    st.squareGeom = q;
    for (let i = 0; i <= 10; i++) {
      line(g, q.x + (i * q.s) / 10, q.y, q.x + (i * q.s) / 10, q.y + q.s, i === 5 ? COLORS.gratMajor : COLORS.grat);
      line(g, q.x, q.y + (i * q.s) / 10, q.x + q.s, q.y + (i * q.s) / 10, i === 5 ? COLORS.gratMajor : COLORS.grat);
    }
    rect(g, q.x, q.y, q.s, q.s, COLORS.gratMajor);
    label(g, "STRINGS", q.x, q.y + q.s + 14, COLORS.dim);
    label(g, "B♭ BORE", q.x + q.s, q.y + q.s + 14, COLORS.dim, "right");
    // Persistence: the recent path, older segments dimmer.
    const t = performance.now();
    st.trail = st.trail.filter((pt) => t - pt[2] < 4000);
    for (let i = 1; i < st.trail.length; i++) {
      const a = st.trail[i - 1], b = st.trail[i];
      const age = (t - b[2]) / 4000;
      g.strokeStyle = COLORS.marker;
      g.globalAlpha = Math.max(0.08, 0.6 * (1 - age));
      g.beginPath();
      g.moveTo(q.x + a[0] * q.s, q.y + (1 - a[1]) * q.s);
      g.lineTo(q.x + b[0] * q.s, q.y + (1 - b[1]) * q.s);
      g.stroke();
    }
    g.globalAlpha = 1;
    const cx = q.x + x * q.s, cy = q.y + (1 - yv) * q.s;
    g.strokeStyle = COLORS.marker;
    g.lineWidth = 1.5;
    g.beginPath();
    g.moveTo(cx - 6, cy); g.lineTo(cx + 6, cy);
    g.moveTo(cx, cy - 6); g.lineTo(cx, cy + 6);
    g.stroke();
  }

  function stringsGeom(w, h, top = 30) {
    const bottom = 22;
    const gap = (h - top - bottom) / 5;
    const L = 44, R = 74;
    const span = w - L - R;
    // Fret k at the 12-TET distance from the nut; fret 12 at the right edge.
    const fretX = (k) => L + ((1 - Math.pow(2, -k / 12)) / 0.5) * span;
    return { top, gap, L, R, span, fretX };
  }

  function drawStrings(c) {
    const { g, w, h } = begin(c);
    const top = header(g, w, [["STRINGS", COLORS.text], ["TAP A FRET · Z X C V B N", COLORS.dim]], "HZ") + 6;
    const q = stringsGeom(w, h, top);
    st.stringsGeom = q;
    for (let f = 0; f <= FRETS; f++) {
      const x = q.fretX(f);
      line(g, x, q.top - 4, x, q.top + 5 * q.gap + 4, f === 0 ? COLORS.dim : COLORS.grat);
    }
    for (const f of [3, 5, 7, 9, 12]) label(g, String(f), (q.fretX(f - 1) + q.fretX(f)) / 2, h - 5, COLORS.dim, "center");
    const s = st.slots, S = st.info?.slots;
    for (let yi = 0; yi < 6; yi++) {
      const string = 5 - yi;
      const y = q.top + yi * q.gap;
      const sounding = s && S ? ((s[S.sounding] | 0) >> string) & 1 : 0;
      const level = s && S ? s[S.stringLevel + string] : 0;
      const bright = sounding ? clamp(0.45 + 0.55 * clamp((DB(level) + 60) / 50, 0, 1), 0.45, 1) : 0.3;
      g.globalAlpha = bright;
      line(g, q.L - 6, y, q.L + q.span, y, COLORS.dry, sounding ? 2 : 1);
      g.globalAlpha = 1;
      label(g, STRING_NAMES[string], 4, y + 4, sounding ? COLORS.text : COLORS.dim);
      if (s && S && sounding) {
        const fret = s[S.stringFret + string] | 0;
        const x = fret === 0 ? q.L - 9 : (q.fretX(fret - 1) + q.fretX(fret)) / 2;
        g.strokeStyle = COLORS.marker;
        g.lineWidth = 1.5;
        g.beginPath();
        g.moveTo(x - 4, y - 4); g.lineTo(x + 4, y + 4);
        g.moveTo(x - 4, y + 4); g.lineTo(x + 4, y - 4);
        g.stroke();
        label(g, s[S.stringHz + string].toFixed(1), w - 4, y + 4, COLORS.text, "right");
      } else {
        label(g, "—", w - 4, y + 4, COLORS.dim, "right");
      }
    }
  }

  function say(text, bad = false) {
    st.message = text;
    st.messageBad = bad;
    st.messageAt = performance.now();
    msg.textContent = text;
    msg.classList.toggle("bad", bad);
  }

  sizeAll();
  paintKeys();
  drawAll();

  return {
    element: root,
    start,
    suspend: () => st.ctx?.suspend(),
    destroy: () => {
      st.destroyed = true;
      io?.disconnect();
      ro?.disconnect();
      st.node?.disconnect();
      st.ctx?.close();
      root.remove();
    },
    pluck,
    setPattern: (id) => start().then(() => {
      const i = st.info.patterns.findIndex((p) => p.id === id || p.label === id);
      if (i < 0 || st.info.patterns[i].id === "score") throw new Error(`unknown pattern ${id}`);
      st.pattern = i;
      st.playing = st.info.patterns[i].id !== "off";
      send({ op: "pattern", index: i });
      paintKeys();
    }),
    loadScore,
    setParam: (id, value) => start().then(() => {
      const p = st.info.params.find((x) => x.id === id || x.index === id);
      if (!p) throw new Error(`unknown param ${id}`);
      setParam(p.index, value);
      paintKeys();
    }),
    setMaster: (db, mute = false) => setMaster(db, mute),
  };
}

// ------------------------------------------------------------------ helpers

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

function canvasBox(cls, aria) {
  const box = el("div", `ovb-screen ${cls}`);
  const canvas = document.createElement("canvas");
  canvas.setAttribute("aria-label", aria);
  box.append(canvas);
  return { box, canvas };
}

function size(c) {
  const r = c.getBoundingClientRect();
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const w = Math.max(1, Math.round(r.width * dpr));
  const h = Math.max(1, Math.round(r.height * dpr));
  if (c.width !== w || c.height !== h) {
    c.width = w;
    c.height = h;
  }
}

function begin(c) {
  const g = c.getContext("2d");
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const w = c.width / dpr, h = c.height / dpr;
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.fillStyle = "#000";
  g.fillRect(0, 0, w, h);
  g.font = `11px ${FONT}`;
  g.lineWidth = 1;
  g.lineCap = "butt";
  return { g, w, h };
}

// A screen's top edge: legend items on the left, units on the right; the
// units drop to a second row when both do not fit. Returns where the
// graticule may start.
function header(g, w, items, units) {
  let x = 4;
  for (const [text, color] of items) {
    label(g, text, x, 12, color);
    x += g.measureText(text).width + 10;
  }
  const uw = g.measureText(units).width;
  if (x + uw + 4 <= w - 4) {
    label(g, units, w - 4, 12, COLORS.dim, "right");
    return 20;
  }
  label(g, units, w - 4, 26, COLORS.dim, "right");
  return 34;
}

function line(g, x0, y0, x1, y1, color, width = 1) {
  g.strokeStyle = color;
  g.lineWidth = width;
  g.beginPath();
  // Half-pixel alignment keeps 1 px graticule lines crisp.
  const a = width % 2 ? 0.5 : 0;
  g.moveTo(Math.round(x0) + a, Math.round(y0) + a);
  g.lineTo(Math.round(x1) + a, Math.round(y1) + a);
  g.stroke();
}

function rect(g, x, y, w, h, color) {
  g.strokeStyle = color;
  g.lineWidth = 1;
  g.strokeRect(Math.round(x) + 0.5, Math.round(y) + 0.5, Math.round(w), Math.round(h));
}

function label(g, text, x, y, color, align = "left") {
  g.fillStyle = color;
  g.textAlign = align;
  g.fillText(text, x, y);
}

function set(node, text, state) {
  if (node.textContent !== text) node.textContent = text;
  node.dataset.state = state || "";
}

function clamp(v, a, b) {
  return v < a ? a : v > b ? b : v;
}

function dbToGain(db) {
  return Math.pow(10, db / 20);
}

function expo(v) {
  if (!Number.isFinite(v)) return "NAN";
  if (v === 0) return "0";
  return v.toExponential(1).toUpperCase();
}

// A radix-2 FFT with its tables made once: magnitude in dBFS (a full-scale
// sine reads about 0) of a Hann-windowed frame, written into `out`.
function makeFft(n) {
  const bits = Math.log2(n) | 0;
  const rev = new Uint32Array(n);
  for (let i = 0; i < n; i++) {
    let r = 0;
    for (let b = 0; b < bits; b++) r |= ((i >> b) & 1) << (bits - 1 - b);
    rev[i] = r;
  }
  const cos = new Float32Array(n / 2), sin = new Float32Array(n / 2);
  for (let i = 0; i < n / 2; i++) {
    cos[i] = Math.cos((2 * Math.PI * i) / n);
    sin[i] = -Math.sin((2 * Math.PI * i) / n);
  }
  const win = new Float32Array(n);
  let wsum = 0;
  for (let i = 0; i < n; i++) {
    win[i] = 0.5 - 0.5 * Math.cos((2 * Math.PI * i) / (n - 1));
    wsum += win[i];
  }
  const re = new Float32Array(n), im = new Float32Array(n);
  return {
    mag(x, out) {
      for (let i = 0; i < n; i++) {
        re[rev[i]] = x[i] * win[i];
        im[rev[i]] = 0;
      }
      for (let size = 2; size <= n; size <<= 1) {
        const half = size >> 1, step = n / size;
        for (let i = 0; i < n; i += size) {
          for (let j = 0, k = 0; j < half; j++, k += step) {
            const a = i + j, b = a + half;
            const tr = re[b] * cos[k] - im[b] * sin[k];
            const ti = re[b] * sin[k] + im[b] * cos[k];
            re[b] = re[a] - tr;
            im[b] = im[a] - ti;
            re[a] += tr;
            im[a] += ti;
          }
        }
      }
      const norm = 2 / wsum;
      for (let i = 0; i < n / 2; i++) {
        const m = Math.sqrt(re[i] * re[i] + im[i] * im[i]) * norm;
        out[i] = 20 * Math.log10(m + 1e-12);
      }
    },
  };
}
