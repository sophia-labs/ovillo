// The audition page: every World in the catalog, live. Serve the repository
// root (so the example scores load): `python3 -m http.server 8766` in the
// repository, then http://localhost:8766/web/ovillo/audition/.
import { Ovillo } from "../ovillo.js";

const $ = (sel, el = document) => el.querySelector(sel);
const h = (tag, attrs = {}, ...kids) => {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === "class") el.className = v;
    else if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
    else if (v !== undefined && v !== null && v !== false) el.setAttribute(k, v === true ? "" : v);
  }
  for (const kid of kids.flat()) if (kid !== null && kid !== undefined) el.append(kid.nodeType ? kid : document.createTextNode(String(kid)));
  return el;
};

/** Example scores in the repository, by kind (fetched from the repository root). */
const EXAMPLES = {
  "steel-string": ["examples/tab/steel_string_tour.mus", "examples/tab/bowed-swell.mus", "examples/tab/bowed-note.mus"],
  piano: ["examples/piano/prelude-c-major.mus"],
  trombone: ["examples/ensemble/before-the-law/doorkeeper/doorkeeper-audition.tsv", "examples/ensemble/before-the-law/doorkeeper/alert.tsv"],
  bench: ["ovillo/scores/first-light.json"],
};
const ROOT = new URL("../../../", import.meta.url);

let ctx = null;
let ov = null;
let catalog = null;
let limiter = null;
const live = new Set();

async function boot() {
  catalog = await Ovillo.catalog(new URL("../ovillo.wasm", import.meta.url));
  const nav = $("#kinds");
  const families = {};
  for (const k of catalog.kinds) (families[k.family] ??= []).push(k);
  const names = { tab: "Tab instruments", piano: "Piano", trombone: "Trombone", tract: "Vocal tract", fart: "Fart", bench: "The bench", "game-sound": "Game sources" };
  for (const [fam, list] of Object.entries(families)) {
    nav.append(h("h2", {}, names[fam] ?? fam));
    for (const k of list) {
      nav.append(h("button", { class: "kind", onclick: (e) => choose(k, e.currentTarget) }, h("b", {}, k.title), h("span", {}, `${k.id} · ${k.events.length} events · ${k.controls.length} controls`)));
    }
  }
}

$("#start").addEventListener("click", async (e) => {
  try {
    ctx = new AudioContext({ sampleRate: 48000, latencyHint: "interactive" });
    ov = await Ovillo.load(ctx, { wasmUrl: new URL("../ovillo.wasm", import.meta.url), workletUrl: new URL("../ovillo-worklet.js", import.meta.url) });
    limiter = new DynamicsCompressorNode(ctx, { threshold: -3, knee: 0, ratio: 20, attack: 0.001, release: 0.1 });
    limiter.connect(ctx.destination);
    e.target.disabled = true;
    e.target.textContent = `audio on (${ctx.sampleRate} Hz)`;
    $("#status").textContent = "";
  } catch (err) {
    $("#status").textContent = String(err.message || err);
  }
});

$("#limiter").addEventListener("change", () => {
  for (const w of live) w.reroute();
});

function choose(def, button) {
  for (const b of document.querySelectorAll(".kind")) b.classList.toggle("on", b === button);
  const card = h("div", { class: "card" });
  card.append(
    h("h3", {}, def.title),
    h("p", { class: "about" }, def.summary),
    h("p", { class: "about" }, `Output: ${def.output.observes} × ${def.output.gain.toPrecision(3)} (${def.output.channels}). Cost: ${def.cost}. Spec: ${def.spec}.`),
  );
  const anatomy = def.anatomies.length > 1 ? h("select", {}, def.anatomies.map((a) => h("option", { value: a.id, selected: a.default, title: a.about }, a.id))) : null;
  const build = h("button", { class: "primary", onclick: async () => {
    if (!ov) { $("#status").textContent = "start audio first"; return; }
    build.disabled = true;
    try {
      const w = await LiveWorld.create(def, anatomy ? anatomy.value : undefined);
      $("#stage").prepend(w.card);
    } catch (err) {
      $("#status").textContent = String(err.message || err);
    }
    build.disabled = false;
  } }, "Build this World");
  card.append(h("div", { class: "row" }, anatomy ? h("label", {}, "anatomy ", anatomy) : null, build));
  card.append(h("details", {}, h("summary", {}, "What it does not model"), h("ul", { class: "limits" }, def.doesNotModel.map((l) => h("li", {}, l))), h("p", { class: "about" }, `See ${def.doesNotModelRef}.`)));
  const stage = $("#stage");
  const old = $(".card.chooser", stage);
  if (old) old.remove();
  card.classList.add("chooser");
  stage.prepend(card);
}

