// The portable Ovillo module, wrapped: no Web Audio, no fetch, no
// TextDecoder (an AudioWorkletGlobalScope has none of them), so the same
// code runs in the worklet, in Node and on a page.
//
//   const core = OvilloCore.instantiate(wasmModule)        // sync (worklet, Node)
//   const w = core.create("trombone", { anatomy, options }, 48000)
//   w.event("note", { pitch: 58, velocity: 0.6 }, frame?)    // frame: World frame, omitted = now
//   w.set("mouth", 0.4, frame?, rampFrames?)
//   const n = w.render(128); w.left(n); w.right(n)           // views into wasm memory
//   w.telemetry(); w.receipt(); w.reset(); w.dispose()
//
// Nothing here makes a sound: every sample comes out of an Ariadne World in
// the wasm module, and the World's ledger is its receipt.

export function utf8Decode(bytes) {
  let s = "";
  for (let i = 0; i < bytes.length;) {
    const b = bytes[i++];
    let c;
    if (b < 0x80) c = b;
    else if (b < 0xe0) c = ((b & 0x1f) << 6) | (bytes[i++] & 0x3f);
    else if (b < 0xf0) c = ((b & 0x0f) << 12) | ((bytes[i++] & 0x3f) << 6) | (bytes[i++] & 0x3f);
    else c = ((b & 0x07) << 18) | ((bytes[i++] & 0x3f) << 12) | ((bytes[i++] & 0x3f) << 6) | (bytes[i++] & 0x3f);
    s += String.fromCodePoint(c);
  }
  return s;
}

