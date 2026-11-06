// The portable Ovillo module in an AudioWorklet. Register with
//   await context.audioWorklet.addModule(new URL("./ovillo-worklet.js", import.meta.url))
// (ovillo.js does it). Each World is one AudioWorkletNode running the
// "ovillo-world" processor; every processor of one AudioContext shares ONE
// wasm instance (this scope's), and owns one World handle in it.
//
// processorOptions: { module: WebAssembly.Module (or bytes), kind, anatomy?, options?,
//   channels: 1 | 2, telemetryHz? }. The World is built synchronously in
// the constructor, so an OfflineAudioContext hears it from its first quantum.
// Building a World runs on the audio thread: other Worlds of this context
// skip while it builds (a few ms for most kinds, longer for a big piano).
//
// Messages in (port):  {type: "event", name, args, at?}  {type: "set", name, value, at?, ramp?}
//   {type: "batch", items: [...]}  {type: "score", id, text, at?}  {type: "receipt", id}
//   {type: "reset", id}  {type: "dispose"}
//   `at` is AudioContext time (s); omitted or negative is the next quantum. `ramp` is seconds.
// Messages out: {type: "ready", info} | {type: "error", message} | {type: "warn", message}
//   {type: "telemetry", values: Float64Array} ~30 times a second
//   {type: "score" | "receipt" | "reset", id, ok, ...}

import { OvilloCore } from "./core.js";

const SHARED = Symbol.for("ovillo.core");

function coreFor(o) {
  const scope = globalThis;
  let c = scope[SHARED];
  if (c && c.broken) throw new Error("the Ovillo module in this AudioContext trapped earlier; make a new AudioContext");
  if (!c) {
    // Synchronous: off the main thread a module of any size may be compiled and instantiated at once.
    const module = o.module instanceof WebAssembly.Module ? o.module : new WebAssembly.Module(o.bytes);
    c = OvilloCore.instantiate(module);
    scope[SHARED] = c;
  }
  return c;
}

class OvilloWorldProcessor extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const o = options.processorOptions || {};
    this.alive = true;
    this.world = null;
    this.channels = o.channels === 1 ? 1 : 2;
    this.natural = 2;
    this.offset = 0;
    this.telemetryEvery = Math.max(128, Math.round(sampleRate / (o.telemetryHz || 30)));
    this.untilTelemetry = this.telemetryEvery;
    this.port.onmessage = (e) => this.onMessage(e.data);
    try {
      this.core = coreFor(o);
      this.world = this.core.create(o.kind, { anatomy: o.anatomy, options: o.options || {} }, sampleRate);
      this.natural = this.world.def.channels;
      this.port.postMessage({ type: "ready", info: this.world.info, frame: this.world.frame() });
    } catch (err) {
      this.port.postMessage({ type: "error", message: String((err && err.message) || err) });
    }
  }

  /** AudioContext time (s) to the World's frame; -1 is "now". */
  frameOf(at) {
    if (at === undefined || at === null || !(at >= 0)) return -1;
    return Math.round(at * sampleRate) + this.offset;
  }

  apply(m) {
    const w = this.world;
    switch (m.type) {
      case "event":
        w.event(m.name, m.args || {}, this.frameOf(m.at));
        break;
      case "set":
        w.set(m.name, m.value, this.frameOf(m.at), Math.round((m.ramp || 0) * sampleRate));
        break;
      case "batch":
        for (const item of m.items) {
          try { this.apply(item); } catch (err) { this.port.postMessage({ type: "warn", message: String(err.message || err) }); }
        }
        break;
      case "score": {
        let reply;
        try { reply = { type: "score", id: m.id, ok: true, read: w.playScore(m.text, this.frameOf(m.at)) }; }
        catch (err) { reply = { type: "score", id: m.id, ok: false, message: String(err.message || err) }; }
        this.port.postMessage(reply);
        break;
      }
      case "receipt": {
        const bytes = w.receiptBytes();
        this.port.postMessage({ type: "receipt", id: m.id, ok: true, bytes }, [bytes.buffer]);
        break;
      }
      case "reset":
        w.reset();
        this.port.postMessage({ type: "reset", id: m.id, ok: true });
        break;
      case "dispose":
        w.dispose();
        this.world = null;
        this.alive = false;
        break;
      default:
        break;
    }
  }

  onMessage(m) {
    if (!this.world) {
      if (m && m.id !== undefined) this.port.postMessage({ type: m.type, id: m.id, ok: false, message: "no World" });
      return;
    }
    try {
      this.apply(m);
    } catch (err) {
      const message = String((err && err.message) || err);
      if (m && m.id !== undefined) this.port.postMessage({ type: m.type, id: m.id, ok: false, message });
      else this.port.postMessage({ type: "warn", message });
    }
  }

  process(_inputs, outputs) {
    if (!this.alive) return false;
    const out = outputs[0];
    if (!out || out.length === 0) return true;
    const n = out[0].length;
    const w = this.world;
    if (!w) {
      for (const ch of out) ch.fill(0);
      return true;
    }
    try {
      // The World's clock and the context's differ by a constant once rendering starts.
      this.offset = w.frame() - currentFrame;
      const k = w.render(n);
      const l = w.left(k);
      if (this.channels === 1 || out.length === 1) {
        if (this.natural === 1) out[0].set(l);
        else {
          const r = w.right(k);
          const o0 = out[0];
          for (let i = 0; i < k; i++) o0[i] = 0.5 * (l[i] + r[i]);
        }
        for (let c = 1; c < out.length; c++) out[c].set(out[0]);
      } else {
        out[0].set(l);
        out[1].set(this.natural === 1 ? l : w.right(k));
      }
      this.untilTelemetry -= n;
      if (this.untilTelemetry <= 0) {
        this.untilTelemetry += this.telemetryEvery;
        const values = w.telemetryArray().slice();
        this.port.postMessage({ type: "telemetry", values }, [values.buffer]);
      }
    } catch (err) {
      // A trap leaves the shared module in an unknown state: every World of this context stops.
      this.core.broken = true;
      this.world = null;
      this.port.postMessage({ type: "error", message: `the World trapped: ${String((err && err.message) || err)}` });
      for (const ch of out) ch.fill(0);
    }
    return true;
  }
}

registerProcessor("ovillo-world", OvilloWorldProcessor);
