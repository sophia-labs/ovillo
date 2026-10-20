// The tract World inside an AudioWorkletProcessor.
//
// The page passes the wasm bytes in processorOptions (a worklet cannot fetch);
// the module is the C-ABI cdylib ovillo-tract-wasm: no imports, no bindgen.
// Each render quantum asks the World for 128 samples and copies them out of
// linear memory. Every few quanta it snapshots the World's state for the
// pictures and posts it to the page with the measured DSP load.

const SNAPSHOT_EVERY = 4;

class TractProcessor extends AudioWorkletProcessor {
  constructor(options) {
    super();
    this.ready = false;
    this.x = null;
    this.blocks = 0;
    this.busyMs = 0;
    this.audioMs = 0;
    this.load = 0;
    this.clock = (globalThis.performance && typeof globalThis.performance.now === 'function')
      ? () => globalThis.performance.now()
      : () => Date.now();
    this.port.onmessage = (e) => this.onMessage(e.data);
    const { bytes, init } = options.processorOptions;
    this.pending = Array.isArray(init) ? init.slice() : [];
    // Synchronous first, so the World is ready for the very first quantum
    // (an OfflineAudioContext renders before an async compile could finish).
    try {
      const instance = new WebAssembly.Instance(new WebAssembly.Module(bytes), {});
      this.boot(instance.exports);
    } catch (_syncRefused) {
      WebAssembly.instantiate(bytes, {}).then(({ instance }) => this.boot(instance.exports))
        .catch((err) => this.port.postMessage({ type: 'error', message: String(err) }));
    }
  }

  boot(x) {
    const sections = x.tract_init(sampleRate);
    this.x = x;
    this.ready = true;
    for (const m of this.pending) this.apply(m);
    this.pending = [];
    this.port.postMessage({ type: 'ready', sections, sampleRate });
  }

  onMessage(m) {
    if (!this.ready) { this.pending.push(m); return; }
    this.apply(m);
  }

  apply(m) {
    const x = this.x;
    switch (m.type) {
      case 'set': x.tract_set(m.param, m.value); break;
      case 'preset': x.tract_preset(m.id); break;
      case 'gesture': x.tract_gesture(m.id); break;
      case 'impact': x.tract_impact(m.position, m.energy); break;
      default: break;
    }
  }

  process(_inputs, outputs) {
    const out = outputs[0];
    const frames = out[0].length;
    if (!this.ready) {
      for (const ch of out) ch.fill(0);
      return true;
    }
    const t0 = this.clock();
    const x = this.x;
    const n = x.tract_process(frames);
    const mono = new Float32Array(x.memory.buffer, x.tract_out_ptr(), n);
    for (const ch of out) ch.set(mono);
    this.blocks++;
    if (this.blocks % SNAPSHOT_EVERY === 0) {
      x.tract_snapshot();
      const state = new Float32Array(x.memory.buffer, x.tract_state_ptr(), x.tract_state_len()).slice();
      this.port.postMessage({ type: 'state', state, load: this.load }, [state.buffer]);
    }
    this.busyMs += this.clock() - t0;
    this.audioMs += (1000 * frames) / sampleRate;
    if (this.audioMs >= 500) {
      this.load = this.busyMs / this.audioMs;
      this.busyMs = 0;
      this.audioMs = 0;
    }
    return true;
  }
}

registerProcessor('tract', TractProcessor);
