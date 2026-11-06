// Ovillo: every World behind one wasm module and one API.
//
//   import { Ovillo } from "./ovillo.js";
//   const ctx = new AudioContext({ sampleRate: 48000 });
//   const ov = await Ovillo.load(ctx);                 // wasm + worklet, once per context
//   const w = await ov.create("trombone");             // a World, its own AudioWorkletNode
//   w.node.connect(panner).connect(ctx.destination);   // spatialization is yours
//   w.set({ mouth: 0.4, lip: 240 });                  // continuous controls
//   w.event("note", { pitch: 58, velocity: 0.6 });     // events (at: AudioContext seconds)
//   w.telemetry; await w.receipt(); w.dispose();
//
//   // offline (Node or a page), the same Worlds and receipts:
//   const { left, right, receipt } = await Ovillo.render({ kind: "steel-string", score, seconds: 10 });
//
// Nothing here synthesizes a sound. A World does the sounding; its ledger
// is its receipt. Mixing, spatialization, limiting and loudness are outside
// the World, and they are the consumer's (README.md, "The boundary").

import { OvilloCore, renderOffline, utf8Decode } from "./core.js";

export { OvilloCore, renderOffline } from "./core.js";

const isNode = typeof process !== "undefined" && !!(process.versions && process.versions.node);

/** The module's bytes, from a URL (a file path in Node) or as given. */
async function loadBytes(src) {
  if (src instanceof ArrayBuffer) return new Uint8Array(src);
  if (ArrayBuffer.isView(src)) return new Uint8Array(src.buffer, src.byteOffset, src.byteLength);
  const url = src ?? new URL("./ovillo.wasm", import.meta.url);
  if (isNode && (typeof url === "string" ? !/^https?:/.test(url) : url.protocol === "file:")) {
    const { readFile } = await import("node:fs/promises");
    return new Uint8Array(await readFile(url));
  }
  const res = await fetch(url);
  if (!res.ok) throw new Error(`fetch ${url}: ${res.status}`);
  return new Uint8Array(await res.arrayBuffer());
}

async function loadModule(src) {
  if (src instanceof WebAssembly.Module) return src;
  return WebAssembly.compile(await loadBytes(src));
}

let nodeCore = null;

export class Ovillo {
  /**
   * Compile the module, register the worklet on `context`, and read the
   * catalog. `wasmUrl` / `workletUrl` default to the files beside this one;
   * `module` (a WebAssembly.Module) skips the compile.
   */
  static async load(context, { wasmUrl, workletUrl, module } = {}) {
    const bytes = module ? null : await loadBytes(wasmUrl);
    const mod = module ?? (await WebAssembly.compile(bytes));
    await context.audioWorklet.addModule(workletUrl ?? new URL("./ovillo-worklet.js", import.meta.url));
    const instance = await WebAssembly.instantiate(mod, {}); // a Module gives the Instance itself
    const core = new OvilloCore(instance.exports);
    if (context.sampleRate !== 48000) {
      console.warn(`ovillo: this AudioContext runs at ${context.sampleRate} Hz; the tab instruments, the piano and the trombone run at 48000 Hz only (new AudioContext({ sampleRate: 48000 }))`);
    }
    return new Ovillo(context, mod, core, bytes);
  }

  /** Render a World offline, in Node or on a page (no AudioContext needed). */
  static async render(spec) {
    let core;
    if (spec.core) core = spec.core;
    else if (spec.wasm || spec.wasmUrl || spec.module) core = await OvilloCore.load(await loadModule(spec.module ?? spec.wasm ?? spec.wasmUrl));
    else core = nodeCore ?? (nodeCore = await OvilloCore.load(await loadModule()));
    return renderOffline(core, spec);
  }

  /** The catalog without an AudioContext. */
  static async catalog(src) {
    const core = await OvilloCore.load(await loadModule(src));
    return core.catalog();
  }

  constructor(context, module, core, bytes = null) {
    this.context = context;
    this.module = module;
    this.bytes = bytes;
    this.core = core;
    this.catalog = core.catalog();
  }

  /** World kinds by id. */
  kinds() {
    return this.catalog.kinds.map((k) => k.id);
  }

  kind(id) {
    return this.core.kind(id);
  }

