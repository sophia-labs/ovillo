// The Ovillo rig in an AudioWorklet: one ovillo-web instance (the simulated
// guitar, its guitarist and the Ariadne World), rendered a quantum at a time.
//
// Messages in (port): {op: "wasm", bytes} once, then {op, ...args} controls.
// Messages out: {op: "ready", info}, {op: "error", message},
// {op: "score", ok, text}, and ~30 times a second {op: "frame", buf, load}
// where buf is one of three ping-ponged ArrayBuffers (transferred, never
// allocated here after start): dry[N], wet[N] (the World before the master),
// then the telemetry slots as f64. The page returns each buffer with
// {op: "return", buf}.

const N = 4096; // frames of history per snapshot (scope, spectrum)
const POST_EVERY_S = 1 / 30;

const now = globalThis.performance && typeof globalThis.performance.now === "function"
  ? () => globalThis.performance.now()
  : () => Date.now();

// AudioWorkletGlobalScope has no TextEncoder/TextDecoder.
function utf8Decode(bytes) {
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

function utf8Encode(str) {
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

class OvilloProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.x = null;
    this.mem = null;
    this.buf = null;
    this.ringDry = new Float32Array(N);
    this.ringWet = new Float32Array(N);
    this.ringAt = 0;
    this.pool = [];
    this.untilPost = 0;
    this.loadEma = 0;
    this.port.onmessage = (e) => this.onMessage(e.data);
  }

  async onMessage(m) {
    if (m.op === "wasm") {
      try {
        const { instance } = await WebAssembly.instantiate(m.bytes, {});
        const x = instance.exports;
        if (x.ovillo_init(sampleRate, (Math.random() * 0xffffffff) >>> 0 || 1) !== 0) {
          throw new Error("ovillo_init failed");
        }
        if (m.master) x.ovillo_set_master_now(m.master.gain, m.master.mute ? 1 : 0);
        this.x = x;
        this.mem = x.memory;
        const len = x.ovillo_info(); // may grow memory: views after
        const info = JSON.parse(this.text(len));
        this.slotCount = info.slots.count;
        this.frameBytes = (2 * N) * 4 + this.slotCount * 8;
        for (let i = 0; i < 3; i++) this.pool.push(new ArrayBuffer(this.frameBytes));
        this.views();
        this.untilPost = Math.round(sampleRate * POST_EVERY_S);
        this.port.postMessage({ op: "ready", info });
      } catch (err) {
        this.port.postMessage({ op: "error", message: String(err && err.message || err) });
      }
      return;
    }
    if (m.op === "return") {
      if (m.buf && m.buf.byteLength === this.frameBytes) this.pool.push(m.buf);
      return;
    }
    const x = this.x;
    if (!x) return;
    switch (m.op) {
      case "pluck": x.ovillo_pluck(m.string, m.fret | 0, m.velocity); break;
      case "pattern": x.ovillo_set_pattern(m.index); break;
      case "transport": x.ovillo_transport(m.on ? 1 : 0); break;
      case "tempo": x.ovillo_set_tempo(m.bpm); break;
      case "param": x.ovillo_set_param(m.index, m.value); break;
      case "pickup": x.ovillo_set_pickup(m.index); break;
      case "guitar": x.ovillo_set_guitar(m.tone, m.volume, m.hum ? 1 : 0); break;
      case "reset": x.ovillo_reset_world(); break;
      case "muteStrings": x.ovillo_mute_strings(); break;
      case "master": x.ovillo_set_master(m.gain, m.mute ? 1 : 0); break;
      case "defaultPatch": {
        x.ovillo_default_patch();
        const len = x.ovillo_info();
        this.port.postMessage({ op: "info", info: JSON.parse(this.text(len)) });
        break;
      }
      case "score": {
        const bytes = utf8Encode(m.json);
        const ptr = x.ovillo_input_ptr(bytes.length);
        new Uint8Array(this.mem.buffer, ptr, bytes.length).set(bytes);
        const rc = x.ovillo_load_score(bytes.length);
        this.port.postMessage({ op: "score", ok: rc === 0, text: this.text(x.ovillo_text_len()), id: m.id });
        break;
      }
    }
    this.views();
  }

  text(len) {
    const bytes = new Uint8Array(this.mem.buffer, this.x.ovillo_text_ptr(), len);
    return utf8Decode(bytes);
  }

  // Typed views into wasm memory; remade whenever memory has grown.
  views() {
    const x = this.x;
    this.buf = this.mem.buffer;
    const max = x.ovillo_max_frames();
    this.left = new Float32Array(this.buf, x.ovillo_left_ptr(), max);
    this.right = new Float32Array(this.buf, x.ovillo_right_ptr(), max);
    this.dry = new Float32Array(this.buf, x.ovillo_dry_ptr(), max);
    this.wet = new Float32Array(this.buf, x.ovillo_wet_ptr(), max);
    this.slots = new Float64Array(this.buf, x.ovillo_telemetry_ptr(), this.slotCount);
    this.quantumViews(this.qn || 128);
  }

  // Views of exactly one quantum, so process() makes no objects.
  quantumViews(n) {
    this.qn = n;
    this.qLeft = this.left.subarray(0, n);
    this.qRight = this.right.subarray(0, n);
  }

  process(_inputs, outputs) {
    const out = outputs[0];
    const x = this.x;
    if (!x || !out || out.length === 0) return true;
    const n = out[0].length;
    const t0 = now();
    x.ovillo_render(n);
    if (this.mem.buffer !== this.buf) this.views();
    if (n !== this.qn) this.quantumViews(n);
    out[0].set(this.qLeft);
    if (out.length > 1) out[1].set(this.qRight);
    const dt = now() - t0;
    const budget = (n / sampleRate) * 1000;
    this.loadEma += (dt / budget - this.loadEma) * 0.02;

    // The history ring the screens read.
    let at = this.ringAt;
    for (let i = 0; i < n; i++) {
      this.ringDry[at] = this.dry[i];
      this.ringWet[at] = this.wet[i];
      at = (at + 1) & (N - 1);
    }
    this.ringAt = at;

    this.untilPost -= n;
    if (this.untilPost <= 0 && this.pool.length > 0) {
      this.untilPost += Math.round(sampleRate * POST_EVERY_S);
      const ab = this.pool.pop();
      const f32 = new Float32Array(ab, 0, 2 * N);
      // Unroll the ring, oldest first.
      f32.set(this.ringDry.subarray(at), 0);
      f32.set(this.ringDry.subarray(0, at), N - at);
      f32.set(this.ringWet.subarray(at), N);
      f32.set(this.ringWet.subarray(0, at), 2 * N - at);
      new Float64Array(ab, 2 * N * 4, this.slotCount).set(this.slots);
      this.port.postMessage({ op: "frame", buf: ab, load: this.loadEma }, [ab]);
    }
    return true;
  }
}

registerProcessor("ovillo", OvilloProcessor);