export function utf8Encode(str) {
  const out = [];
  for (const ch of str) {
    const c = ch.codePointAt(0);
    if (c < 0x80) out.push(c);
    else if (c < 0x800) out.push(0xc0 | (c >> 6), 0x80 | (c & 63));
    else if (c < 0x10000) out.push(0xe0 | (c >> 12), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
    else out.push(0xf0 | (c >> 18), 0x80 | ((c >> 12) & 63), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
  }
  return Uint8Array.from(out);
}

export class OvilloCore {
  /** Instantiate synchronously from a compiled WebAssembly.Module. */
  static instantiate(module) {
    const instance = new WebAssembly.Instance(module, {});
    return new OvilloCore(instance.exports);
  }

  /** Instantiate from bytes or a Module, asynchronously (any thread). */
  static async load(bytesOrModule) {
    const module = bytesOrModule instanceof WebAssembly.Module ? bytesOrModule : await WebAssembly.compile(bytesOrModule);
    return OvilloCore.instantiate(module);
  }

  constructor(exports) {
    this.x = exports;
    if (this.x.ov_abi_version() !== 1) throw new Error(`ovillo.wasm ABI ${this.x.ov_abi_version()}, this core speaks 1`);
    this.maxFrames = this.x.ov_max_frames();
    this.maxArgs = this.x.ov_max_args();
    this._catalog = null;
    this._kinds = null;
  }

  get memory() { return this.x.memory; }

  /** The last text an export wrote. */
  text(len = this.x.ov_text_len()) {
    return utf8Decode(new Uint8Array(this.x.memory.buffer, this.x.ov_text_ptr(), len));
  }

  /** The text's bytes, copied (to post off the audio thread for decoding). */
  textBytes(len = this.x.ov_text_len()) {
    return new Uint8Array(this.x.memory.buffer, this.x.ov_text_ptr(), len).slice();
  }

  input(str) {
    const bytes = utf8Encode(str);
    const ptr = this.x.ov_input_ptr(bytes.length); // may grow memory: view after
    new Uint8Array(this.x.memory.buffer, ptr, bytes.length).set(bytes);
    return bytes.length;
  }

  /** The catalog (`ovillo.portable.catalog.v1`), parsed once. */
  catalog() {
    if (!this._catalog) {
      this._catalog = JSON.parse(this.text(this.x.ov_catalog()));
      this._kinds = new Map(this._catalog.kinds.map((k) => [k.id, k]));
    }
    return this._catalog;
  }

  kind(id) {
    this.catalog();
    const k = this._kinds.get(id);
    if (!k) throw new Error(`no World kind "${id}"; the catalog has ${[...this._kinds.keys()].join(", ")}`);
    return k;
  }

  /** Build a World. `spec`: { anatomy?, options? }. */
  create(kind, spec = {}, sampleRate = 48000) {
    const def = this.kind(kind);
    const request = { kind, anatomy: spec.anatomy ?? undefined, options: spec.options ?? {} };
    const len = this.input(JSON.stringify(request));
    const h = this.x.ov_create(len, sampleRate);
    if (h === 0) throw new Error(this.text());
    return new CoreWorld(this, h, def, sampleRate);
  }

  instances() { return this.x.ov_instances(); }
}

/** Map named arguments onto the catalog's order (enum names to indices). */
export function eventArgs(def, args = {}) {
  const out = new Float64Array(def.args.length);
  def.args.forEach((a, i) => {
    let v = args[a.name];
    if (v === undefined || v === null) v = a.default;
    else if (typeof v === "string") {
      const k = a.values ? a.values.indexOf(v) : -1;
      if (k < 0) throw new Error(`${def.name}.${a.name}: "${v}" is not one of ${(a.values || []).join(", ")}`);
      v = k;
    } else if (typeof v === "boolean") v = v ? 1 : 0;
    out[i] = Number(v);
  });
  return out;
}

export function controlValue(def, value) {
  if (typeof value === "string") {
    const k = def.values ? def.values.indexOf(value) : -1;
    if (k < 0) throw new Error(`${def.name}: "${value}" is not one of ${(def.values || []).join(", ")}`);
    return k;
  }
  if (typeof value === "boolean") return value ? 1 : 0;
  return Number(value);
}

export class CoreWorld {
  constructor(core, handle, def, sampleRate) {
    this.core = core;
    this.h = handle;
    this.def = def;
    this.kind = def.id;
    this.sampleRate = sampleRate;
    this.events = new Map(def.events.map((e, i) => [e.name, { i, e }]));
    this.controls = new Map(def.controls.map((c, i) => [c.name, { i, c }]));
    this.info = JSON.parse(core.text(core.x.ov_info(handle)));
    this.slotNames = this.info.telemetry;
    this._views();
  }

  _views() {
    const x = this.core.x;
    this._buf = x.memory.buffer;
    this._left = new Float32Array(this._buf, x.ov_left_ptr(this.h), this.core.maxFrames);
    this._right = new Float32Array(this._buf, x.ov_right_ptr(this.h), this.core.maxFrames);
    this._tele = new Float64Array(this._buf, x.ov_telemetry_ptr(this.h), x.ov_telemetry_len(this.h));
  }

  _fresh() { if (this.core.x.memory.buffer !== this._buf) this._views(); }

  /** Schedule a catalog event. `frame`: the World frame it lands on (omit: now). */
  event(name, args = {}, frame = -1) {
    const ev = this.events.get(name);
    if (!ev) throw new Error(`${this.kind} has no event "${name}"; it has ${[...this.events.keys()].join(", ")}`);
    const a = eventArgs(ev.e, args);
    const x = this.core.x;
    const ptr = x.ov_args_ptr();
    new Float64Array(x.memory.buffer, ptr, this.core.maxArgs).set(a.subarray(0, this.core.maxArgs));
    if (x.ov_event(this.h, ev.i, frame, a.length) !== 0) throw new Error(this.core.text());
  }

  /** Set a continuous control at `frame` (omit: now), ramping over `rampFrames`. */
  set(name, value, frame = -1, rampFrames = 0) {
    const c = this.controls.get(name);
    if (!c) throw new Error(`${this.kind} has no control "${name}"; it has ${[...this.controls.keys()].join(", ")}`);
    if (this.core.x.ov_set(this.h, c.i, controlValue(c.c, value), frame, rampFrames) !== 0) throw new Error(this.core.text());
  }

  control(name) {
    const c = this.controls.get(name);
    return c ? this.core.x.ov_control(this.h, c.i) : NaN;
  }

  /** Hand a score to the World from `frame` (omit: now). Returns what reading it found. */
  playScore(text, frame = -1) {
    const len = this.core.input(text);
    const rc = this.core.x.ov_play_score(this.h, len, frame);
    const out = this.core.text();
    if (rc !== 0) throw new Error(out);
    return JSON.parse(out);
  }

  /** Render `n` frames (at most maxFrames); read them with left(n), right(n). */
  render(n) {
    const k = this.core.x.ov_render(this.h, n);
    this._fresh();
    return k;
  }

  left(n) { this._fresh(); return this._left.subarray(0, n); }
  right(n) { this._fresh(); return this._right.subarray(0, n); }

  /** The telemetry slots as a Float64Array view (names: slotNames). */
  telemetryArray() { this._fresh(); return this._tele; }

  /** The telemetry as an object of named numbers. */
  telemetry() {
    const t = this.telemetryArray();
    const o = {};
    this.slotNames.forEach((name, i) => { o[name] = t[i]; });
    return o;
  }

  frame() { return this.core.x.ov_frame(this.h); }

  /** Re-read the ledger into telemetry now. */
  refresh() { this.core.x.ov_refresh(this.h); this._fresh(); }

  receipt() { return JSON.parse(this.core.text(this.core.x.ov_receipt(this.h))); }

  receiptBytes() { return this.core.textBytes(this.core.x.ov_receipt(this.h)); }

  lastError() { return this.core.text(this.core.x.ov_error(this.h)); }

  reset() { if (this.core.x.ov_reset(this.h) !== 0) throw new Error(this.core.text()); }

  dispose() { this.core.x.ov_dispose(this.h); this.h = 0; }
}

/**
 * Render a World offline (Node, a Worker, a page): the same path a worklet
 * runs, a block at a time. Times in `events` and `controls` are seconds.
 */
export function renderOffline(core, spec) {
  const sampleRate = spec.sampleRate ?? 48000;
  const options = { ...(spec.options ?? spec.plan ?? {}) };
  if (spec.score !== undefined) options.score = spec.score;
  const w = core.create(spec.kind, { anatomy: spec.anatomy, options }, sampleRate);
  try {
    for (const e of spec.events ?? []) w.event(e.name, e.args ?? {}, Math.round((e.at ?? 0) * sampleRate));
    for (const c of spec.controls ?? []) w.set(c.name, c.value, Math.round((c.at ?? 0) * sampleRate), Math.round((c.ramp ?? 0) * sampleRate));
    let frames = spec.frames ?? (spec.seconds !== undefined ? Math.round(spec.seconds * sampleRate) : undefined);
    if (frames === undefined) {
      const read = w.receipt().scores?.[0];
      if (!read || read.samples === undefined) throw new Error("give seconds (or frames) for a render without a score");
      frames = read.samples + Math.round((spec.tailSeconds ?? options.tailSeconds ?? 2) * sampleRate);
    }
    const left = new Float32Array(frames);
    const right = new Float32Array(frames);
    const block = spec.block ?? 4096;
    let done = 0;
    const t0 = globalThis.performance?.now?.() ?? Date.now();
    while (done < frames) {
      const k = w.render(Math.min(block, frames - done));
      left.set(w.left(k), done);
      right.set(w.right(k), done);
      done += k;
    }
    const wallMs = (globalThis.performance?.now?.() ?? Date.now()) - t0;
    return { left, right, sampleRate, frames, channels: w.def.channels, receipt: w.receipt(), wallSeconds: wallMs / 1000 };
  } finally {
    w.dispose();
  }
}