  /**
   * Build a World of `kind` in the audio thread and give it an AudioWorkletNode.
   * `channels`: 1 (a point source: downmixed if the World listens at two points)
   * or 2; default the World's own (catalog `channels`).
   */
  async create(kind, { anatomy, options = {}, score, channels, telemetryHz = 30 } = {}) {
    const def = this.core.kind(kind);
    const opts = { ...options };
    if (score !== undefined) opts.score = score;
    const ch = channels ?? def.channels;
    const make = (wasm) => new AudioWorkletNode(this.context, "ovillo-world", {
      numberOfInputs: 0,
      numberOfOutputs: 1,
      outputChannelCount: [ch],
      channelCount: ch,
      channelCountMode: "explicit",
      processorOptions: { ...wasm, kind, anatomy, options: opts, channels: ch, telemetryHz },
    });
    let node;
    try {
      node = make({ module: this.module });
    } catch (err) {
      // A browser that cannot clone a WebAssembly.Module into the worklet gets the bytes (compiled there, once).
      if (!this.bytes || !(err && err.name === "DataCloneError")) throw err;
      node = make({ bytes: this.bytes });
    }
    const w = new World(this, node, def, ch);
    await w.ready;
    return w;
  }

  /** Render offline on this thread with this module (blocks: use a Worker for long renders). */
  render(spec) {
    return renderOffline(this.core, spec);
  }
}

export class World {
  constructor(ov, node, def, channels) {
    this.ov = ov;
    this.node = node;
    this.def = def;
    this.kind = def.id;
    this.channels = channels;
    this.info = null;
    this.telemetry = {};
    this.lastError = null;
    this.onTelemetry = null;
    this.onWarning = (message) => console.warn(`ovillo ${def.id}: ${message}`);
    this._pending = new Map();
    this._next = 1;
    this._disposed = false;
    this.ready = new Promise((resolve, reject) => {
      this._ready = { resolve, reject };
    });
    node.port.onmessage = (e) => this._message(e.data);
  }

  _message(m) {
    switch (m.type) {
      case "ready":
        this.info = m.info;
        this._ready.resolve(this);
        break;
      case "error":
        this.lastError = m.message;
        this._ready.reject(new Error(m.message));
        this.onWarning(m.message);
        break;
      case "warn":
        this.lastError = m.message;
        this.onWarning(m.message);
        break;
      case "telemetry": {
        const names = this.info ? this.info.telemetry : [];
        const t = {};
        names.forEach((n, i) => { t[n] = m.values[i]; });
        this.telemetry = t;
        if (this.onTelemetry) this.onTelemetry(t);
        break;
      }
      default: {
        const p = this._pending.get(m.id);
        if (!p) break;
        this._pending.delete(m.id);
        if (!m.ok) p.reject(new Error(m.message));
        else if (m.type === "receipt") p.resolve(JSON.parse(utf8Decode(m.bytes)));
        else p.resolve(m.read ?? undefined);
      }
    }
  }

  _ask(msg) {
    const id = this._next++;
    return new Promise((resolve, reject) => {
      this._pending.set(id, { resolve, reject });
      this.node.port.postMessage({ ...msg, id });
    });
  }

  _check(kind, name) {
    const list = kind === "event" ? this.def.events : this.def.controls;
    if (!list.some((x) => x.name === name)) {
      throw new Error(`${this.kind} has no ${kind} "${name}"; it has ${list.map((x) => x.name).join(", ")}`);
    }
  }

  /**
   * Set continuous controls: `set("mouth", 0.4, { at, ramp })` or
   * `set({ mouth: 0.4, lip: 240 }, { at, ramp })`. `at` is AudioContext time
   * (omit: the next quantum); `ramp` seconds of linear travel.
   */
  set(nameOrValues, value, opts) {
    let items;
    let o;
    if (typeof nameOrValues === "string") {
      this._check("control", nameOrValues);
      items = [{ type: "set", name: nameOrValues, value }];
      o = opts || {};
    } else {
      o = value || {};
      items = Object.entries(nameOrValues).map(([name, v]) => {
        this._check("control", name);
        return { type: "set", name, value: v };
      });
    }
    for (const it of items) { it.at = o.at; it.ramp = o.ramp; }
    this.node.port.postMessage(items.length === 1 ? items[0] : { type: "batch", items });
  }

  /** Schedule an event: `event("pluck", { string: 6, velocity: 0.7 }, { at })`. */
  event(name, args = {}, { at } = {}) {
    this._check("event", name);
    this.node.port.postMessage({ type: "event", name, args, at });
  }

  /** Play a score on the World (the kind's format: catalog `score`). */
  play(text, { at } = {}) {
    return this._ask({ type: "score", text, at });
  }

  /** The World's receipt: its ledger, what was asked of it, the digest of what it rendered. */
  receipt() {
    return this._ask({ type: "receipt" });
  }

  reset() {
    return this._ask({ type: "reset" });
  }

  dispose() {
    if (this._disposed) return;
    this._disposed = true;
    this.node.port.postMessage({ type: "dispose" });
    try { this.node.disconnect(); } catch (_) { /* not connected */ }
  }
}

export default Ovillo;