function argInput(a) {
  if (a.values) return h("select", { "data-arg": a.name }, a.values.map((v, i) => h("option", { value: v, selected: i === a.default }, v)));
  const step = a.units === "index" || a.units === "count" ? 1 : (a.max - a.min) / 1000 || 0.001;
  return h("input", { type: "number", "data-arg": a.name, min: a.min, max: a.max, step, value: a.default, title: a.about });
}

class LiveWorld {
  static async create(def, anatomy) {
    const w = await ov.create(def.id, { anatomy });
    return new LiveWorld(def, anatomy, w);
  }

  constructor(def, anatomy, w) {
    this.def = def;
    this.w = w;
    this.pan = new StereoPannerNode(ctx);
    this.gain = new GainNode(ctx, { gain: 1 });
    this.analyser = new AnalyserNode(ctx, { fftSize: 1024 });
    w.node.connect(this.pan).connect(this.gain).connect(this.analyser);
    this.reroute();
    live.add(this);
    this.card = h("div", { class: "card" });
    this.card.append(h("h3", {}, `${def.title}${anatomy ? ` · ${anatomy}` : ""}`), h("p", { class: "about" }, `plan ${w.info.planDigest.slice(0, 40)}…`));
    this.err = h("div", { class: "err" });
    w.onWarning = (m) => { this.err.textContent = m; };
    this.card.append(this.err);
    if (def.events.length) this.card.append(h("div", { class: "events" }, def.events.map((e) => this.eventBox(e))));
    if (def.controls.length) this.card.append(h("div", { class: "controls" }, def.controls.map((c) => this.controlBox(c))));
    if (def.score) this.card.append(this.scoreBox());
    this.tele = h("div", { class: "tele" });
    this.books = h("div", { class: "books" });
    this.meter = h("div", { class: "meter" }, h("i"));
    this.receiptPre = h("pre");
    this.card.append(
      h("div", { class: "row" },
        h("button", { onclick: () => this.showReceipt() }, "Receipt"),
        h("button", { onclick: () => w.reset() }, "Reset World"),
        h("button", { onclick: () => this.dispose() }, "Dispose"),
        this.meter),
      this.books,
      h("details", {}, h("summary", {}, "Telemetry"), this.tele),
      h("details", {}, h("summary", {}, "Receipt"), this.receiptPre),
      h("div", { class: "mix" }, "The listener's mix, outside the World: ",
        h("label", {}, " pan ", h("input", { type: "range", min: -1, max: 1, step: 0.01, value: 0, oninput: (e) => { this.pan.pan.value = Number(e.target.value); } })),
        h("label", {}, " gain dB ", h("input", { type: "range", min: -40, max: 30, step: 0.5, value: 0, oninput: (e) => { this.gain.gain.value = 10 ** (Number(e.target.value) / 20); } })),
      ),
    );
    w.onTelemetry = (t) => this.paint(t);
    this.buf = new Float32Array(1024);
  }

  reroute() {
    try { this.analyser.disconnect(); } catch (_) { /* first time */ }
    this.analyser.connect($("#limiter").checked ? limiter : ctx.destination);
  }

  eventBox(e) {
    const inputs = e.args.map(argInput);
    const fire = () => {
      const args = {};
      for (const el of inputs) args[el.dataset.arg] = el.tagName === "SELECT" ? el.value : Number(el.value);
      this.w.event(e.name, args);
    };
    return h("div", { class: "ev", title: e.about },
      h("button", { onclick: fire }, e.name),
      e.args.map((a, i) => h("label", { title: a.about }, `${a.name}${a.units && a.units !== "enum" ? ` (${a.units})` : ""}`, inputs[i])));
  }

  controlBox(c) {
    const value = h("span", { class: "v" }, String(c.default));
    let input;
    if (c.values) {
      input = h("select", { onchange: (e) => { this.w.set(c.name, e.target.value); value.textContent = e.target.value; } }, c.values.map((v, i) => h("option", { value: v, selected: i === c.default }, v)));
    } else if (c.steps) {
      input = h("input", { type: "range", min: c.min, max: c.max, step: 1, value: c.default, oninput: (e) => { this.w.set(c.name, Number(e.target.value)); value.textContent = e.target.value; } });
    } else {
      const span = c.max - c.min;
      input = h("input", { type: "range", min: c.min, max: c.max, step: span / 1000, value: c.default, oninput: (e) => {
        // Moved at the control's own sensible rate, ramped over one update.
        this.w.set(c.name, Number(e.target.value), { ramp: 1 / c.rateHz });
        value.textContent = Number(e.target.value).toPrecision(4);
      } });
    }
    return h("div", { class: "ctl", title: `${c.about} — ${c.smoothing}` }, h("label", {}, `${c.name}${c.units ? ` (${c.units})` : ""}`, value), input);
  }

  scoreBox() {
    const text = h("textarea", { spellcheck: "false", placeholder: this.def.score.about });
    const pick = h("select", {}, h("option", { value: "" }, "an example…"), (EXAMPLES[this.def.id] ?? EXAMPLES[this.def.family] ?? []).map((p) => h("option", { value: p }, p)));
    pick.addEventListener("change", async () => {
      if (!pick.value) return;
      try { text.value = await (await fetch(new URL(pick.value, ROOT))).text(); } catch (err) { this.err.textContent = `could not load ${pick.value}: ${err}`; }
    });
    const play = h("button", { onclick: async () => {
      try {
        const read = await this.w.play(text.value);
        this.err.textContent = `score: ${JSON.stringify(read).slice(0, 240)}`;
      } catch (err) { this.err.textContent = String(err.message || err); }
    } }, "Play score");
    return h("details", {}, h("summary", {}, `Score (${this.def.score.format})`), h("p", { class: "about" }, this.def.score.about), h("div", { class: "row" }, pick, play), text);
  }

  paint(t) {
    this.tele.replaceChildren(...Object.entries(t).map(([k, v]) => h("div", {}, h("span", {}, k), h("span", {}, Math.abs(v) >= 1e4 || (Math.abs(v) < 1e-3 && v !== 0) ? v.toExponential(3) : v.toFixed(4)))));
    const cum = t.cumulativeClosureResidual ?? 0;
    const frame = t.maximumFrameResidual ?? 0;
    const ok = Math.abs(frame) < 1e-9;
    this.books.replaceChildren(h("span", { class: ok ? "ok" : "bad" }, `books: source ${(t.sourceWork ?? 0).toExponential(3)} J · loss ${(t.internalLoss ?? 0).toExponential(3)} J · radiated ${(t.radiation ?? 0).toExponential(3)} J · residual ${cum.toExponential(2)} J (largest frame ${frame.toExponential(2)} J) · ${(t.seconds ?? 0).toFixed(1)} s`));
    this.analyser.getFloatTimeDomainData(this.buf);
    let peak = 0;
    for (const v of this.buf) peak = Math.max(peak, Math.abs(v));
    const bar = this.meter.firstChild;
    bar.style.width = `${Math.min(100, peak * 100)}%`;
    bar.style.background = peak >= 1 ? "var(--warn)" : "var(--good)";
  }

  async showReceipt() {
    const r = await this.w.receipt();
    this.receiptPre.textContent = JSON.stringify(r, null, 2);
    this.receiptPre.parentElement.open = true;
  }

  dispose() {
    this.w.dispose();
    live.delete(this);
    this.card.remove();
  }
}

/** `?smoke=1`: build every kind live in turn, touch it, wait for telemetry, dispose (tools/browser.mjs reads the title). */
async function smoke() {
  const out = h("pre", { id: "out" });
  document.body.append(out);
  const report = {};
  $("#start").click();
  for (let i = 0; i < 100 && !ov; i++) await new Promise((r) => setTimeout(r, 100));
  if (!ov) throw new Error("audio did not start");
  for (const def of catalog.kinds) {
    const t0 = performance.now();
    const lw = await LiveWorld.create(def, undefined);
    $("#stage").prepend(lw.card);
    const first = def.events[0];
    if (first) lw.w.event(first.name, {});
    for (const c of def.controls.filter((c) => !c.steps).slice(0, 2)) lw.w.set(c.name, c.min + 0.6 * (c.max - c.min), { ramp: 0.05 });
    await new Promise((r) => setTimeout(r, 700));
    const t = lw.w.telemetry;
    report[def.id] = { buildMs: Math.round(performance.now() - t0 - 700), seconds: t.seconds, frames: t.frame, refused: t.eventsRefused, errors: t.errors, lastError: lw.w.lastError };
    lw.dispose();
  }
  const ok = Object.values(report).every((r) => r.frames > 0 && r.errors === 0);
  out.textContent = JSON.stringify(report, null, 1);
  document.title = ok ? "PASS" : "FAIL";
}

boot()
  .then(() => (new URLSearchParams(location.search).has("smoke") ? smoke() : null))
  .catch((err) => {
    $("#status").textContent = String(err.message || err);
    document.title = "FAIL";
  });
